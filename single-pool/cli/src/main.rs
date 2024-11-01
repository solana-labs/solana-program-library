#![allow(clippy::arithmetic_side_effects)]
#![allow(deprecated)]

use {
    clap::{CommandFactory, Parser},
    solana_clap_v3_utils::input_parsers::Amount,
    solana_client::{
        rpc_config::RpcProgramAccountsConfig,
        rpc_filter::{Memcmp, RpcFilterType},
    },
    solana_sdk::{
        borsh1::try_from_slice_unchecked,
        pubkey::Pubkey,
        signature::{Keypair, Signature, Signer},
        stake,
        transaction::Transaction,
    },
    solana_vote_program::{self as vote_program, vote_state::VoteState},
    spl_single_pool::{
        self, find_default_deposit_account_address, find_pool_address, find_pool_mint_address,
        find_pool_stake_address, instruction::SinglePoolInstruction, state::SinglePool,
    },
    spl_token_client::token::Token,
};

mod config;
use config::*;

mod cli;
use cli::*;

mod output;
use output::*;

mod quarantine;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();
    let matches = Cli::command().get_matches();
    let mut wallet_manager = None;

    let command = cli
        .command
        .clone()
        .with_signers(&matches, &mut wallet_manager)?;
    let config = Config::new(cli, matches, &mut wallet_manager);

    solana_logger::setup_with_default("solana=info");

    let res = command.execute(&config).await?;
    println!("{}", res);

    Ok(())
}

pub type CommandResult = Result<String, Error>;

impl Command {
    pub async fn execute(self, config: &Config) -> CommandResult {
        match self {
            Command::Manage(command) => match command.manage {
                ManageCommand::Initialize(command_config) => {
                    command_initialize(config, command_config).await
                }
                ManageCommand::ReactivatePoolStake(command_config) => {
                    command_reactivate_pool_stake(config, command_config).await
                }
                ManageCommand::CreateTokenMetadata(command_config) => {
                    command_create_metadata(config, command_config).await
                }
                ManageCommand::UpdateTokenMetadata(command_config) => {
                    command_update_metadata(config, command_config).await
                }
            },
            Command::Deposit(command_config) => command_deposit(config, command_config).await,
            Command::Withdraw(command_config) => command_withdraw(config, command_config).await,
            Command::CreateDefaultStake(command_config) => {
                command_create_stake(config, command_config).await
            }
            Command::Display(command_config) => command_display(config, command_config).await,
        }
    }
}

// initialize a new stake pool for a vote account
async fn command_initialize(config: &Config, command_config: InitializeCli) -> CommandResult {
    let payer = config.fee_payer()?;
    let vote_account_address = command_config.vote_account_address;

    println_display(
        config,
        format!(
            "Initializing single-validator stake pool for vote account {}\n",
            vote_account_address,
        ),
    );

    // check if the vote account is valid
    let vote_account = config
        .program_client
        .get_account(vote_account_address)
        .await?;
    if vote_account.is_none() || vote_account.unwrap().owner != vote_program::id() {
        return Err(format!("{} is not a valid vote account", vote_account_address,).into());
    }

    let pool_address = find_pool_address(&spl_single_pool::id(), &vote_account_address);

    // check if the pool has already been initialized
    if config
        .program_client
        .get_account(pool_address)
        .await?
        .is_some()
    {
        return Err(format!(
            "Pool {} for vote account {} already exists",
            pool_address, vote_account_address
        )
        .into());
    }

    let mut instructions = spl_single_pool::instruction::initialize(
        &spl_single_pool::id(),
        &vote_account_address,
        &payer.pubkey(),
        &quarantine::get_rent(config).await?,
        quarantine::get_minimum_delegation(config).await?,
    );

    // get rid of the CreateMetadata instruction if desired, eg if mpl breaks compat
    if command_config.skip_metadata {
        assert_eq!(
            instructions.last().unwrap().data,
            borsh::to_vec(&SinglePoolInstruction::CreateTokenMetadata).unwrap()
        );

        instructions.pop();
    }

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &vec![payer],
        config.program_client.get_latest_blockhash().await?,
    );

    let signature = process_transaction(config, transaction).await?;

    Ok(format_output(
        config,
        "Initialize".to_string(),
        StakePoolOutput {
            pool_address,
            vote_account_address,
            available_stake: 0,
            token_supply: 0,
            signature,
        },
    ))
}

