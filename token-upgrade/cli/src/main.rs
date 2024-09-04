use {
    clap::{crate_description, crate_name, crate_version, Arg, ArgAction, Command},
    solana_clap_v3_utils::{
        input_parsers::{
            parse_url_or_moniker,
            signer::{SignerSource, SignerSourceParserBuilder},
        },
        input_validators::normalize_to_url_if_moniker,
        keypair::{signer_from_path, signer_from_source, SignerFromPathConfig},
    },
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_remote_wallet::remote_wallet::RemoteWalletManager,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        message::Message,
        program_option::COption,
        pubkey::Pubkey,
        signature::{Signature, Signer},
        transaction::Transaction,
    },
    spl_associated_token_account_client::address::get_associated_token_address_with_program_id,
    spl_token_2022::{
        extension::StateWithExtensions,
        state::{Account, Mint},
    },
    spl_token_client::{
        client::{ProgramRpcClient, ProgramRpcClientSendTransaction, RpcClientResponse},
        token::Token,
    },
    spl_token_upgrade::{get_token_upgrade_authority_address, instruction::exchange},
    std::{error::Error, process::exit, rc::Rc, sync::Arc},
};

struct Config {
    commitment_config: CommitmentConfig,
    payer: Arc<dyn Signer>,
    json_rpc_url: String,
    verbose: bool,
}

async fn get_mint_owner_checked(
    rpc_client: &RpcClient,
    mint: &Pubkey,
) -> Result<Pubkey, Box<dyn Error>> {
    let mint_account = rpc_client.get_account(mint).await?;
    let _ = StateWithExtensions::<Mint>::unpack(&mint_account.data)
        .map_err(|_| format!("Account {} is not a valid mint", mint))?;
    Ok(mint_account.owner)
}

async fn escrow_exists_checked(
    rpc_client: &RpcClient,
    escrow: &Pubkey,
    escrow_authority: &Pubkey,
    mint: &Pubkey,
) -> Result<bool, Box<dyn Error>> {
    if let Ok(escrow_account) = rpc_client.get_account(escrow).await {
        let account_state = StateWithExtensions::<Account>::unpack(&escrow_account.data)
            .map_err(|_| format!("Account {} is not a valid account", escrow))?;
        if account_state.base.mint != *mint {
            Err(format!(
                "Escrow account is for mint {}, need an account for mint {}",
                account_state.base.mint, mint
            )
            .into())
        } else if account_state.base.owner != *escrow_authority
            && account_state.base.delegate != COption::Some(*escrow_authority)
        {
            Err(format!("Escrow account {} is neither owned by nor delegated to escrow authority {}, please provide another account", escrow, escrow_authority).into())
        } else {
            Ok(true)
        }
    } else {
        Ok(false)
    }
}

async fn process_create_escrow_account(
    rpc_client: &Arc<RpcClient>,
    payer: &Arc<dyn Signer>,
    original_mint: &Pubkey,
    new_mint: &Pubkey,
    account_keypair: Option<&dyn Signer>,
) -> Result<RpcClientResponse, Box<dyn Error>> {
    let _ = get_mint_owner_checked(rpc_client, original_mint).await?;
    let new_program_id = get_mint_owner_checked(rpc_client, new_mint).await?;
    let escrow_authority =
        get_token_upgrade_authority_address(original_mint, new_mint, &spl_token_upgrade::id());

    let program_client = Arc::new(ProgramRpcClient::new(
        rpc_client.clone(),
        ProgramRpcClientSendTransaction,
    ));
    let token = Token::new(
        program_client.clone(),
        &new_program_id,
        new_mint,
        None,
        payer.clone(),
    );

    let escrow = account_keypair
        .map(|k| k.pubkey())
        .unwrap_or_else(|| token.get_associated_token_address(&escrow_authority));

    if escrow_exists_checked(rpc_client, &escrow, &escrow_authority, new_mint).await? {
        return Err(format!(
            "Escrow account {} already exists, not doing anything",
            escrow
        )
        .into());
    }

    println!(
        "Creating escrow account {} owned by escrow authority {}",
        escrow, escrow_authority
    );
    if let Some(keypair) = account_keypair {
        token
            .create_auxiliary_token_account(keypair, &escrow_authority)
            .await
            .map_err(|e| e.into())
    } else {
        token
            .create_associated_token_account(&escrow_authority)
            .await
            .map_err(|e| e.into())
    }
}

