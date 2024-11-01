//! program state processor

use {
    crate::{
        error::SinglePoolError,
        inline_mpl_token_metadata::{
            self,
            instruction::{create_metadata_accounts_v3, update_metadata_accounts_v2},
            pda::find_metadata_account,
            state::DataV2,
        },
        instruction::SinglePoolInstruction,
        state::{SinglePool, SinglePoolAccountType},
        MINT_DECIMALS, POOL_MINT_AUTHORITY_PREFIX, POOL_MINT_PREFIX, POOL_MPL_AUTHORITY_PREFIX,
        POOL_PREFIX, POOL_STAKE_AUTHORITY_PREFIX, POOL_STAKE_PREFIX,
        VOTE_STATE_AUTHORIZED_WITHDRAWER_END, VOTE_STATE_AUTHORIZED_WITHDRAWER_START,
        VOTE_STATE_DISCRIMINATOR_END,
    },
    borsh::BorshDeserialize,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh1::{get_packed_len, try_from_slice_unchecked},
        entrypoint::ProgramResult,
        msg,
        native_token::LAMPORTS_PER_SOL,
        program::invoke_signed,
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        rent::Rent,
        stake::{
            self,
            state::{Meta, Stake, StakeStateV2},
        },
        stake_history::Epoch,
        system_instruction, system_program,
        sysvar::{clock::Clock, Sysvar},
        vote::program as vote_program,
    },
    spl_token::state::Mint,
};

/// Calculate pool tokens to mint, given outstanding token supply, pool active
/// stake, and deposit active stake
fn calculate_deposit_amount(
    pre_token_supply: u64,
    pre_pool_stake: u64,
    user_stake_to_deposit: u64,
) -> Option<u64> {
    if pre_pool_stake == 0 || pre_token_supply == 0 {
        Some(user_stake_to_deposit)
    } else {
        u64::try_from(
            (user_stake_to_deposit as u128)
                .checked_mul(pre_token_supply as u128)?
                .checked_div(pre_pool_stake as u128)?,
        )
        .ok()
    }
}

/// Calculate pool stake to return, given outstanding token supply, pool active
/// stake, and tokens to redeem
fn calculate_withdraw_amount(
    pre_token_supply: u64,
    pre_pool_stake: u64,
    user_tokens_to_burn: u64,
) -> Option<u64> {
    let numerator = (user_tokens_to_burn as u128).checked_mul(pre_pool_stake as u128)?;
    let denominator = pre_token_supply as u128;
    if numerator < denominator || denominator == 0 {
        Some(0)
    } else {
        u64::try_from(numerator.checked_div(denominator)?).ok()
    }
}

/// Deserialize the stake state from AccountInfo
fn get_stake_state(stake_account_info: &AccountInfo) -> Result<(Meta, Stake), ProgramError> {
    let stake_state = try_from_slice_unchecked::<StakeStateV2>(&stake_account_info.data.borrow())?;

    match stake_state {
        StakeStateV2::Stake(meta, stake, _) => Ok((meta, stake)),
        _ => Err(SinglePoolError::WrongStakeStake.into()),
    }
}

/// Deserialize the stake amount from AccountInfo
fn get_stake_amount(stake_account_info: &AccountInfo) -> Result<u64, ProgramError> {
    Ok(get_stake_state(stake_account_info)?.1.delegation.stake)
}

/// Determine if stake is active
fn is_stake_active_without_history(stake: &Stake, current_epoch: Epoch) -> bool {
    stake.delegation.activation_epoch < current_epoch
        && stake.delegation.deactivation_epoch == Epoch::MAX
}

/// Check pool account address for the validator vote account
fn check_pool_address(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
    check_address: &Pubkey,
) -> Result<u8, ProgramError> {
    check_pool_pda(
        program_id,
        vote_account_address,
        check_address,
        &crate::find_pool_address_and_bump,
        "pool",
        SinglePoolError::InvalidPoolAccount,
    )
}

/// Check pool stake account address for the pool account
fn check_pool_stake_address(
    program_id: &Pubkey,
    pool_address: &Pubkey,
    check_address: &Pubkey,
) -> Result<u8, ProgramError> {
    check_pool_pda(
        program_id,
        pool_address,
        check_address,
        &crate::find_pool_stake_address_and_bump,
        "stake account",
        SinglePoolError::InvalidPoolStakeAccount,
    )
}

/// Check pool mint address for the pool account
fn check_pool_mint_address(
    program_id: &Pubkey,
    pool_address: &Pubkey,
    check_address: &Pubkey,
) -> Result<u8, ProgramError> {
    check_pool_pda(
        program_id,
        pool_address,
        check_address,
        &crate::find_pool_mint_address_and_bump,
        "mint",
        SinglePoolError::InvalidPoolMint,
    )
}

/// Check pool stake authority address for the pool account
fn check_pool_stake_authority_address(
    program_id: &Pubkey,
    pool_address: &Pubkey,
    check_address: &Pubkey,
) -> Result<u8, ProgramError> {
    check_pool_pda(
        program_id,
        pool_address,
        check_address,
        &crate::find_pool_stake_authority_address_and_bump,
        "stake authority",
        SinglePoolError::InvalidPoolStakeAuthority,
    )
}

/// Check pool mint authority address for the pool account
fn check_pool_mint_authority_address(
    program_id: &Pubkey,
    pool_address: &Pubkey,
    check_address: &Pubkey,
) -> Result<u8, ProgramError> {
    check_pool_pda(
        program_id,
        pool_address,
        check_address,
        &crate::find_pool_mint_authority_address_and_bump,
        "mint authority",
        SinglePoolError::InvalidPoolMintAuthority,
    )
}

/// Check pool MPL authority address for the pool account
fn check_pool_mpl_authority_address(
    program_id: &Pubkey,
    pool_address: &Pubkey,
    check_address: &Pubkey,
) -> Result<u8, ProgramError> {
    check_pool_pda(
        program_id,
        pool_address,
        check_address,
        &crate::find_pool_mpl_authority_address_and_bump,
        "MPL authority",
        SinglePoolError::InvalidPoolMplAuthority,
    )
}