// reactivate pool stake account
async fn command_reactivate_pool_stake(
    config: &Config,
    command_config: ReactivateCli,
) -> CommandResult {
    let payer = config.fee_payer()?;
    let pool_address = pool_address_from_args(
        command_config.pool_address,
        command_config.vote_account_address,
    );

    println_display(
        config,
        format!("Reactivating stake account for pool {}\n", pool_address),
    );

    let vote_account_address =
        if let Some(pool_data) = config.program_client.get_account(pool_address).await? {
            try_from_slice_unchecked::<SinglePool>(&pool_data.data)?.vote_account_address
        } else {
            return Err(format!("Pool {} has not been initialized", pool_address).into());
        };

    // the only reason this check is skippable is for testing, otherwise theres no
    // reason
    if !command_config.skip_deactivation_check {
        let current_epoch = config.rpc_client.get_epoch_info().await?.epoch;
        let pool_stake_address = find_pool_stake_address(&spl_single_pool::id(), &pool_address);
        let pool_stake_deactivated = quarantine::get_stake_info(config, &pool_stake_address)
            .await?
            .unwrap()
            .1
            .delegation
            .deactivation_epoch
            <= current_epoch;

        if !pool_stake_deactivated {
            return Err("Pool stake account is neither deactivating nor deactivated".into());
        }
    }

    let instruction = spl_single_pool::instruction::reactivate_pool_stake(
        &spl_single_pool::id(),
        &vote_account_address,
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &vec![payer],
        config.program_client.get_latest_blockhash().await?,
    );

    let signature = process_transaction(config, transaction).await?;

    Ok(format_output(
        config,
        "ReactivatePoolStake".to_string(),
        SignatureOutput { signature },
    ))
}

// deposit stake
async fn command_deposit(config: &Config, command_config: DepositCli) -> CommandResult {
    let payer = config.fee_payer()?;
    let owner = config.default_signer()?;
    let stake_authority = signer_from_arg(command_config.stake_withdraw_authority, &owner)?;
    let lamport_recipient = command_config
        .lamport_recipient_address
        .unwrap_or_else(|| owner.pubkey());

    let current_epoch = config.rpc_client.get_epoch_info().await?.epoch;

    // the cli invocation for this is conceptually simple, but a bit tricky
    // the user can provide pool or vote and let the cli infer the stake account
    // address but they can also provide pool or vote with the stake account, as
    // a safety check first we want to get the pool address if they provided a
    // pool or vote address
    let provided_pool_address = command_config.pool_address.or_else(|| {
        command_config
            .vote_account_address
            .map(|address| find_pool_address(&spl_single_pool::id(), &address))
    });

    // from there we can determine the stake account address
    let stake_account_address =
        if let Some(stake_account_address) = command_config.stake_account_address {
            stake_account_address
        } else if let Some(pool_address) = provided_pool_address {
            assert!(command_config.default_stake_account);
            find_default_deposit_account_address(&pool_address, &stake_authority.pubkey())
        } else {
            unreachable!()
        };

    // now we validate the stake account and definitively resolve the pool address
    let (pool_address, user_stake_active) = if let Some((meta, stake)) =
        quarantine::get_stake_info(config, &stake_account_address).await?
    {
        let derived_pool_address =
            find_pool_address(&spl_single_pool::id(), &stake.delegation.voter_pubkey);

        if let Some(provided_pool_address) = provided_pool_address {
            if provided_pool_address != derived_pool_address {
                return Err(format!(
                    "Provided pool address {} does not match stake account-derived address {}",
                    provided_pool_address, derived_pool_address,
                )
                .into());
            }
        }

        if meta.authorized.withdrawer != stake_authority.pubkey() {
            return Err(format!(
                "Incorrect withdraw authority for stake account {}: got {}, expected {}",
                stake_account_address,
                meta.authorized.withdrawer,
                stake_authority.pubkey(),
            )
            .into());
        }

        if stake.delegation.deactivation_epoch < u64::MAX {
            return Err(format!(
                "Stake account {} is deactivating or deactivated",
                stake_account_address
            )
            .into());
        }

        (
            derived_pool_address,
            stake.delegation.activation_epoch <= current_epoch,
        )
    } else {
        return Err(format!("Could not find stake account {}", stake_account_address).into());
    };

    println_display(
        config,
        format!(
            "Depositing stake from account {} into pool {}\n",
            stake_account_address, pool_address
        ),
    );

    if config
        .program_client
        .get_account(pool_address)
        .await?
        .is_none()
    {
        return Err(format!("Pool {} has not been initialized", pool_address).into());
    }

    let pool_stake_address = find_pool_stake_address(&spl_single_pool::id(), &pool_address);
    let pool_stake_active = quarantine::get_stake_info(config, &pool_stake_address)
        .await?
        .unwrap()
        .1
        .delegation
        .activation_epoch
        <= current_epoch;

    if user_stake_active != pool_stake_active {
        return Err("Activation status mismatch; try again next epoch".into());
    }

    let pool_mint_address = find_pool_mint_address(&spl_single_pool::id(), &pool_address);
    let token = Token::new(
        config.program_client.clone(),
        &spl_token::id(),
        &pool_mint_address,
        None,
        payer.clone(),
    );

    // use token account provided, or get/create the associated account for the
    // client keypair
    let token_account_address = if let Some(account) = command_config.token_account_address {
        account
    } else {
        token
            .get_or_create_associated_account_info(&owner.pubkey())
            .await?;
        token.get_associated_token_address(&owner.pubkey())
    };

    let previous_token_amount = token
        .get_account_info(&token_account_address)
        .await?
        .base
        .amount;

    let instructions = spl_single_pool::instruction::deposit(
        &spl_single_pool::id(),
        &pool_address,
        &stake_account_address,
        &token_account_address,
        &lamport_recipient,
        &stake_authority.pubkey(),
    );

    let mut signers = vec![];
    for signer in [payer.clone(), stake_authority] {
        if !signers.contains(&signer) {
            signers.push(signer);
        }
    }

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &signers,
        config.program_client.get_latest_blockhash().await?,
    );

    let signature = process_transaction(config, transaction).await?;
    let token_amount = token
        .get_account_info(&token_account_address)
        .await?
        .base
        .amount
        - previous_token_amount;

    Ok(format_output(
        config,
        "Deposit".to_string(),
        DepositOutput {
            pool_address,
            token_amount,
            signature,
        },
    ))
}

