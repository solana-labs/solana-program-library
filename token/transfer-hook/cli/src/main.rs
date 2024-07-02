pub mod meta;

use {
    crate::meta::parse_transfer_hook_account_arg,
    clap::{crate_description, crate_name, crate_version, Arg, ArgAction, Command},
    solana_clap_v3_utils::{
        input_parsers::{
            parse_url_or_moniker,
            signer::{SignerSource, SignerSourceParserBuilder},
        },
        input_validators::normalize_to_url_if_moniker,
        keypair::signer_from_path,
    },
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_remote_wallet::remote_wallet::RemoteWalletManager,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        instruction::Instruction,
        pubkey::Pubkey,
        signature::{Signature, Signer},
        system_instruction, system_program,
        transaction::Transaction,
    },
    spl_tlv_account_resolution::{account::ExtraAccountMeta, state::ExtraAccountMetaList},
    spl_transfer_hook_interface::{
        get_extra_account_metas_address,
        instruction::{initialize_extra_account_meta_list, update_extra_account_meta_list},
    },
    std::{process::exit, rc::Rc},
};

// Helper function to calculate the required lamports for rent
async fn calculate_rent_lamports(
    rpc_client: &RpcClient,
    account_address: &Pubkey,
    account_size: usize,
) -> Result<u64, Box<dyn std::error::Error>> {
    let required_lamports = rpc_client
        .get_minimum_balance_for_rent_exemption(account_size)
        .await
        .map_err(|err| format!("error: unable to fetch rent-exemption: {err}"))?;
    let account_info = rpc_client.get_account(account_address).await;
    let current_lamports = account_info.map(|a| a.lamports).unwrap_or(0);
    Ok(required_lamports.saturating_sub(current_lamports))
}

async fn build_transaction_with_rent_transfer(
    rpc_client: &RpcClient,
    payer: &dyn Signer,
    extra_account_metas_address: &Pubkey,
    extra_account_metas: &[ExtraAccountMeta],
    instruction: Instruction,
) -> Result<Transaction, Box<dyn std::error::Error>> {
    let account_size = ExtraAccountMetaList::size_of(extra_account_metas.len())?;
    let transfer_lamports =
        calculate_rent_lamports(rpc_client, extra_account_metas_address, account_size).await?;

    let mut instructions = vec![];
    if transfer_lamports > 0 {
        instructions.push(system_instruction::transfer(
            &payer.pubkey(),
            extra_account_metas_address,
            transfer_lamports,
        ));
    }

    instructions.push(instruction);

    let transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));

    Ok(transaction)
}

async fn sign_and_send_transaction(
    transaction: &mut Transaction,
    rpc_client: &RpcClient,
    payer: &dyn Signer,
    mint_authority: &dyn Signer,
) -> Result<Signature, Box<dyn std::error::Error>> {
    let mut signers = vec![payer];
    if payer.pubkey() != mint_authority.pubkey() {
        signers.push(mint_authority);
    }

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|err| format!("error: unable to get latest blockhash: {err}"))?;

    transaction
        .try_sign(&signers, blockhash)
        .map_err(|err| format!("error: failed to sign transaction: {err}"))?;

    rpc_client
        .send_and_confirm_transaction_with_spinner(transaction)
        .await
        .map_err(|err| format!("error: send transaction: {err}").into())
}

struct Config {
    commitment_config: CommitmentConfig,
    default_signer: Box<dyn Signer>,
    json_rpc_url: String,
    verbose: bool,
}

async fn process_create_extra_account_metas(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    token: &Pubkey,
    extra_account_metas: Vec<ExtraAccountMeta>,
    mint_authority: &dyn Signer,
    payer: &dyn Signer,
) -> Result<Signature, Box<dyn std::error::Error>> {
    let extra_account_metas_address = get_extra_account_metas_address(token, program_id);

    // Check if the extra meta account has already been initialized
    let extra_account_metas_account = rpc_client.get_account(&extra_account_metas_address).await;
    if let Ok(account) = &extra_account_metas_account {
        if account.owner != system_program::id() {
            return Err(format!("error: extra account metas for mint {token} and program {program_id} already exists").into());
        }
    }

    let instruction = initialize_extra_account_meta_list(
        program_id,
        &extra_account_metas_address,
        token,
        &mint_authority.pubkey(),
        &extra_account_metas,
    );

    let mut transaction = build_transaction_with_rent_transfer(
        rpc_client,
        payer,
        &extra_account_metas_address,
        &extra_account_metas,
        instruction,
    )
    .await?;

    sign_and_send_transaction(&mut transaction, rpc_client, payer, mint_authority).await
}

