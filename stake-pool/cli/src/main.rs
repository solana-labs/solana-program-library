#[macro_use]
extern crate lazy_static;

mod client;

use {
    crate::client::*,
    clap::{
        crate_description, crate_name, crate_version, value_t, value_t_or_exit, App, AppSettings,
        Arg, ArgGroup, ArgMatches, SubCommand,
    },
    solana_clap_utils::{
        input_parsers::{keypair_of, pubkey_of},
        input_validators::{
            is_amount, is_keypair, is_keypair_or_ask_keyword, is_parsable, is_pubkey, is_url,
            is_valid_percentage,
        },
        keypair::signer_from_path,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{
        borsh::{get_instance_packed_len, get_packed_len},
        instruction::Instruction,
        program_pack::Pack,
        pubkey::Pubkey,
    },
    solana_remote_wallet::remote_wallet::RemoteWalletManager,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        native_token::{self, Sol},
        signature::{Keypair, Signer},
        signers::Signers,
        system_instruction,
        transaction::Transaction,
    },
    spl_associated_token_account::{create_associated_token_account, get_associated_token_address},
    spl_stake_pool::{
        self, find_stake_program_address, find_transient_stake_program_address,
        find_withdraw_authority_program_address,
        instruction::{DepositType, PreferredValidatorType},
        stake_program::{self, StakeState},
        state::{Fee, FeeType, StakePool, ValidatorList},
    },
    std::{process::exit, sync::Arc},
};

struct Config {
    rpc_client: RpcClient,
    verbose: bool,
    manager: Box<dyn Signer>,
    staker: Box<dyn Signer>,
    depositor: Option<Box<dyn Signer>>,
    sol_depositor: Option<Box<dyn Signer>>,
    token_owner: Box<dyn Signer>,
    fee_payer: Box<dyn Signer>,
    dry_run: bool,
    no_update: bool,
}

type Error = Box<dyn std::error::Error>;
type CommandResult = Result<(), Error>;

const STAKE_STATE_LEN: usize = 200;
lazy_static! {
    static ref MIN_STAKE_BALANCE: u64 = native_token::sol_to_lamports(1.0);
}

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
            Sol(required_balance),
            Sol(balance)
        )
        .into())
    } else {
        Ok(())
    }
}

fn get_signer(
    matches: &ArgMatches<'_>,
    keypair_name: &str,
    keypair_path: &str,
    wallet_manager: &mut Option<Arc<RemoteWalletManager>>,
) -> Box<dyn Signer> {
    signer_from_path(
        matches,
        matches.value_of(keypair_name).unwrap_or(keypair_path),
        keypair_name,
        wallet_manager,
    )
    .unwrap_or_else(|e| {
        eprintln!("error: {}", e);
        exit(1);
    })
}

fn send_transaction_no_wait(
    config: &Config,
    transaction: Transaction,
) -> solana_client::client_error::Result<()> {
    if config.dry_run {
        let result = config.rpc_client.simulate_transaction(&transaction)?;
        println!("Simulate result: {:?}", result);
    } else {
        let signature = config.rpc_client.send_transaction(&transaction)?;
        println!("Signature: {}", signature);
    }
    Ok(())
}

fn send_transaction(
    config: &Config,
    transaction: Transaction,
) -> solana_client::client_error::Result<()> {
    if config.dry_run {
        let result = config.rpc_client.simulate_transaction(&transaction)?;
        println!("Simulate result: {:?}", result);
    } else {
        let signature = config
            .rpc_client
            .send_and_confirm_transaction_with_spinner(&transaction)?;
        println!("Signature: {}", signature);
    }
    Ok(())
}

fn checked_transaction_with_signers<T: Signers>(
    config: &Config,
    instructions: &[Instruction],
    signers: &T,
) -> Result<Transaction, Error> {
    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        instructions,
        Some(&config.fee_payer.pubkey()),
        signers,
        recent_blockhash,
    );

    check_fee_payer_balance(config, fee_calculator.calculate_fee(transaction.message()))?;
    Ok(transaction)
}

#[allow(clippy::too_many_arguments)]
fn command_create_pool(
    config: &Config,
    stake_deposit_authority: Option<Keypair>,
    fee: Fee,
    withdrawal_fee: Fee,
    stake_deposit_fee: Fee,
    stake_referral_fee: u8,
    max_validators: u32,
    stake_pool_keypair: Option<Keypair>,
    mint_keypair: Option<Keypair>,
    reserve_keypair: Option<Keypair>,
) -> CommandResult {
    let reserve_keypair = reserve_keypair.unwrap_or_else(Keypair::new);
    println!("Creating reserve stake {}", reserve_keypair.pubkey());

    let mint_keypair = mint_keypair.unwrap_or_else(Keypair::new);
    println!("Creating mint {}", mint_keypair.pubkey());

    let stake_pool_keypair = stake_pool_keypair.unwrap_or_else(Keypair::new);

    let validator_list = Keypair::new();

    let reserve_stake_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(STAKE_STATE_LEN)?
        + 1;
    let mint_account_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN)?;
    let pool_fee_account_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)?;
    let stake_pool_account_lamports = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(get_packed_len::<StakePool>())?;
    let empty_validator_list = ValidatorList::new(max_validators);
    let validator_list_size = get_instance_packed_len(&empty_validator_list)?;
    let validator_list_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(validator_list_size)?;
    let mut total_rent_free_balances = reserve_stake_balance
        + mint_account_balance
        + pool_fee_account_balance
        + stake_pool_account_lamports
        + validator_list_balance;

    let default_decimals = spl_token::native_mint::DECIMALS;

    // Calculate withdraw authority used for minting pool tokens
    let (withdraw_authority, _) = find_withdraw_authority_program_address(
        &spl_stake_pool::id(),
        &stake_pool_keypair.pubkey(),
    );

    if config.verbose {
        println!("Stake pool withdraw authority {}", withdraw_authority);
    }

    let mut instructions = vec![
        // Account for the stake pool reserve
        system_instruction::create_account(
            &config.fee_payer.pubkey(),
            &reserve_keypair.pubkey(),
            reserve_stake_balance,
            STAKE_STATE_LEN as u64,
            &stake_program::id(),
        ),
        stake_program::initialize(
            &reserve_keypair.pubkey(),
            &stake_program::Authorized {
                staker: withdraw_authority,
                withdrawer: withdraw_authority,
            },
            &stake_program::Lockup::default(),
        ),
        // Account for the stake pool mint
        system_instruction::create_account(
            &config.fee_payer.pubkey(),
            &mint_keypair.pubkey(),
            mint_account_balance,
            spl_token::state::Mint::LEN as u64,
            &spl_token::id(),
        ),
        // Initialize pool token mint account
        spl_token::instruction::initialize_mint(
            &spl_token::id(),
            &mint_keypair.pubkey(),
            &withdraw_authority,
            None,
            default_decimals,
        )?,
    ];

    let pool_fee_account = add_associated_token_account(
        config,
        &mint_keypair.pubkey(),
        &config.manager.pubkey(),
        &mut instructions,
        &mut total_rent_free_balances,
    );
    println!("Creating pool fee collection account {}", pool_fee_account);

    let mut setup_transaction =
        Transaction::new_with_payer(&instructions, Some(&config.fee_payer.pubkey()));

    let mut initialize_transaction = Transaction::new_with_payer(
        &[
            // Validator stake account list storage
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &validator_list.pubkey(),
                validator_list_balance,
                validator_list_size as u64,
                &spl_stake_pool::id(),
            ),
            // Account for the stake pool
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &stake_pool_keypair.pubkey(),
                stake_pool_account_lamports,
                get_packed_len::<StakePool>() as u64,
                &spl_stake_pool::id(),
            ),
            // Initialize stake pool
            spl_stake_pool::instruction::initialize(
                &spl_stake_pool::id(),
                &stake_pool_keypair.pubkey(),
                &config.manager.pubkey(),
                &config.staker.pubkey(),
                &validator_list.pubkey(),
                &reserve_keypair.pubkey(),
                &mint_keypair.pubkey(),
                &pool_fee_account,
                &spl_token::id(),
                stake_deposit_authority.as_ref().map(|x| x.pubkey()),
                fee,
                withdrawal_fee,
                stake_deposit_fee,
                stake_referral_fee,
                max_validators,
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(
        config,
        total_rent_free_balances
            + fee_calculator.calculate_fee(setup_transaction.message())
            + fee_calculator.calculate_fee(initialize_transaction.message()),
    )?;
    let mut setup_signers = vec![config.fee_payer.as_ref(), &mint_keypair, &reserve_keypair];
    unique_signers!(setup_signers);
    setup_transaction.sign(&setup_signers, recent_blockhash);
    send_transaction(config, setup_transaction)?;

    println!("Creating stake pool {}", stake_pool_keypair.pubkey());
    let mut initialize_signers = vec![
        config.fee_payer.as_ref(),
        &stake_pool_keypair,
        &validator_list,
        config.manager.as_ref(),
    ];
    if let Some(stake_deposit_authority) = stake_deposit_authority {
        let mut initialize_signers = initialize_signers.clone();
        initialize_signers.push(&stake_deposit_authority);
        unique_signers!(initialize_signers);
        initialize_transaction.sign(&initialize_signers, recent_blockhash);
    } else {
        unique_signers!(initialize_signers);
        initialize_transaction.sign(&initialize_signers, recent_blockhash);
    }
    send_transaction(config, initialize_transaction)?;
    Ok(())
}