fn check_pool_pda(
    program_id: &Pubkey,
    base_address: &Pubkey,
    check_address: &Pubkey,
    pda_lookup_fn: &dyn Fn(&Pubkey, &Pubkey) -> (Pubkey, u8),
    pda_name: &str,
    pool_error: SinglePoolError,
) -> Result<u8, ProgramError> {
    let (derived_address, bump_seed) = pda_lookup_fn(program_id, base_address);
    if *check_address != derived_address {
        msg!(
            "Incorrect {} address for base {}: expected {}, received {}",
            pda_name,
            base_address,
            derived_address,
            check_address,
        );
        Err(pool_error.into())
    } else {
        Ok(bump_seed)
    }
}

/// Check vote account is owned by the vote program and not a legacy variant
fn check_vote_account(vote_account_info: &AccountInfo) -> Result<(), ProgramError> {
    check_account_owner(vote_account_info, &vote_program::id())?;

    let vote_account_data = &vote_account_info.try_borrow_data()?;
    let state_variant = vote_account_data
        .get(..VOTE_STATE_DISCRIMINATOR_END)
        .and_then(|s| s.try_into().ok())
        .ok_or(SinglePoolError::UnparseableVoteAccount)?;

    match u32::from_le_bytes(state_variant) {
        1 | 2 => Ok(()),
        0 => Err(SinglePoolError::LegacyVoteAccount.into()),
        _ => Err(SinglePoolError::UnparseableVoteAccount.into()),
    }
}

/// Check MPL metadata account address for the pool mint
fn check_mpl_metadata_account_address(
    metadata_address: &Pubkey,
    pool_mint: &Pubkey,
) -> Result<(), ProgramError> {
    let (metadata_account_pubkey, _) = find_metadata_account(pool_mint);
    if metadata_account_pubkey != *metadata_address {
        Err(SinglePoolError::InvalidMetadataAccount.into())
    } else {
        Ok(())
    }
}

/// Check system program address
fn check_system_program(program_id: &Pubkey) -> Result<(), ProgramError> {
    if *program_id != system_program::id() {
        msg!(
            "Expected system program {}, received {}",
            system_program::id(),
            program_id
        );
        Err(ProgramError::IncorrectProgramId)
    } else {
        Ok(())
    }
}

/// Check token program address
fn check_token_program(address: &Pubkey) -> Result<(), ProgramError> {
    if *address != spl_token::id() {
        msg!(
            "Incorrect token program, expected {}, received {}",
            spl_token::id(),
            address
        );
        Err(ProgramError::IncorrectProgramId)
    } else {
        Ok(())
    }
}

/// Check stake program address
fn check_stake_program(program_id: &Pubkey) -> Result<(), ProgramError> {
    if *program_id != stake::program::id() {
        msg!(
            "Expected stake program {}, received {}",
            stake::program::id(),
            program_id
        );
        Err(ProgramError::IncorrectProgramId)
    } else {
        Ok(())
    }
}

/// Check MPL metadata program
fn check_mpl_metadata_program(program_id: &Pubkey) -> Result<(), ProgramError> {
    if *program_id != inline_mpl_token_metadata::id() {
        msg!(
            "Expected MPL metadata program {}, received {}",
            inline_mpl_token_metadata::id(),
            program_id
        );
        Err(ProgramError::IncorrectProgramId)
    } else {
        Ok(())
    }
}

/// Check account owner is the given program
fn check_account_owner(
    account_info: &AccountInfo,
    program_id: &Pubkey,
) -> Result<(), ProgramError> {
    if *program_id != *account_info.owner {
        msg!(
            "Expected account to be owned by program {}, received {}",
            program_id,
            account_info.owner
        );
        Err(ProgramError::IncorrectProgramId)
    } else {
        Ok(())
    }
}

/// Minimum delegation to create a pool
/// We floor at 1sol to avoid over-minting tokens before the relevant feature is
/// active
fn minimum_delegation() -> Result<u64, ProgramError> {
    Ok(std::cmp::max(
        stake::tools::get_minimum_delegation()?,
        LAMPORTS_PER_SOL,
    ))
}

/// Program state handler.
pub struct Processor {}
impl Processor {
    #[allow(clippy::too_many_arguments)]
    fn stake_merge<'a>(
        pool_account_key: &Pubkey,
        source_account: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        bump_seed: u8,
        destination_account: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        stake_history: AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let authority_seeds = &[
            POOL_STAKE_AUTHORITY_PREFIX,
            pool_account_key.as_ref(),
            &[bump_seed],
        ];
        let signers = &[&authority_seeds[..]];

