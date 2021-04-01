//! Program state processor

use {
    crate::{
        borsh::try_from_slice_unchecked,
        error::StakePoolError,
        instruction::{Fee, StakePoolInstruction},
        stake_program,
        state::{AccountType, StakePool, ValidatorList, ValidatorStakeInfo},
        AUTHORITY_DEPOSIT, AUTHORITY_WITHDRAW,
    },
    bincode::deserialize,
    borsh::{BorshDeserialize, BorshSerialize},
    num_traits::FromPrimitive,
    solana_program::{
        account_info::next_account_info,
        account_info::AccountInfo,
        clock::Clock,
        decode_error::DecodeError,
        entrypoint::ProgramResult,
        msg,
        native_token::sol_to_lamports,
        program::{invoke, invoke_signed},
        program_error::PrintProgramError,
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        rent::Rent,
        stake_history::StakeHistory,
        system_instruction,
        sysvar::Sysvar,
    },
    spl_token::state::Mint,
};

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Returns validator address for a particular stake account
    fn get_validator(stake_account_info: &AccountInfo) -> Result<Pubkey, ProgramError> {
        let stake_state: stake_program::StakeState = deserialize(&stake_account_info.data.borrow())
            .or(Err(ProgramError::InvalidAccountData))?;
        match stake_state {
            stake_program::StakeState::Stake(_, stake) => Ok(stake.delegation.voter_pubkey),
            _ => Err(StakePoolError::WrongStakeState.into()),
        }
    }

    /// Checks if validator stake account is a proper program address
    fn is_validator_stake_address(
        vote_account: &Pubkey,
        program_id: &Pubkey,
        stake_pool_info: &AccountInfo,
        stake_account_info: &AccountInfo,
    ) -> bool {
        // Check stake account address validity
        let (stake_address, _) =
            crate::find_stake_program_address(&program_id, &vote_account, &stake_pool_info.key);
        stake_address == *stake_account_info.key
    }

    /// Returns validator address for a particular stake account and checks its validity
    fn get_validator_checked(
        program_id: &Pubkey,
        stake_pool_info: &AccountInfo,
        stake_account_info: &AccountInfo,
    ) -> Result<Pubkey, ProgramError> {
        let vote_account = Self::get_validator(stake_account_info)?;

        if !Self::is_validator_stake_address(
            &vote_account,
            program_id,
            stake_pool_info,
            stake_account_info,
        ) {
            return Err(StakePoolError::InvalidStakeAccountAddress.into());
        }
        Ok(vote_account)
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

        let ix =
            stake_program::split_only(stake_account.key, authority.key, amount, split_stake.key);

        invoke_signed(&ix, &[stake_account, split_stake, authority], signers)
    }

    /// Issue a stake_merge instruction.
    #[allow(clippy::too_many_arguments)]
    fn stake_merge<'a>(
        stake_pool: &Pubkey,
        stake_account: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        authority_type: &[u8],
        bump_seed: u8,
        merge_with: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        stake_history: AccountInfo<'a>,
        stake_program_info: AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let me_bytes = stake_pool.to_bytes();
        let authority_signature_seeds = [&me_bytes[..32], authority_type, &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];

        let ix = stake_program::merge(merge_with.key, stake_account.key, authority.key);

        invoke_signed(
            &ix,
            &[
                merge_with,
                stake_account,
                clock,
                stake_history,
                authority,
                stake_program_info,
            ],
            signers,
        )
    }

    /// Issue a stake_set_manager instruction.
    #[allow(clippy::too_many_arguments)]
    fn stake_authorize<'a>(
        stake_pool: &Pubkey,
        stake_account: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        authority_type: &[u8],
        bump_seed: u8,
        new_staker: &Pubkey,
        staker_auth: stake_program::StakeAuthorize,
        clock: AccountInfo<'a>,
        stake_program_info: AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let me_bytes = stake_pool.to_bytes();
        let authority_signature_seeds = [&me_bytes[..32], authority_type, &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];

        let ix =
            stake_program::authorize(stake_account.key, authority.key, new_staker, staker_auth);

        invoke_signed(
            &ix,
            &[stake_account, clock, authority, stake_program_info],
            signers,
        )
    }

    /// Issue a spl_token `Burn` instruction.
    #[allow(clippy::too_many_arguments)]
    fn token_burn<'a>(
        stake_pool: &Pubkey,
        token_program: AccountInfo<'a>,
        burn_account: AccountInfo<'a>,
        mint: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        authority_type: &[u8],
        bump_seed: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let me_bytes = stake_pool.to_bytes();
        let authority_signature_seeds = [&me_bytes[..32], authority_type, &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];

        let ix = spl_token::instruction::burn(
            token_program.key,
            burn_account.key,
            mint.key,
            authority.key,
            &[],
            amount,
        )?;

        invoke_signed(
            &ix,
            &[burn_account, mint, authority, token_program],
            signers,
        )
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
        let pool_mint_info = next_account_info(account_info_iter)?;
        let manager_fee_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let token_program_info = next_account_info(account_info_iter)?;

        if !manager_info.is_signer {
            return Err(StakePoolError::SignatureMissing.into());
        }

        let mut stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_uninitialized() {
            return Err(StakePoolError::AlreadyInUse.into());
        }

        let mut validator_list =
            try_from_slice_unchecked::<ValidatorList>(&validator_list_info.data.borrow())?;
        if !validator_list.is_uninitialized() {
            return Err(StakePoolError::AlreadyInUse.into());
        }

        let data_length = validator_list_info.data_len();
        let expected_max_validators = ValidatorList::calculate_max_validators(data_length);
        if expected_max_validators != max_validators as usize || max_validators == 0 {
            return Err(StakePoolError::UnexpectedValidatorListAccountSize.into());
        }
        validator_list.account_type = AccountType::ValidatorList;
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
            return Err(StakePoolError::InvalidFeeAccount.into());
        }

        if pool_mint_info.owner != token_program_info.key {
            return Err(ProgramError::IncorrectProgramId);
        }

        if *pool_mint_info.key
            != spl_token::state::Account::unpack_from_slice(&manager_fee_info.data.borrow())?.mint
        {
            return Err(StakePoolError::WrongAccountMint.into());
        }

        let (_, deposit_bump_seed) =
            crate::find_deposit_authority_program_address(program_id, stake_pool_info.key);
        let (withdraw_authority_key, withdraw_bump_seed) =
            crate::find_withdraw_authority_program_address(program_id, stake_pool_info.key);

        let pool_mint = Mint::unpack_from_slice(&pool_mint_info.data.borrow())?;

        if !pool_mint.mint_authority.contains(&withdraw_authority_key) {
            return Err(StakePoolError::WrongMintingAuthority.into());
        }

        validator_list.serialize(&mut *validator_list_info.data.borrow_mut())?;

        msg!("Clock data: {:?}", clock_info.data.borrow());
        msg!("Epoch: {}", clock.epoch);

        stake_pool.account_type = AccountType::StakePool;
        stake_pool.manager = *manager_info.key;
        stake_pool.staker = *staker_info.key;
        stake_pool.deposit_bump_seed = deposit_bump_seed;
        stake_pool.withdraw_bump_seed = withdraw_bump_seed;
        stake_pool.validator_list = *validator_list_info.key;
        stake_pool.pool_mint = *pool_mint_info.key;
        stake_pool.manager_fee_account = *manager_fee_info.key;
        stake_pool.token_program_id = *token_program_info.key;
        stake_pool.last_update_epoch = clock.epoch;
        stake_pool.fee = fee;

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

        if *system_program_info.key != solana_program::system_program::id() {
            return Err(ProgramError::IncorrectProgramId);
        }
        if *stake_program_info.key != stake_program::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

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
        let required_lamports = sol_to_lamports(1.0)
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
        let deposit_info = next_account_info(account_info_iter)?;
        let withdraw_info = next_account_info(account_info_iter)?;
        let validator_list_info = next_account_info(account_info_iter)?;
        let stake_account_info = next_account_info(account_info_iter)?;
        let dest_user_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        let stake_history_info = next_account_info(account_info_iter)?;
        let stake_history = &StakeHistory::from_account_info(stake_history_info)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let stake_program_info = next_account_info(account_info_iter)?;

        if *stake_program_info.key != stake_program::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

        let mut stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        stake_pool.check_authority_withdraw(withdraw_info.key, program_id, stake_pool_info.key)?;
        stake_pool.check_authority_deposit(deposit_info.key, program_id, stake_pool_info.key)?;

        stake_pool.check_staker(staker_info)?;

        if stake_pool.last_update_epoch < clock.epoch {
            return Err(StakePoolError::StakeListAndPoolOutOfDate.into());
        }

        if stake_pool.token_program_id != *token_program_info.key {
            return Err(ProgramError::IncorrectProgramId);
        }
        if stake_pool.pool_mint != *pool_mint_info.key {
            return Err(StakePoolError::WrongPoolMint.into());
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

        let vote_account =
            Self::get_validator_checked(program_id, stake_pool_info, stake_account_info)?;

        if validator_list.contains(&vote_account) {
            return Err(StakePoolError::ValidatorAlreadyAdded.into());
        }

        // Update Withdrawer and Staker authority to the program withdraw authority
        for authority in &[
            stake_program::StakeAuthorize::Withdrawer,
            stake_program::StakeAuthorize::Staker,
        ] {
            Self::stake_authorize(
                stake_pool_info.key,
                stake_account_info.clone(),
                deposit_info.clone(),
                AUTHORITY_DEPOSIT,
                stake_pool.deposit_bump_seed,
                withdraw_info.key,
                *authority,
                clock_info.clone(),
                stake_program_info.clone(),
            )?;
        }

        // Calculate and mint tokens
        let stake_lamports = **stake_account_info.lamports.borrow();
        let token_amount = stake_pool
            .calc_pool_deposit_amount(stake_lamports)
            .ok_or(StakePoolError::CalculationFailure)?;
        Self::token_mint_to(
            stake_pool_info.key,
            token_program_info.clone(),
            pool_mint_info.clone(),
            dest_user_info.clone(),
            withdraw_info.clone(),
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            token_amount,
        )?;

        // Check if stake is warmed up
        Self::check_stake_activation(stake_account_info, clock, stake_history)?;

        validator_list.validators.push(ValidatorStakeInfo {
            vote_account,
            balance: stake_lamports,
            last_update_epoch: clock.epoch,
        });
        validator_list.serialize(&mut *validator_list_info.data.borrow_mut())?;

        stake_pool.pool_total += token_amount;
        stake_pool.stake_total += stake_lamports;
        stake_pool.serialize(&mut *stake_pool_info.data.borrow_mut())?;

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
        let withdraw_info = next_account_info(account_info_iter)?;
        let new_stake_authority_info = next_account_info(account_info_iter)?;
        let validator_list_info = next_account_info(account_info_iter)?;
        let stake_account_info = next_account_info(account_info_iter)?;
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

        stake_pool.check_authority_withdraw(withdraw_info.key, program_id, stake_pool_info.key)?;
        stake_pool.check_staker(staker_info)?;

        if stake_pool.last_update_epoch < clock.epoch {
            return Err(StakePoolError::StakeListAndPoolOutOfDate.into());
        }

        if stake_pool.token_program_id != *token_program_info.key {
            return Err(ProgramError::IncorrectProgramId);
        }
        if stake_pool.pool_mint != *pool_mint_info.key {
            return Err(StakePoolError::WrongPoolMint.into());
        }

        if *validator_list_info.key != stake_pool.validator_list {
            return Err(StakePoolError::InvalidValidatorStakeList.into());
        }

        let mut validator_list =
            try_from_slice_unchecked::<ValidatorList>(&validator_list_info.data.borrow())?;
        if !validator_list.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        let vote_account =
            Self::get_validator_checked(program_id, stake_pool_info, stake_account_info)?;

        if !validator_list.contains(&vote_account) {
            return Err(StakePoolError::ValidatorNotFound.into());
        }

        for authority in &[
            stake_program::StakeAuthorize::Withdrawer,
            stake_program::StakeAuthorize::Staker,
        ] {
            Self::stake_authorize(
                stake_pool_info.key,
                stake_account_info.clone(),
                withdraw_info.clone(),
                AUTHORITY_WITHDRAW,
                stake_pool.withdraw_bump_seed,
                new_stake_authority_info.key,
                *authority,
                clock_info.clone(),
                stake_program_info.clone(),
            )?;
        }

        // Calculate and burn tokens
        let stake_lamports = **stake_account_info.lamports.borrow();
        let token_amount = stake_pool
            .calc_pool_withdraw_amount(stake_lamports)
            .ok_or(StakePoolError::CalculationFailure)?;
        Self::token_burn(
            stake_pool_info.key,
            token_program_info.clone(),
            burn_from_info.clone(),
            pool_mint_info.clone(),
            withdraw_info.clone(),
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            token_amount,
        )?;

        validator_list
            .validators
            .retain(|item| item.vote_account != vote_account);
        validator_list.serialize(&mut *validator_list_info.data.borrow_mut())?;

        stake_pool.pool_total -= token_amount;
        stake_pool.stake_total -= stake_lamports;
        stake_pool.serialize(&mut *stake_pool_info.data.borrow_mut())?;

        Ok(())
    }

    /// Processes `UpdateValidatorListBalance` instruction.
    fn process_update_validator_list_balance(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let validator_list_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        let validator_stake_accounts = account_info_iter.as_slice();

        let mut validator_list =
            try_from_slice_unchecked::<ValidatorList>(&validator_list_info.data.borrow())?;
        if !validator_list.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        let vote_accounts: Vec<Option<Pubkey>> = validator_stake_accounts
            .iter()
            .map(|stake| Self::get_validator(stake).ok())
            .collect();

        let mut changes = false;
        // Do a brute iteration through the list, optimize if necessary
        for validator_stake_record in &mut validator_list.validators {
            if validator_stake_record.last_update_epoch >= clock.epoch {
                continue;
            }
            for (validator_stake_account, vote_account) in
                validator_stake_accounts.iter().zip(vote_accounts.iter())
            {
                if validator_stake_record.vote_account
                    != vote_account.ok_or(StakePoolError::WrongStakeState)?
                {
                    continue;
                }
                validator_stake_record.last_update_epoch = clock.epoch;
                validator_stake_record.balance = **validator_stake_account.lamports.borrow();
                changes = true;
            }
        }

        if changes {
            validator_list.serialize(&mut *validator_list_info.data.borrow_mut())?;
        }

        Ok(())
    }

    /// Processes `UpdateStakePoolBalance` instruction.
    fn process_update_stake_pool_balance(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let validator_list_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;

        let mut stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        if *validator_list_info.key != stake_pool.validator_list {
            return Err(StakePoolError::InvalidValidatorStakeList.into());
        }

        let validator_list =
            try_from_slice_unchecked::<ValidatorList>(&validator_list_info.data.borrow())?;
        if !validator_list.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        let mut total_balance: u64 = 0;
        for validator_stake_record in validator_list.validators {
            if validator_stake_record.last_update_epoch < clock.epoch {
                return Err(StakePoolError::StakeListOutOfDate.into());
            }
            total_balance += validator_stake_record.balance;
        }

        stake_pool.stake_total = total_balance;
        stake_pool.last_update_epoch = clock.epoch;
        stake_pool.serialize(&mut *stake_pool_info.data.borrow_mut())?;

        Ok(())
    }

    /// Check stake activation status
    #[allow(clippy::unnecessary_wraps)]
    fn check_stake_activation(
        _stake_info: &AccountInfo,
        _clock: &Clock,
        _stake_history: &StakeHistory,
    ) -> ProgramResult {
        // TODO: remove conditional compilation when time travel in tests is possible
        //#[cfg(not(feature = "test-bpf"))]
        // This check is commented to make tests run without special command line arguments
        /*{
            let stake_acc_state: stake_program::StakeState =
                deserialize(&stake_info.data.borrow()).unwrap();
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
        }*/
        Ok(())
    }

    /// Processes [Deposit](enum.Instruction.html).
    fn process_deposit(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let validator_list_info = next_account_info(account_info_iter)?;
        let deposit_info = next_account_info(account_info_iter)?;
        let withdraw_info = next_account_info(account_info_iter)?;
        let stake_info = next_account_info(account_info_iter)?;
        let validator_stake_account_info = next_account_info(account_info_iter)?;
        let dest_user_info = next_account_info(account_info_iter)?;
        let manager_fee_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        let stake_history_info = next_account_info(account_info_iter)?;
        let stake_history = &StakeHistory::from_account_info(stake_history_info)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let stake_program_info = next_account_info(account_info_iter)?;

        if *stake_program_info.key != stake_program::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

        let mut stake_pool = StakePool::try_from_slice(&stake_pool_info.data.borrow())?;
        if !stake_pool.is_valid() {
            return Err(StakePoolError::InvalidState.into());
        }

        Self::check_stake_activation(stake_info, clock, stake_history)?;

        stake_pool.check_authority_withdraw(withdraw_info.key, program_id, stake_pool_info.key)?;
        stake_pool.check_authority_deposit(deposit_info.key, program_id, stake_pool_info.key)?;

        if stake_pool.manager_fee_account != *manager_fee_info.key {
            return Err(StakePoolError::InvalidFeeAccount.into());
        }
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

        let vote_account =
            Self::get_validator_checked(program_id, stake_pool_info, validator_stake_account_info)?;

        let validator_list_item = validator_list
            .find_mut(&vote_account)
            .ok_or(StakePoolError::ValidatorNotFound)?;

        let stake_lamports = **stake_info.lamports.borrow();
        let pool_amount = stake_pool
            .calc_pool_deposit_amount(stake_lamports)
            .ok_or(StakePoolError::CalculationFailure)?;

        let fee_amount = stake_pool
            .calc_fee_amount(pool_amount)
            .ok_or(StakePoolError::CalculationFailure)?;

        let user_amount = pool_amount
            .checked_sub(fee_amount)
            .ok_or(StakePoolError::CalculationFailure)?;

        Self::stake_authorize(
            stake_pool_info.key,
            stake_info.clone(),
            deposit_info.clone(),
            AUTHORITY_DEPOSIT,
            stake_pool.deposit_bump_seed,
            withdraw_info.key,
            stake_program::StakeAuthorize::Withdrawer,
            clock_info.clone(),
            stake_program_info.clone(),
        )?;

        Self::stake_authorize(
            stake_pool_info.key,
            stake_info.clone(),
            deposit_info.clone(),
            AUTHORITY_DEPOSIT,
            stake_pool.deposit_bump_seed,
            withdraw_info.key,
            stake_program::StakeAuthorize::Staker,
            clock_info.clone(),
            stake_program_info.clone(),
        )?;

        Self::stake_merge(
            stake_pool_info.key,
            stake_info.clone(),
            withdraw_info.clone(),
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
            withdraw_info.clone(),
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            user_amount,
        )?;

        Self::token_mint_to(
            stake_pool_info.key,
            token_program_info.clone(),
            pool_mint_info.clone(),
            manager_fee_info.clone(),
            withdraw_info.clone(),
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            fee_amount,
        )?;
        stake_pool.pool_total += pool_amount;
        stake_pool.stake_total += stake_lamports;
        stake_pool.serialize(&mut *stake_pool_info.data.borrow_mut())?;

        validator_list_item.balance = **validator_stake_account_info.lamports.borrow();
        validator_list.serialize(&mut *validator_list_info.data.borrow_mut())?;

        Ok(())
    }

    /// Processes [Withdraw](enum.Instruction.html).
    fn process_withdraw(
        program_id: &Pubkey,
        pool_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let validator_list_info = next_account_info(account_info_iter)?;
        let withdraw_info = next_account_info(account_info_iter)?;
        let stake_split_from = next_account_info(account_info_iter)?;
        let stake_split_to = next_account_info(account_info_iter)?;
        let user_stake_authority = next_account_info(account_info_iter)?;
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

        stake_pool.check_authority_withdraw(withdraw_info.key, program_id, stake_pool_info.key)?;

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

        let vote_account =
            Self::get_validator_checked(program_id, stake_pool_info, stake_split_from)?;

        let validator_list_item = validator_list
            .find_mut(&vote_account)
            .ok_or(StakePoolError::ValidatorNotFound)?;

        let stake_amount = stake_pool
            .calc_lamports_withdraw_amount(pool_amount)
            .ok_or(StakePoolError::CalculationFailure)?;

        Self::stake_split(
            stake_pool_info.key,
            stake_split_from.clone(),
            withdraw_info.clone(),
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            stake_amount,
            stake_split_to.clone(),
        )?;

        Self::stake_authorize(
            stake_pool_info.key,
            stake_split_to.clone(),
            withdraw_info.clone(),
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            user_stake_authority.key,
            stake_program::StakeAuthorize::Withdrawer,
            clock_info.clone(),
            stake_program_info.clone(),
        )?;

        Self::stake_authorize(
            stake_pool_info.key,
            stake_split_to.clone(),
            withdraw_info.clone(),
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            user_stake_authority.key,
            stake_program::StakeAuthorize::Staker,
            clock_info.clone(),
            stake_program_info.clone(),
        )?;

        Self::token_burn(
            stake_pool_info.key,
            token_program_info.clone(),
            burn_from_info.clone(),
            pool_mint_info.clone(),
            withdraw_info.clone(),
            AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            pool_amount,
        )?;

        stake_pool.pool_total -= pool_amount;
        stake_pool.stake_total -= stake_amount;
        stake_pool.serialize(&mut *stake_pool_info.data.borrow_mut())?;

        validator_list_item.balance = **stake_split_from.lamports.borrow();
        validator_list.serialize(&mut *validator_list_info.data.borrow_mut())?;

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
                msg!("Instruction: Init");
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
            StakePoolInstruction::UpdateValidatorListBalance => {
                msg!("Instruction: UpdateValidatorListBalance");
                Self::process_update_validator_list_balance(program_id, accounts)
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
        }
    }
}