fn command_vsa_create(
    config: &Config,
    stake_pool_address: &Pubkey,
    vote_account: &Pubkey,
) -> CommandResult {
    let (stake_account, _) =
        find_stake_program_address(&spl_stake_pool::id(), vote_account, stake_pool_address);
    println!(
        "Creating stake account {}, delegated to {}",
        stake_account, vote_account
    );
    let transaction = checked_transaction_with_signers(
        config,
        &[
            // Create new validator stake account address
            spl_stake_pool::instruction::create_validator_stake_account(
                &spl_stake_pool::id(),
                stake_pool_address,
                &config.staker.pubkey(),
                &config.fee_payer.pubkey(),
                &stake_account,
                vote_account,
            ),
        ],
        &[config.fee_payer.as_ref(), config.staker.as_ref()],
    )?;
    send_transaction(config, transaction)?;
    Ok(())
}

fn command_vsa_add(
    config: &Config,
    stake_pool_address: &Pubkey,
    vote_account: &Pubkey,
) -> CommandResult {
    let (stake_account_address, _) =
        find_stake_program_address(&spl_stake_pool::id(), vote_account, stake_pool_address);
    println!(
        "Adding stake account {}, delegated to {}",
        stake_account_address, vote_account
    );
    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;
    let validator_list = get_validator_list(&config.rpc_client, &stake_pool.validator_list)?;
    if validator_list.contains(vote_account) {
        println!(
            "Stake pool already contains validator {}, ignoring",
            vote_account
        );
        return Ok(());
    }

    let stake_state = get_stake_state(&config.rpc_client, &stake_account_address)?;
    if let stake_program::StakeState::Stake(meta, _stake) = stake_state {
        if meta.authorized.withdrawer != config.staker.pubkey() {
            let error = format!(
                "Stake account withdraw authority must be the staker {}, actual {}",
                config.staker.pubkey(),
                meta.authorized.withdrawer
            );
            return Err(error.into());
        }
    } else {
        return Err("Stake account is not active.".into());
    }

    if !config.no_update {
        command_update(config, stake_pool_address, false, false)?;
    }

    let mut signers = vec![config.fee_payer.as_ref(), config.staker.as_ref()];
    unique_signers!(signers);
    let transaction = checked_transaction_with_signers(
        config,
        &[
            spl_stake_pool::instruction::add_validator_to_pool_with_vote(
                &spl_stake_pool::id(),
                &stake_pool,
                stake_pool_address,
                vote_account,
            ),
        ],
        &signers,
    )?;

    send_transaction(config, transaction)?;
    Ok(())
}

fn command_vsa_remove(
    config: &Config,
    stake_pool_address: &Pubkey,
    vote_account: &Pubkey,
    new_authority: &Option<Pubkey>,
) -> CommandResult {
    if !config.no_update {
        command_update(config, stake_pool_address, false, false)?;
    }

    let (stake_account_address, _) =
        find_stake_program_address(&spl_stake_pool::id(), vote_account, stake_pool_address);
    println!(
        "Removing stake account {}, delegated to {}",
        stake_account_address, vote_account
    );

    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;

    let staker_pubkey = config.staker.pubkey();
    let new_authority = new_authority.as_ref().unwrap_or(&staker_pubkey);

    let validator_list = get_validator_list(&config.rpc_client, &stake_pool.validator_list)?;
    let validator_stake_info = validator_list
        .find(vote_account)
        .ok_or("Vote account not found in validator list")?;

    let mut signers = vec![config.fee_payer.as_ref(), config.staker.as_ref()];
    unique_signers!(signers);
    let transaction = checked_transaction_with_signers(
        config,
        &[
            // Create new validator stake account address
            spl_stake_pool::instruction::remove_validator_from_pool_with_vote(
                &spl_stake_pool::id(),
                &stake_pool,
                stake_pool_address,
                vote_account,
                new_authority,
                validator_stake_info.transient_seed_suffix_start,
            ),
        ],
        &signers,
    )?;
    send_transaction(config, transaction)?;
    Ok(())
}

fn command_increase_validator_stake(
    config: &Config,
    stake_pool_address: &Pubkey,
    vote_account: &Pubkey,
    amount: f64,
) -> CommandResult {
    let lamports = native_token::sol_to_lamports(amount);
    if !config.no_update {
        command_update(config, stake_pool_address, false, false)?;
    }

    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;
    let validator_list = get_validator_list(&config.rpc_client, &stake_pool.validator_list)?;
    let validator_stake_info = validator_list
        .find(vote_account)
        .ok_or("Vote account not found in validator list")?;

    let mut signers = vec![config.fee_payer.as_ref(), config.staker.as_ref()];
    unique_signers!(signers);
    let transaction = checked_transaction_with_signers(
        config,
        &[
            spl_stake_pool::instruction::increase_validator_stake_with_vote(
                &spl_stake_pool::id(),
                &stake_pool,
                stake_pool_address,
                vote_account,
                lamports,
                validator_stake_info.transient_seed_suffix_start,
            ),
        ],
        &signers,
    )?;
    send_transaction(config, transaction)?;
    Ok(())
}

fn command_decrease_validator_stake(
    config: &Config,
    stake_pool_address: &Pubkey,
    vote_account: &Pubkey,
    amount: f64,
) -> CommandResult {
    let lamports = native_token::sol_to_lamports(amount);
    if !config.no_update {
        command_update(config, stake_pool_address, false, false)?;
    }

    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;
    let validator_list = get_validator_list(&config.rpc_client, &stake_pool.validator_list)?;
    let validator_stake_info = validator_list
        .find(vote_account)
        .ok_or("Vote account not found in validator list")?;

    let mut signers = vec![config.fee_payer.as_ref(), config.staker.as_ref()];
    unique_signers!(signers);
    let transaction = checked_transaction_with_signers(
        config,
        &[
            spl_stake_pool::instruction::decrease_validator_stake_with_vote(
                &spl_stake_pool::id(),
                &stake_pool,
                stake_pool_address,
                vote_account,
                lamports,
                validator_stake_info.transient_seed_suffix_start,
            ),
        ],
        &signers,
    )?;
    send_transaction(config, transaction)?;
    Ok(())
}

fn command_set_preferred_validator(
    config: &Config,
    stake_pool_address: &Pubkey,
    preferred_type: PreferredValidatorType,
    vote_address: Option<Pubkey>,
) -> CommandResult {
    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;
    let mut signers = vec![config.fee_payer.as_ref(), config.staker.as_ref()];
    unique_signers!(signers);
    let transaction = checked_transaction_with_signers(
        config,
        &[spl_stake_pool::instruction::set_preferred_validator(
            &spl_stake_pool::id(),
            stake_pool_address,
            &config.staker.pubkey(),
            &stake_pool.validator_list,
            preferred_type,
            vote_address,
        )],
        &signers,
    )?;
    send_transaction(config, transaction)?;
    Ok(())
}

fn add_associated_token_account(
    config: &Config,
    mint: &Pubkey,
    owner: &Pubkey,
    instructions: &mut Vec<Instruction>,
    rent_free_balances: &mut u64,
) -> Pubkey {
    // Account for tokens not specified, creating one
    let account = get_associated_token_address(owner, mint);
    if get_token_account(&config.rpc_client, &account, mint).is_err() {
        println!("Creating associated token account {} to receive stake pool tokens of mint {}, owned by {}", account, mint, owner);

        let min_account_balance = config
            .rpc_client
            .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)
            .unwrap();

        instructions.push(create_associated_token_account(
            &config.fee_payer.pubkey(),
            owner,
            mint,
        ));

        *rent_free_balances += min_account_balance;
    } else {
        println!("Using existing associated token account {} to receive stake pool tokens of mint {}, owned by {}", account, mint, owner);
    }

    account
}