async fn process_update_extra_account_metas(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    token: &Pubkey,
    extra_account_metas: Vec<ExtraAccountMeta>,
    mint_authority: &dyn Signer,
    payer: &dyn Signer,
) -> Result<Signature, Box<dyn std::error::Error>> {
    let extra_account_metas_address = get_extra_account_metas_address(token, program_id);

    // Check if the extra meta account has been initialized first
    let extra_account_metas_account = rpc_client.get_account(&extra_account_metas_address).await;
    if extra_account_metas_account.is_err() {
        return Err(format!(
            "error: extra account metas for mint {token} and program {program_id} does not exist"
        )
        .into());
    }

    let instruction = update_extra_account_meta_list(
        program_id,
        &extra_account_metas_address,
        token,
        &mint_authority.pubkey(),
        &extra_account_metas,
    );

    let mut transaction = build_transaction_with_rent_transfer(
        rpc_client,
        payer,
        &extra_account_metas_address,
        &extra_account_metas,
        instruction,
    )
    .await?;

    sign_and_send_transaction(&mut transaction, rpc_client, payer, mint_authority).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
            Arg::new("fee_payer")
                .long("fee-payer")
                .value_name("KEYPAIR")
                .value_parser(SignerSourceParserBuilder::default().allow_all().build())
                .takes_value(true)
                .global(true)
                .help("Filepath or URL to a keypair to pay transaction fee [default: client keypair]"),
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
            Command::new("create-extra-metas")
                .about("Create the extra account metas account for a transfer hook program")
                .arg(
                    Arg::new("program_id")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().allow_file_path().build())
                        .value_name("TRANSFER_HOOK_PROGRAM")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The transfer hook program id"),
                )
                .arg(
                    Arg::new("token")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().allow_file_path().build())
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("The token mint address for the transfer hook"),
                )
                .arg(
                    Arg::new("transfer_hook_accounts")
                        .value_parser(parse_transfer_hook_account_arg)
                        .value_name("TRANSFER_HOOK_ACCOUNTS")
                        .takes_value(true)
                        .action(ArgAction::Append)
                        .min_values(0)
                        .index(3)
                        .help(r#"Additional account(s) required for a transfer hook and their respective configurations, whether they are a fixed address or PDA.

Additional accounts with known fixed addresses can be passed at the command line in the format "<PUBKEY>:<ROLE>". The role must be "readonly", "writable". "readonlySigner", or "writableSigner".

Additional accounts requiring seed configurations can be defined in a configuration file using either JSON or YAML. The format is as follows:

```json
{
    "extraMetas": [
        {
            "pubkey": "39UhV...",
            "role": "readonlySigner"
        },
        {
            "seeds": [
                {
                    "literal": {
                        "bytes": [1, 2, 3, 4, 5, 6]
                    }
                },
                {
                    "accountKey": {
                        "index": 0
                    }
                }
            ],
            "role": "writable"
        }
    ]
}
```

```yaml
extraMetas:
  - pubkey: "39UhV..."
      role: "readonlySigner"
  - seeds:
      - literal:
          bytes: [1, 2, 3, 4, 5, 6]
      - accountKey:
          index: 0
      role: "writable"
```
"#)
                )
                .arg(
                    Arg::new("mint_authority")
                        .long("mint-authority")
                        .value_name("KEYPAIR")
                        .value_parser(SignerSourceParserBuilder::default().allow_all().build())
                        .takes_value(true)
                        .global(true)
                        .help("Filepath or URL to mint-authority keypair [default: client keypair]"),
                )
        )
        .subcommand(
            Command::new("update-extra-metas")
                .about("Update the extra account metas account for a transfer hook program")
                .arg(
                    Arg::new("program_id")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().allow_file_path().build())
                        .value_name("TRANSFER_HOOK_PROGRAM")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The transfer hook program id"),
                )
                .arg(
                    Arg::new("token")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().allow_file_path().build())
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("The token mint address for the transfer hook"),
                )
                .arg(
                    Arg::new("transfer_hook_accounts")
                        .value_parser(parse_transfer_hook_account_arg)
                        .value_name("TRANSFER_HOOK_ACCOUNTS")
                        .takes_value(true)
                        .action(ArgAction::Append)
                        .min_values(0)
                        .index(3)
                        .help(r#"Additional account(s) required for a transfer hook and their respective configurations, whether they are a fixed address or PDA.

Additional accounts with known fixed addresses can be passed at the command line in the format "<PUBKEY>:<ROLE>". The role must be "readonly", "writable". "readonlySigner", or "writableSigner".

Additional accounts requiring seed configurations can be defined in a configuration file using either JSON or YAML. The format is as follows:

```json
{
    "extraMetas": [
        {
            "pubkey": "39UhV...",
            "role": "readonlySigner"
        },
        {
            "seeds": [
                {
                    "literal": {
                        "bytes": [1, 2, 3, 4, 5, 6]
                    }
                },
                {
                    "accountKey": {
                        "index": 0
                    }
                }
            ],
            "role": "writable"
        }
    ]
}
```

```yaml
extraMetas:
  - pubkey: "39UhV..."
      role: "readonlySigner"
  - seeds:
      - literal:
          bytes: [1, 2, 3, 4, 5, 6]
      - accountKey:
          index: 0
      role: "writable"
```
"#)
                )
                .arg(
                    Arg::new("mint_authority")
                        .long("mint-authority")
                        .value_name("KEYPAIR")
                        .value_parser(SignerSourceParserBuilder::default().allow_all().build())
                        .takes_value(true)
                        .global(true)
                        .help("Filepath or URL to mint-authority keypair [default: client keypair]"),
                )
        ).get_matches();

    let (command, matches) = app_matches.subcommand().unwrap();
    let mut wallet_manager: Option<Rc<RemoteWalletManager>> = None;

    let cli_config = if let Some(config_file) = matches.get_one::<String>("config_file") {
        solana_cli_config::Config::load(config_file).unwrap_or_default()
    } else {
        solana_cli_config::Config::default()
    };

    let config = {
        let default_signer = if let Some((signer, _)) =
            SignerSource::try_get_signer(matches, "fee_payer", &mut wallet_manager)?
        {
            signer
        } else {
            signer_from_path(
                matches,
                &cli_config.keypair_path,
                "fee_payer",
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
            default_signer,
            json_rpc_url,
            verbose: matches.try_contains_id("verbose")?,
        }
    };
    solana_logger::setup_with_default("solana=info");

    if config.verbose {
        println!("JSON RPC URL: {}", config.json_rpc_url);
    }
    let rpc_client =
        RpcClient::new_with_commitment(config.json_rpc_url.clone(), config.commitment_config);

    match (command, matches) {
        ("create-extra-metas", arg_matches) => {
            let program_id =
                SignerSource::try_get_pubkey(arg_matches, "program_id", &mut wallet_manager)?
                    .unwrap();
            let token =
                SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)?.unwrap();

            let transfer_hook_accounts = arg_matches
                .get_many::<Vec<ExtraAccountMeta>>("transfer_hook_accounts")
                .unwrap_or_default()
                .flatten()
                .cloned()
                .collect();
            let mint_authority = if let Some((signer, _)) =
                SignerSource::try_get_signer(matches, "mint_authority", &mut wallet_manager)?
            {
                signer
            } else {
                signer_from_path(
                    matches,
                    &cli_config.keypair_path,
                    "mint_authority",
                    &mut wallet_manager,
                )?
            };
            let signature = process_create_extra_account_metas(
                &rpc_client,
                &program_id,
                &token,
                transfer_hook_accounts,
                mint_authority.as_ref(),
                config.default_signer.as_ref(),
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: send transaction: {err}");
                exit(1);
            });
            println!("Signature: {signature}");
        }
        ("update-extra-metas", arg_matches) => {
            let program_id =
                SignerSource::try_get_pubkey(arg_matches, "program_id", &mut wallet_manager)?
                    .unwrap();
            let token =
                SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)?.unwrap();

            let transfer_hook_accounts = arg_matches
                .get_many::<Vec<ExtraAccountMeta>>("transfer_hook_accounts")
                .unwrap_or_default()
                .flatten()
                .cloned()
                .collect();
            let mint_authority = if let Some((signer, _)) =
                SignerSource::try_get_signer(matches, "mint_authority", &mut wallet_manager)?
            {
                signer
            } else {
                signer_from_path(
                    matches,
                    &cli_config.keypair_path,
                    "mint_authority",
                    &mut wallet_manager,
                )?
            };
            let signature = process_update_extra_account_metas(
                &rpc_client,
                &program_id,
                &token,
                transfer_hook_accounts,
                mint_authority.as_ref(),
                config.default_signer.as_ref(),
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: send transaction: {err}");
                exit(1);
            });
            println!("Signature: {signature}");
        }
        _ => unreachable!(),
    };

    Ok(())
}

