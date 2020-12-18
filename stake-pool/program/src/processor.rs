//! Program state processor

use crate::state::ValidatorStakeList;
use crate::{
    error::Error,
    instruction::{InitArgs, StakePoolInstruction},
    stake,
    state::{StakePool, State},
};
use num_traits::FromPrimitive;
use solana_program::{
    account_info::next_account_info, account_info::AccountInfo, decode_error::DecodeError,
    entrypoint::ProgramResult, msg, program::invoke_signed, program_error::PrintProgramError,
    program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
};
use std::convert::TryFrom;

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
        my_info: &Pubkey,
        authority_type: &[u8],
        bump_seed: u8,
    ) -> Result<Pubkey, Error> {
        Pubkey::create_program_address(
            &[&my_info.to_bytes()[..32], authority_type, &[bump_seed]],
            program_id,
        )
        .or(Err(Error::InvalidProgramAddress))
    }
    /// Generates seed bump for stake pool authorities
    pub fn find_authority_bump_seed(
        program_id: &Pubkey,
        my_info: &Pubkey,
        authority_type: &[u8],
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[&my_info.to_bytes()[..32], authority_type], program_id)
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

    /// Processes an [Initialize](enum.Instruction.html).
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
        let token_program_info = next_account_info(account_info_iter)?;

        // Stake pool account should not be already initialized
        if State::Unallocated != State::deserialize(&stake_pool_info.data.borrow())? {
            return Err(Error::AlreadyInUse.into());
        }

        // Check if validator stake list storage is unitialized
        let mut validator_stake_list =
            ValidatorStakeList::deserialize(&validator_stake_list_info.data.borrow())?;
        if validator_stake_list.is_initialized {
            return Err(Error::AlreadyInUse.into());
        }
        validator_stake_list.is_initialized = true;
        validator_stake_list.validators.clear();

        // Numerator should be smaller than or equal to denominator (fee <= 1)
        if init.fee.numerator > init.fee.denominator {
            return Err(Error::FeeTooHigh.into());
        }

        // Check for owner fee account to have proper mint assigned
        if *pool_mint_info.key
            != spl_token::state::Account::unpack_from_slice(&owner_fee_info.data.borrow())?.mint
        {
            return Err(Error::WrongAccountMint.into());
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

        let stake_pool = State::Init(StakePool {
            owner: *owner_info.key,
            deposit_bump_seed,
            withdraw_bump_seed,
            validator_stake_list: *validator_stake_list_info.key,
            pool_mint: *pool_mint_info.key,
            owner_fee_account: *owner_fee_info.key,
            token_program_id: *token_program_info.key,
            stake_total: 0,
            pool_total: 0,
            fee: init.fee,
        });
        stake_pool.serialize(&mut stake_pool_info.data.borrow_mut())
    }

    /// Processes [Deposit](enum.Instruction.html).
    pub fn process_deposit(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        // Stake pool
        let stake_pool_info = next_account_info(account_info_iter)?;
        // Stake pool deposit authority
        let deposit_info = next_account_info(account_info_iter)?;
        // Stake pool withdraw authority
        let withdraw_info = next_account_info(account_info_iter)?;
        // Stake account to join the pool (withdraw should be set to stake pool deposit)
        let stake_info = next_account_info(account_info_iter)?;
        // User account to receive pool tokens
        let dest_user_info = next_account_info(account_info_iter)?;
        // Account to receive pool fee tokens
        let owner_fee_info = next_account_info(account_info_iter)?;
        // Pool token mint account
        let pool_mint_info = next_account_info(account_info_iter)?;
        // (Reserved)
        let reserved = next_account_info(account_info_iter)?;
        // Pool token program id
        let token_program_info = next_account_info(account_info_iter)?;
        // Stake program id
        let stake_program_info = next_account_info(account_info_iter)?;

        let mut stake_pool = State::deserialize(&stake_pool_info.data.borrow())?.stake_pool()?;

        if *withdraw_info.key
            != Self::authority_id(
                program_id,
                stake_pool_info.key,
                Self::AUTHORITY_WITHDRAW,
                stake_pool.withdraw_bump_seed,
            )?
        {
            return Err(Error::InvalidProgramAddress.into());
        }

        if *deposit_info.key
            != Self::authority_id(
                program_id,
                stake_pool_info.key,
                Self::AUTHORITY_DEPOSIT,
                stake_pool.deposit_bump_seed,
            )?
        {
            return Err(Error::InvalidProgramAddress.into());
        }

        if stake_pool.owner_fee_account != *owner_fee_info.key {
            return Err(Error::InvalidInput.into());
        }
        if stake_pool.token_program_id != *token_program_info.key {
            return Err(Error::InvalidInput.into());
        }

        let stake_lamports = **stake_info.lamports.borrow();
        let pool_amount = stake_pool
            .calc_pool_deposit_amount(stake_lamports)
            .ok_or(Error::CalculationFailure)?;

        let fee_amount = stake_pool
            .calc_fee_amount(pool_amount)
            .ok_or(Error::CalculationFailure)?;

        let user_amount = pool_amount
            .checked_sub(fee_amount)
            .ok_or(Error::CalculationFailure)?;

        Self::stake_authorize(
            stake_pool_info.key,
            stake_info.clone(),
            deposit_info.clone(),
            Self::AUTHORITY_DEPOSIT,
            stake_pool.deposit_bump_seed,
            withdraw_info.key,
            stake::StakeAuthorize::Withdrawer,
            reserved.clone(),
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
            reserved.clone(),
            stake_program_info.clone(),
        )?;

        let user_amount = <u64>::try_from(user_amount).or(Err(Error::CalculationFailure))?;
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

        let fee_amount = <u64>::try_from(fee_amount).or(Err(Error::CalculationFailure))?;
        Self::token_mint_to(
            stake_pool_info.key,
            token_program_info.clone(),
            pool_mint_info.clone(),
            owner_fee_info.clone(),
            withdraw_info.clone(),
            Self::AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            fee_amount as u64,
        )?;
        let pool_amount = <u64>::try_from(pool_amount).or(Err(Error::CalculationFailure))?;
        stake_pool.pool_total += pool_amount;
        stake_pool.stake_total += stake_lamports;
        State::Init(stake_pool).serialize(&mut stake_pool_info.data.borrow_mut())?;
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
        // (Reserved)
        let reserved = next_account_info(account_info_iter)?;
        // Pool token program id
        let token_program_info = next_account_info(account_info_iter)?;
        // Stake program id
        let stake_program_info = next_account_info(account_info_iter)?;

        let mut stake_pool = State::deserialize(&stake_pool_info.data.borrow())?.stake_pool()?;

        if *withdraw_info.key
            != Self::authority_id(
                program_id,
                stake_pool_info.key,
                Self::AUTHORITY_WITHDRAW,
                stake_pool.withdraw_bump_seed,
            )?
        {
            return Err(Error::InvalidProgramAddress.into());
        }
        if stake_pool.token_program_id != *token_program_info.key {
            return Err(Error::InvalidInput.into());
        }

        let pool_amount = stake_pool
            .calc_pool_withdraw_amount(stake_amount)
            .ok_or(Error::CalculationFailure)?;
        let pool_amount = <u64>::try_from(pool_amount).or(Err(Error::CalculationFailure))?;

        Self::stake_split(
            stake_pool_info.key,
            stake_split_from.clone(),
            withdraw_info.clone(),
            Self::AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            stake_amount,
            stake_split_to.clone(),
            reserved.clone(),
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
            reserved.clone(),
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
            reserved.clone(),
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
        Ok(())
    }
    /// Processes [Claim](enum.Instruction.html).
    pub fn process_claim(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        // Stake pool
        let stake_pool_info = next_account_info(account_info_iter)?;
        // Stake pool withdraw authority
        let withdraw_info = next_account_info(account_info_iter)?;
        // Stake account to claim
        let stake_to_claim = next_account_info(account_info_iter)?;
        // User account to set as a new withdraw authority
        let user_stake_authority = next_account_info(account_info_iter)?;
        // User account with pool tokens to burn from
        let burn_from_info = next_account_info(account_info_iter)?;
        // Pool token account
        let pool_mint_info = next_account_info(account_info_iter)?;
        // (Reserved)
        let reserved = next_account_info(account_info_iter)?;
        // Pool token program id
        let token_program_info = next_account_info(account_info_iter)?;
        // Stake program id
        let stake_program_info = next_account_info(account_info_iter)?;

        let mut stake_pool = State::deserialize(&stake_pool_info.data.borrow())?.stake_pool()?;

        if *withdraw_info.key
            != Self::authority_id(
                program_id,
                stake_pool_info.key,
                Self::AUTHORITY_WITHDRAW,
                stake_pool.withdraw_bump_seed,
            )?
        {
            return Err(Error::InvalidProgramAddress.into());
        }
        if stake_pool.token_program_id != *token_program_info.key {
            return Err(Error::InvalidInput.into());
        }

        let stake_amount = **stake_to_claim.lamports.borrow();
        let pool_amount = stake_pool
            .calc_pool_withdraw_amount(stake_amount)
            .ok_or(Error::CalculationFailure)?;
        let pool_amount = <u64>::try_from(pool_amount).or(Err(Error::CalculationFailure))?;

        Self::stake_authorize(
            stake_pool_info.key,
            stake_to_claim.clone(),
            withdraw_info.clone(),
            Self::AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            user_stake_authority.key,
            stake::StakeAuthorize::Withdrawer,
            reserved.clone(),
            stake_program_info.clone(),
        )?;

        Self::stake_authorize(
            stake_pool_info.key,
            stake_to_claim.clone(),
            withdraw_info.clone(),
            Self::AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            user_stake_authority.key,
            stake::StakeAuthorize::Staker,
            reserved.clone(),
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

        let stake_pool = State::deserialize(&stake_pool_info.data.borrow())?.stake_pool()?;

        if *owner_info.key != stake_pool.owner {
            return Err(Error::InvalidInput.into());
        }
        if !owner_info.is_signer {
            return Err(Error::InvalidInput.into());
        }

        if *withdraw_info.key
            != Self::authority_id(
                program_id,
                stake_pool_info.key,
                Self::AUTHORITY_WITHDRAW,
                stake_pool.withdraw_bump_seed,
            )?
        {
            return Err(Error::InvalidProgramAddress.into());
        }

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

        if *owner_info.key != stake_pool.owner {
            return Err(Error::InvalidInput.into());
        }
        if !owner_info.is_signer {
            return Err(Error::InvalidInput.into());
        }

        // Check for owner fee account to have proper mint assigned
        if stake_pool.pool_mint
            != spl_token::state::Account::unpack_from_slice(&new_owner_fee_info.data.borrow())?.mint
        {
            return Err(Error::WrongAccountMint.into());
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
            StakePoolInstruction::Deposit => {
                msg!("Instruction: Deposit");
                Self::process_deposit(program_id, accounts)
            }
            StakePoolInstruction::Withdraw(amount) => {
                msg!("Instruction: Withdraw");
                Self::process_withdraw(program_id, amount, accounts)
            }
            StakePoolInstruction::Claim => {
                msg!("Instruction: Claim");
                Self::process_claim(program_id, accounts)
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

impl PrintProgramError for Error {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            Error::AlreadyInUse => msg!("Error: AlreadyInUse"),
            Error::InvalidProgramAddress => msg!("Error: InvalidProgramAddress"),
            Error::InvalidOwner => msg!("Error: InvalidOwner"),
            Error::ExpectedToken => msg!("Error: ExpectedToken"),
            Error::ExpectedAccount => msg!("Error: ExpectedAccount"),
            Error::InvalidSupply => msg!("Error: InvalidSupply"),
            Error::InvalidDelegate => msg!("Error: InvalidDelegate"),
            Error::InvalidState => msg!("Error: InvalidState"),
            Error::InvalidInput => msg!("Error: InvalidInput"),
            Error::InvalidOutput => msg!("Error: InvalidOutput"),
            Error::CalculationFailure => msg!("Error: CalculationFailure"),
            Error::FeeTooHigh => msg!("Error: FeeTooHigh"),
            Error::WrongAccountMint => msg!("Error: WrongAccountMint"),
        }
    }
}
