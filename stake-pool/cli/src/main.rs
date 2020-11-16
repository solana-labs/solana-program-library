use clap::{
    crate_description, crate_name, crate_version, value_t, value_t_or_exit, App, AppSettings, Arg,
    ArgGroup, SubCommand,
};
use solana_account_decoder::UiAccountEncoding;
use solana_clap_utils::{
    input_parsers::pubkey_of,
    input_validators::{is_amount, is_keypair, is_parsable, is_pubkey, is_url},
    keypair::signer_from_path,
};
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::RpcAccountInfoConfig,
    rpc_config::RpcProgramAccountsConfig,
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_program::{instruction::Instruction, program_pack::Pack, pubkey::Pubkey};
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentConfig,
    native_token::*,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_stake_pool::{
    instruction::{
        claim, deposit, initialize as initialize_pool, set_owner, set_staking_authority, withdraw,
        Fee as PoolFee, InitArgs as PoolInitArgs,
    },
    processor::Processor as PoolProcessor,
    stake::authorize as authorize_stake,
    stake::id as stake_program_id,
    stake::merge as merge_stake,
    stake::StakeAuthorize,
    state::StakePool,
    state::State as PoolState,
};
use spl_token::{
    self, instruction::approve as approve_token, instruction::initialize_account,
    instruction::initialize_mint, native_mint, state::Account as TokenAccount,
    state::Mint as TokenMint,
};
use std::process::exit;

struct Config {
    rpc_client: RpcClient,
    verbose: bool,
    owner: Box<dyn Signer>,
    fee_payer: Box<dyn Signer>,
    commitment_config: CommitmentConfig,
}

type Error = Box<dyn std::error::Error>;
type CommandResult = Result<Option<Transaction>, Error>;

const STAKE_STATE_LEN: usize = 200;

macro_rules! unique_signers {
    ($vec:ident) => {
        $vec.sort_by_key(|l| l.pubkey());
        $vec.dedup();
    };
}

fn check_fee_payer_balance(config: &Config, required_balance: u64) -> Result<(), Error> {
    let balance = config.rpc_client.get_balance(&config.fee_payer.pubkey())?;
    if balance < required_balance {
        Err(format!(
            "Fee payer, {}, has insufficient balance: {} required, {} available",
            config.fee_payer.pubkey(),
            lamports_to_sol(required_balance),
            lamports_to_sol(balance)
        )
        .into())
    } else {
        Ok(())
    }
}

fn get_authority_accounts(config: &Config, authority: &Pubkey) -> Vec<(Pubkey, Account)> {
    config
        .rpc_client
        .get_program_accounts_with_config(
            &stake_program_id(),
            RpcProgramAccountsConfig {
                filters: Some(vec![RpcFilterType::Memcmp(Memcmp {
                    offset: 44, // 44 is Withdrawer authority offset in stake accoun stake
                    bytes: MemcmpEncodedBytes::Binary(
                        bs58::encode(authority.to_bytes()).into_string(),
                    ),
                    encoding: None,
                })]),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    ..RpcAccountInfoConfig::default()
                },
            },
        )
        .unwrap()
}

fn _check_owner_balance(config: &Config, required_balance: u64) -> Result<(), Error> {
    let balance = config.rpc_client.get_balance(&config.owner.pubkey())?;
    if balance < required_balance {
        Err(format!(
            "Owner, {}, has insufficient balance: {} required, {} available",
            config.owner.pubkey(),
            lamports_to_sol(required_balance),
            lamports_to_sol(balance)
        )
        .into())
    } else {
        Ok(())
    }
}

