use crate::CommandResult;

/// The `bench` subcommand
use {
    crate::{
        config::Config, owner_address_arg,
        rpc_client_utils::send_and_confirm_messages_with_spinner, Error,
    },
    clap::{value_t_or_exit, App, AppSettings, Arg, ArgMatches, SubCommand},
    solana_clap_utils::{
        input_parsers::pubkey_of_signer,
        input_validators::{is_amount, is_parsable, is_valid_pubkey},
    },
    solana_client::rpc_client::RpcClient,
    solana_remote_wallet::remote_wallet::RemoteWalletManager,
    solana_sdk::{
        message::Message, native_token::Sol, program_pack::Pack, pubkey::Pubkey, signature::Signer,
        system_instruction,
    },
    spl_associated_token_account::*,
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

pub(crate) fn bench_process_command(
    matches: &ArgMatches<'_>,
    config: &Config,
    mut signers: Vec<Box<dyn Signer>>,
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

            command_create_accounts(config, signers, &token, n, &owner)?;
        }
        ("close-accounts", Some(arg_matches)) => {
            let token = pubkey_of_signer(arg_matches, "token", wallet_manager)
                .unwrap()
                .unwrap();
            let n = value_t_or_exit!(arg_matches, "n", usize);
            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);

            command_close_accounts(config, signers, &token, n, &owner)?;
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
            let from = pubkey_of_signer(arg_matches, "from", wallet_manager)
                .unwrap()
                .unwrap_or_else(|| {
                    get_associated_token_address_with_program_id(&owner, &token, &config.program_id)
                });

            command_deposit_into_or_withdraw_from(
                config, signers, &token, n, &owner, ui_amount, &from, true,
            )?;
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
            let to = pubkey_of_signer(arg_matches, "to", wallet_manager)
                .unwrap()
                .unwrap_or_else(|| {
                    get_associated_token_address_with_program_id(&owner, &token, &config.program_id)
                });

            command_deposit_into_or_withdraw_from(
                config, signers, &token, n, &owner, ui_amount, &to, false,
            )?;
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

fn is_valid_token(rpc_client: &RpcClient, token: &Pubkey) -> Result<(), Error> {
    let mint_account_data = rpc_client
        .get_account_data(token)
        .map_err(|err| format!("Token mint {} does not exist: {}", token, err))?;

    spl_token::state::Mint::unpack(&mint_account_data)
        .map(|_| ())
        .map_err(|err| format!("Invalid token mint {}: {}", token, err).into())
}

fn command_create_accounts(
    config: &Config,
    signers: Vec<Box<dyn Signer>>,
    token: &Pubkey,
    n: usize,
    owner: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;

    println!("Scanning accounts...");
    is_valid_token(rpc_client, token)?;

    let minimum_balance_for_rent_exemption = rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::get_packed_len())?;

    let mut lamports_required = 0;

    let token_addresses_with_seed =
        get_token_addresses_with_seed(&config.program_id, token, owner, n);
    let mut messages = vec![];
    for address_chunk in token_addresses_with_seed.chunks(100) {
        let accounts_chunk = rpc_client
            .get_multiple_accounts(&address_chunk.iter().map(|x| x.0).collect::<Vec<_>>())?;

        for (account, (address, seed)) in accounts_chunk.iter().zip(address_chunk) {
            if account.is_none() {
                lamports_required += minimum_balance_for_rent_exemption;
                messages.push(Message::new(
                    &[
                        system_instruction::create_account_with_seed(
                            &config.fee_payer,
                            address,
                            owner,
                            seed,
                            minimum_balance_for_rent_exemption,
                            spl_token::state::Account::get_packed_len() as u64,
                            &config.program_id,
                        ),
                        spl_token::instruction::initialize_account(
                            &config.program_id,
                            address,
                            token,
                            owner,
                        )?,
                    ],
                    Some(&config.fee_payer),
                ));
            }
        }
    }

    send_messages(config, &messages, lamports_required, signers)
}