// withdraw stake
async fn command_withdraw(config: &Config, command_config: WithdrawCli) -> CommandResult {
    let payer = config.fee_payer()?;
    let owner = config.default_signer()?;
    let token_authority = signer_from_arg(command_config.token_authority, &owner)?;
    let stake_authority_address = command_config
        .stake_authority_address
        .unwrap_or_else(|| owner.pubkey());

    let stake_account = Keypair::new();
    let stake_account_address = stake_account.pubkey();

    // since we can't infer pool from token account, the withdraw invocation is
    // rather simpler first get the pool address
    let pool_address = pool_address_from_args(
        command_config.pool_address,
        command_config.vote_account_address,
    );

    if config
        .program_client
        .get_account(pool_address)
        .await?
        .is_none()
    {
        return Err(format!("Pool {} has not been initialized", pool_address).into());
    }

    // now all the mint and token info
    let pool_mint_address = find_pool_mint_address(&spl_single_pool::id(), &pool_address);
    let token = Token::new(
        config.program_client.clone(),
        &spl_token::id(),
        &pool_mint_address,
        None,
        payer.clone(),
    );

    let token_account_address = command_config
        .token_account_address
        .unwrap_or_else(|| token.get_associated_token_address(&owner.pubkey()));

    let token_account = token.get_account_info(&token_account_address).await?;

    let token_amount = match command_config.token_amount.sol_to_lamport() {
        Amount::All => token_account.base.amount,
        Amount::Raw(amount) => amount,
        Amount::Decimal(_) => unreachable!(),
    };

    println_display(
        config,
        format!(
            "Withdrawing from pool {} into new stake account {}; burning {} tokens from {}\n",
            pool_address, stake_account_address, token_amount, token_account_address,
        ),
    );

    if token_amount == 0 {
        return Err("Cannot withdraw zero tokens".into());
    }

    if token_amount > token_account.base.amount {
        return Err(format!(
            "Withdraw amount {} exceeds tokens in account ({})",
            token_amount, token_account.base.amount
        )
        .into());
    }

    // note a delegate authority is not allowed here because we must authorize the
    // pool authority
    if token_account.base.owner != token_authority.pubkey() {
        return Err(format!(
            "Invalid token authority: got {}, actual {}",
            token_account.base.owner,
            token_authority.pubkey()
        )
        .into());
    }

    // create a blank stake account to withdraw into
    let mut instructions = vec![
        quarantine::create_uninitialized_stake_account_instruction(
            config,
            &payer.pubkey(),
            &stake_account_address,
        )
        .await?,
    ];

    // perform the withdrawal
    instructions.extend(spl_single_pool::instruction::withdraw(
        &spl_single_pool::id(),
        &pool_address,
        &stake_account_address,
        &stake_authority_address,
        &token_account_address,
        &token_authority.pubkey(),
        token_amount,
    ));

    // possibly deactivate the new stake account
    if command_config.deactivate {
        instructions.push(stake::instruction::deactivate_stake(
            &stake_account_address,
            &stake_authority_address,
        ));
    }

    let mut signers = vec![];
    for signer in [payer.as_ref(), token_authority.as_ref(), &stake_account] {
        if !signers.contains(&signer) {
            signers.push(signer);
        }
    }

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &signers,
        config.program_client.get_latest_blockhash().await?,
    );

    let signature = process_transaction(config, transaction).await?;
    let stake_amount = if let Some((_, stake)) =
        quarantine::get_stake_info(config, &stake_account_address).await?
    {
        stake.delegation.stake
    } else {
        0
    };

    Ok(format_output(
        config,
        "Withdraw".to_string(),
        WithdrawOutput {
            pool_address,
            stake_account_address,
            stake_amount,
            signature,
        },
    ))
}