fn command_create_pool(config: &Config, fee: PoolFee) -> CommandResult {
    let mint_account = Keypair::new();
    println!("Creating mint {}", mint_account.pubkey());

    let pool_fee_account = Keypair::new();
    println!(
        "Creating pool fee collection account {}",
        pool_fee_account.pubkey()
    );

    let pool_account = Keypair::new();
    println!("Creating stake pool {}", pool_account.pubkey());

    let mint_account_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(TokenMint::LEN)?;
    let pool_fee_account_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(TokenAccount::LEN)?;
    let pool_account_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(PoolState::LEN)?;
    let total_rent_free_balances =
        mint_account_balance + pool_fee_account_balance + pool_account_balance;

    let default_decimals = native_mint::DECIMALS;

    // Calculate withdraw authority used for minting pool tokens
    let (withdraw_authority, _) = PoolProcessor::find_authority_bump_seed(
        &spl_stake_pool::id(),
        &pool_account.pubkey(),
        PoolProcessor::AUTHORITY_WITHDRAW,
    );

    if config.verbose {
        println!("Stake pool withdraw authority {}", withdraw_authority);
    }

    let mut transaction = Transaction::new_with_payer(
        &[
            // Account for the stake pool mint
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &mint_account.pubkey(),
                mint_account_balance,
                TokenMint::LEN as u64,
                &spl_token::id(),
            ),
            // Account for the pool fee accumulation
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &pool_fee_account.pubkey(),
                pool_fee_account_balance,
                TokenAccount::LEN as u64,
                &spl_token::id(),
            ),
            // Account for the stake pool
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &pool_account.pubkey(),
                pool_account_balance,
                PoolState::LEN as u64,
                &spl_stake_pool::id(),
            ),
            // Initialize pool token mint account
            initialize_mint(
                &spl_token::id(),
                &mint_account.pubkey(),
                &withdraw_authority,
                None,
                default_decimals,
            )?,
            // Initialize fee receiver account
            initialize_account(
                &spl_token::id(),
                &pool_fee_account.pubkey(),
                &mint_account.pubkey(),
                &config.owner.pubkey(),
            )?,
            // Initialize stake pool account
            initialize_pool(
                &spl_stake_pool::id(),
                &pool_account.pubkey(),
                &config.owner.pubkey(),
                &mint_account.pubkey(),
                &pool_fee_account.pubkey(),
                &spl_token::id(),
                PoolInitArgs { fee },
            )?,
        ],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(
        config,
        total_rent_free_balances + fee_calculator.calculate_fee(&transaction.message()),
    )?;
    let mut signers = vec![
        config.fee_payer.as_ref(),
        &pool_account,
        &mint_account,
        &pool_fee_account,
    ];
    unique_signers!(signers);
    transaction.sign(&signers, recent_blockhash);
    Ok(Some(transaction))
}