fn command_deposit_stake(
    config: &Config,
    stake_pool_address: &Pubkey,
    stake: &Pubkey,
    pool_token_receiver_account: &Option<Pubkey>,
    referrer_token_account: &Option<Pubkey>,
) -> CommandResult {
    if !config.no_update {
        command_update(config, stake_pool_address, false, false)?;
    }

    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;
    let stake_state = get_stake_state(&config.rpc_client, stake)?;

    if config.verbose {
        println!("Depositing stake account {:?}", stake_state);
    }
    let vote_account = match stake_state {
        StakeState::Stake(_, stake) => Ok(stake.delegation.voter_pubkey),
        _ => Err("Wrong stake account state, must be delegated to validator"),
    }?;

    // Check if this vote account has staking account in the pool
    let validator_list = get_validator_list(&config.rpc_client, &stake_pool.validator_list)?;
    if !validator_list.contains(&vote_account) {
        return Err("Stake account for this validator does not exist in the pool.".into());
    }

    // Calculate validator stake account address linked to the pool
    let (validator_stake_account, _) =
        find_stake_program_address(&spl_stake_pool::id(), &vote_account, stake_pool_address);

    let validator_stake_state = get_stake_state(&config.rpc_client, &validator_stake_account)?;
    println!(
        "Depositing stake {} into stake pool account {}",
        stake, validator_stake_account
    );
    if config.verbose {
        println!("{:?}", validator_stake_state);
    }

    let mut instructions: Vec<Instruction> = vec![];
    let mut signers = vec![config.fee_payer.as_ref(), config.staker.as_ref()];

    let mut total_rent_free_balances: u64 = 0;

    // Create token account if not specified
    let pool_token_receiver_account =
        pool_token_receiver_account.unwrap_or(add_associated_token_account(
            config,
            &stake_pool.pool_mint,
            &config.token_owner.pubkey(),
            &mut instructions,
            &mut total_rent_free_balances,
        ));

    let referrer_token_account = referrer_token_account.unwrap_or(pool_token_receiver_account);

    let pool_withdraw_authority =
        find_withdraw_authority_program_address(&spl_stake_pool::id(), stake_pool_address).0;

    let mut deposit_instructions = if let Some(stake_deposit_authority) = config.depositor.as_ref()
    {
        signers.push(stake_deposit_authority.as_ref());
        if stake_deposit_authority.pubkey() != stake_pool.stake_deposit_authority {
            let error = format!(
                "Invalid deposit authority specified, expected {}, received {}",
                stake_pool.stake_deposit_authority,
                stake_deposit_authority.pubkey()
            );
            return Err(error.into());
        }

        spl_stake_pool::instruction::deposit_stake_with_authority(
            &spl_stake_pool::id(),
            stake_pool_address,
            &stake_pool.validator_list,
            &stake_deposit_authority.pubkey(),
            &pool_withdraw_authority,
            stake,
            &config.staker.pubkey(),
            &validator_stake_account,
            &stake_pool.reserve_stake,
            &pool_token_receiver_account,
            &stake_pool.manager_fee_account,
            &referrer_token_account,
            &stake_pool.pool_mint,
            &spl_token::id(),
        )
    } else {
        spl_stake_pool::instruction::deposit_stake(
            &spl_stake_pool::id(),
            stake_pool_address,
            &stake_pool.validator_list,
            &pool_withdraw_authority,
            stake,
            &config.staker.pubkey(),
            &validator_stake_account,
            &stake_pool.reserve_stake,
            &pool_token_receiver_account,
            &stake_pool.manager_fee_account,
            &referrer_token_account,
            &stake_pool.pool_mint,
            &spl_token::id(),
        )
    };

    instructions.append(&mut deposit_instructions);

    let mut transaction =
        Transaction::new_with_payer(&instructions, Some(&config.fee_payer.pubkey()));

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(
        config,
        total_rent_free_balances + fee_calculator.calculate_fee(transaction.message()),
    )?;
    unique_signers!(signers);
    transaction.sign(&signers, recent_blockhash);
    send_transaction(config, transaction)?;
    Ok(())
}

fn command_deposit_sol(
    config: &Config,
    stake_pool_address: &Pubkey,
    from: &Option<Keypair>,
    pool_token_receiver_account: &Option<Pubkey>,
    referrer_token_account: &Option<Pubkey>,
    amount: f64,
) -> CommandResult {
    if !config.no_update {
        command_update(config, stake_pool_address, false, false)?;
    }

    let amount = native_token::sol_to_lamports(amount);

    // Check withdraw_from balance
    let from_pubkey = from.as_ref().map_or_else(
        || config.fee_payer.try_pubkey().unwrap(),
        |keypair| keypair.try_pubkey().unwrap(),
    );
    let from_balance = config.rpc_client.get_balance(&from_pubkey)?;
    if from_balance < amount {
        return Err(format!(
            "Not enough SOL to deposit into pool: {}.\nMaximum deposit amount is {} SOL.",
            Sol(amount),
            Sol(from_balance)
        )
        .into());
    }

    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;

    let mut instructions: Vec<Instruction> = vec![];

    // ephemeral SOL account just to do the transfer
    let user_sol_transfer = Keypair::new();
    let mut signers = vec![
        config.fee_payer.as_ref(),
        config.staker.as_ref(),
        &user_sol_transfer,
    ];
    if let Some(keypair) = from.as_ref() {
        signers.push(keypair)
    }

    let mut total_rent_free_balances: u64 = 0;

    // Create the ephemeral SOL account
    instructions.push(system_instruction::transfer(
        &from_pubkey,
        &user_sol_transfer.pubkey(),
        amount,
    ));

    // Create token account if not specified
    let pool_token_receiver_account =
        pool_token_receiver_account.unwrap_or(add_associated_token_account(
            config,
            &stake_pool.pool_mint,
            &config.token_owner.pubkey(),
            &mut instructions,
            &mut total_rent_free_balances,
        ));

    let referrer_token_account = referrer_token_account.unwrap_or(pool_token_receiver_account);

    let pool_withdraw_authority =
        find_withdraw_authority_program_address(&spl_stake_pool::id(), stake_pool_address).0;

    let mut deposit_instructions = if let Some(sol_deposit_authority) =
        config.sol_depositor.as_ref()
    {
        let expected_sol_deposit_authority = stake_pool.sol_deposit_authority.ok_or_else(|| {
            "SOL deposit authority specified in arguments but stake pool has none".to_string()
        })?;
        signers.push(sol_deposit_authority.as_ref());
        if sol_deposit_authority.pubkey() != expected_sol_deposit_authority {
            let error = format!(
                "Invalid deposit authority specified, expected {}, received {}",
                expected_sol_deposit_authority,
                sol_deposit_authority.pubkey()
            );
            return Err(error.into());
        }

        spl_stake_pool::instruction::deposit_sol_with_authority(
            &spl_stake_pool::id(),
            stake_pool_address,
            &sol_deposit_authority.pubkey(),
            &pool_withdraw_authority,
            &stake_pool.reserve_stake,
            &user_sol_transfer.pubkey(),
            &pool_token_receiver_account,
            &stake_pool.manager_fee_account,
            &referrer_token_account,
            &stake_pool.pool_mint,
            &spl_token::id(),
            amount,
        )
    } else {
        spl_stake_pool::instruction::deposit_sol(
            &spl_stake_pool::id(),
            stake_pool_address,
            &pool_withdraw_authority,
            &stake_pool.reserve_stake,
            &user_sol_transfer.pubkey(),
            &pool_token_receiver_account,
            &stake_pool.manager_fee_account,
            &referrer_token_account,
            &stake_pool.pool_mint,
            &spl_token::id(),
            amount,
        )
    };

    instructions.append(&mut deposit_instructions);

    let mut transaction =
        Transaction::new_with_payer(&instructions, Some(&config.fee_payer.pubkey()));

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(
        config,
        total_rent_free_balances + fee_calculator.calculate_fee(transaction.message()),
    )?;
    unique_signers!(signers);
    transaction.sign(&signers, recent_blockhash);
    send_transaction(config, transaction)?;
    Ok(())
}