// create token metadata
async fn command_create_metadata(
    config: &Config,
    command_config: CreateMetadataCli,
) -> CommandResult {
    let payer = config.fee_payer()?;

    // first get the pool address
    // i dont check metadata because i dont want to get entangled with mpl
    let pool_address = pool_address_from_args(
        command_config.pool_address,
        command_config.vote_account_address,
    );

    println_display(
        config,
        format!(
            "Creating default token metadata for pool {}\n",
            pool_address
        ),
    );

    if config
        .program_client
        .get_account(pool_address)
        .await?
        .is_none()
    {
        return Err(format!("Pool {} has not been initialized", pool_address).into());
    }

    // and... i guess thats it?

    let instruction = spl_single_pool::instruction::create_token_metadata(
        &spl_single_pool::id(),
        &pool_address,
        &payer.pubkey(),
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &vec![payer],
        config.program_client.get_latest_blockhash().await?,
    );

    let signature = process_transaction(config, transaction).await?;

    Ok(format_output(
        config,
        "CreateTokenMetadata".to_string(),
        SignatureOutput { signature },
    ))
}

// update token metadata
async fn command_update_metadata(
    config: &Config,
    command_config: UpdateMetadataCli,
) -> CommandResult {
    let payer = config.fee_payer()?;
    let owner = config.default_signer()?;
    let authorized_withdrawer = signer_from_arg(command_config.authorized_withdrawer, &owner)?;

    // first get the pool address
    // i dont check metadata because i dont want to get entangled with mpl
    let pool_address = pool_address_from_args(
        command_config.pool_address,
        command_config.vote_account_address,
    );

    println_display(
        config,
        format!("Updating token metadata for pool {}\n", pool_address),
    );

    // we always need the vote account
    let vote_account_address =
        if let Some(pool_data) = config.program_client.get_account(pool_address).await? {
            try_from_slice_unchecked::<SinglePool>(&pool_data.data)?.vote_account_address
        } else {
            return Err(format!("Pool {} has not been initialized", pool_address).into());
        };

    if let Some(vote_account_data) = config
        .program_client
        .get_account(vote_account_address)
        .await?
    {
        let vote_account = VoteState::deserialize(&vote_account_data.data)?;

        if authorized_withdrawer.pubkey() != vote_account.authorized_withdrawer {
            return Err(format!(
                "Invalid authorized withdrawer: got {}, actual {}",
                authorized_withdrawer.pubkey(),
                vote_account.authorized_withdrawer,
            )
            .into());
        }
    } else {
        // we know the pool exists so the vote account must exist
        unreachable!();
    }

    let instruction = spl_single_pool::instruction::update_token_metadata(
        &spl_single_pool::id(),
        &vote_account_address,
        &authorized_withdrawer.pubkey(),
        command_config.token_name,
        command_config.token_symbol,
        command_config.token_uri.unwrap_or_default(),
    );

    let mut signers = vec![];
    for signer in [payer.clone(), authorized_withdrawer] {
        if !signers.contains(&signer) {
            signers.push(signer);
        }
    }

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &signers,
        config.program_client.get_latest_blockhash().await?,
    );

    let signature = process_transaction(config, transaction).await?;

    Ok(format_output(
        config,
        "UpdateTokenMetadata".to_string(),
        SignatureOutput { signature },
    ))
}