fn command_deposit(
    config: &Config,
    pool: &Pubkey,
    stake: &Pubkey,
    token_receiver: &Option<Pubkey>,
) -> CommandResult {
    // Get stake pool state
    let pool_data = config.rpc_client.get_account_data(&pool)?;
    let pool_data: StakePool = PoolState::deserialize(pool_data.as_slice())
        .unwrap()
        .stake_pool()
        .unwrap();

    let mut instructions: Vec<Instruction> = vec![];
    let mut signers = vec![config.fee_payer.as_ref(), config.owner.as_ref()];

    let mut total_rent_free_balances: u64 = 0;

    let token_receiver_account = Keypair::new();

    let token_receiver: Pubkey = match token_receiver {
        Some(value) => *value,
        None => {
            // Account for tokens not specified, creating one
            println!(
                "Creating account to receive tokens {}",
                token_receiver_account.pubkey()
            );

            let token_receiver_account_balance = config
                .rpc_client
                .get_minimum_balance_for_rent_exemption(TokenAccount::LEN)?;

            instructions.extend(vec![
                // Creating new account
                system_instruction::create_account(
                    &config.fee_payer.pubkey(),
                    &token_receiver_account.pubkey(),
                    token_receiver_account_balance,
                    TokenAccount::LEN as u64,
                    &spl_token::id(),
                ),
                // Initialize token receiver account
                initialize_account(
                    &spl_token::id(),
                    &token_receiver_account.pubkey(),
                    &pool_data.pool_mint,
                    &config.owner.pubkey(),
                )?,
            ]);

            signers.push(&token_receiver_account);

            total_rent_free_balances += token_receiver_account_balance;

            token_receiver_account.pubkey()
        }
    };

    // Calculate Deposit and Withdraw stake pool authorities
    let pool_deposit_authority: Pubkey = PoolProcessor::authority_id(
        &spl_stake_pool::id(),
        pool,
        PoolProcessor::AUTHORITY_DEPOSIT,
        pool_data.deposit_bump_seed,
    )
    .unwrap();
    let pool_withdraw_authority: Pubkey = PoolProcessor::authority_id(
        &spl_stake_pool::id(),
        pool,
        PoolProcessor::AUTHORITY_WITHDRAW,
        pool_data.withdraw_bump_seed,
    )
    .unwrap();

    instructions.extend(vec![
        // Set Withdrawer on stake account to Deposit authority of the stake pool
        authorize_stake(
            &stake,
            &config.owner.pubkey(),
            &pool_deposit_authority,
            StakeAuthorize::Withdrawer,
        ),
        // Set Staker on stake account to Deposit authority of the stake pool
        authorize_stake(
            &stake,
            &config.owner.pubkey(),
            &pool_deposit_authority,
            StakeAuthorize::Staker,
        ),
        // Add stake account to the pool
        deposit(
            &spl_stake_pool::id(),
            &pool,
            &pool_deposit_authority,
            &pool_withdraw_authority,
            &stake,
            &token_receiver,
            &pool_data.owner_fee_account,
            &pool_data.pool_mint,
            &spl_token::id(),
            &stake_program_id(),
        )?,
    ]);

    let mut transaction =
        Transaction::new_with_payer(&instructions, Some(&config.fee_payer.pubkey()));

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(
        config,
        total_rent_free_balances + fee_calculator.calculate_fee(&transaction.message()),
    )?;
    unique_signers!(signers);
    transaction.sign(&signers, recent_blockhash);
    Ok(Some(transaction))
}

fn command_list(config: &Config, pool: &Pubkey) -> CommandResult {
    // Get stake pool state
    let pool_data = config.rpc_client.get_account_data(&pool)?;
    let pool_data: StakePool = PoolState::deserialize(pool_data.as_slice())
        .unwrap()
        .stake_pool()
        .unwrap();

    let pool_withdraw_authority: Pubkey = PoolProcessor::authority_id(
        &spl_stake_pool::id(),
        pool,
        PoolProcessor::AUTHORITY_WITHDRAW,
        pool_data.withdraw_bump_seed,
    )
    .unwrap();

    let accounts = get_authority_accounts(config, &pool_withdraw_authority);

    if accounts.is_empty() {
        return Err("No accounts found.".to_string().into());
    }

    let mut total_balance: u64 = 0;
    for (pubkey, account) in accounts {
        let balance = account.lamports;
        total_balance += balance;
        println!("{}\t{} SOL", pubkey, lamports_to_sol(balance));
    }
    println!("Total: {} SOL", lamports_to_sol(total_balance));

    Ok(None)
}

fn stake_amount_to_pool_tokens(pool_data: &StakePool, amount: u64) -> u64 {
    (amount as u128)
        .checked_mul(pool_data.pool_total as u128)
        .unwrap()
        .checked_div(pool_data.stake_total as u128)
        .unwrap() as u64
}

fn pool_tokens_to_stake_amount(pool_data: &StakePool, tokens: u64) -> u64 {
    (tokens as u128)
        .checked_mul(pool_data.stake_total as u128)
        .unwrap()
        .checked_div(pool_data.pool_total as u128)
        .unwrap() as u64
}