        invoke_signed(
            &stake::instruction::merge(destination_account.key, source_account.key, authority.key)
                [0],
            &[
                destination_account,
                source_account,
                clock,
                stake_history,
                authority,
            ],
            signers,
        )
    }

    fn stake_split<'a>(
        pool_account_key: &Pubkey,
        stake_account: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        bump_seed: u8,
        amount: u64,
        split_stake: AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let authority_seeds = &[
            POOL_STAKE_AUTHORITY_PREFIX,
            pool_account_key.as_ref(),
            &[bump_seed],
        ];
        let signers = &[&authority_seeds[..]];

        let split_instruction =
            stake::instruction::split(stake_account.key, authority.key, amount, split_stake.key);

        invoke_signed(
            split_instruction.last().unwrap(),
            &[stake_account, split_stake, authority],
            signers,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn stake_authorize<'a>(
        pool_account_key: &Pubkey,
        stake_account: AccountInfo<'a>,
        stake_authority: AccountInfo<'a>,
        bump_seed: u8,
        new_stake_authority: &Pubkey,
        clock: AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let authority_seeds = &[
            POOL_STAKE_AUTHORITY_PREFIX,
            pool_account_key.as_ref(),
            &[bump_seed],
        ];
        let signers = &[&authority_seeds[..]];

        let authorize_instruction = stake::instruction::authorize(
            stake_account.key,
            stake_authority.key,
            new_stake_authority,
            stake::state::StakeAuthorize::Staker,
            None,
        );

        invoke_signed(
            &authorize_instruction,
            &[
                stake_account.clone(),
                clock.clone(),
                stake_authority.clone(),
            ],
            signers,
        )?;

        let authorize_instruction = stake::instruction::authorize(
            stake_account.key,
            stake_authority.key,
            new_stake_authority,
            stake::state::StakeAuthorize::Withdrawer,
            None,
        );
        invoke_signed(
            &authorize_instruction,
            &[stake_account, clock, stake_authority],
            signers,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn stake_withdraw<'a>(
        pool_account_key: &Pubkey,
        stake_account: AccountInfo<'a>,
        stake_authority: AccountInfo<'a>,
        bump_seed: u8,
        destination_account: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        stake_history: AccountInfo<'a>,
        lamports: u64,
    ) -> Result<(), ProgramError> {
        let authority_seeds = &[
            POOL_STAKE_AUTHORITY_PREFIX,
            pool_account_key.as_ref(),
            &[bump_seed],
        ];
        let signers = &[&authority_seeds[..]];

        let withdraw_instruction = stake::instruction::withdraw(
            stake_account.key,
            stake_authority.key,
            destination_account.key,
            lamports,
            None,
        );

        invoke_signed(
            &withdraw_instruction,
            &[
                stake_account,
                destination_account,
                clock,
                stake_history,
                stake_authority,
            ],
            signers,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn token_mint_to<'a>(
        pool_account_key: &Pubkey,
        token_program: AccountInfo<'a>,
        mint: AccountInfo<'a>,
        destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        bump_seed: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let authority_seeds = &[
            POOL_MINT_AUTHORITY_PREFIX,
            pool_account_key.as_ref(),
            &[bump_seed],
        ];
        let signers = &[&authority_seeds[..]];

        let ix = spl_token::instruction::mint_to(
            token_program.key,
            mint.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?;

        invoke_signed(&ix, &[mint, destination, authority], signers)
    }

    #[allow(clippy::too_many_arguments)]
    fn token_burn<'a>(
        pool_account_key: &Pubkey,
        token_program: AccountInfo<'a>,
        burn_account: AccountInfo<'a>,
        mint: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        bump_seed: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let authority_seeds = &[
            POOL_MINT_AUTHORITY_PREFIX,
            pool_account_key.as_ref(),
            &[bump_seed],
        ];
        let signers = &[&authority_seeds[..]];

        let ix = spl_token::instruction::burn(
            token_program.key,
            burn_account.key,
            mint.key,
            authority.key,
            &[],
            amount,
        )?;

        invoke_signed(&ix, &[burn_account, mint, authority], signers)
    }

    fn process_initialize_pool(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let vote_account_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_stake_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let pool_stake_authority_info = next_account_info(account_info_iter)?;
        let pool_mint_authority_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let clock_info = next_account_info(account_info_iter)?;
        let stake_history_info = next_account_info(account_info_iter)?;
        let stake_config_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let stake_program_info = next_account_info(account_info_iter)?;

        check_vote_account(vote_account_info)?;
        let pool_bump_seed = check_pool_address(program_id, vote_account_info.key, pool_info.key)?;
        let stake_bump_seed =
            check_pool_stake_address(program_id, pool_info.key, pool_stake_info.key)?;
        let mint_bump_seed =
            check_pool_mint_address(program_id, pool_info.key, pool_mint_info.key)?;
        let stake_authority_bump_seed = check_pool_stake_authority_address(
            program_id,
            pool_info.key,
            pool_stake_authority_info.key,
        )?;
        let mint_authority_bump_seed = check_pool_mint_authority_address(
            program_id,
            pool_info.key,
            pool_mint_authority_info.key,
        )?;
        check_system_program(system_program_info.key)?;
        check_token_program(token_program_info.key)?;
        check_stake_program(stake_program_info.key)?;

        let pool_seeds = &[
            POOL_PREFIX,
            vote_account_info.key.as_ref(),
            &[pool_bump_seed],
        ];
        let pool_signers = &[&pool_seeds[..]];

        let stake_seeds = &[
            POOL_STAKE_PREFIX,
            pool_info.key.as_ref(),
            &[stake_bump_seed],
        ];
        let stake_signers = &[&stake_seeds[..]];

        let mint_seeds = &[POOL_MINT_PREFIX, pool_info.key.as_ref(), &[mint_bump_seed]];
        let mint_signers = &[&mint_seeds[..]];

        let stake_authority_seeds = &[
            POOL_STAKE_AUTHORITY_PREFIX,
            pool_info.key.as_ref(),
            &[stake_authority_bump_seed],
        ];
        let stake_authority_signers = &[&stake_authority_seeds[..]];

        let mint_authority_seeds = &[
            POOL_MINT_AUTHORITY_PREFIX,
            pool_info.key.as_ref(),
            &[mint_authority_bump_seed],
        ];
        let mint_authority_signers = &[&mint_authority_seeds[..]];

        // create the pool. user has already transferred in rent
        let pool_space = get_packed_len::<SinglePool>();
        if !rent.is_exempt(pool_info.lamports(), pool_space) {
            return Err(SinglePoolError::WrongRentAmount.into());
        }
        if pool_info.data_len() != 0 {
            return Err(SinglePoolError::PoolAlreadyInitialized.into());
        }

        invoke_signed(
            &system_instruction::allocate(pool_info.key, pool_space as u64),
            &[pool_info.clone()],
            pool_signers,
        )?;

        invoke_signed(
            &system_instruction::assign(pool_info.key, program_id),
            &[pool_info.clone()],
            pool_signers,
        )?;

        let mut pool = try_from_slice_unchecked::<SinglePool>(&pool_info.data.borrow())?;
        pool.account_type = SinglePoolAccountType::Pool;
        pool.vote_account_address = *vote_account_info.key;
        borsh::to_writer(&mut pool_info.data.borrow_mut()[..], &pool)?;

        // create the pool mint. user has already transferred in rent
        let mint_space = spl_token::state::Mint::LEN;

        invoke_signed(
            &system_instruction::allocate(pool_mint_info.key, mint_space as u64),
            &[pool_mint_info.clone()],
            mint_signers,
        )?;

        invoke_signed(
            &system_instruction::assign(pool_mint_info.key, token_program_info.key),
            &[pool_mint_info.clone()],
            mint_signers,
        )?;

        invoke_signed(
            &spl_token::instruction::initialize_mint2(
                token_program_info.key,
                pool_mint_info.key,
                pool_mint_authority_info.key,
                None,
                MINT_DECIMALS,
            )?,
            &[pool_mint_info.clone()],
            mint_authority_signers,
        )?;

        // create the pool stake account. user has already transferred in rent plus at
        // least the minimum
        let minimum_delegation = minimum_delegation()?;
        let stake_space = std::mem::size_of::<stake::state::StakeStateV2>();
        let stake_rent_plus_initial = rent
            .minimum_balance(stake_space)
            .saturating_add(minimum_delegation);

        if pool_stake_info.lamports() < stake_rent_plus_initial {
            return Err(SinglePoolError::WrongRentAmount.into());
        }

        let authorized = stake::state::Authorized::auto(pool_stake_authority_info.key);

        invoke_signed(
            &system_instruction::allocate(pool_stake_info.key, stake_space as u64),
            &[pool_stake_info.clone()],
            stake_signers,
        )?;

        invoke_signed(
            &system_instruction::assign(pool_stake_info.key, stake_program_info.key),
            &[pool_stake_info.clone()],
            stake_signers,
        )?;

        invoke_signed(
            &stake::instruction::initialize_checked(pool_stake_info.key, &authorized),
            &[
                pool_stake_info.clone(),
                rent_info.clone(),
                pool_stake_authority_info.clone(),
                pool_stake_authority_info.clone(),
            ],
            stake_authority_signers,
        )?;

        // delegate stake so it activates
        invoke_signed(
            &stake::instruction::delegate_stake(
                pool_stake_info.key,
                pool_stake_authority_info.key,
                vote_account_info.key,
            ),
            &[
                pool_stake_info.clone(),
                vote_account_info.clone(),
                clock_info.clone(),
                stake_history_info.clone(),
                stake_config_info.clone(),
                pool_stake_authority_info.clone(),
            ],
            stake_authority_signers,
        )?;

        Ok(())
    }

    fn process_reactivate_pool_stake(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let vote_account_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_stake_info = next_account_info(account_info_iter)?;
        let pool_stake_authority_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        let stake_history_info = next_account_info(account_info_iter)?;
        let stake_config_info = next_account_info(account_info_iter)?;
        let stake_program_info = next_account_info(account_info_iter)?;

        check_vote_account(vote_account_info)?;
        check_pool_address(program_id, vote_account_info.key, pool_info.key)?;

        SinglePool::from_account_info(pool_info, program_id)?;

        check_pool_stake_address(program_id, pool_info.key, pool_stake_info.key)?;
        let stake_authority_bump_seed = check_pool_stake_authority_address(
            program_id,
            pool_info.key,
            pool_stake_authority_info.key,
        )?;
        check_stake_program(stake_program_info.key)?;

        let (_, pool_stake_state) = get_stake_state(pool_stake_info)?;
        if pool_stake_state.delegation.deactivation_epoch > clock.epoch {
            return Err(SinglePoolError::WrongStakeStake.into());
        }

        let stake_authority_seeds = &[
            POOL_STAKE_AUTHORITY_PREFIX,
            pool_info.key.as_ref(),
            &[stake_authority_bump_seed],
        ];
        let stake_authority_signers = &[&stake_authority_seeds[..]];

        // delegate stake so it activates
        invoke_signed(
            &stake::instruction::delegate_stake(
                pool_stake_info.key,
                pool_stake_authority_info.key,
                vote_account_info.key,
            ),
            &[
                pool_stake_info.clone(),
                vote_account_info.clone(),
                clock_info.clone(),
                stake_history_info.clone(),
                stake_config_info.clone(),
                pool_stake_authority_info.clone(),
            ],
            stake_authority_signers,
        )?;

        Ok(())
    }

    fn process_deposit_stake(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_info = next_account_info(account_info_iter)?;
        let pool_stake_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let pool_stake_authority_info = next_account_info(account_info_iter)?;
        let pool_mint_authority_info = next_account_info(account_info_iter)?;
        let user_stake_info = next_account_info(account_info_iter)?;
        let user_token_account_info = next_account_info(account_info_iter)?;
        let user_lamport_account_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        let stake_history_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let stake_program_info = next_account_info(account_info_iter)?;

        SinglePool::from_account_info(pool_info, program_id)?;

        check_pool_stake_address(program_id, pool_info.key, pool_stake_info.key)?;
        let stake_authority_bump_seed = check_pool_stake_authority_address(
            program_id,
            pool_info.key,
            pool_stake_authority_info.key,
        )?;
        let mint_authority_bump_seed = check_pool_mint_authority_address(
            program_id,
            pool_info.key,
            pool_mint_authority_info.key,
        )?;
        check_pool_mint_address(program_id, pool_info.key, pool_mint_info.key)?;
        check_token_program(token_program_info.key)?;
        check_stake_program(stake_program_info.key)?;

        if pool_stake_info.key == user_stake_info.key {
            return Err(SinglePoolError::InvalidPoolStakeAccountUsage.into());
        }

        let minimum_delegation = minimum_delegation()?;

        let (_, pool_stake_state) = get_stake_state(pool_stake_info)?;
        let pre_pool_stake = pool_stake_state
            .delegation
            .stake
            .saturating_sub(minimum_delegation);
        msg!("Available stake pre merge {}", pre_pool_stake);

        // user can deposit active stake into an active pool or inactive stake into an
        // activating pool
        let (user_stake_meta, user_stake_state) = get_stake_state(user_stake_info)?;
        if user_stake_meta.authorized
            != stake::state::Authorized::auto(pool_stake_authority_info.key)
            || is_stake_active_without_history(&pool_stake_state, clock.epoch)
                != is_stake_active_without_history(&user_stake_state, clock.epoch)
        {
            return Err(SinglePoolError::WrongStakeStake.into());
        }

        // merge the user stake account, which is preauthed to us, into the pool stake
        // account this merge succeeding implicitly validates authority/lockup
        // of the user stake account
        Self::stake_merge(
            pool_info.key,
            user_stake_info.clone(),
            pool_stake_authority_info.clone(),
            stake_authority_bump_seed,
            pool_stake_info.clone(),
            clock_info.clone(),
            stake_history_info.clone(),
        )?;

        let (pool_stake_meta, pool_stake_state) = get_stake_state(pool_stake_info)?;
        let post_pool_stake = pool_stake_state
            .delegation
            .stake
            .saturating_sub(minimum_delegation);
        let post_pool_lamports = pool_stake_info.lamports();
        msg!("Available stake post merge {}", post_pool_stake);

        // stake lamports added, as a stake difference
        let stake_added = post_pool_stake
            .checked_sub(pre_pool_stake)
            .ok_or(SinglePoolError::ArithmeticOverflow)?;

        // we calculate absolute rather than relative to deposit amount to allow
        // claiming lamports mistakenly transferred in
        let excess_lamports = post_pool_lamports
            .checked_sub(pool_stake_state.delegation.stake)
            .and_then(|amount| amount.checked_sub(pool_stake_meta.rent_exempt_reserve))
            .ok_or(SinglePoolError::ArithmeticOverflow)?;

        // sanity check: the user stake account is empty
        if user_stake_info.lamports() != 0 {
            return Err(SinglePoolError::UnexpectedMathError.into());
        }

        let token_supply = {
            let pool_mint_data = pool_mint_info.try_borrow_data()?;
            let pool_mint = Mint::unpack_from_slice(&pool_mint_data)?;
            pool_mint.supply
        };

        // deposit amount is determined off stake because we return excess rent
        let new_pool_tokens = calculate_deposit_amount(token_supply, pre_pool_stake, stake_added)
            .ok_or(SinglePoolError::UnexpectedMathError)?;

        if new_pool_tokens == 0 {
            return Err(SinglePoolError::DepositTooSmall.into());
        }

        // mint tokens to the user corresponding to their stake deposit
        Self::token_mint_to(
            pool_info.key,
            token_program_info.clone(),
            pool_mint_info.clone(),
            user_token_account_info.clone(),
            pool_mint_authority_info.clone(),
            mint_authority_bump_seed,
            new_pool_tokens,
        )?;

        // return the lamports their stake account previously held for rent-exemption
        if excess_lamports > 0 {
            Self::stake_withdraw(
                pool_info.key,
                pool_stake_info.clone(),
                pool_stake_authority_info.clone(),
                stake_authority_bump_seed,
                user_lamport_account_info.clone(),
                clock_info.clone(),
                stake_history_info.clone(),
                excess_lamports,
            )?;
        }

        Ok(())
    }

    fn process_withdraw_stake(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        user_stake_authority: &Pubkey,
        token_amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_info = next_account_info(account_info_iter)?;
        let pool_stake_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let pool_stake_authority_info = next_account_info(account_info_iter)?;
        let pool_mint_authority_info = next_account_info(account_info_iter)?;
        let user_stake_info = next_account_info(account_info_iter)?;
        let user_token_account_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let stake_program_info = next_account_info(account_info_iter)?;

        SinglePool::from_account_info(pool_info, program_id)?;

        check_pool_stake_address(program_id, pool_info.key, pool_stake_info.key)?;
        let stake_authority_bump_seed = check_pool_stake_authority_address(
            program_id,
            pool_info.key,
            pool_stake_authority_info.key,
        )?;
        let mint_authority_bump_seed = check_pool_mint_authority_address(
            program_id,
            pool_info.key,
            pool_mint_authority_info.key,
        )?;
        check_pool_mint_address(program_id, pool_info.key, pool_mint_info.key)?;
        check_token_program(token_program_info.key)?;
        check_stake_program(stake_program_info.key)?;

        if pool_stake_info.key == user_stake_info.key {
            return Err(SinglePoolError::InvalidPoolStakeAccountUsage.into());
        }

        let minimum_delegation = minimum_delegation()?;

        let pre_pool_stake = get_stake_amount(pool_stake_info)?.saturating_sub(minimum_delegation);
        msg!("Available stake pre split {}", pre_pool_stake);

        let token_supply = {
            let pool_mint_data = pool_mint_info.try_borrow_data()?;
            let pool_mint = Mint::unpack_from_slice(&pool_mint_data)?;
            pool_mint.supply
        };

        // withdraw amount is determined off stake just like deposit amount
        let withdraw_stake = calculate_withdraw_amount(token_supply, pre_pool_stake, token_amount)
            .ok_or(SinglePoolError::UnexpectedMathError)?;

        if withdraw_stake == 0 {
            return Err(SinglePoolError::WithdrawalTooSmall.into());
        }

        // the second case should never be true, but its best to be sure
        if withdraw_stake > pre_pool_stake || withdraw_stake == pool_stake_info.lamports() {
            return Err(SinglePoolError::WithdrawalTooLarge.into());
        }

        // burn user tokens corresponding to the amount of stake they wish to withdraw
        Self::token_burn(
            pool_info.key,
            token_program_info.clone(),
            user_token_account_info.clone(),
            pool_mint_info.clone(),
            pool_mint_authority_info.clone(),
            mint_authority_bump_seed,
            token_amount,
        )?;

        // split stake into a blank stake account the user has created for this purpose
        Self::stake_split(
            pool_info.key,
            pool_stake_info.clone(),
            pool_stake_authority_info.clone(),
            stake_authority_bump_seed,
            withdraw_stake,
            user_stake_info.clone(),
        )?;

        // assign both authorities on the new stake account to the user
        Self::stake_authorize(
            pool_info.key,
            user_stake_info.clone(),
            pool_stake_authority_info.clone(),
            stake_authority_bump_seed,
            user_stake_authority,
            clock_info.clone(),
        )?;

        let post_pool_stake = get_stake_amount(pool_stake_info)?.saturating_sub(minimum_delegation);
        msg!("Available stake post split {}", post_pool_stake);

        Ok(())
    }

    fn process_create_pool_token_metadata(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let pool_mint_authority_info = next_account_info(account_info_iter)?;
        let pool_mpl_authority_info = next_account_info(account_info_iter)?;
        let payer_info = next_account_info(account_info_iter)?;
        let metadata_info = next_account_info(account_info_iter)?;
        let mpl_token_metadata_program_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        let pool = SinglePool::from_account_info(pool_info, program_id)?;

        let mint_authority_bump_seed = check_pool_mint_authority_address(
            program_id,
            pool_info.key,
            pool_mint_authority_info.key,
        )?;
        let mpl_authority_bump_seed = check_pool_mpl_authority_address(
            program_id,
            pool_info.key,
            pool_mpl_authority_info.key,
        )?;
        check_pool_mint_address(program_id, pool_info.key, pool_mint_info.key)?;
        check_system_program(system_program_info.key)?;
        check_account_owner(payer_info, &system_program::id())?;
        check_mpl_metadata_program(mpl_token_metadata_program_info.key)?;
        check_mpl_metadata_account_address(metadata_info.key, pool_mint_info.key)?;

        if !payer_info.is_signer {
            msg!("Payer did not sign metadata creation");
            return Err(SinglePoolError::SignatureMissing.into());
        }

        let vote_address_str = pool.vote_account_address.to_string();
        let token_name = format!("SPL Single Pool {}", &vote_address_str[0..15]);
        let token_symbol = format!("st{}", &vote_address_str[0..7]);

        let new_metadata_instruction = create_metadata_accounts_v3(
            *mpl_token_metadata_program_info.key,
            *metadata_info.key,
            *pool_mint_info.key,
            *pool_mint_authority_info.key,
            *payer_info.key,
            *pool_mpl_authority_info.key,
            token_name,
            token_symbol,
            "".to_string(),
        );

        let mint_authority_seeds = &[
            POOL_MINT_AUTHORITY_PREFIX,
            pool_info.key.as_ref(),
            &[mint_authority_bump_seed],
        ];
        let mpl_authority_seeds = &[
            POOL_MPL_AUTHORITY_PREFIX,
            pool_info.key.as_ref(),
            &[mpl_authority_bump_seed],
        ];
        let signers = &[&mint_authority_seeds[..], &mpl_authority_seeds[..]];

        invoke_signed(
            &new_metadata_instruction,
            &[
                metadata_info.clone(),
                pool_mint_info.clone(),
                pool_mint_authority_info.clone(),
                payer_info.clone(),
                pool_mpl_authority_info.clone(),
                system_program_info.clone(),
            ],
            signers,
        )?;

        Ok(())
    }

    fn process_update_pool_token_metadata(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        symbol: String,
        uri: String,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let vote_account_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_mpl_authority_info = next_account_info(account_info_iter)?;
        let authorized_withdrawer_info = next_account_info(account_info_iter)?;
        let metadata_info = next_account_info(account_info_iter)?;
        let mpl_token_metadata_program_info = next_account_info(account_info_iter)?;

        check_vote_account(vote_account_info)?;
        check_pool_address(program_id, vote_account_info.key, pool_info.key)?;

        let pool = SinglePool::from_account_info(pool_info, program_id)?;
        if pool.vote_account_address != *vote_account_info.key {
            return Err(SinglePoolError::InvalidPoolAccount.into());
        }

        let mpl_authority_bump_seed = check_pool_mpl_authority_address(
            program_id,
            pool_info.key,
            pool_mpl_authority_info.key,
        )?;
        let pool_mint_address = crate::find_pool_mint_address(program_id, pool_info.key);
        check_mpl_metadata_program(mpl_token_metadata_program_info.key)?;
        check_mpl_metadata_account_address(metadata_info.key, &pool_mint_address)?;

        // we use authorized_withdrawer to authenticate the caller controls the vote
        // account this is safer than using an authorized_voter since those keys
        // live hot and validator-operators we spoke with indicated this would
        // be their preference as well
        let vote_account_data = &vote_account_info.try_borrow_data()?;
        let vote_account_withdrawer = vote_account_data
            .get(VOTE_STATE_AUTHORIZED_WITHDRAWER_START..VOTE_STATE_AUTHORIZED_WITHDRAWER_END)
            .and_then(|x| Pubkey::try_from(x).ok())
            .ok_or(SinglePoolError::UnparseableVoteAccount)?;

        if *authorized_withdrawer_info.key != vote_account_withdrawer {
            msg!("Vote account authorized withdrawer does not match the account provided.");
            return Err(SinglePoolError::InvalidMetadataSigner.into());
        }

        if !authorized_withdrawer_info.is_signer {
            msg!("Vote account authorized withdrawer did not sign metadata update.");
            return Err(SinglePoolError::SignatureMissing.into());
        }

        let update_metadata_accounts_instruction = update_metadata_accounts_v2(
            *mpl_token_metadata_program_info.key,
            *metadata_info.key,
            *pool_mpl_authority_info.key,
            None,
            Some(DataV2 {
                name,
                symbol,
                uri,
                seller_fee_basis_points: 0,
                creators: None,
                collection: None,
                uses: None,
            }),
            None,
            Some(true),
        );

        let mpl_authority_seeds = &[
            POOL_MPL_AUTHORITY_PREFIX,
            pool_info.key.as_ref(),
            &[mpl_authority_bump_seed],
        ];
        let signers = &[&mpl_authority_seeds[..]];

        invoke_signed(
            &update_metadata_accounts_instruction,
            &[metadata_info.clone(), pool_mpl_authority_info.clone()],
            signers,
        )?;

        Ok(())
    }

    /// Processes [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = SinglePoolInstruction::try_from_slice(input)?;
        match instruction {
            SinglePoolInstruction::InitializePool => {
                msg!("Instruction: InitializePool");
                Self::process_initialize_pool(program_id, accounts)
            }
            SinglePoolInstruction::ReactivatePoolStake => {
                msg!("Instruction: ReactivatePoolStake");
                Self::process_reactivate_pool_stake(program_id, accounts)
            }
            SinglePoolInstruction::DepositStake => {
                msg!("Instruction: DepositStake");
                Self::process_deposit_stake(program_id, accounts)
            }
            SinglePoolInstruction::WithdrawStake {
                user_stake_authority,
                token_amount,
            } => {
                msg!("Instruction: WithdrawStake");
                Self::process_withdraw_stake(
                    program_id,
                    accounts,
                    &user_stake_authority,
                    token_amount,
                )
            }
            SinglePoolInstruction::CreateTokenMetadata => {
                msg!("Instruction: CreateTokenMetadata");
                Self::process_create_pool_token_metadata(program_id, accounts)
            }
            SinglePoolInstruction::UpdateTokenMetadata { name, symbol, uri } => {
                msg!("Instruction: UpdateTokenMetadata");
                Self::process_update_pool_token_metadata(program_id, accounts, name, symbol, uri)
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects)]
mod tests {
    use {
        super::*,
        approx::assert_relative_eq,
        rand::{
            distributions::{Distribution, Uniform},
            rngs::StdRng,
            seq::{IteratorRandom, SliceRandom},
            Rng, SeedableRng,
        },
        std::collections::BTreeMap,
        test_case::test_case,
    };

    // approximately 6%/yr assuking 146 epochs
    const INFLATION_BASE_RATE: f64 = 0.0004;

    #[derive(Clone, Debug, Default)]
    struct PoolState {
        pub token_supply: u64,
        pub total_stake: u64,
        pub user_token_balances: BTreeMap<Pubkey, u64>,
    }
    impl PoolState {
        // deposits a given amount of stake and returns the equivalent tokens on success
        // note this is written as unsugared do-notation, so *any* failure returns None
        // otherwise returns the value produced by its respective calculate function
        #[rustfmt::skip]
        pub fn deposit(&mut self, user_pubkey: &Pubkey, stake_to_deposit: u64) -> Option<u64> {
            calculate_deposit_amount(self.token_supply, self.total_stake, stake_to_deposit)
                .and_then(|tokens_to_mint| self.token_supply.checked_add(tokens_to_mint)
                .and_then(|new_token_supply| self.total_stake.checked_add(stake_to_deposit)
                .and_then(|new_total_stake| self.user_token_balances.remove(user_pubkey).or(Some(0))
                .and_then(|old_user_token_balance| old_user_token_balance.checked_add(tokens_to_mint)
                .map(|new_user_token_balance| {
                    self.token_supply = new_token_supply;
                    self.total_stake = new_total_stake;
                    let _ = self.user_token_balances.insert(*user_pubkey, new_user_token_balance);
                    tokens_to_mint
            })))))
        }

        // burns a given amount of tokens and returns the equivalent stake on success
        // note this is written as unsugared do-notation, so *any* failure returns None
        // otherwise returns the value produced by its respective calculate function
        #[rustfmt::skip]
        pub fn withdraw(&mut self, user_pubkey: &Pubkey, tokens_to_burn: u64) -> Option<u64> {
            calculate_withdraw_amount(self.token_supply, self.total_stake, tokens_to_burn)
                .and_then(|stake_to_withdraw| self.token_supply.checked_sub(tokens_to_burn)
                .and_then(|new_token_supply| self.total_stake.checked_sub(stake_to_withdraw)
                .and_then(|new_total_stake| self.user_token_balances.remove(user_pubkey)
                .and_then(|old_user_token_balance| old_user_token_balance.checked_sub(tokens_to_burn)
                .map(|new_user_token_balance| {
                    self.token_supply = new_token_supply;
                    self.total_stake = new_total_stake;
                    let _ = self.user_token_balances.insert(*user_pubkey, new_user_token_balance);
                    stake_to_withdraw
            })))))
        }

        // adds an arbitrary amount of stake, as if inflation rewards were granted
        pub fn reward(&mut self, reward_amount: u64) {
            self.total_stake = self.total_stake.checked_add(reward_amount).unwrap();
        }

        // get the token balance for a user
        pub fn tokens(&self, user_pubkey: &Pubkey) -> u64 {
            *self.user_token_balances.get(user_pubkey).unwrap_or(&0)
        }

        // get the amount of stake that belongs to a user
        pub fn stake(&self, user_pubkey: &Pubkey) -> u64 {
            let tokens = self.tokens(user_pubkey);
            if tokens > 0 {
                u64::try_from(tokens as u128 * self.total_stake as u128 / self.token_supply as u128)
                    .unwrap()
            } else {
                0
            }
        }

        // get the share of the pool that belongs to a user, as a float between 0 and 1
        pub fn share(&self, user_pubkey: &Pubkey) -> f64 {
            let tokens = self.tokens(user_pubkey);
            if tokens > 0 {
                tokens as f64 / self.token_supply as f64
            } else {
                0.0
            }
        }
    }

    // this deterministically tests basic behavior of calculate_deposit_amount and
    // calculate_withdraw_amount
    #[test]
    fn simple_deposit_withdraw() {
        let mut pool = PoolState::default();
        let alice = Pubkey::new_unique();
        let bob = Pubkey::new_unique();
        let chad = Pubkey::new_unique();

        // first deposit. alice now has 250
        pool.deposit(&alice, 250).unwrap();
        assert_eq!(pool.tokens(&alice), 250);
        assert_eq!(pool.token_supply, 250);
        assert_eq!(pool.total_stake, 250);

        // second deposit. bob now has 750
        pool.deposit(&bob, 750).unwrap();
        assert_eq!(pool.tokens(&bob), 750);
        assert_eq!(pool.token_supply, 1000);
        assert_eq!(pool.total_stake, 1000);

        // alice controls 25% of the pool and bob controls 75%. rewards should accrue
        // likewise use nice even numbers, we can test fiddly stuff in the
        // stochastic cases
        assert_relative_eq!(pool.share(&alice), 0.25);
        assert_relative_eq!(pool.share(&bob), 0.75);
        pool.reward(1000);
        assert_eq!(pool.stake(&alice), pool.tokens(&alice) * 2);
        assert_eq!(pool.stake(&bob), pool.tokens(&bob) * 2);
        assert_relative_eq!(pool.share(&alice), 0.25);
        assert_relative_eq!(pool.share(&bob), 0.75);

        // alice harvests rewards, reducing her share of the *previous* pool size to
        // 12.5% but because the pool itself has shrunk to 87.5%, its actually
        // more like 14.3% luckily chad deposits immediately after to make our
        // math easier
        let stake_removed = pool.withdraw(&alice, 125).unwrap();
        pool.deposit(&chad, 250).unwrap();
        assert_eq!(stake_removed, 250);
        assert_relative_eq!(pool.share(&alice), 0.125);
        assert_relative_eq!(pool.share(&bob), 0.75);

        // bob and chad exit the pool
        let stake_removed = pool.withdraw(&bob, 750).unwrap();
        assert_eq!(stake_removed, 1500);
        assert_relative_eq!(pool.share(&bob), 0.0);
        pool.withdraw(&chad, 125).unwrap();
        assert_relative_eq!(pool.share(&alice), 1.0);
    }

    // this stochastically tests calculate_deposit_amount and
    // calculate_withdraw_amount the objective is specifically to ensure that
    // the math does not fail on any combination of state changes the no_minimum
    // case is to account for a future where small deposits are possible through
    // multistake
    #[test_case(rand::random(), false, false; "no_rewards")]
    #[test_case(rand::random(), true, false; "with_rewards")]
    #[test_case(rand::random(), true, true; "no_minimum")]
    fn random_deposit_withdraw(seed: u64, with_rewards: bool, no_minimum: bool) {
        println!(
            "TEST SEED: {}. edit the test case to pass this value if needed to debug failures",
            seed
        );
        let mut prng = rand::rngs::StdRng::seed_from_u64(seed);

        // deposit_range is the range of typical deposits within minimum_delegation
        // minnow_range is under the minimum for cases where we test that
        // op_range is how we roll whether to deposit, withdraw, or reward
        // std_range is a standard probability
        let deposit_range = Uniform::from(LAMPORTS_PER_SOL..LAMPORTS_PER_SOL * 1000);
        let minnow_range = Uniform::from(1..LAMPORTS_PER_SOL);
        let op_range = Uniform::from(if with_rewards { 0.0..1.0 } else { 0.0..0.65 });
        let std_range = Uniform::from(0.0..1.0);

        let deposit_amount = |prng: &mut StdRng| {
            if no_minimum && prng.gen_bool(0.2) {
                minnow_range.sample(prng)
            } else {
                deposit_range.sample(prng)
            }
        };

        // run everything a number of times to get a good sample
        for _ in 0..100 {
            // PoolState tracks all outstanding tokens and the total combined stake
            // there is no reasonable way to track "deposited stake" because reward accrual
            // makes this concept incoherent a token corresponds to a
            // percentage, not a stake value
            let mut pool = PoolState::default();

            // generate between 1 and 100 users and have ~half of them deposit
            // note for most of these tests we adhere to the minimum delegation
            // one of the thing we want to see is deposit size being many ooms larger than
            // reward size
            let mut users = vec![];
            let user_count: usize = prng.gen_range(1..=100);
            for _ in 0..user_count {
                let user = Pubkey::new_unique();

                if prng.gen_bool(0.5) {
                    pool.deposit(&user, deposit_amount(&mut prng)).unwrap();
                }

                users.push(user);
            }

            // now we do a set of arbitrary operations and confirm invariants hold
            // we underweight withdraw a little bit to lessen the chances we random walk to
            // an empty pool
            for _ in 0..1000 {
                match op_range.sample(&mut prng) {
                    // deposit a random amount of stake for tokens with a random user
                    // check their stake, tokens, and share increase by the expected amount
                    n if n <= 0.35 => {
                        let user = users.choose(&mut prng).unwrap();
                        let prev_share = pool.share(user);
                        let prev_stake = pool.stake(user);
                        let prev_token_supply = pool.token_supply;
                        let prev_total_stake = pool.total_stake;

                        let stake_deposited = deposit_amount(&mut prng);
                        let tokens_minted = pool.deposit(user, stake_deposited).unwrap();

                        // stake increased by exactly the deposit amount
                        assert_eq!(pool.total_stake - prev_total_stake, stake_deposited);

                        // calculated stake fraction is within 2 lamps of deposit amount
                        assert!(
                            (pool.stake(user) as i64 - prev_stake as i64 - stake_deposited as i64)
                                .abs()
                                <= 2
                        );

                        // tokens increased by exactly the mint amount
                        assert_eq!(pool.token_supply - prev_token_supply, tokens_minted);

                        // tokens per supply increased with stake per total
                        if prev_total_stake > 0 {
                            assert_relative_eq!(
                                pool.share(user) - prev_share,
                                pool.stake(user) as f64 / pool.total_stake as f64
                                    - prev_stake as f64 / prev_total_stake as f64,
                                epsilon = 1e-6
                            );
                        }
                    }

                    // burn a random amount of tokens from a random user with outstanding deposits
                    // check their stake, tokens, and share decrease by the expected amount
                    n if n > 0.35 && n <= 0.65 => {
                        if let Some(user) = users
                            .iter()
                            .filter(|user| pool.tokens(user) > 0)
                            .choose(&mut prng)
                        {
                            let prev_tokens = pool.tokens(user);
                            let prev_share = pool.share(user);
                            let prev_stake = pool.stake(user);
                            let prev_token_supply = pool.token_supply;
                            let prev_total_stake = pool.total_stake;

                            let tokens_burned = if std_range.sample(&mut prng) <= 0.1 {
                                prev_tokens
                            } else {
                                prng.gen_range(0..prev_tokens)
                            };
                            let stake_received = pool.withdraw(user, tokens_burned).unwrap();

                            // stake decreased by exactly the withdraw amount
                            assert_eq!(prev_total_stake - pool.total_stake, stake_received);

                            // calculated stake fraction is within 2 lamps of withdraw amount
                            assert!(
                                (prev_stake as i64
                                    - pool.stake(user) as i64
                                    - stake_received as i64)
                                    .abs()
                                    <= 2
                            );

                            // tokens decreased by the burn amount
                            assert_eq!(prev_token_supply - pool.token_supply, tokens_burned);

                            // tokens per supply decreased with stake per total
                            if pool.total_stake > 0 {
                                assert_relative_eq!(
                                    prev_share - pool.share(user),
                                    prev_stake as f64 / prev_total_stake as f64
                                        - pool.stake(user) as f64 / pool.total_stake as f64,
                                    epsilon = 1e-6
                                );
                            }
                        };
                    }

                    // run a single epoch worth of rewards
                    // check all user shares stay the same and stakes increase by the expected
                    // amount
                    _ => {
                        assert!(with_rewards);

                        let prev_shares_stakes = users
                            .iter()
                            .map(|user| (user, pool.share(user), pool.stake(user)))
                            .filter(|(_, _, stake)| stake > &0)
                            .collect::<Vec<_>>();

                        pool.reward((pool.total_stake as f64 * INFLATION_BASE_RATE) as u64);

                        for (user, prev_share, prev_stake) in prev_shares_stakes {
                            // shares are the same before and after
                            assert_eq!(pool.share(user), prev_share);

                            let curr_stake = pool.stake(user);
                            let stake_share = prev_stake as f64 * INFLATION_BASE_RATE;
                            let stake_diff = (curr_stake - prev_stake) as f64;

                            // stake increase is within 2 lamps when calculated as a difference or a
                            // percentage
                            assert!((stake_share - stake_diff).abs() <= 2.0);
                        }
                    }
                }
            }
        }
    }
}