// create default stake account
async fn command_create_stake(config: &Config, command_config: CreateStakeCli) -> CommandResult {
    let payer = config.fee_payer()?;
    let owner = config.default_signer()?;
    let stake_authority_address = command_config
        .stake_authority_address
        .unwrap_or_else(|| owner.pubkey());

    let pool_address = pool_address_from_args(
        command_config.pool_address,
        command_config.vote_account_address,
    );

    println_display(
        config,
        format!("Creating default stake account for pool {}\n", pool_address),
    );

    let vote_account_address =
        if let Some(vote_account_address) = command_config.vote_account_address {
            vote_account_address
        } else if let Some(pool_data) = config.program_client.get_account(pool_address).await? {
            try_from_slice_unchecked::<SinglePool>(&pool_data.data)?.vote_account_address
        } else {
            return Err(format!(
                "Cannot determine vote account address from uninitialized pool {}",
                pool_address,
            )
            .into());
        };

    if command_config.vote_account_address.is_some()
        && config
            .program_client
            .get_account(pool_address)
            .await?
            .is_none()
    {
        eprintln_display(
            config,
            format!("warning: Pool {} has not been initialized", pool_address),
        );
    }

    let instructions = spl_single_pool::instruction::create_and_delegate_user_stake(
        &spl_single_pool::id(),
        &vote_account_address,
        &stake_authority_address,
        &quarantine::get_rent(config).await?,
        command_config.lamports,
    );

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &vec![payer],
        config.program_client.get_latest_blockhash().await?,
    );

    let signature = process_transaction(config, transaction).await?;

    Ok(format_output(
        config,
        "CreateDefaultStake".to_string(),
        CreateStakeOutput {
            pool_address,
            stake_account_address: find_default_deposit_account_address(
                &pool_address,
                &stake_authority_address,
            ),
            signature,
        },
    ))
}

// display stake pool(s)
async fn command_display(config: &Config, command_config: DisplayCli) -> CommandResult {
    if command_config.all {
        // the filter isn't necessary now but makes the cli forward-compatible
        let pools = config
            .rpc_client
            .get_program_accounts_with_config(
                &spl_single_pool::id(),
                RpcProgramAccountsConfig {
                    filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                        0,
                        vec![1],
                    ))]),
                    ..RpcProgramAccountsConfig::default()
                },
            )
            .await?;

        let mut displays = vec![];
        for pool in pools {
            let vote_account_address =
                try_from_slice_unchecked::<SinglePool>(&pool.1.data)?.vote_account_address;
            displays.push(get_pool_display(config, pool.0, Some(vote_account_address)).await?);
        }

        Ok(format_output(
            config,
            "DisplayAll".to_string(),
            StakePoolListOutput(displays),
        ))
    } else {
        let pool_address = pool_address_from_args(
            command_config.pool_address,
            command_config.vote_account_address,
        );

        Ok(format_output(
            config,
            "Display".to_string(),
            get_pool_display(config, pool_address, None).await?,
        ))
    }
}

async fn get_pool_display(
    config: &Config,
    pool_address: Pubkey,
    maybe_vote_account: Option<Pubkey>,
) -> Result<StakePoolOutput, Error> {
    let vote_account_address = if let Some(address) = maybe_vote_account {
        address
    } else if let Some(pool_data) = config.program_client.get_account(pool_address).await? {
        if let Ok(data) = try_from_slice_unchecked::<SinglePool>(&pool_data.data) {
            data.vote_account_address
        } else {
            return Err(format!(
                "Failed to parse account at {}; is this a pool?",
                pool_address
            )
            .into());
        }
    } else {
        return Err(format!("Pool {} does not exist", pool_address).into());
    };

    let pool_stake_address = find_pool_stake_address(&spl_single_pool::id(), &pool_address);
    let available_stake =
        if let Some((_, stake)) = quarantine::get_stake_info(config, &pool_stake_address).await? {
            stake.delegation.stake - quarantine::get_minimum_delegation(config).await?
        } else {
            unreachable!()
        };

    let pool_mint_address = find_pool_mint_address(&spl_single_pool::id(), &pool_address);
    let token_supply = config
        .rpc_client
        .get_token_supply(&pool_mint_address)
        .await?
        .amount
        .parse::<u64>()?;

    Ok(StakePoolOutput {
        pool_address,
        vote_account_address,
        available_stake,
        token_supply,
        signature: None,
    })
}

async fn process_transaction(
    config: &Config,
    transaction: Transaction,
) -> Result<Option<Signature>, Error> {
    if config.dry_run {
        let simulation_data = config.rpc_client.simulate_transaction(&transaction).await?;

        if config.verbose() {
            if let Some(logs) = simulation_data.value.logs {
                for log in logs {
                    println!("    {}", log);
                }
            }

            println!(
                "\nSimulation succeeded, consumed {} compute units",
                simulation_data.value.units_consumed.unwrap()
            );
        } else {
            println_display(config, "Simulation succeeded".to_string());
        }

        Ok(None)
    } else {
        Ok(Some(
            config
                .rpc_client
                .send_and_confirm_transaction_with_spinner(&transaction)
                .await?,
        ))
    }
}
