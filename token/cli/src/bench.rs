/// The `bench` subcommand
use {
    crate::{config::Config, owner_address_arg, CommandResult, Error},
    clap::{value_t_or_exit, App, AppSettings, Arg, ArgMatches, SubCommand},
    solana_clap_utils::{
        input_parsers::pubkey_of_signer,
        input_validators::{is_amount, is_parsable, is_valid_pubkey},
    },
    solana_client::{
        nonblocking::rpc_client::RpcClient, rpc_client::RpcClient as BlockingRpcClient,
        tpu_client::TpuClient, tpu_client::TpuClientConfig,
    },
    solana_remote_wallet::remote_wallet::RemoteWalletManager,
    solana_sdk::{
        message::Message, native_token::Sol, program_pack::Pack, pubkey::Pubkey, signature::Signer,
        system_instruction,
    },
    spl_associated_token_account::*,
    spl_token_2022::{
        extension::StateWithExtensions,
        instruction,
        state::{Account, Mint},
    },
    std::{sync::Arc, time::Instant},
};

pub(crate) trait BenchSubCommand {
    fn bench_subcommand(self) -> Self;
}

impl BenchSubCommand for App<'_, '_> {
    fn bench_subcommand(self) -> Self {
        self.subcommand(
            SubCommand::with_name("bench")
                .about("Token benchmarking facilities")
                .setting(AppSettings::InferSubcommands)
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("create-accounts")
                        .about("Create multiple token accounts for benchmarking")
                        .arg(
                            Arg::with_name("token")
                                .validator(is_valid_pubkey)
                                .value_name("TOKEN_ADDRESS")
                                .takes_value(true)
                                .index(1)
                                .required(true)
                                .help("The token that the accounts will hold"),
                        )
                        .arg(
                            Arg::with_name("n")
                                .validator(is_parsable::<usize>)
                                .value_name("N")
                                .takes_value(true)
                                .index(2)
                                .required(true)
                                .help("The number of accounts to create"),
                        )
                        .arg(owner_address_arg()),
                )
                .subcommand(
                    SubCommand::with_name("close-accounts")
                        .about("Close multiple token accounts used for benchmarking")
                        .arg(
                            Arg::with_name("token")
                                .validator(is_valid_pubkey)
                                .value_name("TOKEN_ADDRESS")
                                .takes_value(true)
                                .index(1)
                                .required(true)
                                .help("The token that the accounts held"),
                        )
                        .arg(
                            Arg::with_name("n")
                                .validator(is_parsable::<usize>)
                                .value_name("N")
                                .takes_value(true)
                                .index(2)
                                .required(true)
                                .help("The number of accounts to close"),
                        )
                        .arg(owner_address_arg()),
                )
                .subcommand(
                    SubCommand::with_name("deposit-into")
                        .about("Deposit tokens into multiple accounts")
                        .arg(
                            Arg::with_name("token")
                                .validator(is_valid_pubkey)
                                .value_name("TOKEN_ADDRESS")
                                .takes_value(true)
                                .index(1)
                                .required(true)
                                .help("The token that the accounts will hold"),
                        )
                        .arg(
                            Arg::with_name("n")
                                .validator(is_parsable::<usize>)
                                .value_name("N")
                                .takes_value(true)
                                .index(2)
                                .required(true)
                                .help("The number of accounts to deposit into"),
                        )
                        .arg(
                            Arg::with_name("amount")
                                .validator(is_amount)
                                .value_name("TOKEN_AMOUNT")
                                .takes_value(true)
                                .index(3)
                                .required(true)
                                .help("Amount to deposit into each account, in tokens"),
                        )
                        .arg(
                            Arg::with_name("from")
                                .long("from")
                                .validator(is_valid_pubkey)
                                .value_name("SOURCE_TOKEN_ACCOUNT_ADDRESS")
                                .takes_value(true)
                                .help("The source token account address [default: associated token account for --owner]")
                        )
                        .arg(owner_address_arg()),
                )
                .subcommand(
                    SubCommand::with_name("withdraw-from")
                        .about("Withdraw tokens from multiple accounts")
                        .arg(
                            Arg::with_name("token")
                                .validator(is_valid_pubkey)
                                .value_name("TOKEN_ADDRESS")
                                .takes_value(true)
                                .index(1)
                                .required(true)
                                .help("The token that the accounts hold"),
                        )
                        .arg(
                            Arg::with_name("n")
                                .validator(is_parsable::<usize>)
                                .value_name("N")
                                .takes_value(true)
                                .index(2)
                                .required(true)
                                .help("The number of accounts to withdraw from"),
                        )
                        .arg(
                            Arg::with_name("amount")
                                .validator(is_amount)
                                .value_name("TOKEN_AMOUNT")
                                .takes_value(true)
                                .index(3)
                                .required(true)
                                .help("Amount to withdraw from each account, in tokens"),
                        )
                        .arg(
                            Arg::with_name("to")
                                .long("to")
                                .validator(is_valid_pubkey)
                                .value_name("RECIPIENT_TOKEN_ACCOUNT_ADDRESS")
                                .takes_value(true)
                                .help("The recipient token account address [default: associated token account for --owner]")
                        )
                        .arg(owner_address_arg()),
                ),
        )
    }
}