fn command_list(config: &Config, stake_pool_address: &Pubkey) -> CommandResult {
    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;
    let validator_list = get_validator_list(&config.rpc_client, &stake_pool.validator_list)?;
    let pool_mint = get_token_mint(&config.rpc_client, &stake_pool.pool_mint)?;
    let epoch_info = config.rpc_client.get_epoch_info()?;
    let pool_withdraw_authority =
        find_withdraw_authority_program_address(&spl_stake_pool::id(), stake_pool_address).0;
    let sol_deposit_authority = stake_pool
        .sol_deposit_authority
        .map_or("None".into(), |pubkey| pubkey.to_string());

    if config.verbose {
        println!("Stake Pool Info");
        println!("===============");
        println!("Stake Pool: {}", stake_pool_address);
        println!("Validator List: {}", stake_pool.validator_list);
        println!("Manager: {}", stake_pool.manager);
        println!("Staker: {}", stake_pool.staker);
        println!("Depositor: {}", stake_pool.stake_deposit_authority);
        println!("SOL Deposit Authority: {}", sol_deposit_authority);
        println!("Withdraw Authority: {}", pool_withdraw_authority);
        println!("Pool Token Mint: {}", stake_pool.pool_mint);
        println!("Fee Account: {}", stake_pool.manager_fee_account);
    } else {
        println!("Stake Pool: {}", stake_pool_address);
        println!("Pool Token Mint: {}", stake_pool.pool_mint);
    }

    if let Some(preferred_deposit_validator) = stake_pool.preferred_deposit_validator_vote_address {
        println!(
            "Preferred Deposit Validator: {}",
            preferred_deposit_validator
        );
    }
    if let Some(preferred_withdraw_validator) = stake_pool.preferred_withdraw_validator_vote_address
    {
        println!(
            "Preferred Withraw Validator: {}",
            preferred_withdraw_validator
        );
    }

    // Display fees information
    if stake_pool.fee.numerator > 0 && stake_pool.fee.denominator > 0 {
        println!("Epoch Fee: {} of epoch rewards", stake_pool.fee);
    } else {
        println!("Epoch Fee: none");
    }
    if stake_pool.withdrawal_fee.numerator > 0 && stake_pool.withdrawal_fee.denominator > 0 {
        println!(
            "Withdrawal Fee: {} of withdrawal amount",
            stake_pool.withdrawal_fee
        );
    } else {
        println!("Withdrawal Fee: none");
    }
    if stake_pool.stake_deposit_fee.numerator > 0 && stake_pool.stake_deposit_fee.denominator > 0 {
        println!(
            "Stake Deposit Fee: {} of staked amount",
            stake_pool.stake_deposit_fee
        );
    } else {
        println!("Stake Deposit Fee: none");
    }
    if stake_pool.sol_deposit_fee.numerator > 0 && stake_pool.sol_deposit_fee.denominator > 0 {
        println!(
            "SOL Deposit Fee: {} of deposit amount",
            stake_pool.sol_deposit_fee
        );
    } else {
        println!("SOL Deposit Fee: none");
    }
    if stake_pool.sol_referral_fee > 0 {
        println!(
            "SOL Deposit Referral Fee: {}% of SOL Deposit Fee",
            stake_pool.sol_referral_fee
        );
    } else {
        println!("SOL Deposit Referral Fee: none");
    }
    if stake_pool.stake_referral_fee > 0 {
        println!(
            "Stake Deposit Referral Fee: {}% of Stake Deposit Fee",
            stake_pool.stake_referral_fee
        );
    } else {
        println!("Stake Deposit Referral Fee: none");
    }

    if config.verbose {
        println!();
        println!("Stake Accounts");
        println!("--------------");
    }
    let reserve_stake = config.rpc_client.get_account(&stake_pool.reserve_stake)?;
    let minimum_reserve_stake_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(STAKE_STATE_LEN)?
        + 1;
    println!(
        "Reserve Account: {}\tAvailable Balance: {}",
        stake_pool.reserve_stake,
        Sol(reserve_stake.lamports - minimum_reserve_stake_balance),
    );

    for validator in &validator_list.validators {
        if config.verbose {
            let (stake_account_address, _) = find_stake_program_address(
                &spl_stake_pool::id(),
                &validator.vote_account_address,
                stake_pool_address,
            );
            let (transient_stake_account_address, _) = find_transient_stake_program_address(
                &spl_stake_pool::id(),
                &validator.vote_account_address,
                stake_pool_address,
                validator.transient_seed_suffix_start,
            );
            println!(
                "Vote Account: {}\tStake Account: {}\tActive Balance: {}\tTransient Stake Account: {}\tTransient Balance: {}\tLast Update Epoch: {}{}",
                validator.vote_account_address,
                stake_account_address,
                Sol(validator.active_stake_lamports),
                transient_stake_account_address,
                Sol(validator.transient_stake_lamports),
                validator.last_update_epoch,
                if validator.last_update_epoch != epoch_info.epoch {
                    " [UPDATE REQUIRED]"
                } else {
                    ""
                }
            );
        } else {
            println!(
                "Vote Account: {}\tBalance: {}\tLast Update Epoch: {}",
                validator.vote_account_address,
                Sol(validator.stake_lamports()),
                validator.last_update_epoch,
            );
        }
    }

    if config.verbose {
        println!();
    }
    println!(
        "Total Pool Stake: {}{}",
        Sol(stake_pool.total_stake_lamports),
        if stake_pool.last_update_epoch != epoch_info.epoch {
            " [UPDATE REQUIRED]"
        } else {
            ""
        }
    );
    println!(
        "Total Pool Tokens: {}",
        spl_token::amount_to_ui_amount(stake_pool.pool_token_supply, pool_mint.decimals)
    );
    println!(
        "Current Number of Validators: {}",
        validator_list.validators.len()
    );
    println!(
        "Max Number of Validators: {}",
        validator_list.header.max_validators
    );

    Ok(())
}

fn command_update(
    config: &Config,
    stake_pool_address: &Pubkey,
    force: bool,
    no_merge: bool,
) -> CommandResult {
    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;
    let epoch_info = config.rpc_client.get_epoch_info()?;

    if stake_pool.last_update_epoch == epoch_info.epoch {
        if force {
            println!("Update not required, but --force flag specified, so doing it anyway");
        } else {
            println!("Update not required");
            return Ok(());
        }
    }

    let validator_list = get_validator_list(&config.rpc_client, &stake_pool.validator_list)?;

    let (mut update_list_instructions, final_instructions) =
        spl_stake_pool::instruction::update_stake_pool(
            &spl_stake_pool::id(),
            &stake_pool,
            &validator_list,
            stake_pool_address,
            no_merge,
        );

    let update_list_instructions_len = update_list_instructions.len();
    if update_list_instructions_len > 0 {
        let last_instruction = update_list_instructions.split_off(update_list_instructions_len - 1);
        // send the first ones without waiting
        for instruction in update_list_instructions {
            let transaction = checked_transaction_with_signers(
                config,
                &[instruction],
                &[config.fee_payer.as_ref()],
            )?;
            send_transaction_no_wait(config, transaction)?;
        }

        // wait on the last one
        let transaction = checked_transaction_with_signers(
            config,
            &last_instruction,
            &[config.fee_payer.as_ref()],
        )?;
        send_transaction(config, transaction)?;
    }
    let transaction = checked_transaction_with_signers(
        config,
        &final_instructions,
        &[config.fee_payer.as_ref()],
    )?;
    send_transaction(config, transaction)?;

    Ok(())
}

#[derive(PartialEq, Debug)]
struct WithdrawAccount {
    stake_address: Pubkey,
    vote_address: Option<Pubkey>,
    pool_amount: u64,
}

fn prepare_withdraw_accounts(
    rpc_client: &RpcClient,
    stake_pool: &StakePool,
    pool_withdraw_authority: &Pubkey,
    pool_amount: u64,
) -> Result<Vec<WithdrawAccount>, Error> {
    let mut accounts =
        get_stake_accounts_by_withdraw_authority(rpc_client, pool_withdraw_authority)?;
    if accounts.is_empty() {
        return Err("No accounts found.".to_string().into());
    }
    let min_balance = rpc_client.get_minimum_balance_for_rent_exemption(STAKE_STATE_LEN)? + 1;
    let pool_mint = get_token_mint(rpc_client, &stake_pool.pool_mint)?;

    // Sort from highest to lowest balance
    accounts.sort_by(|a, b| b.1.cmp(&a.1));

    // Prepare the list of accounts to withdraw from
    let mut withdraw_from: Vec<WithdrawAccount> = vec![];
    let mut remaining_amount = pool_amount;

    // Go through available accounts and withdraw from largest to smallest
    for (stake_address, lamports, stake) in accounts {
        if lamports <= min_balance {
            continue;
        }
        let available_for_withdrawal = stake_pool
            .calc_lamports_withdraw_amount(lamports - *MIN_STAKE_BALANCE)
            .unwrap();
        let pool_amount = u64::min(available_for_withdrawal, remaining_amount);

        // Those accounts will be withdrawn completely with `claim` instruction
        withdraw_from.push(WithdrawAccount {
            stake_address,
            vote_address: stake.delegation().map(|x| x.voter_pubkey),
            pool_amount,
        });
        remaining_amount -= pool_amount;

        if remaining_amount == 0 {
            break;
        }
    }

    // Not enough stake to withdraw the specified amount
    if remaining_amount > 0 {
        return Err(format!(
            "No stake accounts found in this pool with enough balance to withdraw {} pool tokens.",
            spl_token::amount_to_ui_amount(pool_amount, pool_mint.decimals)
        )
        .into());
    }

    Ok(withdraw_from)
}