fn command_withdraw(
    config: &Config,
    pool: &Pubkey,
    amount: u64,
    burn_from: &Pubkey,
    stake_receiver: &Option<Pubkey>,
) -> CommandResult {
    // Get stake pool state
    let pool_data = config.rpc_client.get_account_data(&pool)?;
    let pool_data: StakePool = PoolState::deserialize(pool_data.as_slice())
        .unwrap()
        .stake_pool()
        .unwrap();

    let pool_withdraw_authority: Pubkey = PoolProcessor::authority_id(
        &spl_stake_pool::id(),
        pool,
        PoolProcessor::AUTHORITY_WITHDRAW,
        pool_data.withdraw_bump_seed,
    )
    .unwrap();

    // Check burn_from account type
    let account_data = config.rpc_client.get_account_data(&burn_from)?;
    let account_data: TokenAccount =
        TokenAccount::unpack_from_slice(account_data.as_slice()).unwrap();

    if account_data.mint != pool_data.pool_mint {
        return Err("Wrong token account.".to_string().into());
    }

    // Check burn_from balance
    let max_withdraw_amount = pool_tokens_to_stake_amount(&pool_data, account_data.amount);
    if max_withdraw_amount < amount {
        return Err(format!(
            "Not enough token balance to withdraw {} SOL.\nMaximum withdraw amount is {} SOL.",
            lamports_to_sol(amount),
            lamports_to_sol(max_withdraw_amount)
        )
        .into());
    }

    let mut accounts = get_authority_accounts(config, &pool_withdraw_authority);
    if accounts.is_empty() {
        return Err("No accounts found.".to_string().into());
    }
    // Sort from lowest to highest balance
    accounts.sort_by(|a, b| a.1.lamports.cmp(&b.1.lamports));

    for (stake_pubkey, account) in accounts {
        if account.lamports < amount {
            continue;
        }
        // Account found to claim or withdraw
        println!("Withdrawing from account {}", stake_pubkey);

        let mut instructions: Vec<Instruction> = vec![];
        let mut signers = vec![config.fee_payer.as_ref(), config.owner.as_ref()];

        let stake_receiver_account = Keypair::new();

        let mut total_rent_free_balances: u64 = 0;

        // Calculate amount of tokens to burn
        let tokens_to_burn = stake_amount_to_pool_tokens(&pool_data, amount);

        instructions.push(
            // Approve spending token
            approve_token(
                &spl_token::id(),
                &burn_from,
                &pool_withdraw_authority,
                &config.owner.pubkey(),
                &[],
                tokens_to_burn,
            )?,
        );

        if amount == account.lamports {
            // Claim to get whole account
            instructions.push(claim(
                &spl_stake_pool::id(),
                &pool,
                &pool_withdraw_authority,
                &stake_pubkey,
                &config.owner.pubkey(),
                &burn_from,
                &pool_data.pool_mint,
                &spl_token::id(),
                &stake_program_id(),
            )?);
            // Merge into stake_receiver (if present)
            if let Some(merge_into) = stake_receiver {
                instructions.push(merge_stake(
                    &merge_into,
                    &stake_pubkey,
                    &config.owner.pubkey(),
                ));
            }
        } else {
            // Withdraw to split account
            let stake_receiver: Pubkey = match stake_receiver {
                Some(value) => *value,
                None => {
                    // Account for tokens not specified, creating one
                    println!(
                        "Creating account to receive stake {}",
                        stake_receiver_account.pubkey()
                    );

                    let stake_receiver_account_balance = config
                        .rpc_client
                        .get_minimum_balance_for_rent_exemption(STAKE_STATE_LEN)?;

                    instructions.push(
                        // Creating new account
                        system_instruction::create_account(
                            &config.fee_payer.pubkey(),
                            &stake_receiver_account.pubkey(),
                            stake_receiver_account_balance,
                            STAKE_STATE_LEN as u64,
                            &stake_program_id(),
                        ),
                    );

                    signers.push(&stake_receiver_account);

                    total_rent_free_balances += stake_receiver_account_balance;

                    stake_receiver_account.pubkey()
                }
            };

            instructions.push(withdraw(
                &spl_stake_pool::id(),
                &pool,
                &pool_withdraw_authority,
                &stake_pubkey,
                &stake_receiver,
                &config.owner.pubkey(),
                &burn_from,
                &pool_data.pool_mint,
                &spl_token::id(),
                &stake_program_id(),
                amount,
            )?);
        }

        let mut transaction =
            Transaction::new_with_payer(&instructions, Some(&config.fee_payer.pubkey()));

        let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
        check_fee_payer_balance(
            config,
            total_rent_free_balances + fee_calculator.calculate_fee(&transaction.message()),
        )?;
        unique_signers!(signers);
        transaction.sign(&signers, recent_blockhash);

        return Ok(Some(transaction));
    }

    Err(format!(
        "No stake accounts found in this pool with enough balance to withdraw {} SOL.",
        lamports_to_sol(amount)
    )
    .into())
}

