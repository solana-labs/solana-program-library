//! Program state processor

use {
    crate::{
        borsh::try_from_slice_unchecked,
        error::StakePoolError,
        find_deposit_authority_program_address,
        instruction::{PreferredValidatorType, StakePoolInstruction},
        minimum_reserve_lamports, minimum_stake_lamports, stake_program,
        state::{AccountType, Fee, StakePool, StakeStatus, ValidatorList, ValidatorStakeInfo},
        AUTHORITY_DEPOSIT, AUTHORITY_WITHDRAW, MINIMUM_ACTIVE_STAKE, TRANSIENT_STAKE_SEED,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    num_traits::FromPrimitive,
    solana_program::{
        account_info::next_account_info,
        account_info::AccountInfo,
        clock::{Clock, Epoch},
        decode_error::DecodeError,
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        program_error::PrintProgramError,
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        rent::Rent,
        stake_history::StakeHistory,
        system_instruction, system_program,
        sysvar::Sysvar,
    },
    spl_token::state::Mint,
};

/// Deserialize the stake state from AccountInfo
fn get_stake_state(
    stake_account_info: &AccountInfo,
) -> Result<(stake_program::Meta, stake_program::Stake), ProgramError> {
    let stake_state =
        try_from_slice_unchecked::<stake_program::StakeState>(&stake_account_info.data.borrow())?;
    match stake_state {
        stake_program::StakeState::Stake(meta, stake) => Ok((meta, stake)),
        _ => Err(StakePoolError::WrongStakeState.into()),
    }
}

/// Check validity of vote address for a particular stake account
fn check_validator_stake_address(
    program_id: &Pubkey,
    stake_pool_address: &Pubkey,
    stake_account_address: &Pubkey,
    vote_address: &Pubkey,
) -> Result<(), ProgramError> {
    // Check stake account address validity
    let (validator_stake_address, _) =
        crate::find_stake_program_address(&program_id, &vote_address, &stake_pool_address);
    if validator_stake_address != *stake_account_address {
        msg!(
            "Incorrect stake account address for vote {}, expected {}, received {}",
            vote_address,
            validator_stake_address,
            stake_account_address
        );
        Err(StakePoolError::InvalidStakeAccountAddress.into())
    } else {
        Ok(())
    }
}

/// Check validity of vote address for a particular stake account
fn check_transient_stake_address(
    program_id: &Pubkey,
    stake_pool_address: &Pubkey,
    stake_account_address: &Pubkey,
    vote_address: &Pubkey,
) -> Result<u8, ProgramError> {
    // Check stake account address validity
    let (transient_stake_address, bump_seed) = crate::find_transient_stake_program_address(
        &program_id,
        &vote_address,
        &stake_pool_address,
    );
    if transient_stake_address != *stake_account_address {
        Err(StakePoolError::InvalidStakeAccountAddress.into())
    } else {
        Ok(bump_seed)
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

/// Check stake program address
fn check_stake_program(program_id: &Pubkey) -> Result<(), ProgramError> {
    if *program_id != stake_program::id() {
        msg!(
            "Expected stake program {}, received {}",
            stake_program::id(),
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

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Issue a stake_deactivate instruction.
    #[allow(clippy::too_many_arguments)]
    fn stake_delegate<'a>(
        stake_info: AccountInfo<'a>,
        vote_account_info: AccountInfo<'a>,
        clock_info: AccountInfo<'a>,
        stake_history_info: AccountInfo<'a>,
        stake_config_info: AccountInfo<'a>,
        authority_info: AccountInfo<'a>,
        stake_pool: &Pubkey,
        authority_type: &[u8],
        bump_seed: u8,
    ) -> Result<(), ProgramError> {
        let authority_signature_seeds =
            [&stake_pool.to_bytes()[..32], authority_type, &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];

        let ix = stake_program::delegate_stake(
            stake_info.key,
            authority_info.key,
            vote_account_info.key,
        );

        invoke_signed(
            &ix,
            &[
                stake_info,
                vote_account_info,
                clock_info,
                stake_history_info,
                stake_config_info,
                authority_info,
            ],
            signers,
        )
    }

    /// Issue a stake_deactivate instruction.
    fn stake_deactivate<'a>(
        stake_info: AccountInfo<'a>,
        clock_info: AccountInfo<'a>,
        authority_info: AccountInfo<'a>,
        stake_pool: &Pubkey,
        authority_type: &[u8],
        bump_seed: u8,
    ) -> Result<(), ProgramError> {
        let authority_signature_seeds =
            [&stake_pool.to_bytes()[..32], authority_type, &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];

        let ix = stake_program::deactivate_stake(stake_info.key, authority_info.key);

        invoke_signed(&ix, &[stake_info, clock_info, authority_info], signers)
    }

    /// Issue a stake_split instruction.
    fn stake_split<'a>(
        stake_pool: &Pubkey,
        stake_account: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        authority_type: &[u8],
        bump_seed: u8,
        amount: u64,
        split_stake: AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let me_bytes = stake_pool.to_bytes();
        let authority_signature_seeds = [&me_bytes[..32], authority_type, &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];

        let split_instruction =
            stake_program::split_only(stake_account.key, authority.key, amount, split_stake.key);

        invoke_signed(
            &split_instruction,
            &[stake_account, split_stake, authority],
            signers,
        )
    }

    /// Issue a stake_merge instruction.
    #[allow(clippy::too_many_arguments)]
    fn stake_merge<'a>(
        stake_pool: &Pubkey,
        source_account: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        authority_type: &[u8],
        bump_seed: u8,
        destination_account: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        stake_history: AccountInfo<'a>,
        stake_program_info: AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let me_bytes = stake_pool.to_bytes();
        let authority_signature_seeds = [&me_bytes[..32], authority_type, &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];

        let merge_instruction =
            stake_program::merge(destination_account.key, source_account.key, authority.key);

        invoke_signed(
            &merge_instruction,
            &[
                destination_account,
                source_account,
                clock,
                stake_history,
                authority,
                stake_program_info,
            ],
            signers,
        )
    }

    /// Issue stake_program::authorize instructions to update both authorities
    fn stake_authorize<'a>(
        stake_account: AccountInfo<'a>,
        stake_authority: AccountInfo<'a>,
        new_stake_authority: &Pubkey,
        clock: AccountInfo<'a>,
        stake_program_info: AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let authorize_instruction = stake_program::authorize(
            stake_account.key,
            stake_authority.key,
            new_stake_authority,
            stake_program::StakeAuthorize::Staker,
        );

        invoke(
            &authorize_instruction,
            &[
                stake_account.clone(),
                clock.clone(),
                stake_authority.clone(),
                stake_program_info.clone(),
            ],
        )?;

        let authorize_instruction = stake_program::authorize(
            stake_account.key,
            stake_authority.key,
            new_stake_authority,
            stake_program::StakeAuthorize::Withdrawer,
        );

        invoke(
            &authorize_instruction,
            &[stake_account, clock, stake_authority, stake_program_info],
        )
    }

    /// Issue stake_program::authorize instructions to update both authorities
    #[allow(clippy::too_many_arguments)]
    fn stake_authorize_signed<'a>(
        stake_pool: &Pubkey,
        stake_account: AccountInfo<'a>,
        stake_authority: AccountInfo<'a>,
        authority_type: &[u8],
        bump_seed: u8,
        new_stake_authority: &Pubkey,
        clock: AccountInfo<'a>,
        stake_program_info: AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let me_bytes = stake_pool.to_bytes();
        let authority_signature_seeds = [&me_bytes[..32], authority_type, &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];

        let authorize_instruction = stake_program::authorize(
            stake_account.key,
            stake_authority.key,
            new_stake_authority,
            stake_program::StakeAuthorize::Staker,
        );

        invoke_signed(
            &authorize_instruction,
            &[
                stake_account.clone(),
                clock.clone(),
                stake_authority.clone(),
                stake_program_info.clone(),
            ],
            signers,
        )?;

        let authorize_instruction = stake_program::authorize(
            stake_account.key,
            stake_authority.key,
            new_stake_authority,
            stake_program::StakeAuthorize::Withdrawer,
        );
        invoke_signed(
            &authorize_instruction,
            &[stake_account, clock, stake_authority, stake_program_info],
            signers,
        )
    }

    /// Issue a spl_token `Burn` instruction.
    #[allow(clippy::too_many_arguments)]
    fn token_burn<'a>(
        token_program: AccountInfo<'a>,
        burn_account: AccountInfo<'a>,
        mint: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let ix = spl_token::instruction::burn(
            token_program.key,
            burn_account.key,
            mint.key,
            authority.key,
            &[],
            amount,
        )?;

        invoke(&ix, &[burn_account, mint, authority, token_program])
    }

    /// Issue a spl_token `MintTo` instruction.
    #[allow(clippy::too_many_arguments)]
    fn token_mint_to<'a>(
        stake_pool: &Pubkey,
        token_program: AccountInfo<'a>,
        mint: AccountInfo<'a>,
        destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        authority_type: &[u8],
        bump_seed: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let me_bytes = stake_pool.to_bytes();
        let authority_signature_seeds = [&me_bytes[..32], authority_type, &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];

        let ix = spl_token::instruction::mint_to(
            token_program.key,
            mint.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?;

        invoke_signed(&ix, &[mint, destination, authority, token_program], signers)
    }

    /// Processes `Initialize` instruction.
    fn process_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        fee: Fee,
        max_validators: u32,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let staker_info = next_account_info(account_info_iter)?;
        let validator_list_info = next_account_info(account_info_iter)?;
        let reserve_stake_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let manager_fee_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let token_program_info = next_account_info(account_info_iter)?;

        if !manager_info.is_signer {
            msg!("Manager did not sign initialization");
            return Err(StakePoolError::SignatureMissing.into());
        }

        let mut stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_uninitialized() {
            msg!("Provided stake pool already in use");
            return Err(StakePoolError::AlreadyInUse.into());
        }

        let mut validator_list =
            try_from_slice_unchecked::<ValidatorList>(&validator_list_info.data.borrow())?;
        if !validator_list.is_uninitialized() {
            msg!("Provided validator list already in use");
            return Err(StakePoolError::AlreadyInUse.into());
        }

        let data_length = validator_list_info.data_len();
        let expected_max_validators = ValidatorList::calculate_max_validators(data_length);
        if expected_max_validators != max_validators as usize || max_validators == 0 {
            msg!(
                "Incorrect validator list size provided, expected {}, provided {}",
                expected_max_validators,
                max_validators
            );
            return Err(StakePoolError::UnexpectedValidatorListAccountSize.into());
        }
        validator_list.account_type = AccountType::ValidatorList;
        validator_list.preferred_deposit_validator_vote_address = None;
        validator_list.preferred_withdraw_validator_vote_address = None;
        validator_list.validators.clear();
        validator_list.max_validators = max_validators;

        if !rent.is_exempt(stake_pool_info.lamports(), stake_pool_info.data_len()) {
            msg!("Stake pool not rent-exempt");
            return Err(ProgramError::AccountNotRentExempt);
        }

        if !rent.is_exempt(
            validator_list_info.lamports(),
            validator_list_info.data_len(),
        ) {
            msg!("Validator stake list not rent-exempt");
            return Err(ProgramError::AccountNotRentExempt);
        }

        // Numerator should be smaller than or equal to denominator (fee <= 1)
        if fee.numerator > fee.denominator {
            return Err(StakePoolError::FeeTooHigh.into());
        }

        if manager_fee_info.owner != token_program_info.key {
            return Err(ProgramError::IncorrectProgramId);
        }

        if pool_mint_info.owner != token_program_info.key {
            return Err(ProgramError::IncorrectProgramId);
        }

        if *pool_mint_info.key
            != spl_token::state::Account::unpack_from_slice(&manager_fee_info.data.borrow())?.mint
        {
            return Err(StakePoolError::WrongAccountMint.into());
        }

        let deposit_authority = match next_account_info(account_info_iter) {
            Ok(deposit_authority_info) => *deposit_authority_info.key,
            Err(_) => find_deposit_authority_program_address(program_id, stake_pool_info.key).0,
        };
        let (withdraw_authority_key, withdraw_bump_seed) =
            crate::find_withdraw_authority_program_address(program_id, stake_pool_info.key);

        let pool_mint = Mint::unpack_from_slice(&pool_mint_info.data.borrow())?;

        if pool_mint.supply != 0 {
            return Err(StakePoolError::NonZeroPoolTokenSupply.into());
        }

        if !pool_mint.mint_authority.contains(&withdraw_authority_key) {
            return Err(StakePoolError::WrongMintingAuthority.into());
        }

        if *reserve_stake_info.owner != stake_program::id() {
            msg!("Reserve stake account not owned by stake program");
            return Err(ProgramError::IncorrectProgramId);
        }
        let stake_state = try_from_slice_unchecked::<stake_program::StakeState>(
            &reserve_stake_info.data.borrow(),
        )?;
        let total_stake_lamports = if let stake_program::StakeState::Initialized(meta) = stake_state
        {
            if meta.lockup != stake_program::Lockup::default() {
                msg!("Reserve stake account has some lockup");
                return Err(StakePoolError::WrongStakeState.into());
            }

            if meta.authorized.staker != withdraw_authority_key {
                msg!(
                    "Reserve stake account has incorrect staker {}, should be {}",
                    meta.authorized.staker,
                    withdraw_authority_key
                );
                return Err(StakePoolError::WrongStakeState.into());
            }

            if meta.authorized.withdrawer != withdraw_authority_key {
                msg!(
                    "Reserve stake account has incorrect withdrawer {}, should be {}",
                    meta.authorized.staker,
                    withdraw_authority_key
                );
                return Err(StakePoolError::WrongStakeState.into());
            }
            reserve_stake_info
                .lamports()
                .checked_sub(minimum_reserve_lamports(&meta))
                .ok_or(StakePoolError::CalculationFailure)?
        } else {
            msg!("Reserve stake account not in intialized state");
            return Err(StakePoolError::WrongStakeState.into());
        };

        validator_list.serialize(&mut *validator_list_info.data.borrow_mut())?;

        stake_pool.account_type = AccountType::StakePool;
        stake_pool.manager = *manager_info.key;
        stake_pool.staker = *staker_info.key;
        stake_pool.reserve_stake = *reserve_stake_info.key;
        stake_pool.deposit_authority = deposit_authority;
        stake_pool.withdraw_bump_seed = withdraw_bump_seed;
        stake_pool.validator_list = *validator_list_info.key;
        stake_pool.pool_mint = *pool_mint_info.key;
        stake_pool.manager_fee_account = *manager_fee_info.key;
        stake_pool.token_program_id = *token_program_info.key;
        stake_pool.last_update_epoch = clock.epoch;
        stake_pool.fee = fee;
        stake_pool.total_stake_lamports = total_stake_lamports;

        stake_pool
            .serialize(&mut *stake_pool_info.data.borrow_mut())
            .map_err(|e| e.into())
    }

    /// Processes `CreateValidatorStakeAccount` instruction.
    fn process_create_validator_stake_account(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let staker_info = next_account_info(account_info_iter)?;
        let funder_info = next_account_info(account_info_iter)?;
        let stake_account_info = next_account_info(account_info_iter)?;
        let validator_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let clock_info = next_account_info(account_info_iter)?;
        let stake_history_info = next_account_info(account_info_iter)?;
        let stake_config_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let stake_program_info = next_account_info(account_info_iter)?;

        if stake_pool_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        let stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }
        stake_pool.check_staker(staker_info)?;

        check_system_program(system_program_info.key)?;
        check_stake_program(stake_program_info.key)?;

        let (stake_address, bump_seed) = crate::find_stake_program_address(
            &program_id,
            &validator_info.key,
            &stake_pool_info.key,
        );
        if stake_address != *stake_account_info.key {
            return Err(StakePoolError::InvalidStakeAccountAddress.into());
        }

        let stake_account_signer_seeds: &[&[_]] = &[
            &validator_info.key.to_bytes()[..32],
            &stake_pool_info.key.to_bytes()[..32],
            &[bump_seed],
        ];

        // Fund the stake account with 1 SOL + rent-exempt balance
        let required_lamports = MINIMUM_ACTIVE_STAKE
            + rent.minimum_balance(std::mem::size_of::<stake_program::StakeState>());

        // Create new stake account
        invoke_signed(
            &system_instruction::create_account(
                &funder_info.key,
                &stake_account_info.key,
                required_lamports,
                std::mem::size_of::<stake_program::StakeState>() as u64,
                &stake_program::id(),
            ),
            &[funder_info.clone(), stake_account_info.clone()],
            &[&stake_account_signer_seeds],
        )?;

        invoke(
            &stake_program::initialize(
                &stake_account_info.key,
                &stake_program::Authorized {
                    staker: *staker_info.key,
                    withdrawer: *staker_info.key,
                },
                &stake_program::Lockup::default(),
            ),
            &[
                stake_account_info.clone(),
                rent_info.clone(),
                stake_program_info.clone(),
            ],
        )?;

        invoke(
            &stake_program::delegate_stake(
                &stake_account_info.key,
                &staker_info.key,
                &validator_info.key,
            ),
            &[
                stake_account_info.clone(),
                validator_info.clone(),
                clock_info.clone(),
                stake_history_info.clone(),
                stake_config_info.clone(),
                staker_info.clone(),
            ],
        )
    }

    /// Processes `AddValidatorToPool` instruction.
    fn process_add_validator_to_pool(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let staker_info = next_account_info(account_info_iter)?;
        let withdraw_authority_info = next_account_info(account_info_iter)?;
        let validator_list_info = next_account_info(account_info_iter)?;
        let stake_account_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        let _stake_history_info = next_account_info(account_info_iter)?;
        //let stake_history = &StakeHistory::from_account_info(stake_history_info)?;
        let stake_program_info = next_account_info(account_info_iter)?;

        check_stake_program(stake_program_info.key)?;

        if stake_pool_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        let stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        stake_pool.check_authority_withdraw(
            withdraw_authority_info.key,
            program_id,
            stake_pool_info.key,
        )?;

        stake_pool.check_staker(staker_info)?;

        if stake_pool.last_update_epoch < clock.epoch {
            return Err(StakePoolError::StakeListAndPoolOutOfDate.into());
        }

        if *validator_list_info.key != stake_pool.validator_list {
            return Err(StakePoolError::InvalidValidatorStakeList.into());
        }

        let mut validator_list =
            try_from_slice_unchecked::<ValidatorList>(&validator_list_info.data.borrow())?;
        if !validator_list.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }
        if validator_list.max_validators as usize == validator_list.validators.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }

        let (meta, stake) = get_stake_state(stake_account_info)?;
        let vote_account_address = stake.delegation.voter_pubkey;
        check_validator_stake_address(
            program_id,
            stake_pool_info.key,
            stake_account_info.key,
            &vote_account_address,
        )?;

        if meta.lockup != stake_program::Lockup::default() {
            msg!("Validator stake account has a lockup");
            return Err(StakePoolError::WrongStakeState.into());
        }

        if validator_list.contains(&vote_account_address) {
            return Err(StakePoolError::ValidatorAlreadyAdded.into());
        }

        // Check amount of lamports
        let stake_lamports = **stake_account_info.lamports.borrow();
        let minimum_lamport_amount = minimum_stake_lamports(&meta);
        if stake_lamports != minimum_lamport_amount {
            msg!(
                "Error: attempting to add stake with {} lamports, must have {} lamports",
                stake_lamports,
                minimum_lamport_amount
            );
            return Err(StakePoolError::StakeLamportsNotEqualToMinimum.into());
        }

        // Check if stake is warmed up
        //Self::check_stake_activation(stake_account_info, clock, stake_history)?;

        // Update Withdrawer and Staker authority to the program withdraw authority
        Self::stake_authorize(
            stake_account_info.clone(),
            staker_info.clone(),
            withdraw_authority_info.key,
            clock_info.clone(),
            stake_program_info.clone(),
        )?;

        validator_list.validators.push(ValidatorStakeInfo {
            status: StakeStatus::Active,
            vote_account_address,
            stake_lamports: stake_lamports.saturating_sub(minimum_lamport_amount),
            last_update_epoch: clock.epoch,
        });
        validator_list.serialize(&mut *validator_list_info.data.borrow_mut())?;

        Ok(())
    }

    /// Processes `RemoveValidatorFromPool` instruction.
    fn process_remove_validator_from_pool(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let staker_info = next_account_info(account_info_iter)?;
        let withdraw_authority_info = next_account_info(account_info_iter)?;
        let new_stake_authority_info = next_account_info(account_info_iter)?;
        let validator_list_info = next_account_info(account_info_iter)?;
        let stake_account_info = next_account_info(account_info_iter)?;
        let transient_stake_account_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        let stake_program_info = next_account_info(account_info_iter)?;

        check_stake_program(stake_program_info.key)?;
        check_account_owner(stake_pool_info, program_id)?;

        let stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        stake_pool.check_authority_withdraw(
            withdraw_authority_info.key,
            program_id,
            stake_pool_info.key,
        )?;
        stake_pool.check_staker(staker_info)?;

        if stake_pool.last_update_epoch < clock.epoch {
            return Err(StakePoolError::StakeListAndPoolOutOfDate.into());
        }

        stake_pool.check_validator_list(validator_list_info)?;

        let mut validator_list =
            try_from_slice_unchecked::<ValidatorList>(&validator_list_info.data.borrow())?;
        if !validator_list.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        let (meta, stake) = get_stake_state(stake_account_info)?;
        let vote_account_address = stake.delegation.voter_pubkey;
        check_validator_stake_address(
            program_id,
            stake_pool_info.key,
            stake_account_info.key,
            &vote_account_address,
        )?;
        check_transient_stake_address(
            program_id,
            stake_pool_info.key,
            transient_stake_account_info.key,
            &vote_account_address,
        )?;

        let maybe_validator_list_entry = validator_list.find_mut(&vote_account_address);
        if maybe_validator_list_entry.is_none() {
            msg!(
                "Vote account {} not found in stake pool",
                vote_account_address
            );
            return Err(StakePoolError::ValidatorNotFound.into());
        }
        let mut validator_list_entry = maybe_validator_list_entry.unwrap();

        let stake_lamports = **stake_account_info.lamports.borrow();
        let required_lamports = minimum_stake_lamports(&meta);
        if stake_lamports != required_lamports {
            msg!(
                "Attempting to remove validator account with {} lamports, must have {} lamports",
                stake_lamports,
                required_lamports
            );
            return Err(StakePoolError::StakeLamportsNotEqualToMinimum.into());
        }

        // check that the transient stake account doesn't exist
        let new_status = if let Ok((_meta, stake)) = get_stake_state(transient_stake_account_info) {
            if stake.delegation.deactivation_epoch == Epoch::MAX {
                msg!(
                    "Transient stake {} activating, can't remove stake {} on validator {}",
                    transient_stake_account_info.key,
                    stake_account_info.key,
                    vote_account_address
                );
                return Err(StakePoolError::WrongStakeState.into());
            } else {
                // stake is deactivating, mark the entry as such
                StakeStatus::DeactivatingTransient
            }
        } else {
            StakeStatus::ReadyForRemoval
        };

        Self::stake_authorize_signed(
            stake_pool_info.key,
            stake_account_info.clone(),
            withdraw_authority_info.clone(),
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            new_stake_authority_info.key,
            clock_info.clone(),
            stake_program_info.clone(),
        )?;

        match new_status {
            StakeStatus::DeactivatingTransient => validator_list_entry.status = new_status,
            StakeStatus::ReadyForRemoval => validator_list
                .validators
                .retain(|item| item.vote_account_address != vote_account_address),
            _ => unreachable!(),
        }

        if validator_list.preferred_deposit_validator_vote_address == Some(vote_account_address) {
            validator_list.preferred_deposit_validator_vote_address = None;
        }
        if validator_list.preferred_withdraw_validator_vote_address == Some(vote_account_address) {
            validator_list.preferred_withdraw_validator_vote_address = None;
        }
        validator_list.serialize(&mut *validator_list_info.data.borrow_mut())?;

        Ok(())
    }

    /// Processes `DecreaseValidatorStake` instruction.
    fn process_decrease_validator_stake(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        lamports: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let staker_info = next_account_info(account_info_iter)?;
        let withdraw_authority_info = next_account_info(account_info_iter)?;
        let validator_list_info = next_account_info(account_info_iter)?;
        let validator_stake_account_info = next_account_info(account_info_iter)?;
        let transient_stake_account_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let stake_program_info = next_account_info(account_info_iter)?;

        check_system_program(system_program_info.key)?;
        check_stake_program(stake_program_info.key)?;
        check_account_owner(stake_pool_info, program_id)?;

        let stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_valid() {
            msg!("Expected valid stake pool");
            return Err(StakePoolError::InvalidState.into());
        }

        stake_pool.check_authority_withdraw(
            withdraw_authority_info.key,
            program_id,
            stake_pool_info.key,
        )?;
        stake_pool.check_staker(staker_info)?;

        if stake_pool.last_update_epoch < clock.epoch {
            return Err(StakePoolError::StakeListAndPoolOutOfDate.into());
        }

        stake_pool.check_validator_list(validator_list_info)?;

        let validator_list =
            try_from_slice_unchecked::<ValidatorList>(&validator_list_info.data.borrow())?;
        if !validator_list.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        let (_meta, stake) = get_stake_state(validator_stake_account_info)?;
        let vote_account_address = stake.delegation.voter_pubkey;
        check_validator_stake_address(
            program_id,
            stake_pool_info.key,
            validator_stake_account_info.key,
            &vote_account_address,
        )?;

        let transient_stake_bump_seed = check_transient_stake_address(
            program_id,
            stake_pool_info.key,
            transient_stake_account_info.key,
            &vote_account_address,
        )?;
        let transient_stake_account_signer_seeds: &[&[_]] = &[
            TRANSIENT_STAKE_SEED,
            &vote_account_address.to_bytes()[..32],
            &stake_pool_info.key.to_bytes()[..32],
            &[transient_stake_bump_seed],
        ];

        if !validator_list.contains(&vote_account_address) {
            msg!(
                "Vote account {} not found in stake pool",
                vote_account_address
            );
            return Err(StakePoolError::ValidatorNotFound.into());
        }

        let stake_rent = rent.minimum_balance(std::mem::size_of::<stake_program::StakeState>());
        if lamports <= stake_rent {
            msg!(
                "Need more than {} lamports for transient stake to be rent-exempt, {} provided",
                stake_rent,
                lamports
            );
            return Err(ProgramError::AccountNotRentExempt);
        }

        // create transient stake account
        invoke_signed(
            &system_instruction::create_account(
                &transient_stake_account_info.key, // doesn't matter since no lamports are transferred
                &transient_stake_account_info.key,
                0,
                std::mem::size_of::<stake_program::StakeState>() as u64,
                &stake_program::id(),
            ),
            &[transient_stake_account_info.clone()],
            &[&transient_stake_account_signer_seeds],
        )?;

        // split into transient stake account
        Self::stake_split(
            stake_pool_info.key,
            validator_stake_account_info.clone(),
            withdraw_authority_info.clone(),
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            lamports,
            transient_stake_account_info.clone(),
        )?;

        // deactivate transient stake
        Self::stake_deactivate(
            transient_stake_account_info.clone(),
            clock_info.clone(),
            withdraw_authority_info.clone(),
            stake_pool_info.key,
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
        )?;

        Ok(())
    }

    /// Processes `IncreaseValidatorStake` instruction.
    fn process_increase_validator_stake(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        lamports: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let staker_info = next_account_info(account_info_iter)?;
        let withdraw_authority_info = next_account_info(account_info_iter)?;
        let validator_list_info = next_account_info(account_info_iter)?;
        let reserve_stake_account_info = next_account_info(account_info_iter)?;
        let transient_stake_account_info = next_account_info(account_info_iter)?;
        let validator_vote_account_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let stake_history_info = next_account_info(account_info_iter)?;
        let stake_config_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let stake_program_info = next_account_info(account_info_iter)?;

        check_system_program(system_program_info.key)?;
        check_stake_program(stake_program_info.key)?;
        check_account_owner(stake_pool_info, program_id)?;

        let stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_valid() {
            msg!("Expected valid stake pool");
            return Err(StakePoolError::InvalidState.into());
        }

        stake_pool.check_authority_withdraw(
            withdraw_authority_info.key,
            program_id,
            stake_pool_info.key,
        )?;
        stake_pool.check_staker(staker_info)?;

        if stake_pool.last_update_epoch < clock.epoch {
            return Err(StakePoolError::StakeListAndPoolOutOfDate.into());
        }

        stake_pool.check_validator_list(validator_list_info)?;

        let mut validator_list =
            try_from_slice_unchecked::<ValidatorList>(&validator_list_info.data.borrow())?;
        if !validator_list.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        stake_pool.check_reserve_stake(reserve_stake_account_info)?;

        let vote_account_address = validator_vote_account_info.key;

        let transient_stake_bump_seed = check_transient_stake_address(
            program_id,
            stake_pool_info.key,
            transient_stake_account_info.key,
            &vote_account_address,
        )?;
        let transient_stake_account_signer_seeds: &[&[_]] = &[
            TRANSIENT_STAKE_SEED,
            &vote_account_address.to_bytes()[..32],
            &stake_pool_info.key.to_bytes()[..32],
            &[transient_stake_bump_seed],
        ];

        let maybe_validator_list_entry = validator_list.find_mut(&vote_account_address);
        if maybe_validator_list_entry.is_none() {
            msg!(
                "Vote account {} not found in stake pool",
                vote_account_address
            );
            return Err(StakePoolError::ValidatorNotFound.into());
        }
        let mut validator_list_entry = maybe_validator_list_entry.unwrap();

        if validator_list_entry.status != StakeStatus::Active {
            msg!("Validator is marked for removal and no longer allows increases");
            return Err(StakePoolError::ValidatorNotFound.into());
        }

        let stake_rent = rent.minimum_balance(std::mem::size_of::<stake_program::StakeState>());
        let minimum_lamports = MINIMUM_ACTIVE_STAKE + stake_rent;
        if lamports < minimum_lamports {
            msg!(
                "Need more than {} lamports for transient stake to be rent-exempt and mergeable, {} provided",
                minimum_lamports,
                lamports
            );
            return Err(ProgramError::AccountNotRentExempt);
        }

        if reserve_stake_account_info
            .lamports()
            .saturating_sub(lamports)
            <= stake_rent
        {
            let max_split_amount = reserve_stake_account_info
                .lamports()
                .saturating_sub(stake_rent);
            msg!(
                "Reserve stake does not have enough lamports for increase, must be less than {}, {} requested",
                max_split_amount,
                lamports
            );
            return Err(ProgramError::InsufficientFunds);
        }

        // create transient stake account
        invoke_signed(
            &system_instruction::create_account(
                &transient_stake_account_info.key, // doesn't matter since no lamports are transferred
                &transient_stake_account_info.key,
                0,
                std::mem::size_of::<stake_program::StakeState>() as u64,
                &stake_program::id(),
            ),
            &[transient_stake_account_info.clone()],
            &[&transient_stake_account_signer_seeds],
        )?;

        // split into transient stake account
        Self::stake_split(
            stake_pool_info.key,
            reserve_stake_account_info.clone(),
            withdraw_authority_info.clone(),
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            lamports,
            transient_stake_account_info.clone(),
        )?;

        // activate transient stake to validator
        Self::stake_delegate(
            transient_stake_account_info.clone(),
            validator_vote_account_info.clone(),
            clock_info.clone(),
            stake_history_info.clone(),
            stake_config_info.clone(),
            withdraw_authority_info.clone(),
            stake_pool_info.key,
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
        )?;

        validator_list_entry.stake_lamports = validator_list_entry
            .stake_lamports
            .checked_add(lamports)
            .ok_or(StakePoolError::CalculationFailure)?;
        validator_list.serialize(&mut *validator_list_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process `SetPreferredValidator` instruction
    fn process_set_preferred_validator(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        validator_type: PreferredValidatorType,
        vote_account_address: Option<Pubkey>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let staker_info = next_account_info(account_info_iter)?;
        let validator_list_info = next_account_info(account_info_iter)?;

        check_account_owner(stake_pool_info, program_id)?;
        check_account_owner(validator_list_info, program_id)?;

        let stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_valid() {
            msg!("Expected valid stake pool");
            return Err(StakePoolError::InvalidState.into());
        }

        stake_pool.check_staker(staker_info)?;
        stake_pool.check_validator_list(validator_list_info)?;

        let mut validator_list =
            try_from_slice_unchecked::<ValidatorList>(&validator_list_info.data.borrow())?;
        if !validator_list.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        if let Some(vote_account_address) = vote_account_address {
            if !validator_list.contains(&vote_account_address) {
                msg!("Validator for {} not present in the stake pool, cannot set as preferred deposit account");
                return Err(StakePoolError::ValidatorNotFound.into());
            }
        }

        match validator_type {
            PreferredValidatorType::Deposit => {
                validator_list.preferred_deposit_validator_vote_address = vote_account_address
            }
            PreferredValidatorType::Withdraw => {
                validator_list.preferred_withdraw_validator_vote_address = vote_account_address
            }
        };
        validator_list.serialize(&mut *validator_list_info.data.borrow_mut())?;
        Ok(())
    }

    /// Processes `UpdateValidatorListBalance` instruction.
    fn process_update_validator_list_balance(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        start_index: u32,
        no_merge: bool,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let withdraw_authority_info = next_account_info(account_info_iter)?;
        let validator_list_info = next_account_info(account_info_iter)?;
        let reserve_stake_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        let stake_history_info = next_account_info(account_info_iter)?;
        let stake_program_info = next_account_info(account_info_iter)?;
        let validator_stake_accounts = account_info_iter.as_slice();

        check_account_owner(stake_pool_info, program_id)?;
        let stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }
        stake_pool.check_validator_list(&validator_list_info)?;
        stake_pool.check_authority_withdraw(
            withdraw_authority_info.key,
            program_id,
            stake_pool_info.key,
        )?;
        stake_pool.check_reserve_stake(reserve_stake_info)?;
        check_stake_program(stake_program_info.key)?;

        let mut validator_list =
            try_from_slice_unchecked::<ValidatorList>(&validator_list_info.data.borrow())?;
        if !validator_list.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        let mut changes = false;
        let validator_iter = &mut validator_list
            .validators
            .iter_mut()
            .skip(start_index as usize)
            .zip(validator_stake_accounts.chunks_exact(2));
        for (validator_stake_record, validator_stakes) in validator_iter {
            // chunks_exact means that we always get 2 elements, making this safe
            let validator_stake_info = validator_stakes.first().unwrap();
            let transient_stake_info = validator_stakes.last().unwrap();
            if check_validator_stake_address(
                program_id,
                stake_pool_info.key,
                validator_stake_info.key,
                &validator_stake_record.vote_account_address,
            )
            .is_err()
            {
                continue;
            };
            if check_transient_stake_address(
                program_id,
                stake_pool_info.key,
                transient_stake_info.key,
                &validator_stake_record.vote_account_address,
            )
            .is_err()
            {
                continue;
            };

            let mut stake_lamports = 0;
            let validator_stake_state = try_from_slice_unchecked::<stake_program::StakeState>(
                &validator_stake_info.data.borrow(),
            )
            .ok();
            let transient_stake_state = try_from_slice_unchecked::<stake_program::StakeState>(
                &transient_stake_info.data.borrow(),
            )
            .ok();

            // Possible merge situations for transient stake
            //  * active -> merge into validator stake
            //  * activating -> nothing, just account its lamports
            //  * deactivating -> nothing, just account its lamports
            //  * inactive -> merge into reserve stake
            //  * not a stake -> ignore
            match transient_stake_state {
                Some(stake_program::StakeState::Initialized(_meta)) => {
                    if no_merge {
                        stake_lamports += transient_stake_info.lamports();
                    } else {
                        // merge into reserve
                        Self::stake_merge(
                            stake_pool_info.key,
                            transient_stake_info.clone(),
                            withdraw_authority_info.clone(),
                            AUTHORITY_WITHDRAW,
                            stake_pool.withdraw_bump_seed,
                            reserve_stake_info.clone(),
                            clock_info.clone(),
                            stake_history_info.clone(),
                            stake_program_info.clone(),
                        )?;
                        if validator_stake_record.status == StakeStatus::DeactivatingTransient {
                            // the validator stake was previously removed, and
                            // now this entry can be removed totally
                            validator_stake_record.status = StakeStatus::ReadyForRemoval;
                        }
                    }
                }
                Some(stake_program::StakeState::Stake(_, stake)) => {
                    if no_merge {
                        stake_lamports += transient_stake_info.lamports();
                    } else if stake.delegation.deactivation_epoch < clock.epoch {
                        // deactivated, merge into reserve
                        Self::stake_merge(
                            stake_pool_info.key,
                            transient_stake_info.clone(),
                            withdraw_authority_info.clone(),
                            AUTHORITY_WITHDRAW,
                            stake_pool.withdraw_bump_seed,
                            reserve_stake_info.clone(),
                            clock_info.clone(),
                            stake_history_info.clone(),
                            stake_program_info.clone(),
                        )?;
                        if validator_stake_record.status == StakeStatus::DeactivatingTransient {
                            // the validator stake was previously removed, and
                            // now this entry can be removed totally
                            validator_stake_record.status = StakeStatus::ReadyForRemoval;
                        }
                    } else if stake.delegation.activation_epoch < clock.epoch {
                        if let Some(stake_program::StakeState::Stake(_, validator_stake)) =
                            validator_stake_state
                        {
                            if stake_program::active_stakes_can_merge(&stake, &validator_stake)
                                .is_ok()
                            {
                                Self::stake_merge(
                                    stake_pool_info.key,
                                    transient_stake_info.clone(),
                                    withdraw_authority_info.clone(),
                                    AUTHORITY_WITHDRAW,
                                    stake_pool.withdraw_bump_seed,
                                    validator_stake_info.clone(),
                                    clock_info.clone(),
                                    stake_history_info.clone(),
                                    stake_program_info.clone(),
                                )?;
                            } else {
                                msg!("Stake activating or just active, not ready to merge");
                                stake_lamports += transient_stake_info.lamports();
                            }
                        } else {
                            msg!("Transient stake is activating or active, but validator stake is not, need to add the validator stake account on {} back into the stake pool", stake.delegation.voter_pubkey);
                            stake_lamports += transient_stake_info.lamports();
                        }
                    } else {
                        msg!("Transient stake not ready to be merged anywhere");
                        stake_lamports += transient_stake_info.lamports();
                    }
                }
                None
                | Some(stake_program::StakeState::Uninitialized)
                | Some(stake_program::StakeState::RewardsPool) => {} // do nothing
            }

            // Status for validator stake
            //  * active -> do everything
            //  * any other state / not a stake -> error state, but account for transient stake
            match validator_stake_state {
                Some(stake_program::StakeState::Stake(meta, _)) => {
                    if validator_stake_record.status == StakeStatus::Active {
                        stake_lamports += validator_stake_info
                            .lamports()
                            .saturating_sub(minimum_stake_lamports(&meta));
                    } else {
                        msg!("Validator stake account no longer part of the pool, ignoring");
                    }
                }
                Some(stake_program::StakeState::Initialized(_))
                | Some(stake_program::StakeState::Uninitialized)
                | Some(stake_program::StakeState::RewardsPool)
                | None => {
                    msg!("Validator stake account no longer part of the pool, ignoring");
                }
            }

            validator_stake_record.last_update_epoch = clock.epoch;
            validator_stake_record.stake_lamports = stake_lamports;
            changes = true;
        }

        if changes {
            validator_list.serialize(&mut *validator_list_info.data.borrow_mut())?;
        }

        Ok(())
    }

    /// Processes `UpdateStakePoolBalance` instruction.
    fn process_update_stake_pool_balance(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let withdraw_info = next_account_info(account_info_iter)?;
        let validator_list_info = next_account_info(account_info_iter)?;
        let reserve_stake_info = next_account_info(account_info_iter)?;
        let manager_fee_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let mut stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }
        stake_pool.check_mint(pool_mint_info)?;
        stake_pool.check_authority_withdraw(withdraw_info.key, program_id, stake_pool_info.key)?;
        stake_pool.check_reserve_stake(reserve_stake_info)?;
        if stake_pool.manager_fee_account != *manager_fee_info.key {
            return Err(StakePoolError::InvalidFeeAccount.into());
        }

        if *validator_list_info.key != stake_pool.validator_list {
            return Err(StakePoolError::InvalidValidatorStakeList.into());
        }
        if stake_pool.token_program_id != *token_program_info.key {
            return Err(ProgramError::IncorrectProgramId);
        }

        let mut validator_list =
            try_from_slice_unchecked::<ValidatorList>(&validator_list_info.data.borrow())?;
        if !validator_list.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        let previous_lamports = stake_pool.total_stake_lamports;
        let reserve_stake = try_from_slice_unchecked::<stake_program::StakeState>(
            &reserve_stake_info.data.borrow(),
        )?;
        let mut total_stake_lamports =
            if let stake_program::StakeState::Initialized(meta) = reserve_stake {
                reserve_stake_info
                    .lamports()
                    .checked_sub(minimum_reserve_lamports(&meta))
                    .ok_or(StakePoolError::CalculationFailure)?
            } else {
                msg!("Reserve stake account in unknown state, aborting");
                return Err(StakePoolError::WrongStakeState.into());
            };
        for validator_stake_record in &validator_list.validators {
            if validator_stake_record.last_update_epoch < clock.epoch {
                return Err(StakePoolError::StakeListOutOfDate.into());
            }
            total_stake_lamports = total_stake_lamports
                .checked_add(validator_stake_record.stake_lamports)
                .ok_or(StakePoolError::CalculationFailure)?;
        }

        let reward_lamports = total_stake_lamports.saturating_sub(previous_lamports);
        let fee = stake_pool
            .calc_fee_amount(reward_lamports)
            .ok_or(StakePoolError::CalculationFailure)?;

        if fee > 0 {
            Self::token_mint_to(
                stake_pool_info.key,
                token_program_info.clone(),
                pool_mint_info.clone(),
                manager_fee_info.clone(),
                withdraw_info.clone(),
                AUTHORITY_WITHDRAW,
                stake_pool.withdraw_bump_seed,
                fee,
            )?;

            stake_pool.pool_token_supply = stake_pool
                .pool_token_supply
                .checked_add(fee)
                .ok_or(StakePoolError::CalculationFailure)?;
        }
        validator_list
            .validators
            .retain(|item| item.status != StakeStatus::ReadyForRemoval);
        validator_list.serialize(&mut *validator_list_info.data.borrow_mut())?;
        stake_pool.total_stake_lamports = total_stake_lamports;
        stake_pool.last_update_epoch = clock.epoch;
        stake_pool.serialize(&mut *stake_pool_info.data.borrow_mut())?;

        Ok(())
    }

    /// Check stake activation status
    #[allow(clippy::unnecessary_wraps)]
    fn _check_stake_activation(
        stake_info: &AccountInfo,
        clock: &Clock,
        stake_history: &StakeHistory,
    ) -> ProgramResult {
        let stake_acc_state =
            try_from_slice_unchecked::<stake_program::StakeState>(&stake_info.data.borrow())
                .unwrap();
        let delegation = stake_acc_state.delegation();
        if let Some(delegation) = delegation {
            let target_epoch = clock.epoch;
            let history = Some(stake_history);
            let fix_stake_deactivate = true;
            let (effective, activating, deactivating) = delegation
                .stake_activating_and_deactivating(target_epoch, history, fix_stake_deactivate);
            if activating != 0 || deactivating != 0 || effective == 0 {
                return Err(StakePoolError::UserStakeNotActive.into());
            }
        } else {
            return Err(StakePoolError::WrongStakeState.into());
        }
        Ok(())
    }

    /// Processes [Deposit](enum.Instruction.html).
    fn process_deposit(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let validator_list_info = next_account_info(account_info_iter)?;
        let deposit_authority_info = next_account_info(account_info_iter)?;
        let withdraw_authority_info = next_account_info(account_info_iter)?;
        let stake_info = next_account_info(account_info_iter)?;
        let validator_stake_account_info = next_account_info(account_info_iter)?;
        let dest_user_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        let stake_history_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let stake_program_info = next_account_info(account_info_iter)?;

        if *stake_program_info.key != stake_program::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

        let mut stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        //Self::check_stake_activation(stake_info, clock, stake_history)?;

        stake_pool.check_authority_withdraw(
            withdraw_authority_info.key,
            program_id,
            stake_pool_info.key,
        )?;
        stake_pool.check_deposit_authority(deposit_authority_info.key)?;
        stake_pool.check_mint(pool_mint_info)?;

        if stake_pool.token_program_id != *token_program_info.key {
            return Err(ProgramError::IncorrectProgramId);
        }

        if *validator_list_info.key != stake_pool.validator_list {
            return Err(StakePoolError::InvalidValidatorStakeList.into());
        }

        if stake_pool.last_update_epoch < clock.epoch {
            return Err(StakePoolError::StakeListAndPoolOutOfDate.into());
        }

        let mut validator_list =
            try_from_slice_unchecked::<ValidatorList>(&validator_list_info.data.borrow())?;
        if !validator_list.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        let (meta, stake) = get_stake_state(validator_stake_account_info)?;
        let vote_account_address = stake.delegation.voter_pubkey;
        check_validator_stake_address(
            program_id,
            stake_pool_info.key,
            validator_stake_account_info.key,
            &vote_account_address,
        )?;
        if let Some(preferred_deposit) = validator_list.preferred_deposit_validator_vote_address {
            if preferred_deposit != vote_account_address {
                return Err(StakePoolError::IncorrectDepositVoteAddress.into());
            }
        }

        let validator_list_item = validator_list
            .find_mut(&vote_account_address)
            .ok_or(StakePoolError::ValidatorNotFound)?;

        if validator_list_item.status != StakeStatus::Active {
            msg!("Validator is marked for removal and no longer accepting deposits");
            return Err(StakePoolError::ValidatorNotFound.into());
        }

        let stake_lamports = **stake_info.lamports.borrow();
        let new_pool_tokens = stake_pool
            .calc_pool_tokens_for_deposit(stake_lamports)
            .ok_or(StakePoolError::CalculationFailure)?;

        msg!(
            "lamports pre merge {}",
            validator_stake_account_info.lamports()
        );

        let (deposit_authority_program_address, deposit_bump_seed) =
            find_deposit_authority_program_address(program_id, stake_pool_info.key);
        if *deposit_authority_info.key == deposit_authority_program_address {
            Self::stake_authorize_signed(
                stake_pool_info.key,
                stake_info.clone(),
                deposit_authority_info.clone(),
                AUTHORITY_DEPOSIT,
                deposit_bump_seed,
                withdraw_authority_info.key,
                clock_info.clone(),
                stake_program_info.clone(),
            )?;
        } else {
            Self::stake_authorize(
                stake_info.clone(),
                deposit_authority_info.clone(),
                withdraw_authority_info.key,
                clock_info.clone(),
                stake_program_info.clone(),
            )?;
        }

        Self::stake_merge(
            stake_pool_info.key,
            stake_info.clone(),
            withdraw_authority_info.clone(),
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            validator_stake_account_info.clone(),
            clock_info.clone(),
            stake_history_info.clone(),
            stake_program_info.clone(),
        )?;

        Self::token_mint_to(
            stake_pool_info.key,
            token_program_info.clone(),
            pool_mint_info.clone(),
            dest_user_info.clone(),
            withdraw_authority_info.clone(),
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            new_pool_tokens,
        )?;

        stake_pool.pool_token_supply = stake_pool
            .pool_token_supply
            .checked_add(new_pool_tokens)
            .ok_or(StakePoolError::CalculationFailure)?;
        stake_pool.total_stake_lamports = stake_pool
            .total_stake_lamports
            .checked_add(stake_lamports)
            .ok_or(StakePoolError::CalculationFailure)?;
        stake_pool.serialize(&mut *stake_pool_info.data.borrow_mut())?;

        msg!(
            "lamports post merge {}",
            validator_stake_account_info.lamports()
        );
        validator_list_item.stake_lamports = validator_stake_account_info
            .lamports()
            .checked_sub(minimum_stake_lamports(&meta))
            .ok_or(StakePoolError::CalculationFailure)?;
        validator_list.serialize(&mut *validator_list_info.data.borrow_mut())?;

        Ok(())
    }

    /// Processes [Withdraw](enum.Instruction.html).
    fn process_withdraw(
        program_id: &Pubkey,
        pool_tokens: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let validator_list_info = next_account_info(account_info_iter)?;
        let withdraw_authority_info = next_account_info(account_info_iter)?;
        let stake_split_from = next_account_info(account_info_iter)?;
        let stake_split_to = next_account_info(account_info_iter)?;
        let user_stake_authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let burn_from_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let stake_program_info = next_account_info(account_info_iter)?;

        if *stake_program_info.key != stake_program::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

        let mut stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        stake_pool.check_mint(pool_mint_info)?;
        stake_pool.check_authority_withdraw(
            withdraw_authority_info.key,
            program_id,
            stake_pool_info.key,
        )?;

        if stake_pool.token_program_id != *token_program_info.key {
            return Err(ProgramError::IncorrectProgramId);
        }

        if *validator_list_info.key != stake_pool.validator_list {
            return Err(StakePoolError::InvalidValidatorStakeList.into());
        }

        if stake_pool.last_update_epoch < clock.epoch {
            return Err(StakePoolError::StakeListAndPoolOutOfDate.into());
        }

        let mut validator_list =
            try_from_slice_unchecked::<ValidatorList>(&validator_list_info.data.borrow())?;
        if !validator_list.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        let withdraw_lamports = stake_pool
            .calc_lamports_withdraw_amount(pool_tokens)
            .ok_or(StakePoolError::CalculationFailure)?;

        let validator_list_item = if *stake_split_from.key == stake_pool.reserve_stake {
            // check that the validator stake accounts have no withdrawable stake
            if let Some(withdrawable_entry) = validator_list
                .validators
                .iter()
                .find(|&&x| x.stake_lamports != 0)
            {
                let (validator_stake_address, _) = crate::find_stake_program_address(
                    &program_id,
                    &withdrawable_entry.vote_account_address,
                    stake_pool_info.key,
                );
                msg!("Error withdrawing from reserve: validator stake account {} has {} lamports available, please use that first.", validator_stake_address, withdrawable_entry.stake_lamports);
                return Err(StakePoolError::StakeLamportsNotEqualToMinimum.into());
            }

            // check that reserve has enough (should never fail, but who knows?)
            let stake_state = try_from_slice_unchecked::<stake_program::StakeState>(
                &stake_split_from.data.borrow(),
            )?;
            let meta = stake_state.meta().ok_or(StakePoolError::WrongStakeState)?;
            stake_split_from
                .lamports()
                .checked_sub(minimum_reserve_lamports(&meta))
                .ok_or(StakePoolError::StakeLamportsNotEqualToMinimum)?;
            None
        } else {
            let (meta, stake) = get_stake_state(stake_split_from)?;
            let vote_account_address = stake.delegation.voter_pubkey;
            check_validator_stake_address(
                program_id,
                stake_pool_info.key,
                stake_split_from.key,
                &vote_account_address,
            )?;

            if let Some(preferred_withdraw_validator) =
                validator_list.preferred_withdraw_validator_vote_address
            {
                let preferred_validator_info = validator_list
                    .find(&preferred_withdraw_validator)
                    .ok_or(StakePoolError::ValidatorNotFound)?;
                if preferred_withdraw_validator != vote_account_address
                    && preferred_validator_info.stake_lamports > 0
                {
                    msg!("Validator vote address {} is preferred for withdrawals, it currently has {} lamports available. Please withdraw those before using other validator stake accounts.", preferred_withdraw_validator, preferred_validator_info.stake_lamports);
                    return Err(StakePoolError::IncorrectWithdrawVoteAddress.into());
                }
            }

            let validator_list_item = validator_list
                .find_mut(&vote_account_address)
                .ok_or(StakePoolError::ValidatorNotFound)?;

            if validator_list_item.status != StakeStatus::Active {
                msg!("Validator is marked for removal and no longer allowing withdrawals");
                return Err(StakePoolError::ValidatorNotFound.into());
            }

            let required_lamports = minimum_stake_lamports(&meta);
            let current_lamports = stake_split_from.lamports();
            let remaining_lamports = current_lamports.saturating_sub(withdraw_lamports);
            if remaining_lamports < required_lamports {
                msg!("Attempting to withdraw {} lamports from validator account with {} lamports, {} must remain", withdraw_lamports, current_lamports, required_lamports);
                return Err(StakePoolError::StakeLamportsNotEqualToMinimum.into());
            }
            Some(validator_list_item)
        };

        Self::token_burn(
            token_program_info.clone(),
            burn_from_info.clone(),
            pool_mint_info.clone(),
            user_transfer_authority_info.clone(),
            pool_tokens,
        )?;

        Self::stake_split(
            stake_pool_info.key,
            stake_split_from.clone(),
            withdraw_authority_info.clone(),
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            withdraw_lamports,
            stake_split_to.clone(),
        )?;

        Self::stake_authorize_signed(
            stake_pool_info.key,
            stake_split_to.clone(),
            withdraw_authority_info.clone(),
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            user_stake_authority_info.key,
            clock_info.clone(),
            stake_program_info.clone(),
        )?;

        stake_pool.pool_token_supply = stake_pool
            .pool_token_supply
            .checked_sub(pool_tokens)
            .ok_or(StakePoolError::CalculationFailure)?;
        stake_pool.total_stake_lamports = stake_pool
            .total_stake_lamports
            .checked_sub(withdraw_lamports)
            .ok_or(StakePoolError::CalculationFailure)?;
        stake_pool.serialize(&mut *stake_pool_info.data.borrow_mut())?;

        if let Some(validator_list_item) = validator_list_item {
            validator_list_item.stake_lamports = validator_list_item
                .stake_lamports
                .checked_sub(withdraw_lamports)
                .ok_or(StakePoolError::CalculationFailure)?;
            validator_list.serialize(&mut *validator_list_info.data.borrow_mut())?;
        }

        Ok(())
    }

    /// Processes [SetManager](enum.Instruction.html).
    fn process_set_manager(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let new_manager_info = next_account_info(account_info_iter)?;
        let new_manager_fee_info = next_account_info(account_info_iter)?;

        let mut stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        stake_pool.check_manager(manager_info)?;

        if stake_pool.pool_mint
            != spl_token::state::Account::unpack_from_slice(&new_manager_fee_info.data.borrow())?
                .mint
        {
            return Err(StakePoolError::WrongAccountMint.into());
        }

        stake_pool.manager = *new_manager_info.key;
        stake_pool.manager_fee_account = *new_manager_fee_info.key;
        stake_pool.serialize(&mut *stake_pool_info.data.borrow_mut())?;
        Ok(())
    }

    /// Processes [SetFee](enum.Instruction.html).
    fn process_set_fee(_program_id: &Pubkey, accounts: &[AccountInfo], fee: Fee) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;

        let mut stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        stake_pool.check_manager(manager_info)?;

        if stake_pool.last_update_epoch < clock.epoch {
            return Err(StakePoolError::StakeListAndPoolOutOfDate.into());
        }

        // Numerator should be smaller than or equal to denominator (fee <= 1)
        if fee.numerator > fee.denominator {
            msg!(
                "Fee greater than 100%, numerator {}, denominator {}",
                fee.numerator,
                fee.denominator
            );
            return Err(StakePoolError::FeeTooHigh.into());
        }

        stake_pool.fee = fee;
        stake_pool.serialize(&mut *stake_pool_info.data.borrow_mut())?;
        Ok(())
    }

    /// Processes [SetManager](enum.Instruction.html).
    fn process_set_staker(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let set_staker_authority_info = next_account_info(account_info_iter)?;
        let new_staker_info = next_account_info(account_info_iter)?;

        let mut stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        let staker_signed = stake_pool.check_staker(set_staker_authority_info);
        let manager_signed = stake_pool.check_manager(set_staker_authority_info);
        if staker_signed.is_err() && manager_signed.is_err() {
            return Err(StakePoolError::SignatureMissing.into());
        }
        stake_pool.staker = *new_staker_info.key;
        stake_pool.serialize(&mut *stake_pool_info.data.borrow_mut())?;
        Ok(())
    }

    /// Processes [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = StakePoolInstruction::try_from_slice(input)?;
        match instruction {
            StakePoolInstruction::Initialize {
                fee,
                max_validators,
            } => {
                msg!("Instruction: Initialize stake pool");
                Self::process_initialize(program_id, accounts, fee, max_validators)
            }
            StakePoolInstruction::CreateValidatorStakeAccount => {
                msg!("Instruction: CreateValidatorStakeAccount");
                Self::process_create_validator_stake_account(program_id, accounts)
            }
            StakePoolInstruction::AddValidatorToPool => {
                msg!("Instruction: AddValidatorToPool");
                Self::process_add_validator_to_pool(program_id, accounts)
            }
            StakePoolInstruction::RemoveValidatorFromPool => {
                msg!("Instruction: RemoveValidatorFromPool");
                Self::process_remove_validator_from_pool(program_id, accounts)
            }
            StakePoolInstruction::DecreaseValidatorStake(amount) => {
                msg!("Instruction: DecreaseValidatorStake");
                Self::process_decrease_validator_stake(program_id, accounts, amount)
            }
            StakePoolInstruction::IncreaseValidatorStake(amount) => {
                msg!("Instruction: IncreaseValidatorStake");
                Self::process_increase_validator_stake(program_id, accounts, amount)
            }
            StakePoolInstruction::SetPreferredValidator {
                validator_type,
                validator_vote_address,
            } => {
                msg!("Instruction: SetPreferredValidator");
                Self::process_set_preferred_validator(
                    program_id,
                    accounts,
                    validator_type,
                    validator_vote_address,
                )
            }
            StakePoolInstruction::UpdateValidatorListBalance {
                start_index,
                no_merge,
            } => {
                msg!("Instruction: UpdateValidatorListBalance");
                Self::process_update_validator_list_balance(
                    program_id,
                    accounts,
                    start_index,
                    no_merge,
                )
            }
            StakePoolInstruction::UpdateStakePoolBalance => {
                msg!("Instruction: UpdateStakePoolBalance");
                Self::process_update_stake_pool_balance(program_id, accounts)
            }
            StakePoolInstruction::Deposit => {
                msg!("Instruction: Deposit");
                Self::process_deposit(program_id, accounts)
            }
            StakePoolInstruction::Withdraw(amount) => {
                msg!("Instruction: Withdraw");
                Self::process_withdraw(program_id, amount, accounts)
            }
            StakePoolInstruction::SetManager => {
                msg!("Instruction: SetManager");
                Self::process_set_manager(program_id, accounts)
            }
            StakePoolInstruction::SetFee { fee } => {
                msg!("Instruction: SetFee");
                Self::process_set_fee(program_id, accounts, fee)
            }
            StakePoolInstruction::SetStaker => {
                msg!("Instruction: SetStaker");
                Self::process_set_staker(program_id, accounts)
            }
        }
    }
}