#[allow(clippy::too_many_arguments)]
async fn process_exchange(
    rpc_client: &Arc<RpcClient>,
    payer: &Arc<dyn Signer>,
    original_mint: &Pubkey,
    new_mint: &Pubkey,
    owner: &Arc<dyn Signer>,
    burn_from: Option<Pubkey>,
    escrow: Option<Pubkey>,
    destination: Option<Pubkey>,
    multisig_pubkeys: &[Pubkey],
    bulk_signers: Vec<Arc<dyn Signer>>,
) -> Result<Signature, Box<dyn Error>> {
    let original_program_id = get_mint_owner_checked(rpc_client, original_mint).await?;
    let new_program_id = get_mint_owner_checked(rpc_client, new_mint).await?;
    let escrow_authority =
        get_token_upgrade_authority_address(original_mint, new_mint, &spl_token_upgrade::id());

    let burn_from = burn_from.unwrap_or_else(|| {
        get_associated_token_address_with_program_id(
            &owner.pubkey(),
            original_mint,
            &original_program_id,
        )
    });

    let escrow = escrow.unwrap_or_else(|| {
        get_associated_token_address_with_program_id(&escrow_authority, new_mint, &new_program_id)
    });

    let destination = destination.unwrap_or_else(|| {
        get_associated_token_address_with_program_id(&owner.pubkey(), new_mint, &new_program_id)
    });

    println!(
        "Burning tokens from account {}, receiving tokens into account {}",
        burn_from, destination
    );
    let mut transaction = Transaction::new_unsigned(Message::new(
        &[exchange(
            &spl_token_upgrade::id(),
            &burn_from,
            original_mint,
            &escrow,
            &destination,
            new_mint,
            &original_program_id,
            &new_program_id,
            &owner.pubkey(),
            &multisig_pubkeys.iter().collect::<Vec<_>>(),
        )],
        Some(&payer.pubkey()),
    ));

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|err| format!("error: unable to get latest blockhash: {}", err))?;

    transaction
        .try_sign(&bulk_signers, blockhash)
        .map_err(|err| format!("error: failed to sign transaction: {}", err))?;

    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .await
        .map_err(|err| format!("error: send transaction: {}", err))?;

    Ok(signature)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let app_matches = Command::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg({
            let arg = Arg::new("config_file")
                .short('C')
                .long("config")
                .value_name("PATH")
                .takes_value(true)
                .global(true)
                .help("Configuration file to use");
            if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
                arg.default_value(config_file)
            } else {
                arg
            }
        })
        .arg(
            Arg::new("payer")
                .long("payer")
                .value_name("KEYPAIR")
                .value_parser(SignerSourceParserBuilder::default().allow_all().build())
                .takes_value(true)
                .global(true)
                .help("Filepath or URL to a keypair [default: client keypair]"),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .takes_value(false)
                .global(true)
                .help("Show additional information"),
        )
        .arg(
            Arg::new("json_rpc_url")
                .short('u')
                .long("url")
                .value_name("URL")
                .takes_value(true)
                .global(true)
                .value_parser(parse_url_or_moniker)
                .help("JSON RPC URL for the cluster [default: value from configuration file]"),
        )
        .subcommand(
            Command::new("create-escrow").about("Create token account for the program escrow")
            .arg(
                Arg::new("original_mint")
                .value_parser(SignerSourceParserBuilder::default().allow_all().build())
                    .value_name("ADDRESS")
                    .required(true)
                    .takes_value(true)
                    .index(1)
                    .help("Original mint address, whose tokens will be burned")
            )
            .arg(
                Arg::new("new_mint")
                .value_parser(SignerSourceParserBuilder::default().allow_all().build())
                    .value_name("ADDRESS")
                    .required(true)
                    .takes_value(true)
                    .index(2)
                    .help("New mint address, whose tokens will be transferred to users")
            )
            .arg(
                Arg::new("account_keypair")
                    .value_name("ACCOUNT_KEYPAIR")
                    .value_parser(SignerSourceParserBuilder::default().allow_all().build())
                    .takes_value(true)
                    .index(3)
                    .help("Specify the account keypair. This may be a keypair file or the ASK keyword. [default: associated token account for escrow authority]"),
            )
        )
        .subcommand(
            Command::new("exchange").about("Exchange original tokens for new tokens")
            .arg(
                Arg::new("original_mint")
                .value_parser(SignerSourceParserBuilder::default().allow_all().build())
                    .value_name("ADDRESS")
                    .required(true)
                    .takes_value(true)
                    .index(1)
                    .help("Original mint address, whose tokens will be burned")
            )
            .arg(
                Arg::new("new_mint")
                .value_parser(SignerSourceParserBuilder::default().allow_all().build())
                    .value_name("ADDRESS")
                    .required(true)
                    .takes_value(true)
                    .index(2)
                    .help("New mint address, whose tokens will be transferred to users")
            )
            .arg(
                Arg::new("owner")
                    .long("owner")
                    .value_name("OWNER_KEYPAIR")
                    .value_parser(SignerSourceParserBuilder::default().allow_all().build())
                    .takes_value(true)
                    .help("Specify the owner or delegate for the burnt account. This may be a keypair file or the ASK keyword. [default: fee payer]"),
            )
            .arg(
                Arg::new("burn_from")
                    .long("burn-from")
                    .value_name("BURN_TOKEN_ACCOUNT_ADDRESS")
                    .value_parser(SignerSourceParserBuilder::default().allow_all().build())
                    .takes_value(true)
                    .help("Specify the burnt account address. [default: associated token account for owner on original mint]"),
            )
            .arg(
                Arg::new("escrow")
                    .long("escrow")
                    .value_name("ESCROW_TOKEN_ACCOUNT_ADDRESS")
                    .value_parser(SignerSourceParserBuilder::default().allow_all().build())
                    .takes_value(true)
                    .help("Specify the escrow account address to transfer from. [default: associated token account for the escrow authority on new mint]"),
            )
            .arg(
                Arg::new("destination")
                    .long("destination")
                    .value_name("DESTINATION_ACCOUNT_ADDRESS")
                    .value_parser(SignerSourceParserBuilder::default().allow_all().build())
                    .takes_value(true)
                    .help("Specify the destination account to receive new tokens. [default: associated token account for owner on new mint]"),
            )
            .arg(
                Arg::new("multisig_signer")
                    .long("multisig-signer")
                    .value_parser(SignerSourceParserBuilder::default().allow_all().build())
                    .value_name("MULTISIG_SIGNER")
                    .takes_value(true)
                    .action(ArgAction::Append)
                    .min_values(0)
                    .max_values(spl_token_2022::instruction::MAX_SIGNERS)
                    .help("Member signer of a multisig account")
            )
        )
        .get_matches();

    let (command, matches) = app_matches.subcommand().unwrap();
    let mut wallet_manager: Option<Rc<RemoteWalletManager>> = None;

    let config = {
        let cli_config = if let Some(config_file) = matches.try_get_one::<String>("config_file")? {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };

        let payer = if let Ok(Some((signer, _))) =
            SignerSource::try_get_signer(matches, "payer", &mut wallet_manager)
        {
            Box::new(signer)
        } else {
            signer_from_path(
                matches,
                &cli_config.keypair_path,
                "payer",
                &mut wallet_manager,
            )?
        };

        let json_rpc_url = normalize_to_url_if_moniker(
            matches
                .get_one::<String>("json_rpc_url")
                .unwrap_or(&cli_config.json_rpc_url),
        );

        Config {
            commitment_config: CommitmentConfig::confirmed(),
            payer: Arc::from(payer),
            json_rpc_url,
            verbose: matches.try_contains_id("verbose")?,
        }
    };
    solana_logger::setup_with_default("solana=info");

    if config.verbose {
        println!("JSON RPC URL: {}", config.json_rpc_url);
    }
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        config.json_rpc_url.clone(),
        config.commitment_config,
    ));

    match (command, matches) {
        ("create-escrow", arg_matches) => {
            let original_mint =
                SignerSource::try_get_pubkey(arg_matches, "original_mint", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let new_mint =
                SignerSource::try_get_pubkey(arg_matches, "new_mint", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let account_keypair =
                SignerSource::try_get_signer(matches, "account_keypair", &mut wallet_manager)?
                    .map(|(signer, _)| signer);
            let response = process_create_escrow_account(
                &rpc_client,
                &config.payer,
                &original_mint,
                &new_mint,
                account_keypair.as_ref().map(|k| k.as_ref()),
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: create escrow: {}", err);
                exit(1);
            });
            println!("{}", response);
        }
        ("exchange", arg_matches) => {
            let mut bulk_signers = vec![config.payer.clone()];
            let mut multisig_pubkeys = vec![];

            if let Some(sources) = arg_matches.try_get_many::<SignerSource>("multisig_signer")? {
                for (i, source) in sources.enumerate() {
                    let name = format!("{}-{}", "multisig_signer", i.saturating_add(1));
                    let signer =
                        signer_from_source(arg_matches, source, &name, &mut wallet_manager)
                            .unwrap_or_else(|e| {
                                eprint!("error parsing multisig signer: {}", e);
                                exit(1);
                            });
                    let signer_pubkey = signer.pubkey();
                    let signer = Arc::from(signer);
                    if !bulk_signers.contains(&signer) {
                        bulk_signers.push(signer);
                    }
                    if !multisig_pubkeys.contains(&signer_pubkey) {
                        multisig_pubkeys.push(signer_pubkey);
                    }
                }
            }

            let original_mint =
                SignerSource::try_get_pubkey(arg_matches, "original_mint", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let new_mint =
                SignerSource::try_get_pubkey(arg_matches, "new_mint", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let signer_config = SignerFromPathConfig {
                allow_null_signer: !multisig_pubkeys.is_empty(),
            };
            let owner = if let Ok(Some((signer, _))) =
                SignerSource::try_get_signer(matches, "owner", &mut wallet_manager)
            {
                Arc::from(signer)
            } else {
                config.payer.clone()
            };
            if !signer_config.allow_null_signer && !bulk_signers.contains(&owner) {
                bulk_signers.push(owner.clone());
            }
            let burn_from =
                SignerSource::try_get_pubkey(arg_matches, "burn_from", &mut wallet_manager)
                    .unwrap();
            let escrow =
                SignerSource::try_get_pubkey(arg_matches, "escrow", &mut wallet_manager).unwrap();
            let destination =
                SignerSource::try_get_pubkey(arg_matches, "destination", &mut wallet_manager)
                    .unwrap();

            let signature = process_exchange(
                &rpc_client,
                &config.payer,
                &original_mint,
                &new_mint,
                &owner,
                burn_from,
                escrow,
                destination,
                &multisig_pubkeys,
                bulk_signers,
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: send transaction: {}", err);
                exit(1);
            });
            println!("Signature: {}", signature);
        }
        _ => unreachable!(),
    };

    Ok(())
}

#[cfg(test)]
mod test {
    use {
        super::*,
        solana_sdk::{bpf_loader_upgradeable, signer::keypair::Keypair},
        solana_test_validator::{TestValidator, TestValidatorGenesis, UpgradeableProgramInfo},
        spl_token_client::client::{ProgramClient, SendTransaction, SimulateTransaction},
        std::path::PathBuf,
    };

    async fn new_validator_for_test() -> (TestValidator, Keypair) {
        solana_logger::setup();
        let mut test_validator_genesis = TestValidatorGenesis::default();
        test_validator_genesis.add_upgradeable_programs_with_path(&[UpgradeableProgramInfo {
            program_id: spl_token_upgrade::id(),
            loader: bpf_loader_upgradeable::id(),
            program_path: PathBuf::from("../../target/deploy/spl_token_upgrade.so"),
            upgrade_authority: Pubkey::new_unique(),
        }]);
        test_validator_genesis.start_async().await
    }

    async fn setup_mint<T: SendTransaction + SimulateTransaction>(
        program_id: &Pubkey,
        mint_authority: &Pubkey,
        decimals: u8,
        payer: Arc<dyn Signer>,
        client: Arc<dyn ProgramClient<T>>,
    ) -> Token<T> {
        let mint_account = Keypair::new();
        let token = Token::new(
            client,
            program_id,
            &mint_account.pubkey(),
            Some(decimals),
            payer,
        );
        token
            .create_mint(mint_authority, None, vec![], &[&mint_account])
            .await
            .unwrap();
        token
    }

    #[tokio::test]
    async fn success_create_escrow() {
        let (test_validator, payer) = new_validator_for_test().await;
        let payer: Arc<dyn Signer> = Arc::new(payer);
        let rpc_client = Arc::new(test_validator.get_async_rpc_client());
        let client = Arc::new(ProgramRpcClient::new(
            rpc_client.clone(),
            ProgramRpcClientSendTransaction,
        ));

        let mint_authority = Keypair::new();
        let decimals = 2;

        let original_token = setup_mint(
            &spl_token::id(),
            &mint_authority.pubkey(),
            decimals,
            payer.clone(),
            client.clone(),
        )
        .await;
        let new_token = setup_mint(
            &spl_token_2022::id(),
            &mint_authority.pubkey(),
            decimals,
            payer.clone(),
            client.clone(),
        )
        .await;

        let account_keypair = Keypair::new();
        assert!(process_create_escrow_account(
            &rpc_client,
            &payer,
            original_token.get_address(),
            new_token.get_address(),
            Some(&account_keypair)
        )
        .await
        .is_ok());
        let escrow_authority = get_token_upgrade_authority_address(
            original_token.get_address(),
            new_token.get_address(),
            &spl_token_upgrade::id(),
        );
        let escrow = new_token
            .get_account_info(&account_keypair.pubkey())
            .await
            .unwrap();
        assert_eq!(escrow.base.owner, escrow_authority);
        assert_eq!(&escrow.base.mint, new_token.get_address());

        assert!(process_create_escrow_account(
            &rpc_client,
            &payer,
            original_token.get_address(),
            new_token.get_address(),
            None
        )
        .await
        .is_ok());
        let escrow = new_token
            .get_account_info(&new_token.get_associated_token_address(&escrow_authority))
            .await
            .unwrap();
        assert_eq!(escrow.base.owner, escrow_authority);
        assert_eq!(&escrow.base.mint, new_token.get_address());
    }

    #[tokio::test]
    async fn success_exchange_associated_accounts() {
        let (test_validator, payer) = new_validator_for_test().await;
        let payer: Arc<dyn Signer> = Arc::new(payer);
        let rpc_client = Arc::new(test_validator.get_async_rpc_client());
        let client = Arc::new(ProgramRpcClient::new(
            rpc_client.clone(),
            ProgramRpcClientSendTransaction,
        ));

        let mint_authority = Keypair::new();
        let decimals = 2;

        let original_token = setup_mint(
            &spl_token::id(),
            &mint_authority.pubkey(),
            decimals,
            payer.clone(),
            client.clone(),
        )
        .await;
        let new_token = setup_mint(
            &spl_token_2022::id(),
            &mint_authority.pubkey(),
            decimals,
            payer.clone(),
            client.clone(),
        )
        .await;

        process_create_escrow_account(
            &rpc_client,
            &payer,
            original_token.get_address(),
            new_token.get_address(),
            None,
        )
        .await
        .unwrap();

        let user = Keypair::new();
        let amount = 1_000_000_000_000;
        original_token
            .create_associated_token_account(&user.pubkey())
            .await
            .unwrap();
        let burn_from = original_token.get_associated_token_address(&user.pubkey());

        original_token
            .mint_to(
                &burn_from,
                &mint_authority.pubkey(),
                amount,
                &[&mint_authority],
            )
            .await
            .unwrap();

        // mint tokens to the escrow
        let escrow_authority = get_token_upgrade_authority_address(
            original_token.get_address(),
            new_token.get_address(),
            &spl_token_upgrade::id(),
        );
        let escrow = new_token.get_associated_token_address(&escrow_authority);
        new_token
            .mint_to(
                &escrow,
                &mint_authority.pubkey(),
                amount,
                &[&mint_authority],
            )
            .await
            .unwrap();

        new_token
            .create_associated_token_account(&user.pubkey())
            .await
            .unwrap();
        let destination = new_token.get_associated_token_address(&user.pubkey());

        let user: Arc<dyn Signer> = Arc::new(user);
        process_exchange(
            &rpc_client,
            &payer,
            original_token.get_address(),
            new_token.get_address(),
            &user,
            None,
            None,
            None,
            &[],
            vec![payer.clone(), user.clone()],
        )
        .await
        .unwrap();

        let burn_account = original_token.get_account_info(&burn_from).await.unwrap();
        assert_eq!(burn_account.base.amount, 0);

        let escrow_account = new_token.get_account_info(&escrow).await.unwrap();
        assert_eq!(escrow_account.base.amount, 0);

        let destination_account = new_token.get_account_info(&destination).await.unwrap();
        assert_eq!(destination_account.base.amount, amount);
    }

    #[tokio::test]
    async fn success_exchange_auxiliary_accounts() {
        let (test_validator, payer) = new_validator_for_test().await;
        let payer: Arc<dyn Signer> = Arc::new(payer);
        let rpc_client = Arc::new(test_validator.get_async_rpc_client());
        let client = Arc::new(ProgramRpcClient::new(
            rpc_client.clone(),
            ProgramRpcClientSendTransaction,
        ));

        let mint_authority = Keypair::new();
        let decimals = 2;

        let original_token = setup_mint(
            &spl_token::id(),
            &mint_authority.pubkey(),
            decimals,
            payer.clone(),
            client.clone(),
        )
        .await;
        let new_token = setup_mint(
            &spl_token_2022::id(),
            &mint_authority.pubkey(),
            decimals,
            payer.clone(),
            client.clone(),
        )
        .await;

        let escrow = Keypair::new();
        process_create_escrow_account(
            &rpc_client,
            &payer,
            original_token.get_address(),
            new_token.get_address(),
            Some(&escrow),
        )
        .await
        .unwrap();
        let escrow = escrow.pubkey();

        let user = Keypair::new();
        let amount = 1_000_000_000_000;
        let burn_from = Keypair::new();
        original_token
            .create_auxiliary_token_account(&burn_from, &user.pubkey())
            .await
            .unwrap();
        let burn_from = burn_from.pubkey();

        original_token
            .mint_to(
                &burn_from,
                &mint_authority.pubkey(),
                amount,
                &[&mint_authority],
            )
            .await
            .unwrap();

        // mint tokens to the escrow
        new_token
            .mint_to(
                &escrow,
                &mint_authority.pubkey(),
                amount,
                &[&mint_authority],
            )
            .await
            .unwrap();

        let destination = Keypair::new();
        new_token
            .create_auxiliary_token_account(&destination, &user.pubkey())
            .await
            .unwrap();
        let destination = destination.pubkey();

        let user: Arc<dyn Signer> = Arc::new(user);
        process_exchange(
            &rpc_client,
            &payer,
            original_token.get_address(),
            new_token.get_address(),
            &user,
            Some(burn_from),
            Some(escrow),
            Some(destination),
            &[],
            vec![payer.clone(), user.clone()],
        )
        .await
        .unwrap();

        let burn_account = original_token.get_account_info(&burn_from).await.unwrap();
        assert_eq!(burn_account.base.amount, 0);

        let escrow_account = new_token.get_account_info(&escrow).await.unwrap();
        assert_eq!(escrow_account.base.amount, 0);

        let destination_account = new_token.get_account_info(&destination).await.unwrap();
        assert_eq!(destination_account.base.amount, amount);
    }
}
