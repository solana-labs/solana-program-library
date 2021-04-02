#[macro_use]
extern crate lazy_static;

mod client;

use {
    crate::client::*,
    clap::{
        crate_description, crate_name, crate_version, value_t, value_t_or_exit, App, AppSettings,
        Arg, ArgGroup, SubCommand,
    },
    solana_clap_utils::{
        input_parsers::pubkey_of,
        input_validators::{is_amount, is_keypair, is_parsable, is_pubkey, is_url},
        keypair::signer_from_path,
    },
    solana_client::{rpc_client::RpcClient, rpc_response::StakeActivationState},
    solana_program::{
        borsh::get_packed_len, instruction::Instruction, program_pack::Pack, pubkey::Pubkey,
    },
    solana_sdk::{
        commitment_config::CommitmentConfig,
        native_token::{self, Sol},
        signature::{Keypair, Signer},
        system_instruction,
        transaction::Transaction,
    },
    spl_stake_pool::{
        self,
        borsh::get_instance_packed_len,
        find_deposit_authority_program_address, find_stake_program_address,
        find_withdraw_authority_program_address,
        stake_program::{self, StakeAuthorize, StakeState},
        state::{StakePool, ValidatorList},
    },
    std::process::exit,
};

struct Config {
    rpc_client: RpcClient,
    verbose: bool,
    manager: Box<dyn Signer>,
    staker: Box<dyn Signer>,
    token_owner: Box<dyn Signer>,
    fee_payer: Box<dyn Signer>,
    dry_run: bool,
    no_update: bool,
}

type Error = Box<dyn std::error::Error>;
type CommandResult = Result<(), Error>;

const STAKE_STATE_LEN: usize = 200;
const MAX_ACCOUNTS_TO_UPDATE: usize = 10;
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

fn command_create_pool(
    config: &Config,
    fee: spl_stake_pool::instruction::Fee,
    max_validators: u32,
) -> CommandResult {
    let mint_account = Keypair::new();
    println!("Creating mint {}", mint_account.pubkey());

    let pool_fee_account = Keypair::new();
    println!(
        "Creating pool fee collection account {}",
        pool_fee_account.pubkey()
    );

    let stake_pool_keypair = Keypair::new();
    println!("Creating stake pool {}", stake_pool_keypair.pubkey());

    let validator_list = Keypair::new();

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
    let total_rent_free_balances = mint_account_balance
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

    let mut transaction = Transaction::new_with_payer(
        &[
            // Account for the stake pool mint
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &mint_account.pubkey(),
                mint_account_balance,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            // Account for the pool fee accumulation
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &pool_fee_account.pubkey(),
                pool_fee_account_balance,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            // Account for the stake pool
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &stake_pool_keypair.pubkey(),
                stake_pool_account_lamports,
                get_packed_len::<StakePool>() as u64,
                &spl_stake_pool::id(),
            ),
            // Validator stake account list storage
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &validator_list.pubkey(),
                validator_list_balance,
                validator_list_size as u64,
                &spl_stake_pool::id(),
            ),
            // Initialize pool token mint account
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint_account.pubkey(),
                &withdraw_authority,
                None,
                default_decimals,
            )?,
            // Initialize fee receiver account
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &pool_fee_account.pubkey(),
                &mint_account.pubkey(),
                &config.manager.pubkey(),
            )?,
            // Initialize stake pool account
            spl_stake_pool::instruction::initialize(
                &spl_stake_pool::id(),
                &stake_pool_keypair.pubkey(),
                &config.manager.pubkey(),
                &config.staker.pubkey(),
                &validator_list.pubkey(),
                &mint_account.pubkey(),
                &pool_fee_account.pubkey(),
                &spl_token::id(),
                fee,
                max_validators,
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
        &stake_pool_keypair,
        &validator_list,
        &mint_account,
        &pool_fee_account,
        config.manager.as_ref(),
    ];
    unique_signers!(signers);
    transaction.sign(&signers, recent_blockhash);
    send_transaction(&config, transaction)?;
    Ok(())
}

