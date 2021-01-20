//! Program state processor

use crate::{
    error::StakePoolError,
    instruction::{InitArgs, StakePoolInstruction},
    stake,
    state::{StakePool, State, ValidatorStakeInfo, ValidatorStakeList},
    PROGRAM_VERSION,
};
use bincode::deserialize;
use num_traits::FromPrimitive;
use solana_program::{
    account_info::next_account_info,
    account_info::AccountInfo,
    clock::Clock,
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::PrintProgramError,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Suffix for deposit authority seed
    pub const AUTHORITY_DEPOSIT: &'static [u8] = b"deposit";
    /// Suffix for withdraw authority seed
    pub const AUTHORITY_WITHDRAW: &'static [u8] = b"withdraw";
    /// Calculates the authority id by generating a program address.
    pub fn authority_id(
        program_id: &Pubkey,
        stake_pool: &Pubkey,
        authority_type: &[u8],
        bump_seed: u8,
    ) -> Result<Pubkey, ProgramError> {
        Pubkey::create_program_address(
            &[&stake_pool.to_bytes()[..32], authority_type, &[bump_seed]],
            program_id,
        )
        .map_err(|_| StakePoolError::InvalidProgramAddress.into())
    }
    /// Generates seed bump for stake pool authorities
    pub fn find_authority_bump_seed(
        program_id: &Pubkey,
        stake_pool: &Pubkey,
        authority_type: &[u8],
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[&stake_pool.to_bytes()[..32], authority_type], program_id)
    }
    /// Generates stake account address for the validator
    pub fn find_stake_address_for_validator(
        program_id: &Pubkey,
        validator: &Pubkey,
        stake_pool: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[&validator.to_bytes()[..32], &stake_pool.to_bytes()[..32]],
            program_id,
        )
    }

    /// Checks withdraw or deposit authority
    pub fn check_authority(
        authority_to_check: &Pubkey,
        program_id: &Pubkey,
        stake_pool_key: &Pubkey,
        authority_type: &[u8],
        bump_seed: u8,
    ) -> Result<(), ProgramError> {
        if *authority_to_check
            != Self::authority_id(program_id, stake_pool_key, authority_type, bump_seed)?
        {
            return Err(StakePoolError::InvalidProgramAddress.into());
        }
        Ok(())
    }

    /// Returns validator address for a particular stake account
    pub fn get_validator(stake_account_info: &AccountInfo) -> Result<Pubkey, ProgramError> {
        let stake_state: stake::StakeState = deserialize(&stake_account_info.data.borrow())
            .or(Err(ProgramError::InvalidAccountData))?;
        match stake_state {
            stake::StakeState::Stake(_, stake) => Ok(stake.delegation.voter_pubkey),
            _ => Err(StakePoolError::WrongStakeState.into()),
        }
    }

    /// Checks if validator stake account is a proper program address
    pub fn is_validator_stake_address(
        validator_account: &Pubkey,
        program_id: &Pubkey,
        stake_pool_info: &AccountInfo,
        stake_account_info: &AccountInfo,
    ) -> bool {
        // Check stake account address validity
        let (stake_address, _) = Self::find_stake_address_for_validator(
            &program_id,
            &validator_account,
            &stake_pool_info.key,
        );
        stake_address == *stake_account_info.key
    }

    /// Returns validator address for a particular stake account and checks its validity
    pub fn get_validator_checked(
        program_id: &Pubkey,
        stake_pool_info: &AccountInfo,
        stake_account_info: &AccountInfo,
    ) -> Result<Pubkey, ProgramError> {
        let validator_account = Self::get_validator(stake_account_info)?;

        if !Self::is_validator_stake_address(
            &validator_account,
            program_id,
            stake_pool_info,
            stake_account_info,
        ) {
            return Err(StakePoolError::InvalidStakeAccountAddress.into());
        }
        Ok(validator_account)
    }

    /// Issue a stake_split instruction.
    #[allow(clippy::too_many_arguments)]
    pub fn stake_split<'a>(
        stake_pool: &Pubkey,
        stake_account: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        authority_type: &[u8],
        bump_seed: u8,
        amount: u64,
        split_stake: AccountInfo<'a>,
        reserved: AccountInfo<'a>,
        stake_program_info: AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let me_bytes = stake_pool.to_bytes();
        let authority_signature_seeds = [&me_bytes[..32], authority_type, &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];

        let ix = stake::split_only(stake_account.key, authority.key, amount, split_stake.key);

        invoke_signed(
            &ix,
            &[
                stake_account,
                reserved,
                authority,
                split_stake,
                stake_program_info,
            ],
            signers,
        )
    }

    /// Issue a stake_merge instruction.
    #[allow(clippy::too_many_arguments)]
    pub fn stake_merge<'a>(
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

        let ix = stake::merge(merge_with.key, stake_account.key, authority.key);

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

    /// Issue a stake_set_owner instruction.
    #[allow(clippy::too_many_arguments)]
    pub fn stake_authorize<'a>(
        stake_pool: &Pubkey,
        stake_account: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        authority_type: &[u8],
        bump_seed: u8,
        new_staker: &Pubkey,
        staker_auth: stake::StakeAuthorize,
        reserved: AccountInfo<'a>,
        stake_program_info: AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let me_bytes = stake_pool.to_bytes();
        let authority_signature_seeds = [&me_bytes[..32], authority_type, &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];

        let ix = stake::authorize(stake_account.key, authority.key, new_staker, staker_auth);

        invoke_signed(
            &ix,
            &[stake_account, reserved, authority, stake_program_info],
            signers,
        )
    }

    /// Issue a spl_token `Burn` instruction.
    #[allow(clippy::too_many_arguments)]
    pub fn token_burn<'a>(
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
    pub fn token_mint_to<'a>(
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
    pub fn process_initialize(
        program_id: &Pubkey,
        init: InitArgs,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let owner_info = next_account_info(account_info_iter)?;
        let validator_stake_list_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let owner_fee_info = next_account_info(account_info_iter)?;
        // Clock sysvar account
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        // Token program ID
        let token_program_info = next_account_info(account_info_iter)?;

        // Stake pool account should not be already initialized
        if State::Unallocated != State::deserialize(&stake_pool_info.data.borrow())? {
            return Err(StakePoolError::AlreadyInUse.into());
        }

        // Check if validator stake list storage is unitialized
        let mut validator_stake_list =
            ValidatorStakeList::deserialize(&validator_stake_list_info.data.borrow())?;
        if validator_stake_list.is_initialized {
            return Err(StakePoolError::AlreadyInUse.into());
        }
        validator_stake_list.is_initialized = true;
        validator_stake_list.validators.clear();

        // Numerator should be smaller than or equal to denominator (fee <= 1)
        if init.fee.numerator > init.fee.denominator {
            return Err(StakePoolError::FeeTooHigh.into());
        }

        // Check for owner fee account to have proper mint assigned
        if *pool_mint_info.key
            != spl_token::state::Account::unpack_from_slice(&owner_fee_info.data.borrow())?.mint
        {
            return Err(StakePoolError::WrongAccountMint.into());
        }

        // Check pool mint program ID
        if pool_mint_info.owner != token_program_info.key {
            return Err(ProgramError::IncorrectProgramId);
        }

        let (_, deposit_bump_seed) = Self::find_authority_bump_seed(
            program_id,
            stake_pool_info.key,
            Self::AUTHORITY_DEPOSIT,
        );
        let (_, withdraw_bump_seed) = Self::find_authority_bump_seed(
            program_id,
            stake_pool_info.key,
            Self::AUTHORITY_WITHDRAW,
        );

        validator_stake_list.serialize(&mut validator_stake_list_info.data.borrow_mut())?;

        msg!("Clock data: {:?}", clock_info.data.borrow());
        msg!("Epoch: {}", clock.epoch);

        let stake_pool = State::Init(StakePool {
            version: PROGRAM_VERSION,
            owner: *owner_info.key,
            deposit_bump_seed,
            withdraw_bump_seed,
            validator_stake_list: *validator_stake_list_info.key,
            pool_mint: *pool_mint_info.key,
            owner_fee_account: *owner_fee_info.key,
            token_program_id: *token_program_info.key,
            stake_total: 0,
            pool_total: 0,
            last_update_epoch: clock.epoch,
            fee: init.fee,
        });
        stake_pool.serialize(&mut stake_pool_info.data.borrow_mut())
    }

    /// Processes `CreateValidatorStakeAccount` instruction.
    pub fn process_create_validator_stake_account(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        // Stake pool account
        let stake_pool_info = next_account_info(account_info_iter)?;
        // Account creation funder account
        let funder_info = next_account_info(account_info_iter)?;
        // Stake account to be created
        let stake_account_info = next_account_info(account_info_iter)?;
        // Validator this stake account will vote for
        let validator_info = next_account_info(account_info_iter)?;
        // Stake authority for the new stake account
        let stake_authority_info = next_account_info(account_info_iter)?;
        // Withdraw authority for the new stake account
        let withdraw_authority_info = next_account_info(account_info_iter)?;
        // Rent sysvar account
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        // System program id
        let system_program_info = next_account_info(account_info_iter)?;
        // Staking program id
        let stake_program_info = next_account_info(account_info_iter)?;

        // Check program ids
        if *system_program_info.key != solana_program::system_program::id() {
            return Err(ProgramError::IncorrectProgramId);
        }
        if *stake_program_info.key != stake::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

        // Check stake account address validity
        let (stake_address, bump_seed) = Self::find_stake_address_for_validator(
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

        // Fund the associated token account with the minimum balance to be rent exempt
        let required_lamports = 1 + rent.minimum_balance(std::mem::size_of::<stake::StakeState>());
        // Fund new account
        invoke(
            &system_instruction::transfer(
                &funder_info.key,
                stake_account_info.key,
                required_lamports,
            ),
            &[
                funder_info.clone(),
                stake_account_info.clone(),
                system_program_info.clone(),
            ],
        )?;
        // Allocate account space
        invoke_signed(
            &system_instruction::allocate(
                stake_account_info.key,
                std::mem::size_of::<stake::StakeState>() as u64,
            ),
            &[stake_account_info.clone(), system_program_info.clone()],
            &[&stake_account_signer_seeds],
        )?;
        // Assign account to the stake program
        invoke_signed(
            &system_instruction::assign(stake_account_info.key, &stake::id()),
            &[stake_account_info.clone(), system_program_info.clone()],
            &[&stake_account_signer_seeds],
        )?;
        // Initialize stake account
        invoke(
            &stake::initialize(
                &stake_account_info.key,
                &stake::Authorized {
                    staker: *stake_authority_info.key,
                    withdrawer: *withdraw_authority_info.key,
                },
                &stake::Lockup::default(),
            ),
            &[
                stake_account_info.clone(),
                rent_info.clone(),
                stake_program_info.clone(),
            ],
        )
    }

    /// Processes `AddValidatorStakeAccount` instruction.
    pub fn process_add_validator_stake_account(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        // Stake pool account
        let stake_pool_info = next_account_info(account_info_iter)?;
        // Pool owner account
        let owner_info = next_account_info(account_info_iter)?;
        // Stake pool deposit authority
        let deposit_info = next_account_info(account_info_iter)?;
        // Stake pool withdraw authority
        let withdraw_info = next_account_info(account_info_iter)?;
        // Account storing validator stake list
        let validator_stake_list_info = next_account_info(account_info_iter)?;
        // Stake account to add to the pool
        let stake_account_info = next_account_info(account_info_iter)?;
        // User account to receive pool tokens
        let dest_user_info = next_account_info(account_info_iter)?;
        // Pool token mint account
        let pool_mint_info = next_account_info(account_info_iter)?;
        // Clock sysvar account
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        // Pool token program id
        let token_program_info = next_account_info(account_info_iter)?;
        // Staking program id
        let stake_program_info = next_account_info(account_info_iter)?;

        // Check program ids
        if *stake_program_info.key != stake::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

        // Get stake pool stake (and check if it is iniaialized)
        let mut stake_pool = State::deserialize(&stake_pool_info.data.borrow())?.stake_pool()?;

        // Check authority accounts
        stake_pool.check_authority_withdraw(withdraw_info.key, program_id, stake_pool_info.key)?;
        stake_pool.check_authority_deposit(deposit_info.key, program_id, stake_pool_info.key)?;

        // Check owner validity and signature
        stake_pool.check_owner(owner_info)?;

        // Check stake pool last update epoch
        if stake_pool.last_update_epoch < clock.epoch {
            return Err(StakePoolError::StakeListAndPoolOutOfDate.into());
        }

        if stake_pool.token_program_id != *token_program_info.key {
            return Err(StakePoolError::InvalidProgramAddress.into());
        }
        if stake_pool.pool_mint != *pool_mint_info.key {
            return Err(StakePoolError::WrongPoolMint.into());
        }

        // Check validator stake account list storage
        if *validator_stake_list_info.key != stake_pool.validator_stake_list {
            return Err(StakePoolError::InvalidValidatorStakeList.into());
        }

        // Read validator stake list account and check if it is valid
        let mut validator_stake_list =
            ValidatorStakeList::deserialize(&validator_stake_list_info.data.borrow())?;
        if !validator_stake_list.is_initialized {
            return Err(StakePoolError::InvalidState.into());
        }

        let validator_account =
            Self::get_validator_checked(program_id, stake_pool_info, stake_account_info)?;

        if validator_stake_list.contains(&validator_account) {
            return Err(StakePoolError::ValidatorAlreadyAdded.into());
        }

        // Update Withdrawer and Staker authority to the program withdraw authority
        for authority in &[
            stake::StakeAuthorize::Withdrawer,
            stake::StakeAuthorize::Staker,
        ] {
            Self::stake_authorize(
                stake_pool_info.key,
                stake_account_info.clone(),
                deposit_info.clone(),
                Self::AUTHORITY_DEPOSIT,
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
            Self::AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            token_amount,
        )?;

        // TODO: Check if stake is warmed up

        // Add validator to the list and save
        validator_stake_list.validators.push(ValidatorStakeInfo {
            validator_account,
            balance: stake_lamports,
            last_update_epoch: clock.epoch,
        });
        validator_stake_list.serialize(&mut validator_stake_list_info.data.borrow_mut())?;

        // Save amounts to the stake pool state
        stake_pool.pool_total += token_amount;
        // Only update stake total if the last state update epoch is current
        stake_pool.stake_total += stake_lamports;
        State::Init(stake_pool).serialize(&mut stake_pool_info.data.borrow_mut())?;

        Ok(())
    }

    /// Processes `RemoveValidatorStakeAccount` instruction.
    pub fn process_remove_validator_stake_account(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        // Stake pool account
        let stake_pool_info = next_account_info(account_info_iter)?;
        // Pool owner account
        let owner_info = next_account_info(account_info_iter)?;
        // Stake pool withdraw authority
        let withdraw_info = next_account_info(account_info_iter)?;
        // New stake authority
        let new_stake_authority_info = next_account_info(account_info_iter)?;
        // Account storing validator stake list
        let validator_stake_list_info = next_account_info(account_info_iter)?;
        // Stake account to remove from the pool
        let stake_account_info = next_account_info(account_info_iter)?;
        // User account with pool tokens to burn from
        let burn_from_info = next_account_info(account_info_iter)?;
        // Pool token mint account
        let pool_mint_info = next_account_info(account_info_iter)?;
        // Clock sysvar account
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        // Pool token program id
        let token_program_info = next_account_info(account_info_iter)?;
        // Staking program id
        let stake_program_info = next_account_info(account_info_iter)?;

        // Check program ids
        if *stake_program_info.key != stake::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

        // Get stake pool stake (and check if it is iniaialized)
        let mut stake_pool = State::deserialize(&stake_pool_info.data.borrow())?.stake_pool()?;

        // Check authority account
        stake_pool.check_authority_withdraw(withdraw_info.key, program_id, stake_pool_info.key)?;

        // Check owner validity and signature
        stake_pool.check_owner(owner_info)?;

        // Check stake pool last update epoch
        if stake_pool.last_update_epoch < clock.epoch {
            return Err(StakePoolError::StakeListAndPoolOutOfDate.into());
        }

        if stake_pool.token_program_id != *token_program_info.key {
            return Err(StakePoolError::InvalidProgramAddress.into());
        }
        if stake_pool.pool_mint != *pool_mint_info.key {
            return Err(StakePoolError::WrongPoolMint.into());
        }

        // Check validator stake account list storage
        if *validator_stake_list_info.key != stake_pool.validator_stake_list {
            return Err(StakePoolError::InvalidValidatorStakeList.into());
        }

        // Read validator stake list account and check if it is valid
        let mut validator_stake_list =
            ValidatorStakeList::deserialize(&validator_stake_list_info.data.borrow())?;
        if !validator_stake_list.is_initialized {
            return Err(StakePoolError::InvalidState.into());
        }

        let validator_account =
            Self::get_validator_checked(program_id, stake_pool_info, stake_account_info)?;

        if !validator_stake_list.contains(&validator_account) {
            return Err(StakePoolError::ValidatorNotFound.into());
        }

        // Update Withdrawer and Staker authority to the provided authority
        for authority in &[
            stake::StakeAuthorize::Withdrawer,
            stake::StakeAuthorize::Staker,
        ] {
            Self::stake_authorize(
                stake_pool_info.key,
                stake_account_info.clone(),
                withdraw_info.clone(),
                Self::AUTHORITY_WITHDRAW,
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
            Self::AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            token_amount,
        )?;

        // Remove validator from the list and save
        validator_stake_list
            .validators
            .retain(|item| item.validator_account != validator_account);
        validator_stake_list.serialize(&mut validator_stake_list_info.data.borrow_mut())?;

        // Save amounts to the stake pool state
        stake_pool.pool_total -= token_amount;
        // Only update stake total if the last state update epoch is current
        stake_pool.stake_total -= stake_lamports;
        State::Init(stake_pool).serialize(&mut stake_pool_info.data.borrow_mut())?;

        Ok(())
    }

    /// Processes `UpdateListBalance` instruction.
    pub fn process_update_list_balance(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        // Account storing validator stake list
        let validator_stake_list_info = next_account_info(account_info_iter)?;
        // Clock sysvar account
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        // Validator stake accounts
        let validator_stake_accounts = account_info_iter.as_slice();

        // Read validator stake list account and check if it is valid
        let mut validator_stake_list =
            ValidatorStakeList::deserialize(&validator_stake_list_info.data.borrow())?;
        if !validator_stake_list.is_initialized {
            return Err(StakePoolError::InvalidState.into());
        }

        let validator_accounts: Vec<Option<Pubkey>> = validator_stake_accounts
            .iter()
            .map(|stake| Self::get_validator(stake).ok())
            .collect();

        let mut changes = false;
        // Do a brute iteration through the list, optimize if necessary
        for validator_stake_record in &mut validator_stake_list.validators {
            if validator_stake_record.last_update_epoch >= clock.epoch {
                continue;
            }
            for (validator_stake_account, validator_account) in validator_stake_accounts
                .iter()
                .zip(validator_accounts.iter())
            {
                if validator_stake_record.validator_account
                    != validator_account.ok_or(StakePoolError::WrongStakeState)?
                {
                    continue;
                }
                validator_stake_record.last_update_epoch = clock.epoch;
                validator_stake_record.balance = **validator_stake_account.lamports.borrow();
                changes = true;
            }
        }

        if changes {
            validator_stake_list.serialize(&mut validator_stake_list_info.data.borrow_mut())?;
        }

        Ok(())
    }

    /// Processes `UpdatePoolBalance` instruction.
    pub fn process_update_pool_balance(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        // Stake pool account
        let stake_pool_info = next_account_info(account_info_iter)?;
        // Account storing validator stake list
        let validator_stake_list_info = next_account_info(account_info_iter)?;
        // Clock sysvar account
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;

        // Get stake pool stake (and check if it is iniaialized)
        let mut stake_pool = State::deserialize(&stake_pool_info.data.borrow())?.stake_pool()?;

        // Check validator stake account list storage
        if *validator_stake_list_info.key != stake_pool.validator_stake_list {
            return Err(StakePoolError::InvalidValidatorStakeList.into());
        }

        // Read validator stake list account and check if it is valid
        let validator_stake_list =
            ValidatorStakeList::deserialize(&validator_stake_list_info.data.borrow())?;
        if !validator_stake_list.is_initialized {
            return Err(StakePoolError::InvalidState.into());
        }

        let mut total_balance: u64 = 0;
        for validator_stake_record in validator_stake_list.validators {
            if validator_stake_record.last_update_epoch < clock.epoch {
                return Err(StakePoolError::StakeListOutOfDate.into());
            }
            total_balance += validator_stake_record.balance;
        }

        stake_pool.stake_total = total_balance;
        stake_pool.last_update_epoch = clock.epoch;
        State::Init(stake_pool).serialize(&mut stake_pool_info.data.borrow_mut())?;

        Ok(())
    }

    /// Processes [Deposit](enum.Instruction.html).
    pub fn process_deposit(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        // Stake pool
        let stake_pool_info = next_account_info(account_info_iter)?;
        // Account storing validator stake list
        let validator_stake_list_info = next_account_info(account_info_iter)?;
        // Stake pool deposit authority
        let deposit_info = next_account_info(account_info_iter)?;
        // Stake pool withdraw authority
        let withdraw_info = next_account_info(account_info_iter)?;
        // Stake account to join the pool (withdraw should be set to stake pool deposit)
        let stake_info = next_account_info(account_info_iter)?;
        // Valdidator stake account to merge with
        let validator_stake_account_info = next_account_info(account_info_iter)?;
        // User account to receive pool tokens
        let dest_user_info = next_account_info(account_info_iter)?;
        // Account to receive pool fee tokens
        let owner_fee_info = next_account_info(account_info_iter)?;
        // Pool token mint account
        let pool_mint_info = next_account_info(account_info_iter)?;
        // Clock sysvar account
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        // Stake history sysvar account
        let stake_history_info = next_account_info(account_info_iter)?;
        // Pool token program id
        let token_program_info = next_account_info(account_info_iter)?;
        // Stake program id
        let stake_program_info = next_account_info(account_info_iter)?;

        // Check program ids
        if *stake_program_info.key != stake::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

        let mut stake_pool = State::deserialize(&stake_pool_info.data.borrow())?.stake_pool()?;

        // Check authority accounts
        stake_pool.check_authority_withdraw(withdraw_info.key, program_id, stake_pool_info.key)?;
        stake_pool.check_authority_deposit(deposit_info.key, program_id, stake_pool_info.key)?;

        if stake_pool.owner_fee_account != *owner_fee_info.key {
            return Err(StakePoolError::InvalidFeeAccount.into());
        }
        if stake_pool.token_program_id != *token_program_info.key {
            return Err(StakePoolError::InvalidProgramAddress.into());
        }

        // Check validator stake account list storage
        if *validator_stake_list_info.key != stake_pool.validator_stake_list {
            return Err(StakePoolError::InvalidValidatorStakeList.into());
        }

        // Check stake pool last update epoch
        if stake_pool.last_update_epoch < clock.epoch {
            return Err(StakePoolError::StakeListAndPoolOutOfDate.into());
        }

        // Read validator stake list account and check if it is valid
        let mut validator_stake_list =
            ValidatorStakeList::deserialize(&validator_stake_list_info.data.borrow())?;
        if !validator_stake_list.is_initialized {
            return Err(StakePoolError::InvalidState.into());
        }

        let validator_account =
            Self::get_validator_checked(program_id, stake_pool_info, validator_stake_account_info)?;

        let validator_list_item = validator_stake_list
            .find_mut(&validator_account)
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
            Self::AUTHORITY_DEPOSIT,
            stake_pool.deposit_bump_seed,
            withdraw_info.key,
            stake::StakeAuthorize::Withdrawer,
            clock_info.clone(),
            stake_program_info.clone(),
        )?;

        Self::stake_authorize(
            stake_pool_info.key,
            stake_info.clone(),
            deposit_info.clone(),
            Self::AUTHORITY_DEPOSIT,
            stake_pool.deposit_bump_seed,
            withdraw_info.key,
            stake::StakeAuthorize::Staker,
            clock_info.clone(),
            stake_program_info.clone(),
        )?;

        Self::stake_merge(
            stake_pool_info.key,
            stake_info.clone(),
            withdraw_info.clone(),
            Self::AUTHORITY_WITHDRAW,
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
            Self::AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            user_amount,
        )?;

        Self::token_mint_to(
            stake_pool_info.key,
            token_program_info.clone(),
            pool_mint_info.clone(),
            owner_fee_info.clone(),
            withdraw_info.clone(),
            Self::AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            fee_amount,
        )?;
        stake_pool.pool_total += pool_amount;
        stake_pool.stake_total += stake_lamports;
        State::Init(stake_pool).serialize(&mut stake_pool_info.data.borrow_mut())?;

        validator_list_item.balance = **validator_stake_account_info.lamports.borrow();
        validator_stake_list.serialize(&mut validator_stake_list_info.data.borrow_mut())?;

        Ok(())
    }

    /// Processes [Withdraw](enum.Instruction.html).
    pub fn process_withdraw(
        program_id: &Pubkey,
        stake_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        // Stake pool
        let stake_pool_info = next_account_info(account_info_iter)?;
        // Account storing validator stake list
        let validator_stake_list_info = next_account_info(account_info_iter)?;
        // Stake pool withdraw authority
        let withdraw_info = next_account_info(account_info_iter)?;
        // Stake account to split
        let stake_split_from = next_account_info(account_info_iter)?;
        // Unitialized stake account to receive withdrawal
        let stake_split_to = next_account_info(account_info_iter)?;
        // User account to set as a new withdraw authority
        let user_stake_authority = next_account_info(account_info_iter)?;
        // User account with pool tokens to burn from
        let burn_from_info = next_account_info(account_info_iter)?;
        // Pool token mint account
        let pool_mint_info = next_account_info(account_info_iter)?;
        // Clock sysvar account
        let clock_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;
        // Pool token program id
        let token_program_info = next_account_info(account_info_iter)?;
        // Stake program id
        let stake_program_info = next_account_info(account_info_iter)?;

        // Check program ids
        if *stake_program_info.key != stake::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

        let mut stake_pool = State::deserialize(&stake_pool_info.data.borrow())?.stake_pool()?;

        // Check authority account
        stake_pool.check_authority_withdraw(withdraw_info.key, program_id, stake_pool_info.key)?;

        if stake_pool.token_program_id != *token_program_info.key {
            return Err(StakePoolError::InvalidProgramAddress.into());
        }

        // Check validator stake account list storage
        if *validator_stake_list_info.key != stake_pool.validator_stake_list {
            return Err(StakePoolError::InvalidValidatorStakeList.into());
        }

        // Check stake pool last update epoch
        if stake_pool.last_update_epoch < clock.epoch {
            return Err(StakePoolError::StakeListAndPoolOutOfDate.into());
        }

        // Read validator stake list account and check if it is valid
        let mut validator_stake_list =
            ValidatorStakeList::deserialize(&validator_stake_list_info.data.borrow())?;
        if !validator_stake_list.is_initialized {
            return Err(StakePoolError::InvalidState.into());
        }

        let validator_account =
            Self::get_validator_checked(program_id, stake_pool_info, stake_split_from)?;

        let validator_list_item = validator_stake_list
            .find_mut(&validator_account)
            .ok_or(StakePoolError::ValidatorNotFound)?;

        let pool_amount = stake_pool
            .calc_pool_withdraw_amount(stake_amount)
            .ok_or(StakePoolError::CalculationFailure)?;

        Self::stake_split(
            stake_pool_info.key,
            stake_split_from.clone(),
            withdraw_info.clone(),
            Self::AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            stake_amount,
            stake_split_to.clone(),
            clock_info.clone(),
            stake_program_info.clone(),
        )?;

        Self::stake_authorize(
            stake_pool_info.key,
            stake_split_to.clone(),
            withdraw_info.clone(),
            Self::AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            user_stake_authority.key,
            stake::StakeAuthorize::Withdrawer,
            clock_info.clone(),
            stake_program_info.clone(),
        )?;

        Self::stake_authorize(
            stake_pool_info.key,
            stake_split_to.clone(),
            withdraw_info.clone(),
            Self::AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            user_stake_authority.key,
            stake::StakeAuthorize::Staker,
            clock_info.clone(),
            stake_program_info.clone(),
        )?;

        Self::token_burn(
            stake_pool_info.key,
            token_program_info.clone(),
            burn_from_info.clone(),
            pool_mint_info.clone(),
            withdraw_info.clone(),
            Self::AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            pool_amount,
        )?;

        stake_pool.pool_total -= pool_amount;
        stake_pool.stake_total -= stake_amount;
        State::Init(stake_pool).serialize(&mut stake_pool_info.data.borrow_mut())?;

        validator_list_item.balance = **stake_split_from.lamports.borrow();
        validator_stake_list.serialize(&mut validator_stake_list_info.data.borrow_mut())?;

        Ok(())
    }
    /// Processes [SetStakeAuthority](enum.Instruction.html).
    pub fn process_set_staking_auth(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let owner_info = next_account_info(account_info_iter)?;
        let withdraw_info = next_account_info(account_info_iter)?;
        let stake_info = next_account_info(account_info_iter)?;
        let staker_info = next_account_info(account_info_iter)?;
        // (Reserved)
        let reserved = next_account_info(account_info_iter)?;
        // Stake program id
        let stake_program_info = next_account_info(account_info_iter)?;

        // Check program ids
        if *stake_program_info.key != stake::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

        let stake_pool = State::deserialize(&stake_pool_info.data.borrow())?.stake_pool()?;

        // Check authority account
        stake_pool.check_authority_withdraw(withdraw_info.key, program_id, stake_pool_info.key)?;

        // Check owner validity and signature
        stake_pool.check_owner(owner_info)?;

        Self::stake_authorize(
            stake_pool_info.key,
            stake_info.clone(),
            withdraw_info.clone(),
            Self::AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            staker_info.key,
            stake::StakeAuthorize::Staker,
            reserved.clone(),
            stake_program_info.clone(),
        )?;
        Ok(())
    }

    /// Processes [SetOwner](enum.Instruction.html).
    pub fn process_set_owner(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let owner_info = next_account_info(account_info_iter)?;
        let new_owner_info = next_account_info(account_info_iter)?;
        let new_owner_fee_info = next_account_info(account_info_iter)?;

        let mut stake_pool = State::deserialize(&stake_pool_info.data.borrow())?.stake_pool()?;

        // Check owner validity and signature
        stake_pool.check_owner(owner_info)?;

        // Check for owner fee account to have proper mint assigned
        if stake_pool.pool_mint
            != spl_token::state::Account::unpack_from_slice(&new_owner_fee_info.data.borrow())?.mint
        {
            return Err(StakePoolError::WrongAccountMint.into());
        }

        stake_pool.owner = *new_owner_info.key;
        stake_pool.owner_fee_account = *new_owner_fee_info.key;
        State::Init(stake_pool).serialize(&mut stake_pool_info.data.borrow_mut())?;
        Ok(())
    }
    /// Processes [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = StakePoolInstruction::deserialize(input)?;
        match instruction {
            StakePoolInstruction::Initialize(init) => {
                msg!("Instruction: Init");
                Self::process_initialize(program_id, init, accounts)
            }
            StakePoolInstruction::CreateValidatorStakeAccount => {
                msg!("Instruction: CreateValidatorStakeAccount");
                Self::process_create_validator_stake_account(program_id, accounts)
            }
            StakePoolInstruction::AddValidatorStakeAccount => {
                msg!("Instruction: AddValidatorStakeAccount");
                Self::process_add_validator_stake_account(program_id, accounts)
            }
            StakePoolInstruction::RemoveValidatorStakeAccount => {
                msg!("Instruction: RemoveValidatorStakeAccount");
                Self::process_remove_validator_stake_account(program_id, accounts)
            }
            StakePoolInstruction::UpdateListBalance => {
                msg!("Instruction: UpdateListBalance");
                Self::process_update_list_balance(program_id, accounts)
            }
            StakePoolInstruction::UpdatePoolBalance => {
                msg!("Instruction: UpdatePoolBalance");
                Self::process_update_pool_balance(program_id, accounts)
            }
            StakePoolInstruction::Deposit => {
                msg!("Instruction: Deposit");
                Self::process_deposit(program_id, accounts)
            }
            StakePoolInstruction::Withdraw(amount) => {
                msg!("Instruction: Withdraw");
                Self::process_withdraw(program_id, amount, accounts)
            }
            StakePoolInstruction::SetStakingAuthority => {
                msg!("Instruction: SetStakingAuthority");
                Self::process_set_staking_auth(program_id, accounts)
            }
            StakePoolInstruction::SetOwner => {
                msg!("Instruction: SetOwner");
                Self::process_set_owner(program_id, accounts)
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
            StakePoolError::AlreadyInUse => msg!("Error: AlreadyInUse"),
            StakePoolError::InvalidProgramAddress => msg!("Error: InvalidProgramAddress"),
            StakePoolError::InvalidState => msg!("Error: InvalidState"),
            StakePoolError::CalculationFailure => msg!("Error: CalculationFailure"),
            StakePoolError::FeeTooHigh => msg!("Error: FeeTooHigh"),
            StakePoolError::WrongAccountMint => msg!("Error: WrongAccountMint"),
            StakePoolError::NonZeroBalance => msg!("Error: NonZeroBalance"),
            StakePoolError::WrongOwner => msg!("Error: WrongOwner"),
            StakePoolError::SignatureMissing => msg!("Error: SignatureMissing"),
            StakePoolError::InvalidValidatorStakeList => msg!("Error: InvalidValidatorStakeList"),
            StakePoolError::InvalidFeeAccount => msg!("Error: InvalidFeeAccount"),
            StakePoolError::WrongPoolMint => msg!("Error: WrongPoolMint"),
            StakePoolError::WrongStakeState => msg!("Error: WrongStakeState"),
            StakePoolError::ValidatorAlreadyAdded => msg!("Error: ValidatorAlreadyAdded"),
            StakePoolError::ValidatorNotFound => msg!("Error: ValidatorNotFound"),
            StakePoolError::InvalidStakeAccountAddress => msg!("Error: InvalidStakeAccountAddress"),
            StakePoolError::StakeListOutOfDate => msg!("Error: StakeListOutOfDate"),
            StakePoolError::StakeListAndPoolOutOfDate => msg!("Error: StakeListAndPoolOutOfDate"),
            StakePoolError::UnknownValidatorStakeAccount => {
                msg!("Error: UnknownValidatorStakeAccount")
            }
        }
    }
}