#[cfg(test)]
mod test {
    use {
        super::*,
        solana_sdk::{
            account::Account, bpf_loader_upgradeable, instruction::AccountMeta,
            program_option::COption, signer::keypair::Keypair,
        },
        solana_test_validator::{TestValidator, TestValidatorGenesis, UpgradeableProgramInfo},
        spl_token_2022::{
            extension::{ExtensionType, StateWithExtensionsMut},
            state::Mint,
        },
        spl_token_client::{
            client::{ProgramRpcClient, ProgramRpcClientSendTransaction},
            token::Token,
        },
        std::{path::PathBuf, sync::Arc},
    };

    async fn new_validator_for_test(
        program_id: Pubkey,
        mint_authority: &Pubkey,
        decimals: u8,
    ) -> (TestValidator, Keypair) {
        solana_logger::setup();
        let mut test_validator_genesis = TestValidatorGenesis::default();
        test_validator_genesis.add_upgradeable_programs_with_path(&[UpgradeableProgramInfo {
            program_id,
            loader: bpf_loader_upgradeable::id(),
            program_path: PathBuf::from("../../../target/deploy/spl_transfer_hook_example.so"),
            upgrade_authority: Pubkey::new_unique(),
        }]);

        let mint_size = ExtensionType::try_calculate_account_len::<Mint>(&[]).unwrap();
        let mut mint_data = vec![0; mint_size];
        let mut state =
            StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut mint_data).unwrap();
        let token_amount = 1_000_000_000_000;
        state.base = Mint {
            mint_authority: COption::Some(*mint_authority),
            supply: token_amount,
            decimals,
            is_initialized: true,
            freeze_authority: COption::None,
        };
        state.pack_base();
        test_validator_genesis.add_account(
            spl_transfer_hook_example::mint::id(),
            Account {
                lamports: 1_000_000_000,
                data: mint_data,
                owner: spl_token_2022::id(),
                ..Account::default()
            }
            .into(),
        );
        test_validator_genesis.start_async().await
    }

    #[tokio::test]
    async fn test_create() {
        let program_id = Pubkey::new_unique();

        let decimals = 2;
        let mint_authority = Keypair::new();
        let (test_validator, payer) =
            new_validator_for_test(program_id, &mint_authority.pubkey(), decimals).await;
        let payer: Arc<dyn Signer> = Arc::new(payer);
        let rpc_client = Arc::new(test_validator.get_async_rpc_client());
        let client = Arc::new(ProgramRpcClient::new(
            rpc_client.clone(),
            ProgramRpcClientSendTransaction,
        ));

        let token = Token::new(
            client.clone(),
            &spl_token_2022::id(),
            &spl_transfer_hook_example::mint::id(),
            Some(decimals),
            payer.clone(),
        );

        let required_address = Pubkey::new_unique();
        let accounts = [AccountMeta::new_readonly(required_address, false)];
        process_create_extra_account_metas(
            &rpc_client,
            &program_id,
            token.get_address(),
            accounts.iter().map(|a| a.into()).collect(),
            &mint_authority,
            payer.as_ref(),
        )
        .await
        .unwrap();

        let extra_account_metas_address =
            get_extra_account_metas_address(token.get_address(), &program_id);
        let account = rpc_client
            .get_account(&extra_account_metas_address)
            .await
            .unwrap();
        assert_eq!(account.owner, program_id);
    }
}