fn command_vsa_create(
    config: &Config,
    stake_pool_address: &Pubkey,
    vote_account: &Pubkey,
) -> CommandResult {
    let (stake_account, _) =
        find_stake_program_address(&spl_stake_pool::id(), &vote_account, &stake_pool_address);

    println!("Creating stake account {}", stake_account);

    let mut transaction = Transaction::new_with_payer(
        &[
            // Create new validator stake account address
            spl_stake_pool::instruction::create_validator_stake_account(
                &spl_stake_pool::id(),
                &stake_pool_address,
                &config.staker.pubkey(),
                &config.fee_payer.pubkey(),
                &stake_account,
                &vote_account,
            )?,
        ],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(config, fee_calculator.calculate_fee(&transaction.message()))?;
    transaction.sign(
        &[config.fee_payer.as_ref(), config.staker.as_ref()],
        recent_blockhash,
    );
    send_transaction(&config, transaction)?;
    Ok(())
}

fn command_vsa_add(
    config: &Config,
    stake_pool_address: &Pubkey,
    stake: &Pubkey,
    token_receiver: &Option<Pubkey>,
) -> CommandResult {
    if config.rpc_client.get_stake_activation(*stake, None)?.state != StakeActivationState::Active {
        return Err("Stake account is not active.".into());
    }

    if !config.no_update {
        command_update(config, stake_pool_address)?;
    }

    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;

    let mut total_rent_free_balances: u64 = 0;

    let token_receiver_account = Keypair::new();

    let mut instructions: Vec<Instruction> = vec![];
    let mut signers = vec![config.fee_payer.as_ref(), config.staker.as_ref()];

    // Create token account if not specified
    let token_receiver = unwrap_create_token_account(
        &config,
        &token_receiver,
        &token_receiver_account,
        &stake_pool.pool_mint,
        &mut instructions,
        |balance| {
            signers.push(&token_receiver_account);
            total_rent_free_balances += balance;
        },
    )?;

    // Calculate Deposit and Withdraw stake pool authorities
    let pool_deposit_authority =
        find_deposit_authority_program_address(&spl_stake_pool::id(), stake_pool_address).0;

    let pool_withdraw_authority =
        find_withdraw_authority_program_address(&spl_stake_pool::id(), stake_pool_address).0;

    instructions.extend(vec![
        // Set Withdrawer on stake account to Deposit authority of the stake pool
        stake_program::authorize(
            &stake,
            &config.staker.pubkey(),
            &pool_deposit_authority,
            StakeAuthorize::Withdrawer,
        ),
        // Set Staker on stake account to Deposit authority of the stake pool
        stake_program::authorize(
            &stake,
            &config.staker.pubkey(),
            &pool_deposit_authority,
            StakeAuthorize::Staker,
        ),
        // Add validator stake account to the pool
        spl_stake_pool::instruction::add_validator_to_pool(
            &spl_stake_pool::id(),
            &stake_pool_address,
            &config.staker.pubkey(),
            &pool_deposit_authority,
            &pool_withdraw_authority,
            &stake_pool.validator_list,
            &stake,
            &token_receiver,
            &stake_pool.pool_mint,
            &spl_token::id(),
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
    send_transaction(&config, transaction)?;
    Ok(())
}

fn command_vsa_remove(
    config: &Config,
    stake_pool_address: &Pubkey,
    stake: &Pubkey,
    withdraw_from: &Pubkey,
    new_authority: &Option<Pubkey>,
) -> CommandResult {
    if !config.no_update {
        command_update(config, stake_pool_address)?;
    }

    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;
    let pool_withdraw_authority =
        find_withdraw_authority_program_address(&spl_stake_pool::id(), stake_pool_address).0;

    let staker_pubkey = config.staker.pubkey();
    let new_authority = new_authority.as_ref().unwrap_or(&staker_pubkey);

    // Calculate amount of tokens to withdraw
    let stake_account = config.rpc_client.get_account(&stake)?;
    let tokens_to_withdraw = stake_pool
        .calc_pool_withdraw_amount(stake_account.lamports)
        .unwrap();

    // Check balance and mint
    let token_account =
        get_token_account(&config.rpc_client, &withdraw_from, &stake_pool.pool_mint)?;

    if token_account.amount < tokens_to_withdraw {
        let pool_mint = get_token_mint(&config.rpc_client, &stake_pool.pool_mint)?;
        return Err(format!(
            "Not enough balance to burn to remove validator stake account from the pool. {} pool tokens needed.",
            spl_token::amount_to_ui_amount(tokens_to_withdraw, pool_mint.decimals)
        ).into());
    }

    let mut transaction = Transaction::new_with_payer(
        &[
            // Approve spending token
            spl_token::instruction::approve(
                &spl_token::id(),
                &withdraw_from,
                &pool_withdraw_authority,
                &config.token_owner.pubkey(),
                &[],
                tokens_to_withdraw,
            )?,
            // Create new validator stake account address
            spl_stake_pool::instruction::remove_validator_from_pool(
                &spl_stake_pool::id(),
                &stake_pool_address,
                &config.staker.pubkey(),
                &pool_withdraw_authority,
                &new_authority,
                &stake_pool.validator_list,
                &stake,
                &withdraw_from,
                &stake_pool.pool_mint,
                &spl_token::id(),
            )?,
        ],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(config, fee_calculator.calculate_fee(&transaction.message()))?;
    transaction.sign(
        &[config.fee_payer.as_ref(), config.staker.as_ref()],
        recent_blockhash,
    );
    send_transaction(&config, transaction)?;
    Ok(())
}

fn unwrap_create_token_account<F>(
    config: &Config,
    token_optional: &Option<Pubkey>,
    keypair: &Keypair,
    mint: &Pubkey,
    instructions: &mut Vec<Instruction>,
    handler: F,
) -> Result<Pubkey, Error>
where
    F: FnOnce(u64),
{
    let result = match token_optional {
        Some(value) => *value,
        None => {
            // Account for tokens not specified, creating one
            println!("Creating account to receive tokens {}", keypair.pubkey());

            let min_account_balance = config
                .rpc_client
                .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)?;

            instructions.extend(vec![
                // Creating new account
                system_instruction::create_account(
                    &config.fee_payer.pubkey(),
                    &keypair.pubkey(),
                    min_account_balance,
                    spl_token::state::Account::LEN as u64,
                    &spl_token::id(),
                ),
                // Initialize token receiver account
                spl_token::instruction::initialize_account(
                    &spl_token::id(),
                    &keypair.pubkey(),
                    mint,
                    &config.token_owner.pubkey(),
                )?,
            ]);

            handler(min_account_balance);

            keypair.pubkey()
        }
    };
    Ok(result)
}

fn command_deposit(
    config: &Config,
    stake_pool_address: &Pubkey,
    stake: &Pubkey,
    token_receiver: &Option<Pubkey>,
) -> CommandResult {
    if !config.no_update {
        command_update(config, stake_pool_address)?;
    }

    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;
    let stake_state = get_stake_state(&config.rpc_client, &stake)?;

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
    println!("Depositing into stake account {}", validator_stake_account);
    if config.verbose {
        println!("{:?}", validator_stake_state);
    }

    let mut instructions: Vec<Instruction> = vec![];
    let mut signers = vec![config.fee_payer.as_ref(), config.staker.as_ref()];

    let mut total_rent_free_balances: u64 = 0;

    let token_receiver_account = Keypair::new();

    // Create token account if not specified
    let token_receiver = unwrap_create_token_account(
        &config,
        &token_receiver,
        &token_receiver_account,
        &stake_pool.pool_mint,
        &mut instructions,
        |balance| {
            signers.push(&token_receiver_account);
            total_rent_free_balances += balance;
        },
    )?;

    // Calculate Deposit and Withdraw stake pool authorities
    let pool_deposit_authority =
        find_deposit_authority_program_address(&spl_stake_pool::id(), stake_pool_address).0;

    let pool_withdraw_authority =
        find_withdraw_authority_program_address(&spl_stake_pool::id(), stake_pool_address).0;

    instructions.extend(vec![
        // Set Withdrawer on stake account to Deposit authority of the stake pool
        stake_program::authorize(
            &stake,
            &config.staker.pubkey(),
            &pool_deposit_authority,
            StakeAuthorize::Withdrawer,
        ),
        // Set Staker on stake account to Deposit authority of the stake pool
        stake_program::authorize(
            &stake,
            &config.staker.pubkey(),
            &pool_deposit_authority,
            StakeAuthorize::Staker,
        ),
        // Add stake account to the pool
        spl_stake_pool::instruction::deposit(
            &spl_stake_pool::id(),
            &stake_pool_address,
            &stake_pool.validator_list,
            &pool_deposit_authority,
            &pool_withdraw_authority,
            &stake,
            &validator_stake_account,
            &token_receiver,
            &stake_pool.manager_fee_account,
            &stake_pool.pool_mint,
            &spl_token::id(),
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
    send_transaction(&config, transaction)?;
    Ok(())
}

fn command_list(config: &Config, stake_pool_address: &Pubkey) -> CommandResult {
    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;

    if config.verbose {
        println!("Current validator list");
        let validator_list = get_validator_list(&config.rpc_client, &stake_pool.validator_list)?;
        for validator in validator_list.validators {
            println!(
                "Vote Account: {}\tBalance: {}\tEpoch: {}",
                validator.vote_account, validator.balance, validator.last_update_epoch
            );
        }
    }

    let pool_withdraw_authority =
        find_withdraw_authority_program_address(&spl_stake_pool::id(), stake_pool_address).0;

    let accounts =
        get_stake_accounts_by_withdraw_authority(&config.rpc_client, &pool_withdraw_authority)?;
    if accounts.is_empty() {
        return Err("No accounts found.".to_string().into());
    }

    let mut total_lamports: u64 = 0;
    for (pubkey, lamports, stake_state) in accounts {
        total_lamports += lamports;
        println!(
            "Stake Account: {}\tVote Account: {}\t{}",
            pubkey,
            stake_state.delegation().expect("delegation").voter_pubkey,
            Sol(lamports)
        );
    }
    println!("Total Stake: {}", Sol(total_lamports));

    Ok(())
}

fn command_update(config: &Config, stake_pool_address: &Pubkey) -> CommandResult {
    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;
    let validator_list = get_validator_list(&config.rpc_client, &stake_pool.validator_list)?;
    let epoch_info = config.rpc_client.get_epoch_info()?;

    let accounts_to_update: Vec<Pubkey> = validator_list
        .validators
        .iter()
        .filter_map(|item| {
            if item.last_update_epoch >= epoch_info.epoch {
                None
            } else {
                let (stake_account, _) = find_stake_program_address(
                    &spl_stake_pool::id(),
                    &item.vote_account,
                    &stake_pool_address,
                );
                Some(stake_account)
            }
        })
        .collect();

    let mut instructions: Vec<Instruction> = vec![];

    for accounts_chunk in accounts_to_update.chunks(MAX_ACCOUNTS_TO_UPDATE) {
        instructions.push(spl_stake_pool::instruction::update_validator_list_balance(
            &spl_stake_pool::id(),
            &stake_pool.validator_list,
            &accounts_chunk,
        )?);
    }

    if instructions.is_empty() && stake_pool.last_update_epoch == epoch_info.epoch {
        println!("Stake pool balances are up to date, no update required.");
        Ok(())
    } else {
        println!("Updating stake pool...");
        instructions.push(spl_stake_pool::instruction::update_stake_pool_balance(
            &spl_stake_pool::id(),
            stake_pool_address,
            &stake_pool.validator_list,
        )?);

        let mut transaction =
            Transaction::new_with_payer(&instructions, Some(&config.fee_payer.pubkey()));

        let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
        check_fee_payer_balance(config, fee_calculator.calculate_fee(&transaction.message()))?;
        transaction.sign(&[config.fee_payer.as_ref()], recent_blockhash);
        send_transaction(&config, transaction)?;
        Ok(())
    }
}

#[derive(PartialEq, Debug)]
struct WithdrawAccount {
    address: Pubkey,
    pool_amount: u64,
}

fn prepare_withdraw_accounts(
    rpc_client: &RpcClient,
    stake_pool: &StakePool,
    pool_withdraw_authority: &Pubkey,
    pool_amount: u64,
) -> Result<Vec<WithdrawAccount>, Error> {
    let mut accounts =
        get_stake_accounts_by_withdraw_authority(rpc_client, &pool_withdraw_authority)?;
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
    for (address, lamports, _) in accounts {
        if lamports <= min_balance {
            continue;
        }
        let available_for_withdrawal = stake_pool
            .calc_lamports_withdraw_amount(lamports - *MIN_STAKE_BALANCE)
            .unwrap();
        let pool_amount = u64::min(available_for_withdrawal, remaining_amount);

        // Those accounts will be withdrawn completely with `claim` instruction
        withdraw_from.push(WithdrawAccount {
            address,
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
    pool_amount: f64,
    withdraw_from: &Pubkey,
    stake_receiver_param: &Option<Pubkey>,
) -> CommandResult {
    if !config.no_update {
        command_update(config, stake_pool_address)?;
    }

    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;
    let pool_mint = get_token_mint(&config.rpc_client, &stake_pool.pool_mint)?;
    let pool_amount = spl_token::ui_amount_to_amount(pool_amount, pool_mint.decimals);

    let pool_withdraw_authority =
        find_withdraw_authority_program_address(&spl_stake_pool::id(), stake_pool_address).0;

    // Check withdraw_from account type
    let token_account =
        get_token_account(&config.rpc_client, &withdraw_from, &stake_pool.pool_mint)?;

    // Check withdraw_from balance
    if token_account.amount < pool_amount {
        return Err(format!(
            "Not enough token balance to withdraw {} pool tokens.\nMaximum withdraw amount is {} pool tokens.",
            spl_token::amount_to_ui_amount(pool_amount, pool_mint.decimals),
            spl_token::amount_to_ui_amount(token_account.amount, pool_mint.decimals)
        )
        .into());
    }

    // Get the list of accounts to withdraw from
    let withdraw_accounts = prepare_withdraw_accounts(
        &config.rpc_client,
        &stake_pool,
        &pool_withdraw_authority,
        pool_amount,
    )?;

    // Construct transaction to withdraw from withdraw_accounts account list
    let mut instructions: Vec<Instruction> = vec![];
    let mut signers = vec![config.fee_payer.as_ref(), config.token_owner.as_ref()];
    let stake_receiver_account = Keypair::new(); // Will be added to signers if creating new account

    instructions.push(
        // Approve spending token
        spl_token::instruction::approve(
            &spl_token::id(),
            &withdraw_from,
            &pool_withdraw_authority,
            &config.token_owner.pubkey(),
            &[],
            pool_amount,
        )?,
    );

    // Use separate mutable variable because withdraw might create a new account
    let mut stake_receiver: Option<Pubkey> = *stake_receiver_param;

    let mut total_rent_free_balances = 0;

    // Go through prepared accounts and withdraw/claim them
    for withdraw_account in withdraw_accounts {
        // Convert pool tokens amount to lamports
        let sol_withdraw_amount = stake_pool
            .calc_lamports_withdraw_amount(withdraw_account.pool_amount)
            .unwrap();

        println!(
            "Withdrawing from account {}, amount {}, {} pool tokens",
            withdraw_account.address,
            Sol(sol_withdraw_amount),
            spl_token::amount_to_ui_amount(withdraw_account.pool_amount, pool_mint.decimals),
        );

        if stake_receiver.is_none() {
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
                    &stake_program::id(),
                ),
            );

            signers.push(&stake_receiver_account);

            total_rent_free_balances += stake_receiver_account_balance;

            stake_receiver = Some(stake_receiver_account.pubkey());
        }

        instructions.push(spl_stake_pool::instruction::withdraw(
            &spl_stake_pool::id(),
            &stake_pool_address,
            &stake_pool.validator_list,
            &pool_withdraw_authority,
            &withdraw_account.address,
            &stake_receiver.unwrap(), // Cannot be none at this point
            &config.staker.pubkey(),
            &withdraw_from,
            &stake_pool.pool_mint,
            &spl_token::id(),
            withdraw_account.pool_amount,
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
    send_transaction(&config, transaction)?;
    Ok(())
}

fn command_set_manager(
    config: &Config,
    stake_pool_address: &Pubkey,
    new_manager: &Option<Pubkey>,
    new_fee_receiver: &Option<Pubkey>,
) -> CommandResult {
    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;

    // If new accounts are missing in the arguments use the old ones
    let new_manager = match new_manager {
        None => stake_pool.manager,
        Some(value) => *value,
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

    let mut transaction = Transaction::new_with_payer(
        &[spl_stake_pool::instruction::set_manager(
            &spl_stake_pool::id(),
            &stake_pool_address,
            &config.manager.pubkey(),
            &new_manager,
            &new_fee_receiver,
        )?],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(config, fee_calculator.calculate_fee(&transaction.message()))?;
    let mut signers = vec![config.fee_payer.as_ref(), config.manager.as_ref()];
    unique_signers!(signers);
    transaction.sign(&signers, recent_blockhash);
    send_transaction(&config, transaction)?;
    Ok(())
}

fn command_set_staker(
    config: &Config,
    stake_pool_address: &Pubkey,
    new_staker: &Pubkey,
) -> CommandResult {
    let mut transaction = Transaction::new_with_payer(
        &[spl_stake_pool::instruction::set_staker(
            &spl_stake_pool::id(),
            &stake_pool_address,
            &config.manager.pubkey(),
            &new_staker,
        )?],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(config, fee_calculator.calculate_fee(&transaction.message()))?;
    let mut signers = vec![config.fee_payer.as_ref(), config.manager.as_ref()];
    unique_signers!(signers);
    transaction.sign(&signers, recent_blockhash);
    send_transaction(&config, transaction)?;
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
            Arg::with_name("dry_run")
                .long("dry-run")
                .takes_value(false)
                .global(true)
                .help("Simluate transaction instead of executing"),
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
                Arg::with_name("max_validators")
                    .long("max-validators")
                    .short("m")
                    .validator(is_parsable::<u32>)
                    .value_name("NUMBER")
                    .takes_value(true)
                    .required(true)
                    .help("Max number of validators included in the stake pool"),
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
                Arg::with_name("stake_account")
                    .index(2)
                    .validator(is_pubkey)
                    .value_name("STAKE_ACCOUNT_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake account to add to the pool"),
            )
            .arg(
                Arg::with_name("token_receiver")
                    .long("token-receiver")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .help("Account to receive pool token. Must be initialized account of the stake pool token. \
                          Defaults to the new pool token account."),
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
                Arg::with_name("stake_account")
                    .index(2)
                    .validator(is_pubkey)
                    .value_name("STAKE_ACCOUNT_ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Stake account to remove from the pool"),
            )
            .arg(
                Arg::with_name("withdraw_from")
                    .long("withdraw-from")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Token account to withdraw pool token from. \
                          Must have enough tokens for the full stake address balance."),
            )
            .arg(
                Arg::with_name("new_authority")
                    .long("new-authority")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .help("New authority to set as Staker and Withdrawer in the stake account removed from the pool.
                          Defaults to the wallet owner pubkey."),
            )
        )
        .subcommand(SubCommand::with_name("deposit")
            .about("Add stake account to the stake pool")
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
                    .help("Account to receive pool token. Must be initialized account of the stake pool token. \
                          Defaults to the new pool token account."),
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
        )
        .subcommand(SubCommand::with_name("withdraw")
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
                    .long("amount")
                    .validator(is_amount)
                    .value_name("AMOUNT")
                    .takes_value(true)
                    .required(true)
                    .help("Amount of pool tokens to withdraw for activated stake."),
            )
            .arg(
                Arg::with_name("withdraw_from")
                    .long("withdraw-from")
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .required(true)
                    .help("Account to withdraw tokens from. Must be owned by the client."),
            )
            .arg(
                Arg::with_name("stake_receiver")
                    .long("stake-receiver")
                    .validator(is_pubkey)
                    .value_name("STAKE_ACCOUNT_ADDRESS")
                    .takes_value(true)
                    .help("Stake account to receive SOL from the stake pool. Defaults to a new stake account."),
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
                    .validator(is_pubkey)
                    .value_name("ADDRESS")
                    .takes_value(true)
                    .help("Public key for the new stake pool manager."),
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

        let staker = signer_from_path(
            &matches,
            &cli_config.keypair_path,
            "staker",
            &mut wallet_manager,
        )
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            exit(1);
        });
        let manager = signer_from_path(
            &matches,
            &cli_config.keypair_path,
            "manager",
            &mut wallet_manager,
        )
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            exit(1);
        });
        let token_owner = signer_from_path(
            &matches,
            &cli_config.keypair_path,
            "token_owner",
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
        let dry_run = matches.is_present("dry_run");
        let no_update = matches.is_present("no_update");

        Config {
            rpc_client: RpcClient::new_with_commitment(json_rpc_url, CommitmentConfig::confirmed()),
            verbose,
            manager,
            staker,
            token_owner,
            fee_payer,
            dry_run,
            no_update,
        }
    };

    let _ = match matches.subcommand() {
        ("create-pool", Some(arg_matches)) => {
            let numerator = value_t_or_exit!(arg_matches, "fee_numerator", u64);
            let denominator = value_t_or_exit!(arg_matches, "fee_denominator", u64);
            let max_validators = value_t_or_exit!(arg_matches, "max_validators", u32);
            command_create_pool(
                &config,
                spl_stake_pool::instruction::Fee {
                    denominator,
                    numerator,
                },
                max_validators,
            )
        }
        ("create-validator-stake", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let vote_account_address = pubkey_of(arg_matches, "vote_account").unwrap();
            command_vsa_create(&config, &stake_pool_address, &vote_account_address)
        }
        ("add-validator", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let stake_account = pubkey_of(arg_matches, "stake_account").unwrap();
            let token_receiver: Option<Pubkey> = pubkey_of(arg_matches, "token_receiver");
            command_vsa_add(
                &config,
                &stake_pool_address,
                &stake_account,
                &token_receiver,
            )
        }
        ("remove-validator", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let stake_account = pubkey_of(arg_matches, "stake_account").unwrap();
            let withdraw_from = pubkey_of(arg_matches, "withdraw_from").unwrap();
            let new_authority: Option<Pubkey> = pubkey_of(arg_matches, "new_authority");
            command_vsa_remove(
                &config,
                &stake_pool_address,
                &stake_account,
                &withdraw_from,
                &new_authority,
            )
        }
        ("deposit", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let stake_account = pubkey_of(arg_matches, "stake_account").unwrap();
            let token_receiver: Option<Pubkey> = pubkey_of(arg_matches, "token_receiver");
            command_deposit(
                &config,
                &stake_pool_address,
                &stake_account,
                &token_receiver,
            )
        }
        ("list", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            command_list(&config, &stake_pool_address)
        }
        ("update", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            command_update(&config, &stake_pool_address)
        }
        ("withdraw", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let withdraw_from = pubkey_of(arg_matches, "withdraw_from").unwrap();
            let pool_amount = value_t_or_exit!(arg_matches, "amount", f64);
            let stake_receiver: Option<Pubkey> = pubkey_of(arg_matches, "stake_receiver");
            command_withdraw(
                &config,
                &stake_pool_address,
                pool_amount,
                &withdraw_from,
                &stake_receiver,
            )
        }
        ("set-manager", Some(arg_matches)) => {
            let stake_pool_address = pubkey_of(arg_matches, "pool").unwrap();
            let new_manager: Option<Pubkey> = pubkey_of(arg_matches, "new_manager");
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
        _ => unreachable!(),
    }
    .map_err(|err| {
        eprintln!("{}", err);
        exit(1);
    });
}