pub(crate) async fn bench_process_command(
    matches: &ArgMatches<'_>,
    config: &Config<'_>,
    mut signers: Vec<Arc<dyn Signer>>,
    wallet_manager: &mut Option<Arc<RemoteWalletManager>>,
) -> CommandResult {
    assert!(!config.sign_only);

    match matches.subcommand() {
        ("create-accounts", Some(arg_matches)) => {
            let token = pubkey_of_signer(arg_matches, "token", wallet_manager)
                .unwrap()
                .unwrap();
            let n = value_t_or_exit!(arg_matches, "n", usize);

            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);

            command_create_accounts(config, signers, &token, n, &owner).await?;
        }
        ("close-accounts", Some(arg_matches)) => {
            let token = pubkey_of_signer(arg_matches, "token", wallet_manager)
                .unwrap()
                .unwrap();
            let n = value_t_or_exit!(arg_matches, "n", usize);
            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);

            command_close_accounts(config, signers, &token, n, &owner).await?;
        }
        ("deposit-into", Some(arg_matches)) => {
            let token = pubkey_of_signer(arg_matches, "token", wallet_manager)
                .unwrap()
                .unwrap();
            let n = value_t_or_exit!(arg_matches, "n", usize);
            let ui_amount = value_t_or_exit!(arg_matches, "amount", f64);
            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let from = pubkey_of_signer(arg_matches, "from", wallet_manager).unwrap();
            command_deposit_into_or_withdraw_from(
                config, signers, &token, n, &owner, ui_amount, from, true,
            )
            .await?;
        }
        ("withdraw-from", Some(arg_matches)) => {
            let token = pubkey_of_signer(arg_matches, "token", wallet_manager)
                .unwrap()
                .unwrap();
            let n = value_t_or_exit!(arg_matches, "n", usize);
            let ui_amount = value_t_or_exit!(arg_matches, "amount", f64);
            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let to = pubkey_of_signer(arg_matches, "to", wallet_manager).unwrap();
            command_deposit_into_or_withdraw_from(
                config, signers, &token, n, &owner, ui_amount, to, false,
            )
            .await?;
        }
        _ => unreachable!(),
    }

    Ok("".to_string())
}

fn get_token_address_with_seed(
    program_id: &Pubkey,
    token: &Pubkey,
    owner: &Pubkey,
    i: usize,
) -> (Pubkey, String) {
    let seed = format!("{}{}", i, token)[..31].to_string();
    (
        Pubkey::create_with_seed(owner, &seed, program_id).unwrap(),
        seed,
    )
}

fn get_token_addresses_with_seed(
    program_id: &Pubkey,
    token: &Pubkey,
    owner: &Pubkey,
    n: usize,
) -> Vec<(Pubkey, String)> {
    (0..n)
        .map(|i| get_token_address_with_seed(program_id, token, owner, i))
        .collect()
}

async fn get_valid_mint_program_id(
    rpc_client: &RpcClient,
    token: &Pubkey,
) -> Result<Pubkey, Error> {
    let mint_account = rpc_client
        .get_account(token)
        .await
        .map_err(|err| format!("Token mint {} does not exist: {}", token, err))?;

    StateWithExtensions::<Mint>::unpack(&mint_account.data)
        .map_err(|err| format!("Invalid token mint {}: {}", token, err))?;
    Ok(mint_account.owner)
}

async fn command_create_accounts(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    token: &Pubkey,
    n: usize,
    owner: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;

    println!("Scanning accounts...");
    let program_id = get_valid_mint_program_id(rpc_client, token).await?;

    let minimum_balance_for_rent_exemption = rpc_client
        .get_minimum_balance_for_rent_exemption(Account::get_packed_len())
        .await?;

    let mut lamports_required = 0;

    let token_addresses_with_seed = get_token_addresses_with_seed(&program_id, token, owner, n);
    let mut messages = vec![];
    for address_chunk in token_addresses_with_seed.chunks(100) {
        let accounts_chunk = rpc_client
            .get_multiple_accounts(&address_chunk.iter().map(|x| x.0).collect::<Vec<_>>())
            .await?;

        for (account, (address, seed)) in accounts_chunk.iter().zip(address_chunk) {
            if account.is_none() {
                lamports_required += minimum_balance_for_rent_exemption;
                messages.push(Message::new(
                    &[
                        system_instruction::create_account_with_seed(
                            &config.fee_payer()?.pubkey(),
                            address,
                            owner,
                            seed,
                            minimum_balance_for_rent_exemption,
                            Account::get_packed_len() as u64,
                            &program_id,
                        ),
                        instruction::initialize_account(&program_id, address, token, owner)?,
                    ],
                    Some(&config.fee_payer()?.pubkey()),
                ));
            }
        }
    }

    send_messages(config, &messages, lamports_required, signers).await
}