fn command_withdraw(
    config: &Config,
    stake_pool_address: &Pubkey,
    use_reserve: bool,
    vote_account_address: &Option<Pubkey>,
    stake_receiver_param: &Option<Pubkey>,
    pool_token_account: &Option<Pubkey>,
    pool_amount: f64,
) -> CommandResult {
    if !config.no_update {
        command_update(config, stake_pool_address, false, false)?;
    }

    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;
    let pool_mint = get_token_mint(&config.rpc_client, &stake_pool.pool_mint)?;
    let pool_amount = spl_token::ui_amount_to_amount(pool_amount, pool_mint.decimals);

    let pool_withdraw_authority =
        find_withdraw_authority_program_address(&spl_stake_pool::id(), stake_pool_address).0;

    let pool_token_account = pool_token_account.unwrap_or(get_associated_token_address(
        &config.token_owner.pubkey(),
        &stake_pool.pool_mint,
    ));
    let token_account = get_token_account(
        &config.rpc_client,
        &pool_token_account,
        &stake_pool.pool_mint,
    )?;

    // Check withdraw_from balance
    if token_account.amount < pool_amount {
        return Err(format!(
            "Not enough token balance to withdraw {} pool tokens.\nMaximum withdraw amount is {} pool tokens.",
            spl_token::amount_to_ui_amount(pool_amount, pool_mint.decimals),
            spl_token::amount_to_ui_amount(token_account.amount, pool_mint.decimals)
        )
        .into());
    }

    let withdraw_accounts = if use_reserve {
        vec![WithdrawAccount {
            stake_address: stake_pool.reserve_stake,
            vote_address: None,
            pool_amount,
        }]
    } else if let Some(vote_account_address) = vote_account_address {
        let (stake_account_address, _) = find_stake_program_address(
            &spl_stake_pool::id(),
            vote_account_address,
            stake_pool_address,
        );
        let stake_account = config.rpc_client.get_account(&stake_account_address)?;
        let available_for_withdrawal = stake_pool
            .calc_lamports_withdraw_amount(stake_account.lamports - *MIN_STAKE_BALANCE)
            .unwrap();
        if available_for_withdrawal < pool_amount {
            return Err(format!(
                "Not enough lamports available for withdrawal from {}, {} asked, {} available",
                stake_account_address, pool_amount, available_for_withdrawal
            )
            .into());
        }
        vec![WithdrawAccount {
            stake_address: stake_account_address,
            vote_address: Some(*vote_account_address),
            pool_amount,
        }]
    } else {
        // Get the list of accounts to withdraw from
        prepare_withdraw_accounts(
            &config.rpc_client,
            &stake_pool,
            &pool_withdraw_authority,
            pool_amount,
        )?
    };

    // Construct transaction to withdraw from withdraw_accounts account list
    let mut instructions: Vec<Instruction> = vec![];
    let user_transfer_authority = Keypair::new(); // ephemeral keypair just to do the transfer
    let mut signers = vec![
        config.fee_payer.as_ref(),
        config.token_owner.as_ref(),
        &user_transfer_authority,
    ];
    let mut new_stake_keypairs = vec![];

    instructions.push(
        // Approve spending token
        spl_token::instruction::approve(
            &spl_token::id(),
            &pool_token_account,
            &user_transfer_authority.pubkey(),
            &config.token_owner.pubkey(),
            &[],
            pool_amount,
        )?,
    );

    let mut total_rent_free_balances = 0;

    // Go through prepared accounts and withdraw/claim them
    for withdraw_account in withdraw_accounts {
        // Convert pool tokens amount to lamports
        let sol_withdraw_amount = stake_pool
            .calc_lamports_withdraw_amount(withdraw_account.pool_amount)
            .unwrap();

        if let Some(vote_address) = withdraw_account.vote_address {
            println!(
                "Withdrawing {}, or {} pool tokens, from stake account {}, delegated to {}",
                Sol(sol_withdraw_amount),
                spl_token::amount_to_ui_amount(withdraw_account.pool_amount, pool_mint.decimals),
                withdraw_account.stake_address,
                vote_address,
            );
        } else {
            println!(
                "Withdrawing {}, or {} pool tokens, from stake account {}",
                Sol(sol_withdraw_amount),
                spl_token::amount_to_ui_amount(withdraw_account.pool_amount, pool_mint.decimals),
                withdraw_account.stake_address,
            );
        }

        // Use separate mutable variable because withdraw might create a new account
        let stake_receiver = stake_receiver_param.unwrap_or_else(|| {
            // Account for tokens not specified, creating one
            let stake_receiver_account = Keypair::new(); // Will be added to signers if creating new account
            let stake_receiver_pubkey = stake_receiver_account.pubkey();
            println!(
                "Creating account to receive stake {}",
                stake_receiver_pubkey
            );

            let stake_receiver_account_balance = config
                .rpc_client
                .get_minimum_balance_for_rent_exemption(STAKE_STATE_LEN)
                .unwrap();

            instructions.push(
                // Creating new account
                system_instruction::create_account(
                    &config.fee_payer.pubkey(),
                    &stake_receiver_pubkey,
                    stake_receiver_account_balance,
                    STAKE_STATE_LEN as u64,
                    &stake_program::id(),
                ),
            );

            total_rent_free_balances += stake_receiver_account_balance;
            new_stake_keypairs.push(stake_receiver_account);
            stake_receiver_pubkey
        });

        instructions.push(spl_stake_pool::instruction::withdraw_stake(
            &spl_stake_pool::id(),
            stake_pool_address,
            &stake_pool.validator_list,
            &pool_withdraw_authority,
            &withdraw_account.stake_address,
            &stake_receiver,
            &config.staker.pubkey(),
            &user_transfer_authority.pubkey(),
            &pool_token_account,
            &stake_pool.manager_fee_account,
            &stake_pool.pool_mint,
            &spl_token::id(),
            withdraw_account.pool_amount,
        ));
    }

    let mut transaction =
        Transaction::new_with_payer(&instructions, Some(&config.fee_payer.pubkey()));

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(
        config,
        total_rent_free_balances + fee_calculator.calculate_fee(transaction.message()),
    )?;
    for new_stake_keypair in &new_stake_keypairs {
        signers.push(new_stake_keypair);
    }
    unique_signers!(signers);
    transaction.sign(&signers, recent_blockhash);
    send_transaction(config, transaction)?;
    Ok(())
}

fn command_set_manager(
    config: &Config,
    stake_pool_address: &Pubkey,
    new_manager: &Option<Keypair>,
    new_fee_receiver: &Option<Pubkey>,
) -> CommandResult {
    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;

    // If new accounts are missing in the arguments use the old ones
    let (new_manager_pubkey, mut signers): (Pubkey, Vec<&dyn Signer>) = match new_manager {
        None => (stake_pool.manager, vec![]),
        Some(value) => (value.pubkey(), vec![value]),
    };
    let new_fee_receiver = match new_fee_receiver {
        None => stake_pool.manager_fee_account,
        Some(value) => {
            // Check for fee receiver being a valid token account and have to same mint as the stake pool
            let token_account =
                get_token_account(&config.rpc_client, value, &stake_pool.pool_mint)?;
            if token_account.mint != stake_pool.pool_mint {
                return Err("Fee receiver account belongs to a different mint"
                    .to_string()
                    .into());
            }
            *value
        }
    };

    signers.append(&mut vec![
        config.fee_payer.as_ref(),
        config.manager.as_ref(),
    ]);
    unique_signers!(signers);
    let transaction = checked_transaction_with_signers(
        config,
        &[spl_stake_pool::instruction::set_manager(
            &spl_stake_pool::id(),
            stake_pool_address,
            &config.manager.pubkey(),
            &new_manager_pubkey,
            &new_fee_receiver,
        )],
        &signers,
    )?;
    send_transaction(config, transaction)?;
    Ok(())
}

fn command_set_staker(
    config: &Config,
    stake_pool_address: &Pubkey,
    new_staker: &Pubkey,
) -> CommandResult {
    let mut signers = vec![config.fee_payer.as_ref(), config.manager.as_ref()];
    unique_signers!(signers);
    let transaction = checked_transaction_with_signers(
        config,
        &[spl_stake_pool::instruction::set_staker(
            &spl_stake_pool::id(),
            stake_pool_address,
            &config.manager.pubkey(),
            new_staker,
        )],
        &signers,
    )?;
    send_transaction(config, transaction)?;
    Ok(())
}

fn command_set_deposit_authority(
    config: &Config,
    stake_pool_address: &Pubkey,
    new_sol_deposit_authority: Option<Pubkey>,
    deposit_type: DepositType,
) -> CommandResult {
    let mut signers = vec![config.fee_payer.as_ref(), config.manager.as_ref()];
    unique_signers!(signers);
    let transaction = checked_transaction_with_signers(
        config,
        &[spl_stake_pool::instruction::set_deposit_authority(
            &spl_stake_pool::id(),
            stake_pool_address,
            &config.manager.pubkey(),
            new_sol_deposit_authority.as_ref(),
            deposit_type,
        )],
        &signers,
    )?;
    send_transaction(config, transaction)?;
    Ok(())
}

fn command_set_fee(
    config: &Config,
    stake_pool_address: &Pubkey,
    new_fee: FeeType,
) -> CommandResult {
    let mut signers = vec![config.fee_payer.as_ref(), config.manager.as_ref()];
    unique_signers!(signers);
    let transaction = checked_transaction_with_signers(
        config,
        &[spl_stake_pool::instruction::set_fee(
            &spl_stake_pool::id(),
            stake_pool_address,
            &config.manager.pubkey(),
            new_fee,
        )],
        &signers,
    )?;
    send_transaction(config, transaction)?;
    Ok(())
}

fn command_list_all_pools(config: &Config) -> CommandResult {
    let all_pools = get_stake_pools(&config.rpc_client)?;
    let count = all_pools.len();
    for (address, stake_pool, validator_list) in all_pools {
        println!(
            "Address: {}\tManager: {}\tLamports: {}\tPool tokens: {}\tValidators: {}",
            address,
            stake_pool.manager,
            stake_pool.total_stake_lamports,
            stake_pool.pool_token_supply,
            validator_list.validators.len()
        );
    }
    println!("Total number of pools: {}", count);
    Ok(())
}