fn command_set_staking_auth(
    config: &Config,
    pool: &Pubkey,
    stake_account: &Pubkey,
    new_staker: &Pubkey,
) -> CommandResult {
    let pool_data = config.rpc_client.get_account_data(&pool)?;
    let pool_data: StakePool = PoolState::deserialize(pool_data.as_slice())
        .unwrap()
        .stake_pool()
        .unwrap();

    let pool_withdraw_authority: Pubkey = PoolProcessor::authority_id(
        &spl_stake_pool::id(),
        pool,
        PoolProcessor::AUTHORITY_WITHDRAW,
        pool_data.withdraw_bump_seed,
    )
    .unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[set_staking_authority(
            &spl_stake_pool::id(),
            &pool,
            &config.owner.pubkey(),
            &pool_withdraw_authority,
            &stake_account,
            &new_staker,
            &stake_program_id(),
        )?],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(config, fee_calculator.calculate_fee(&transaction.message()))?;
    let mut signers = vec![config.fee_payer.as_ref(), config.owner.as_ref()];
    unique_signers!(signers);
    transaction.sign(&signers, recent_blockhash);
    Ok(Some(transaction))
}

fn command_set_owner(
    config: &Config,
    pool: &Pubkey,
    new_owner: &Option<Pubkey>,
    new_fee_receiver: &Option<Pubkey>,
) -> CommandResult {
    let pool_data = config.rpc_client.get_account_data(&pool)?;
    let pool_data: StakePool = PoolState::deserialize(pool_data.as_slice())
        .unwrap()
        .stake_pool()
        .unwrap();

    // If new accounts are missing in the arguments use the old ones
    let new_owner: Pubkey = match new_owner {
        None => pool_data.owner,
        Some(value) => *value,
    };
    let new_fee_receiver: Pubkey = match new_fee_receiver {
        None => pool_data.owner_fee_account,
        Some(value) => {
            // Check for fee receiver being a valid token account and have to same mint as the stake pool
            let account_data = config.rpc_client.get_account_data(value)?;
            let account_data: TokenAccount =
                match TokenAccount::unpack_from_slice(account_data.as_slice()) {
                    Ok(data) => data,
                    Err(_) => {
                        return Err(format!("{} is not a token account", value).into());
                    }
                };
            if account_data.mint != pool_data.pool_mint {
                return Err("Fee receiver account belongs to a different mint"
                    .to_string()
                    .into());
            }
            *value
        }
    };

    let mut transaction = Transaction::new_with_payer(
        &[set_owner(
            &spl_stake_pool::id(),
            &pool,
            &config.owner.pubkey(),
            &new_owner,
            &new_fee_receiver,
        )?],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(config, fee_calculator.calculate_fee(&transaction.message()))?;
    let mut signers = vec![config.fee_payer.as_ref(), config.owner.as_ref()];
    unique_signers!(signers);
    transaction.sign(&signers, recent_blockhash);
    Ok(Some(transaction))
}

fn main() {
    let matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg({
            let arg = Arg::with_name("config_file")
                .short("C")
                .long("config")
                .value_name("PATH")
                .takes_value(true)
                .global(true)
                .help("Configuration file to use");
            if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
                arg.default_value(&config_file)
            } else {
                arg
            }
        })
        .arg(
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .takes_value(false)
                .global(true)
                .help("Show additional information"),
        )
        .arg(
            Arg::with_name("json_rpc_url")
                .long("url")
                .value_name("URL")
                .takes_value(true)
                .validator(is_url)
                .help("JSON RPC URL for the cluster.  Default from the configuration file."),
        )
        .arg(
            Arg::with_name("owner")
                .long("owner")
                .value_name("KEYPAIR")
                .validator(is_keypair)
                .takes_value(true)
                .help(
                    "Specify the stake pool or stake account owner. \
                     This may be a keypair file, the ASK keyword. \
                     Defaults to the client keypair.",
                ),
        )
        .arg(
            Arg::with_name("fee_payer")
                .long("fee-payer")
                .value_name("KEYPAIR")
                .validator(is_keypair)
                .takes_value(true)
                .help(
                    "Specify the fee-payer account. \
                     This may be a keypair file, the ASK keyword. \
                     Defaults to the client keypair.",
                ),
        )
        .subcommand(SubCommand::with_name("create-pool").about("Create a new stake pool")
            .arg(
                Arg::with_name("fee_numerator")
                    .long("fee-numerator")
                    .short("n")
                    .validator(is_parsable::<u64>)
                    .value_name("NUMERATOR")
                    .takes_value(true)
                    .required(true)
                    .help("Fee numerator, fee amount is numerator divided by denominator."),
            )
            .arg(
                Arg::with_name("fee_denominator")
                    .long("fee-denominator")
                    .short("d")
                    .validator(is_parsable::<u64>)
                    .value_name("DENOMINATOR")
                    .takes_value(true)
                    .required(true)
                    .help("Fee denominator, fee amount is numerator divided by denominator."),
            )
        )
        .subcommand(SubCommand::with_name("deposit").about("Add stake account to the stake pool")
            .arg(
                Arg::with_name("pool")
                    .long("pool")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address"),
            )
            .arg(
                Arg::with_name("stake")
                    .long("stake")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake address to join the pool"),
            )
            .arg(
                Arg::with_name("token_receiver")
                    .long("token-receiver")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .help("Account to receive pool token. Must be initialized account of the stake pool token. Defaults to the new pool token account."),
            )
        )
        .subcommand(SubCommand::with_name("list").about("List stake accounts managed by this pool")
            .arg(
                Arg::with_name("pool")
                    .long("pool")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address."),
            )
        )
        .subcommand(SubCommand::with_name("withdraw").about("Withdraw amount from the stake pool")
            .arg(
                Arg::with_name("pool")
                    .long("pool")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address."),
            )
            .arg(
                Arg::with_name("amount")
                    .long("amount")
                    .validator(is_amount)
                    .value_name("AMOUNT")
                    .takes_value(true)
                    .required(true)
                    .help("Amount in SOL to withdraw from the pool."),
            )
            .arg(
                Arg::with_name("burn_from")
                    .long("burn-from")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Account to burn tokens from. Must be owned by the client."),
            )
            .arg(
                Arg::with_name("stake_receiver")
                    .long("stake-receiver")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .help("Stake account to receive SOL from the stake pool. Defaults to a new stake account."),
            )
        )
        .subcommand(SubCommand::with_name("set-staking-auth").about("Changes staking authority of one of the accounts from the stake pool.")
            .arg(
                Arg::with_name("pool")
                    .long("pool")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address."),
            )
            .arg(
                Arg::with_name("stake_account")
                    .long("stake-account")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake account address to change staking authority."),
            )
            .arg(
                Arg::with_name("new_staker")
                    .long("new-staker")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Public key of the new staker account."),
            )
        )
        .subcommand(SubCommand::with_name("set-owner").about("Changes owner or fee receiver account for the stake pool.")
            .arg(
                Arg::with_name("pool")
                    .long("pool")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address."),
            )
            .arg(
                Arg::with_name("new_owner")
                    .long("new-owner")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .help("Public key for the new stake pool owner."),
            )
            .arg(
                Arg::with_name("new_fee_receiver")
                    .long("new-fee-receiver")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .help("Public key for the new account to set as the stake pool fee receiver."),
            )
            .group(ArgGroup::with_name("new_accounts")
                .arg("new_owner")
                .arg("new_fee_receiver")
                .required(true)
                .multiple(true)
            )
        )
        .get_matches();

    let mut wallet_manager = None;
    let config = {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };
        let json_rpc_url = value_t!(matches, "json_rpc_url", String)
            .unwrap_or_else(|_| cli_config.json_rpc_url.clone());

        let owner = signer_from_path(
            &matches,
            &cli_config.keypair_path,
            "owner",
            &mut wallet_manager,
        )
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            exit(1);
        });
        let fee_payer = signer_from_path(
            &matches,
            &cli_config.keypair_path,
            "fee_payer",
            &mut wallet_manager,
        )
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            exit(1);
        });
        let verbose = matches.is_present("verbose");

        Config {
            rpc_client: RpcClient::new(json_rpc_url),
            verbose,
            owner,
            fee_payer,
            commitment_config: CommitmentConfig::single(),
        }
    };

    solana_logger::setup_with_default("solana=info");

    let _ = match matches.subcommand() {
        ("create-pool", Some(arg_matches)) => {
            let numerator = value_t_or_exit!(arg_matches, "fee_numerator", u64);
            let denominator = value_t_or_exit!(arg_matches, "fee_denominator", u64);
            command_create_pool(
                &config,
                PoolFee {
                    numerator,
                    denominator,
                },
            )
        }
        ("deposit", Some(arg_matches)) => {
            let pool_account: Pubkey = pubkey_of(arg_matches, "pool").unwrap();
            let stake_account: Pubkey = pubkey_of(arg_matches, "stake").unwrap();
            let token_receiver: Option<Pubkey> = pubkey_of(arg_matches, "token_receiver");
            command_deposit(&config, &pool_account, &stake_account, &token_receiver)
        }
        ("list", Some(arg_matches)) => {
            let pool_account: Pubkey = pubkey_of(arg_matches, "pool").unwrap();
            command_list(&config, &pool_account)
        }
        ("withdraw", Some(arg_matches)) => {
            let pool_account: Pubkey = pubkey_of(arg_matches, "pool").unwrap();
            let burn_from: Pubkey = pubkey_of(arg_matches, "burn_from").unwrap();
            let amount: u64 = sol_to_lamports(value_t_or_exit!(arg_matches, "amount", f64));
            let stake_receiver: Option<Pubkey> = pubkey_of(arg_matches, "stake_receiver");
            command_withdraw(&config, &pool_account, amount, &burn_from, &stake_receiver)
        }
        ("set-staking-auth", Some(arg_matches)) => {
            let pool_account: Pubkey = pubkey_of(arg_matches, "pool").unwrap();
            let stake_account: Pubkey = pubkey_of(arg_matches, "stake_account").unwrap();
            let new_staker: Pubkey = pubkey_of(arg_matches, "new_staker").unwrap();
            command_set_staking_auth(&config, &pool_account, &stake_account, &new_staker)
        }
        ("set-owner", Some(arg_matches)) => {
            let pool_account: Pubkey = pubkey_of(arg_matches, "pool").unwrap();
            let new_owner: Option<Pubkey> = pubkey_of(arg_matches, "new_owner");
            let new_fee_receiver: Option<Pubkey> = pubkey_of(arg_matches, "new_fee_receiver");
            command_set_owner(&config, &pool_account, &new_owner, &new_fee_receiver)
        }
        _ => unreachable!(),
    }
    .and_then(|transaction| {
        if let Some(transaction) = transaction {
            // TODO: Upgrade to solana-client 1.3 and
            // `send_and_confirm_transaction_with_spinner_and_commitment()` with single
            // confirmation by default for better UX
            let signature = config
                .rpc_client
                .send_and_confirm_transaction_with_spinner_and_commitment(
                    &transaction,
                    config.commitment_config,
                )?;
            println!("Signature: {}", signature);
        }
        Ok(())
    })
    .map_err(|err| {
        eprintln!("{}", err);
        exit(1);
    });
}