async fn command_close_accounts(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    token: &Pubkey,
    n: usize,
    owner: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;

    println!("Scanning accounts...");
    let program_id = get_valid_mint_program_id(rpc_client, token).await?;

    let token_addresses_with_seed = get_token_addresses_with_seed(&program_id, token, owner, n);
    let mut messages = vec![];
    for address_chunk in token_addresses_with_seed.chunks(100) {
        let accounts_chunk = rpc_client
            .get_multiple_accounts(&address_chunk.iter().map(|x| x.0).collect::<Vec<_>>())
            .await?;

        for (account, (address, _seed)) in accounts_chunk.iter().zip(address_chunk) {
            if let Some(account) = account {
                match StateWithExtensions::<Account>::unpack(&account.data) {
                    Ok(token_account) => {
                        if token_account.base.amount != 0 {
                            eprintln!(
                                "Token account {} holds a balance; unable to close it",
                                address,
                            );
                        } else {
                            messages.push(Message::new(
                                &[instruction::close_account(
                                    &program_id,
                                    address,
                                    owner,
                                    owner,
                                    &[],
                                )?],
                                Some(&config.fee_payer()?.pubkey()),
                            ));
                        }
                    }
                    Err(err) => {
                        eprintln!("Invalid token account {}: {}", address, err)
                    }
                }
            }
        }
    }

    send_messages(config, &messages, 0, signers).await
}

#[allow(clippy::too_many_arguments)]
async fn command_deposit_into_or_withdraw_from(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    token: &Pubkey,
    n: usize,
    owner: &Pubkey,
    ui_amount: f64,
    from_or_to: Option<Pubkey>,
    deposit_into: bool,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;

    println!("Scanning accounts...");
    let program_id = get_valid_mint_program_id(rpc_client, token).await?;

    let mint_info = config.get_mint_info(token, None).await?;
    let from_or_to = from_or_to
        .unwrap_or_else(|| get_associated_token_address_with_program_id(owner, token, &program_id));
    config.check_account(&from_or_to, Some(*token)).await?;
    let amount = spl_token::ui_amount_to_amount(ui_amount, mint_info.decimals);

    let token_addresses_with_seed = get_token_addresses_with_seed(&program_id, token, owner, n);
    let mut messages = vec![];
    for address_chunk in token_addresses_with_seed.chunks(100) {
        let accounts_chunk = rpc_client
            .get_multiple_accounts(&address_chunk.iter().map(|x| x.0).collect::<Vec<_>>())
            .await?;

        for (account, (address, _seed)) in accounts_chunk.iter().zip(address_chunk) {
            if account.is_some() {
                messages.push(Message::new(
                    &[instruction::transfer_checked(
                        &program_id,
                        if deposit_into { &from_or_to } else { address },
                        token,
                        if deposit_into { address } else { &from_or_to },
                        owner,
                        &[],
                        amount,
                        mint_info.decimals,
                    )?],
                    Some(&config.fee_payer()?.pubkey()),
                ));
            } else {
                eprintln!("Token account does not exist: {}", address)
            }
        }
    }

    send_messages(config, &messages, 0, signers).await
}

async fn send_messages(
    config: &Config<'_>,
    messages: &[Message],
    mut lamports_required: u64,
    signers: Vec<Arc<dyn Signer>>,
) -> Result<(), Error> {
    if messages.is_empty() {
        println!("Nothing to do");
        return Ok(());
    }

    let blockhash = config.rpc_client.get_latest_blockhash().await?;
    let mut message = messages[0].clone();
    message.recent_blockhash = blockhash;
    lamports_required +=
        config.rpc_client.get_fee_for_message(&message).await? * messages.len() as u64;

    println!(
        "Sending {:?} messages for ~{}",
        messages.len(),
        Sol(lamports_required)
    );

    crate::check_fee_payer_balance(config, lamports_required).await?;

    // TODO use async tpu client once it's available in 1.11
    let start = Instant::now();
    let rpc_client = BlockingRpcClient::new(config.rpc_client.url());
    let tpu_client = TpuClient::new(
        Arc::new(rpc_client),
        &config.websocket_url,
        TpuClientConfig::default(),
    )?;
    let transaction_errors =
        tpu_client.send_and_confirm_messages_with_spinner(messages, &signers)?;
    for (i, transaction_error) in transaction_errors.into_iter().enumerate() {
        if let Some(transaction_error) = transaction_error {
            println!("Message {} failed with {:?}", i, transaction_error);
        }
    }
    let elapsed = Instant::now().duration_since(start);
    let tps = messages.len() as f64 / elapsed.as_secs_f64();
    println!(
        "Average TPS: {:.2}\nElapsed time: {} seconds",
        tps,
        elapsed.as_secs_f64(),
    );

    let stats = config.rpc_client.get_transport_stats();
    println!("Total RPC requests: {}", stats.request_count);
    println!(
        "Total RPC time: {:.2} seconds",
        stats.elapsed_time.as_secs_f64()
    );
    if stats.rate_limited_time != std::time::Duration::default() {
        println!(
            "Total idle time due to RPC rate limiting: {:.2} seconds",
            stats.rate_limited_time.as_secs_f64()
        );
    }

    Ok(())
}