fn main() {
    solana_logger::setup_with_default("solana=info");

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
                arg.default_value(config_file)
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
            Arg::with_name("dry_run")
                .long("dry-run")
                .takes_value(false)
                .global(true)
                .help("Simulate transaction instead of executing"),
        )
        .arg(
            Arg::with_name("no_update")
                .long("no-update")
                .takes_value(false)
                .global(true)
                .help("Do not automatically update the stake pool if needed"),
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
            Arg::with_name("staker")
                .long("staker")
                .value_name("KEYPAIR")
                .validator(is_keypair)
                .takes_value(true)
                .help(
                    "Specify the stake pool staker. \
                     This may be a keypair file, the ASK keyword. \
                     Defaults to the client keypair.",
                ),
        )
        .arg(
            Arg::with_name("manager")
                .long("manager")
                .value_name("KEYPAIR")
                .validator(is_keypair)
                .takes_value(true)
                .help(
                    "Specify the stake pool manager. \
                     This may be a keypair file, the ASK keyword. \
                     Defaults to the client keypair.",
                ),
        )
        .arg(
            Arg::with_name("depositor")
                .long("depositor")
                .value_name("KEYPAIR")
                .validator(is_keypair)
                .takes_value(true)
                .help(
                    "Specify the stake pool depositor. \
                     This may be a keypair file, the ASK keyword.",
                ),
        )
        .arg(
            Arg::with_name("token_owner")
                .long("token-owner")
                .value_name("KEYPAIR")
                .validator(is_keypair)
                .takes_value(true)
                .help(
                    "Specify the owner of the pool token account. \
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
        .subcommand(SubCommand::with_name("create-pool")
            .about("Create a new stake pool")
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
            .arg(
                Arg::with_name("withdrawal_fee_numerator")
                    .long("withdrawal-fee-numerator")
                    .validator(is_parsable::<u64>)
                    .value_name("NUMERATOR")
                    .takes_value(true)
                    .requires("withdrawal_fee_denominator")
                    .help("Withdrawal fee numerator, fee amount is numerator divided by denominator [default: 0]"),
            ).arg(
                Arg::with_name("withdrawal_fee_denominator")
                    .long("withdrawal-fee-denominator")
                    .validator(is_parsable::<u64>)
                    .value_name("DENOMINATOR")
                    .takes_value(true)
                    .requires("withdrawal_fee_numerator")
                    .help("Withdrawal fee denominator, fee amount is numerator divided by denominator [default: 0]"),
            )
            .arg(
                Arg::with_name("deposit_fee_numerator")
                    .long("deposit-fee-numerator")
                    .validator(is_parsable::<u64>)
                    .value_name("NUMERATOR")
                    .takes_value(true)
                    .requires("deposit_fee_denominator")
                    .help("Deposit fee numerator, fee amount is numerator divided by denominator [default: 0]"),
            ).arg(
                Arg::with_name("deposit_fee_denominator")
                    .long("deposit-fee-denominator")
                    .validator(is_parsable::<u64>)
                    .value_name("DENOMINATOR")
                    .takes_value(true)
                    .requires("deposit_fee_numerator")
                    .help("Deposit fee denominator, fee amount is numerator divided by denominator [default: 0]"),
            )
            .arg(
                Arg::with_name("referral_fee")
                    .long("referral-fee")
                    .validator(is_valid_percentage)
                    .value_name("FEE_PERCENTAGE")
                    .takes_value(true)
                    .help("Referral fee percentage, maximum 100"),
            )
            .arg(
                Arg::with_name("max_validators")
                    .long("max-validators")
                    .short("m")
                    .validator(is_parsable::<u32>)
                    .value_name("NUMBER")
                    .takes_value(true)
                    .required(true)
                    .help("Max number of validators included in the stake pool"),
            )
            .arg(
                Arg::with_name("stake_deposit_authority")
                    .long("stake-deposit-authority")
                    .short("a")
                    .validator(is_keypair_or_ask_keyword)
                    .value_name("STAKE_DEPOSIT_AUTHORITY_KEYPAIR")
                    .takes_value(true)
                    .help("Deposit authority required to sign all deposits into the stake pool"),
            )
            .arg(
                Arg::with_name("pool_keypair")
                    .long("pool-keypair")
                    .short("p")
                    .validator(is_keypair_or_ask_keyword)
                    .value_name("PATH")
                    .takes_value(true)
                    .help("Stake pool keypair [default: new keypair]"),
            )
            .arg(
                Arg::with_name("mint_keypair")
                    .long("mint-keypair")
                    .validator(is_keypair_or_ask_keyword)
                    .value_name("PATH")
                    .takes_value(true)
                    .help("Stake pool mint keypair [default: new keypair]"),
            )
            .arg(
                Arg::with_name("reserve_keypair")
                    .long("reserve-keypair")
                    .validator(is_keypair_or_ask_keyword)
                    .value_name("PATH")
                    .takes_value(true)
                    .help("Stake pool reserve keypair [default: new keypair]"),
            )
        )
        .subcommand(SubCommand::with_name("create-validator-stake")
            .about("Create a new stake account to use with the pool. Must be signed by the pool staker.")
            .arg(
                Arg::with_name("pool")
                    .index(1)
                    .validator(is_pubkey)
                    .value_name("POOL_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address"),
            )
            .arg(
                Arg::with_name("vote_account")
                    .index(2)
                    .validator(is_pubkey)
                    .value_name("VOTE_ACCOUNT_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("The validator vote account that this stake will be delegated to"),
            )
        )
        .subcommand(SubCommand::with_name("add-validator")
            .about("Add validator account to the stake pool. Must be signed by the pool staker.")
            .arg(
                Arg::with_name("pool")
                    .index(1)
                    .validator(is_pubkey)
                    .value_name("POOL_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address"),
            )
            .arg(
                Arg::with_name("vote_account")
                    .index(2)
                    .validator(is_pubkey)
                    .value_name("VOTE_ACCOUNT_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("The validator vote account that the stake is delegated to"),
            )
        )
        .subcommand(SubCommand::with_name("remove-validator")
            .about("Remove validator account from the stake pool. Must be signed by the pool staker.")
            .arg(
                Arg::with_name("pool")
                    .index(1)
                    .validator(is_pubkey)
                    .value_name("POOL_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address"),
            )
            .arg(
                Arg::with_name("vote_account")
                    .index(2)
                    .validator(is_pubkey)
                    .value_name("VOTE_ACCOUNT_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Vote account for the validator to remove from the pool"),
            )
            .arg(
                Arg::with_name("new_authority")
                    .long("new-authority")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .help("New authority to set as Staker and Withdrawer in the stake account removed from the pool.
                          Defaults to the client keypair."),
            )
        )
        .subcommand(SubCommand::with_name("increase-validator-stake")
            .about("Increase stake to a validator, drawing from the stake pool reserve. Must be signed by the pool staker.")
            .arg(
                Arg::with_name("pool")
                    .index(1)
                    .validator(is_pubkey)
                    .value_name("POOL_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address"),
            )
            .arg(
                Arg::with_name("vote_account")
                    .index(2)
                    .validator(is_pubkey)
                    .value_name("VOTE_ACCOUNT_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Vote account for the validator to increase stake to"),
            )
            .arg(
                Arg::with_name("amount")
                    .index(3)
                    .validator(is_amount)
                    .value_name("AMOUNT")
                    .takes_value(true)
                    .help("Amount in SOL to add to the validator stake account. Must be at least the rent-exempt amount for a stake plus 1 SOL for merging."),
            )
        )
        .subcommand(SubCommand::with_name("decrease-validator-stake")
            .about("Decrease stake to a validator, splitting from the active stake. Must be signed by the pool staker.")
            .arg(
                Arg::with_name("pool")
                    .index(1)
                    .validator(is_pubkey)
                    .value_name("POOL_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address"),
            )
            .arg(
                Arg::with_name("vote_account")
                    .index(2)
                    .validator(is_pubkey)
                    .value_name("VOTE_ACCOUNT_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Vote account for the validator to decrease stake from"),
            )
            .arg(
                Arg::with_name("amount")
                    .index(3)
                    .validator(is_amount)
                    .value_name("AMOUNT")
                    .takes_value(true)
                    .help("Amount in SOL to remove from the validator stake account. Must be at least the rent-exempt amount for a stake."),
            )
        )
        .subcommand(SubCommand::with_name("set-preferred-validator")
            .about("Set the preferred validator for deposits or withdrawals. Must be signed by the pool staker.")
            .arg(
                Arg::with_name("pool")
                    .index(1)
                    .validator(is_pubkey)
                    .value_name("POOL_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address"),
            )
            .arg(
                Arg::with_name("preferred_type")
                    .index(2)
                    .value_name("OPERATION")
                    .possible_values(&["deposit", "withdraw"]) // PreferredValidatorType enum
                    .takes_value(true)
                    .required(true)
                    .help("Operation for which to restrict the validator"),
            )
            .arg(
                Arg::with_name("vote_account")
                    .long("vote-account")
                    .validator(is_pubkey)
                    .value_name("VOTE_ACCOUNT_ADDRESS")
                    .takes_value(true)
                    .help("Vote account for the validator that users must deposit into."),
            )
            .arg(
                Arg::with_name("unset")
                    .long("unset")
                    .takes_value(false)
                    .help("Unset the preferred validator."),
            )
            .group(ArgGroup::with_name("validator")
                .arg("vote_account")
                .arg("unset")
                .required(true)
            )
        )
        .subcommand(SubCommand::with_name("deposit-stake")
            .about("Deposit active stake account into the stake pool in exchange for pool tokens")
            .arg(
                Arg::with_name("pool")
                    .index(1)
                    .validator(is_pubkey)
                    .value_name("POOL_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address"),
            )
            .arg(
                Arg::with_name("stake_account")
                    .index(2)
                    .validator(is_pubkey)
                    .value_name("STAKE_ACCOUNT_ADDRESS")
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
                    .help("Account to receive the minted pool tokens. \
                          Defaults to the token-owner's associated pool token account. \
                          Creates the account if it does not exist."),
            )
            .arg(
                Arg::with_name("referrer")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .help("Pool token account to receive the referral fees for deposits. \
                          Defaults to the token receiver."),
            )
        )
        .subcommand(SubCommand::with_name("deposit-sol")
            .about("Deposit SOL into the stake pool in exchange for pool tokens")
            .arg(
                Arg::with_name("pool")
                    .index(1)
                    .validator(is_pubkey)
                    .value_name("POOL_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address"),
            ).arg(
                Arg::with_name("amount")
                    .index(2)
                    .validator(is_amount)
                    .value_name("AMOUNT")
                    .takes_value(true)
                    .help("Amount in SOL to deposit into the stake pool reserve account."),
            )
            .arg(
                Arg::with_name("from")
                    .long("from")
                    .validator(is_keypair_or_ask_keyword)
                    .value_name("KEYPAIR")
                    .takes_value(true)
                    .help("Source account of funds. \
                          Defaults to the fee payer."),
            )
            .arg(
                Arg::with_name("token_receiver")
                    .long("token-receiver")
                    .validator(is_pubkey)
                    .value_name("POOL_TOKEN_RECEIVER_ADDRESS")
                    .takes_value(true)
                    .help("Account to receive the minted pool tokens. \
                          Defaults to the token-owner's associated pool token account. \
                          Creates the account if it does not exist."),
            )
            .arg(
                Arg::with_name("referrer")
                    .long("referrer")
                    .validator(is_pubkey)
                    .value_name("REFERRER_TOKEN_ADDRESS")
                    .takes_value(true)
                    .help("Account to receive the referral fees for deposits. \
                          Defaults to the token receiver."),
            )
        )
        .subcommand(SubCommand::with_name("list")
            .about("List stake accounts managed by this pool")
            .arg(
                Arg::with_name("pool")
                    .index(1)
                    .validator(is_pubkey)
                    .value_name("POOL_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address."),
            )
        )
        .subcommand(SubCommand::with_name("update")
            .about("Updates all balances in the pool after validator stake accounts receive rewards.")
            .arg(
                Arg::with_name("pool")
                    .index(1)
                    .validator(is_pubkey)
                    .value_name("POOL_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address."),
            )
            .arg(
                Arg::with_name("force")
                    .long("force")
                    .takes_value(false)
                    .help("Update all balances, even if it has already been performed this epoch."),
            )
            .arg(
                Arg::with_name("no_merge")
                    .long("no-merge")
                    .takes_value(false)
                    .help("Do not automatically merge transient stakes. Useful if the stake pool is in an expected state, but the balances still need to be updated."),
            )
        )
        .subcommand(SubCommand::with_name("withdraw-stake")
            .about("Withdraw amount from the stake pool")
            .arg(
                Arg::with_name("pool")
                    .index(1)
                    .validator(is_pubkey)
                    .value_name("POOL_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address."),
            )
            .arg(
                Arg::with_name("amount")
                    .index(2)
                    .validator(is_amount)
                    .value_name("AMOUNT")
                    .takes_value(true)
                    .required(true)
                    .help("Amount of pool tokens to withdraw for activated stake."),
            )
            .arg(
                Arg::with_name("pool_account")
                    .long("pool-account")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .help("Pool token account to withdraw tokens from. Defaults to the token-owner's associated token account."),
            )
            .arg(
                Arg::with_name("stake_receiver")
                    .long("stake-receiver")
                    .validator(is_pubkey)
                    .value_name("STAKE_ACCOUNT_ADDRESS")
                    .takes_value(true)
                    .requires("withdraw_from")
                    .help("Stake account to receive SOL from the stake pool. Defaults to a new stake account."),
            )
            .arg(
                Arg::with_name("vote_account")
                    .long("vote-account")
                    .validator(is_pubkey)
                    .value_name("VOTE_ACCOUNT_ADDRESS")
                    .takes_value(true)
                    .help("Validator to withdraw from. Defaults to the largest validator stakes in the pool."),
            )
            .arg(
                Arg::with_name("use_reserve")
                    .long("use-reserve")
                    .takes_value(false)
                    .help("Withdraw from the stake pool's reserve. Only possible if all validator stakes are at the minimum possible amount."),
            )
            .group(ArgGroup::with_name("withdraw_from")
                .arg("use_reserve")
                .arg("vote_account")
            )
        )
        .subcommand(SubCommand::with_name("set-manager")
            .about("Change manager or fee receiver account for the stake pool. Must be signed by the current manager.")
            .arg(
                Arg::with_name("pool")
                    .index(1)
                    .validator(is_pubkey)
                    .value_name("POOL_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address."),
            )
            .arg(
                Arg::with_name("new_manager")
                    .long("new-manager")
                    .validator(is_keypair)
                    .value_name("KEYPAIR")
                    .takes_value(true)
                    .help("Keypair for the new stake pool manager."),
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
                .arg("new_manager")
                .arg("new_fee_receiver")
                .required(true)
                .multiple(true)
            )
        )
        .subcommand(SubCommand::with_name("set-staker")
            .about("Change staker account for the stake pool. Must be signed by the manager or current staker.")
            .arg(
                Arg::with_name("pool")
                    .index(1)
                    .validator(is_pubkey)
                    .value_name("POOL_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address."),
            )
            .arg(
                Arg::with_name("new_staker")
                    .index(2)
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .help("Public key for the new stake pool staker."),
            )
        )
        .subcommand(SubCommand::with_name("set-deposit-authority")
            .about("Change deposit authority account for the stake pool. Must be signed by the manager.")
            .arg(
                Arg::with_name("pool")
                    .index(1)
                    .validator(is_pubkey)
                    .value_name("POOL_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address."),
            )
            .arg(
                Arg::with_name("deposit_type")
                    .index(2)
                    .value_name("DEPOSIT_TYPE")
                    .possible_values(&["stake", "sol"]) // DepositType enum
                    .takes_value(true)
                    .required(true)
                    .help("Deposit type to be updated."),
            )
            .arg(
                Arg::with_name("new_stake_deposit_authority")
                    .index(3)
                    .validator(is_pubkey)
                    .value_name("ADDRESS_OR_NONE")
                    .takes_value(true)
                    .help("'none', or a public key for the new stake pool sol deposit authority."),
            )
            .arg(
                Arg::with_name("unset")
                    .long("unset")
                    .takes_value(false)
                    .help("Unset the stake deposit authority. The program will use a program derived address.")
            )
            .group(ArgGroup::with_name("validator")
                .arg("new_stake_deposit_authority")
                .arg("unset")
                .required(true)
            )
        )
        .subcommand(SubCommand::with_name("set-fee")
            .about("Change the [management/withdrawal/stake deposit/sol deposit] fee assessed by the stake pool. Must be signed by the manager.")
            .arg(
                Arg::with_name("pool")
                    .index(1)
                    .validator(is_pubkey)
                    .value_name("POOL_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address."),
            )
            .arg(Arg::with_name("fee_type")
                .index(2)
                .value_name("FEE_TYPE")
                .possible_values(&["epoch", "stake-deposit", "sol-deposit", "withdrawal"]) // FeeType enum
                .takes_value(true)
                .required(true)
                .help("Fee type to be updated."),
            )
            .arg(
                Arg::with_name("fee_numerator")
                    .index(3)
                    .validator(is_parsable::<u64>)
                    .value_name("NUMERATOR")
                    .takes_value(true)
                    .required(true)
                    .help("Fee numerator, fee amount is numerator divided by denominator."),
            )
            .arg(
                Arg::with_name("fee_denominator")
                    .index(4)
                    .validator(is_parsable::<u64>)
                    .value_name("DENOMINATOR")
                    .takes_value(true)
                    .required(true)
                    .help("Fee denominator, fee amount is numerator divided by denominator."),
            )
        )
        .subcommand(SubCommand::with_name("set-referral-fee")
            .about("Change the referral fee assessed by the stake pool for stake deposits. Must be signed by the manager.")
            .arg(
                Arg::with_name("pool")
                    .index(1)
                    .validator(is_pubkey)
                    .value_name("POOL_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake pool address."),
            )
            .arg(Arg::with_name("fee_type")
                .index(2)
                .value_name("FEE_TYPE")
                .possible_values(&["stake", "sol"]) // FeeType enum, kind of
                .takes_value(true)
                .required(true)
                .help("Fee type to be updated."),
            )
            .arg(
                Arg::with_name("fee")
                    .index(3)
                    .validator(is_valid_percentage)
                    .value_name("FEE_PERCENTAGE")
                    .takes_value(true)
                    .required(true)
                    .help("Fee percentage, maximum 100"),
            )
        )
        .subcommand(SubCommand::with_name("list-all")
            .about("List information about all stake pools")
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

        let staker = get_signer(
            &matches,
            "staker",
            &cli_config.keypair_path,
            &mut wallet_manager,
        );

        let depositor = if matches.is_present("depositor") {
            Some(get_signer(
                &matches,
                "depositor",
                &cli_config.keypair_path,
                &mut wallet_manager,
            ))
        } else {
            None
        };
        let sol_depositor = if matches.is_present("sol_depositor") {
            Some(get_signer(
                &matches,
                "sol_depositor",
                &cli_config.keypair_path,
                &mut wallet_manager,
            ))
        } else {
            None
        };
        let manager = get_signer(
            &matches,
            "manager",
            &cli_config.keypair_path,
            &mut wallet_manager,
        );
        let token_owner = get_signer(
            &matches,
            "token_owner",
            &cli_config.keypair_path,
            &mut wallet_manager,
        );
        let fee_payer = get_signer(
            &matches,
            "fee_payer",
            &cli_config.keypair_path,
            &mut wallet_manager,
        );
        let verbose = matches.is_present("verbose");
        let dry_run = matches.is_present("dry_run");
        let no_update = matches.is_present("no_update");

        Config {
            rpc_client: RpcClient::new_with_commitment(json_rpc_url, CommitmentConfig::confirmed()),
            verbose,
            manager,
            staker,
            depositor,
            sol_depositor,
            token_owner,
            fee_payer,
            dry_run,
            no_update,
        }
    };

    let _ = match matches.subcommand() {
        ("create-pool", Some(arg_matches)) => {
            let stake_deposit_authority = keypair_of(arg_matches, "stake_deposit_authority");
            let numerator = value_t_or_exit!(arg_matches, "fee_numerator", u64);
            let denominator = value_t_or_exit!(arg_matches, "fee_denominator", u64);
            let w_numerator = value_t!(arg_matches, "withdrawal_fee_numerator", u64);
            let w_denominator = value_t!(arg_matches, "withdrawal_fee_denominator", u64);
            let d_numerator = value_t!(arg_matches, "deposit_fee_numerator", u64);
            let d_denominator = value_t!(arg_matches, "deposit_fee_denominator", u64);
            let referral_fee = value_t!(arg_matches, "referral_fee", u8);
            let max_validators = value_t_or_exit!(arg_matches, "max_validators", u32);
            let pool_keypair = keypair_of(arg_matches, "pool_keypair");
            let mint_keypair = keypair_of(arg_matches, "mint_keypair");
            let reserve_keypair = keypair_of(arg_matches, "reserve_keypair");
            command_create_pool(
                &config,
                stake_deposit_authority,
                Fee {
                    denominator,
                    numerator,
                },
                Fee {
                    numerator: w_numerator.unwrap_or(0),
                    denominator: w_denominator.unwrap_or(0),
                },
                Fee {
                    numerator: d_numerator.unwrap_or(0),
                    denominator: d_denominator.unwrap_or(0),
                },
                referral_fee.unwrap_or(0),
                max_validators,
                pool_keypair,
                mint_keypair,
                reserve_keypair,
            )
        }
        ("create-validator-stake", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let vote_account_address = pubkey_of(arg_matches, "vote_account").unwrap();
            command_vsa_create(&config, &stake_pool_address, &vote_account_address)
        }
        ("add-validator", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let vote_account_address = pubkey_of(arg_matches, "vote_account").unwrap();
            command_vsa_add(&config, &stake_pool_address, &vote_account_address)
        }
        ("remove-validator", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let vote_account = pubkey_of(arg_matches, "vote_account").unwrap();
            let new_authority: Option<Pubkey> = pubkey_of(arg_matches, "new_authority");
            command_vsa_remove(&config, &stake_pool_address, &vote_account, &new_authority)
        }
        ("increase-validator-stake", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let vote_account = pubkey_of(arg_matches, "vote_account").unwrap();
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            command_increase_validator_stake(&config, &stake_pool_address, &vote_account, amount)
        }
        ("decrease-validator-stake", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let vote_account = pubkey_of(arg_matches, "vote_account").unwrap();
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            command_decrease_validator_stake(&config, &stake_pool_address, &vote_account, amount)
        }
        ("set-preferred-validator", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let preferred_type = match arg_matches.value_of("preferred_type").unwrap() {
                "deposit" => PreferredValidatorType::Deposit,
                "withdraw" => PreferredValidatorType::Withdraw,
                _ => unreachable!(),
            };
            let vote_account = pubkey_of(arg_matches, "vote_account");
            let _unset = arg_matches.is_present("unset");
            // since unset and vote_account can't both be set, if unset is set
            // then vote_account will be None, which is valid for the program
            command_set_preferred_validator(
                &config,
                &stake_pool_address,
                preferred_type,
                vote_account,
            )
        }
        ("deposit-stake", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let stake_account = pubkey_of(arg_matches, "stake_account").unwrap();
            let token_receiver: Option<Pubkey> = pubkey_of(arg_matches, "token_receiver");
            let referrer: Option<Pubkey> = pubkey_of(arg_matches, "referrer");
            command_deposit_stake(
                &config,
                &stake_pool_address,
                &stake_account,
                &token_receiver,
                &referrer,
            )
        }
        ("deposit-sol", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let token_receiver: Option<Pubkey> = pubkey_of(arg_matches, "token_receiver");
            let referrer: Option<Pubkey> = pubkey_of(arg_matches, "referrer");
            let from = keypair_of(arg_matches, "from");
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            command_deposit_sol(
                &config,
                &stake_pool_address,
                &from,
                &token_receiver,
                &referrer,
                amount,
            )
        }
        ("list", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            command_list(&config, &stake_pool_address)
        }
        ("update", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let no_merge = arg_matches.is_present("no_merge");
            let force = arg_matches.is_present("force");
            command_update(&config, &stake_pool_address, force, no_merge)
        }
        ("withdraw-stake", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let vote_account = pubkey_of(arg_matches, "vote_account");
            let pool_account = pubkey_of(arg_matches, "pool_account");
            let pool_amount = value_t_or_exit!(arg_matches, "amount", f64);
            let stake_receiver = pubkey_of(arg_matches, "stake_receiver");
            let use_reserve = arg_matches.is_present("use_reserve");
            command_withdraw(
                &config,
                &stake_pool_address,
                use_reserve,
                &vote_account,
                &stake_receiver,
                &pool_account,
                pool_amount,
            )
        }
        ("set-manager", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let new_manager: Option<Keypair> = keypair_of(arg_matches, "new_manager");
            let new_fee_receiver: Option<Pubkey> = pubkey_of(arg_matches, "new_fee_receiver");
            command_set_manager(
                &config,
                &stake_pool_address,
                &new_manager,
                &new_fee_receiver,
            )
        }
        ("set-staker", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let new_staker = pubkey_of(arg_matches, "new_staker").unwrap();
            command_set_staker(&config, &stake_pool_address, &new_staker)
        }
        ("set-deposit-authority", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let new_stake_deposit_authority = pubkey_of(arg_matches, "new_stake_deposit_authority");
            let deposit_type = match arg_matches.value_of("deposit_type").unwrap() {
                "sol" => DepositType::Sol,
                "stake" => DepositType::Stake,
                _ => unreachable!(),
            };
            let _unset = arg_matches.is_present("unset");
            command_set_deposit_authority(
                &config,
                &stake_pool_address,
                new_stake_deposit_authority,
                deposit_type,
            )
        }
        ("set-fee", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let numerator = value_t_or_exit!(arg_matches, "fee_numerator", u64);
            let denominator = value_t_or_exit!(arg_matches, "fee_denominator", u64);
            let new_fee = Fee {
                denominator,
                numerator,
            };
            match arg_matches.value_of("fee_type").unwrap() {
                "epoch" => command_set_fee(&config, &stake_pool_address, FeeType::Epoch(new_fee)),
                "stake-deposit" => {
                    command_set_fee(&config, &stake_pool_address, FeeType::StakeDeposit(new_fee))
                }
                "sol-deposit" => {
                    command_set_fee(&config, &stake_pool_address, FeeType::SolDeposit(new_fee))
                }
                "withdrawal" => {
                    command_set_fee(&config, &stake_pool_address, FeeType::Withdrawal(new_fee))
                }
                _ => unreachable!(),
            }
        }
        ("set-referral-fee", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let fee = value_t_or_exit!(arg_matches, "fee", u8);
            if fee > 100u8 {
                panic!("Invalid fee {}%. Fee needs to be in range [0-100]", fee);
            }
            let fee_type = match arg_matches.value_of("fee_type").unwrap() {
                "sol" => FeeType::SolReferral(fee),
                "stake" => FeeType::StakeReferral(fee),
                _ => unreachable!(),
            };
            command_set_fee(&config, &stake_pool_address, fee_type)
        }
        ("list-all", _) => command_list_all_pools(&config),
        _ => unreachable!(),
    }
    .map_err(|err| {
        eprintln!("{}", err);
        exit(1);
    });
}