impl PrintProgramError for StakePoolError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            StakePoolError::AlreadyInUse => msg!("Error: The account cannot be initialized because it is already being used"),
            StakePoolError::InvalidProgramAddress => msg!("Error: The program address provided doesn't match the value generated by the program"),
            StakePoolError::InvalidState => msg!("Error: The stake pool state is invalid"),
            StakePoolError::CalculationFailure => msg!("Error: The calculation failed"),
            StakePoolError::FeeTooHigh => msg!("Error: Stake pool fee > 1"),
            StakePoolError::WrongAccountMint => msg!("Error: Token account is associated with the wrong mint"),
            StakePoolError::WrongManager => msg!("Error: Wrong pool manager account"),
            StakePoolError::SignatureMissing => msg!("Error: Required signature is missing"),
            StakePoolError::InvalidValidatorStakeList => msg!("Error: Invalid validator stake list account"),
            StakePoolError::InvalidFeeAccount => msg!("Error: Invalid manager fee account"),
            StakePoolError::WrongPoolMint => msg!("Error: Specified pool mint account is wrong"),
            StakePoolError::WrongStakeState => msg!("Error: Stake account is not in the state expected by the program"),
            StakePoolError::UserStakeNotActive => msg!("Error: User stake is not active"),
            StakePoolError::ValidatorAlreadyAdded => msg!("Error: Stake account voting for this validator already exists in the pool"),
            StakePoolError::ValidatorNotFound => msg!("Error: Stake account for this validator not found in the pool"),
            StakePoolError::InvalidStakeAccountAddress => msg!("Error: Stake account address not properly derived from the validator address"),
            StakePoolError::StakeListOutOfDate => msg!("Error: Identify validator stake accounts with old balances and update them"),
            StakePoolError::StakeListAndPoolOutOfDate => msg!("Error: First update old validator stake account balances and then pool stake balance"),
            StakePoolError::UnknownValidatorStakeAccount => {
                msg!("Error: Validator stake account is not found in the list storage")
            }
            StakePoolError::WrongMintingAuthority => msg!("Error: Wrong minting authority set for mint pool account"),
            StakePoolError::UnexpectedValidatorListAccountSize=> msg!("Error: The size of the given validator stake list does match the expected amount"),
            StakePoolError::WrongStaker=> msg!("Error: Wrong pool staker account"),
            StakePoolError::NonZeroPoolTokenSupply => msg!("Error: Pool token supply is not zero on initialization"),
            StakePoolError::StakeLamportsNotEqualToMinimum => msg!("Error: The lamports in the validator stake account is not equal to the minimum"),
            StakePoolError::IncorrectDepositVoteAddress => msg!("Error: The provided deposit stake account is not delegated to the preferred deposit vote account"),
            StakePoolError::IncorrectWithdrawVoteAddress => msg!("Error: The provided withdraw stake account is not the preferred deposit vote account"),
        }
    }
}