fn command_close_accounts(
    config: &Config,
    signers: Vec<Box<dyn Signer>>,
    token: &Pubkey,
    n: usize,
    owner: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;

    println!("Scanning accounts...");
    is_valid_token(rpc_client, token)?;

    let token_addresses_with_seed =
        get_token_addresses_with_seed(&config.program_id, token, owner, n);
    let mut messages = vec![];
    for address_chunk in token_addresses_with_seed.chunks(100) {
        let accounts_chunk = rpc_client
            .get_multiple_accounts(&address_chunk.iter().map(|x| x.0).collect::<Vec<_>>())?;

        for (account, (address, _seed)) in accounts_chunk.iter().zip(address_chunk) {
            if let Some(account) = account {
                match spl_token::state::Account::unpack(&account.data) {
                    Ok(token_account) => {
                        if token_account.amount != 0 {
                            eprintln!(
                                "Token account {} holds a balance; unable to close it",
                                address,
                            );
                        } else {
                            messages.push(Message::new(
                                &[spl_token::instruction::close_account(
                                    &config.program_id,
                                    address,
                                    owner,
                                    owner,
                                    &[],
                                )?],
                                Some(&config.fee_payer),
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

    send_messages(config, &messages, 0, signers)
}

#[allow(clippy::too_many_arguments)]
fn command_deposit_into_or_withdraw_from(
    config: &Config,
    signers: Vec<Box<dyn Signer>>,
    token: &Pubkey,
    n: usize,
    owner: &Pubkey,
    ui_amount: f64,
    from_or_to: &Pubkey,
    deposit_into: bool,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;

    println!("Scanning accounts...");
    is_valid_token(rpc_client, token)?;

    let (mint_pubkey, decimals) = crate::resolve_mint_info(config, from_or_to, Some(*token), None)?;
    if mint_pubkey != *token {
        return Err(format!("Source account {} is not a {} token", from_or_to, token).into());
    }
    let amount = spl_token::ui_amount_to_amount(ui_amount, decimals);

    let token_addresses_with_seed =
        get_token_addresses_with_seed(&config.program_id, token, owner, n);
    let mut messages = vec![];
    for address_chunk in token_addresses_with_seed.chunks(100) {
        let accounts_chunk = rpc_client
            .get_multiple_accounts(&address_chunk.iter().map(|x| x.0).collect::<Vec<_>>())?;

        for (account, (address, _seed)) in accounts_chunk.iter().zip(address_chunk) {
            if account.is_some() {
                messages.push(Message::new(
                    &[spl_token::instruction::transfer_checked(
                        &config.program_id,
                        if deposit_into { from_or_to } else { address },
                        token,
                        if deposit_into { address } else { from_or_to },
                        owner,
                        &[],
                        amount,
                        decimals,
                    )?],
                    Some(&config.fee_payer),
                ));
            } else {
                eprintln!("Token account does not exist: {}", address)
            }
        }
    }

    send_messages(config, &messages, 0, signers)
}

fn send_messages(
    config: &Config,
    messages: &[Message],
    mut lamports_required: u64,
    signers: Vec<Box<dyn Signer>>,
) -> Result<(), Error> {
    if messages.is_empty() {
        println!("Nothing to do");
        return Ok(());
    }

    let (_blockhash, fee_calculator, _last_valid_block_height) = config
        .rpc_client
        .get_recent_blockhash_with_commitment(config.rpc_client.commitment())?
        .value;

    lamports_required += messages
        .iter()
        .map(|message| fee_calculator.calculate_fee(message))
        .sum::<u64>();

    println!(
        "Sending {:?} messages for ~{}",
        messages.len(),
        Sol(lamports_required)
    );

    crate::check_fee_payer_balance(config, lamports_required)?;

    let start = Instant::now();
    let transaction_errors = send_and_confirm_messages_with_spinner(
        config.rpc_client.clone(),
        &config.websocket_url,
        messages,
        &signers,
    )?;

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
