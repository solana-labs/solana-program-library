//! Program state processor

#![cfg(feature = "program")]

use crate::{
    error::Error,
    instruction::{InitArgs, StakePoolInstruction},
    stake,
    state::{StakePool, State},
};
use num_traits::FromPrimitive;
#[cfg(not(target_arch = "bpf"))]
use solana_sdk::instruction::Instruction;
#[cfg(target_arch = "bpf")]
use solana_sdk::program::invoke_signed;
use solana_sdk::{
    account_info::next_account_info, account_info::AccountInfo, decode_error::DecodeError,
    entrypoint::ProgramResult, info, program_error::PrintProgramError, program_error::ProgramError,
    pubkey::Pubkey,
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
    ) -> u8 {
        let (_pubkey, bump_seed) =
            Pubkey::find_program_address(&[&my_info.to_bytes()[..32], authority_type], program_id);
        bump_seed
    }

    /// Issue a stake_split instruction.
    pub fn stake_split<'a>(
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

        let ix = stake::split_only(stake_account.key, authority.key, amount, split_stake.key);

        invoke_signed(&ix, &[stake_account, authority, split_stake], signers)
    }

    /// Issue a stake_set_owner instruction.
    pub fn stake_authorize<'a>(
        stake_pool: &Pubkey,
        stake_account: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        authority_type: &[u8],
        bump_seed: u8,
        new_staker: &Pubkey,
        staker_auth: stake::StakeAuthorize,
    ) -> Result<(), ProgramError> {
        let me_bytes = stake_pool.to_bytes();
        let authority_signature_seeds = [&me_bytes[..32], authority_type, &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];

        let ix = stake::authorize(stake_account.key, authority.key, new_staker, staker_auth);

        invoke_signed(&ix, &[stake_account, authority], signers)
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
        let pool_mint_info = next_account_info(account_info_iter)?;
        let owner_fee_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        // Stake pool account should not be already initialized
        if State::Unallocated != State::deserialize(&stake_pool_info.data.borrow())? {
            return Err(Error::AlreadyInUse.into());
        }

        // Numerator should be smaller than or equal to denominator (fee <= 1)
        if init.fee.numerator > init.fee.denominator {
            return Err(Error::FeeTooHigh.into());
        }

        let stake_pool = State::Init(StakePool {
            owner: *owner_info.key,
            deposit_bump_seed: Self::find_authority_bump_seed(
                program_id,
                stake_pool_info.key,
                Self::AUTHORITY_DEPOSIT,
            ),
            withdraw_bump_seed: Self::find_authority_bump_seed(
                program_id,
                stake_pool_info.key,
                Self::AUTHORITY_WITHDRAW,
            ),
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
        // Pool token program id
        let token_program_info = next_account_info(account_info_iter)?;

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
        )?;

        let user_amount = <u64>::try_from(user_amount).or(Err(Error::CalculationFailure))?;
        Self::token_mint_to(
            stake_pool_info.key,
            token_program_info.clone(),
            pool_mint_info.clone(),
            dest_user_info.clone(),
            withdraw_info.clone(),
            Self::AUTHORITY_DEPOSIT,
            stake_pool.deposit_bump_seed,
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
        // Pool token program id
        let token_program_info = next_account_info(account_info_iter)?;

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
        )?;

        Self::stake_authorize(
            stake_pool_info.key,
            stake_split_to.clone(),
            withdraw_info.clone(),
            Self::AUTHORITY_WITHDRAW,
            stake_pool.withdraw_bump_seed,
            user_stake_authority.key,
            stake::StakeAuthorize::Withdrawer,
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
        // Pool token program id
        let token_program_info = next_account_info(account_info_iter)?;

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
                info!("Instruction: Init");
                Self::process_initialize(program_id, init, accounts)
            }
            StakePoolInstruction::Deposit => {
                info!("Instruction: Deposit");
                Self::process_deposit(program_id, accounts)
            }
            StakePoolInstruction::Withdraw(amount) => {
                info!("Instruction: Withdraw");
                Self::process_withdraw(program_id, amount, accounts)
            }
            StakePoolInstruction::Claim => {
                info!("Instruction: Claim");
                Self::process_claim(program_id, accounts)
            }
            StakePoolInstruction::SetStakingAuthority => {
                info!("Instruction: SetStakingAuthority");
                Self::process_set_staking_auth(program_id, accounts)
            }
            StakePoolInstruction::SetOwner => {
                info!("Instruction: SetOwner");
                Self::process_set_owner(program_id, accounts)
            }
        }
    }
}

// Test program id for the stake-pool program.
#[cfg(not(target_arch = "bpf"))]
const STAKE_POOL_PROGRAM_ID: Pubkey = Pubkey::new_from_array([2u8; 32]);

// Test program id for the token program.
#[cfg(not(target_arch = "bpf"))]
const TOKEN_PROGRAM_ID: Pubkey = Pubkey::new_from_array([1u8; 32]);

/// Routes invokes to the token program, used for testing.
/// TODO add routing to stake program for testing
#[cfg(not(target_arch = "bpf"))]
pub fn invoke_signed<'a>(
    instruction: &Instruction,
    account_infos: &[AccountInfo<'a>],
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    // mimic check for token program in accounts
    if !account_infos.iter().any(|x| *x.key == TOKEN_PROGRAM_ID) {
        return Err(ProgramError::InvalidAccountData);
    }

    let mut new_account_infos = vec![];
    for meta in instruction.accounts.iter() {
        for account_info in account_infos.iter() {
            if meta.pubkey == *account_info.key {
                let mut new_account_info = account_info.clone();
                for seeds in signers_seeds.iter() {
                    let signer =
                        Pubkey::create_program_address(seeds, &STAKE_POOL_PROGRAM_ID).unwrap();
                    if *account_info.key == signer {
                        new_account_info.is_signer = true;
                    }
                }
                new_account_infos.push(new_account_info);
            }
        }
    }
    spl_token::processor::Processor::process(
        &instruction.program_id,
        &new_account_infos,
        &instruction.data,
    )
}

impl PrintProgramError for Error {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            Error::AlreadyInUse => info!("Error: AlreadyInUse"),
            Error::InvalidProgramAddress => info!("Error: InvalidProgramAddress"),
            Error::InvalidOwner => info!("Error: InvalidOwner"),
            Error::ExpectedToken => info!("Error: ExpectedToken"),
            Error::ExpectedAccount => info!("Error: ExpectedAccount"),
            Error::InvalidSupply => info!("Error: InvalidSupply"),
            Error::InvalidDelegate => info!("Error: InvalidDelegate"),
            Error::InvalidState => info!("Error: InvalidState"),
            Error::InvalidInput => info!("Error: InvalidInput"),
            Error::InvalidOutput => info!("Error: InvalidOutput"),
            Error::CalculationFailure => info!("Error: CalculationFailure"),
            Error::FeeTooHigh => info!("Error: FeeTooHigh"),
        }
    }
}

// Pull in syscall stubs when building for non-BPF targets
#[cfg(not(target_arch = "bpf"))]
solana_sdk::program_stubs!();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::initialize;
    use crate::instruction::Fee;
    use crate::instruction::InitArgs;
    use core::mem::size_of;
    use solana_sdk::{
        account::Account, account_info::create_is_signer_account_infos, instruction::Instruction,
        program_pack::Pack, rent::Rent, sysvar::rent,
    };
    use spl_token::{
        instruction::{initialize_account, initialize_mint},
        processor::Processor as SplProcessor,
        state::{Account as SplAccount, Mint as SplMint},
    };

    fn pubkey_rand() -> Pubkey {
        Pubkey::new(&rand::random::<[u8; 32]>())
    }

    fn do_process_instruction(
        instruction: Instruction,
        accounts: Vec<&mut Account>,
    ) -> ProgramResult {
        let mut meta = instruction
            .accounts
            .iter()
            .zip(accounts)
            .map(|(account_meta, account)| (&account_meta.pubkey, account_meta.is_signer, account))
            .collect::<Vec<_>>();

        let account_infos = create_is_signer_account_infos(&mut meta);
        if instruction.program_id == STAKE_POOL_PROGRAM_ID {
            Processor::process(&instruction.program_id, &account_infos, &instruction.data)
        } else {
            SplProcessor::process(&instruction.program_id, &account_infos, &instruction.data)
        }
    }

    fn account_minimum_balance() -> u64 {
        Rent::default().minimum_balance(SplAccount::get_packed_len())
    }

    fn mint_minimum_balance() -> u64 {
        Rent::default().minimum_balance(SplMint::get_packed_len())
    }

    fn mint_token(
        program_id: &Pubkey,
        mint_key: &Pubkey,
        mut mint_account: &mut Account,
        authority_key: &Pubkey,
        amount: u64,
    ) -> (Pubkey, Account) {
        let account_key = pubkey_rand();
        let mut account_account = Account::new(
            account_minimum_balance(),
            SplAccount::get_packed_len(),
            &program_id,
        );
        let mut authority_account = Account::default();
        let mut rent_sysvar_account = rent::create_account(1, &Rent::free());

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, authority_key).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut authority_account,
                &mut rent_sysvar_account,
            ],
        )
        .unwrap();

        do_process_instruction(
            spl_token::instruction::mint_to(
                &program_id,
                &mint_key,
                &account_key,
                &authority_key,
                &[],
                amount,
            )
            .unwrap(),
            vec![
                &mut mint_account,
                &mut account_account,
                &mut authority_account,
            ],
        )
        .unwrap();

        (account_key, account_account)
    }

    fn create_mint(program_id: &Pubkey, authority_key: &Pubkey) -> (Pubkey, Account) {
        let mint_key = pubkey_rand();
        let mut mint_account = Account::new(
            mint_minimum_balance(),
            SplMint::get_packed_len(),
            &program_id,
        );
        let mut rent_sysvar_account = rent::create_account(1, &Rent::free());

        // create token mint
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, authority_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar_account],
        )
        .unwrap();

        (mint_key, mint_account)
    }

    #[test]
    fn test_initialize() {
        let stake_pool_key = pubkey_rand();
        let mut stake_pool_account = Account::new(0, size_of::<State>(), &STAKE_POOL_PROGRAM_ID);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let authority_key = pubkey_rand();

        let (pool_mint_key, mut pool_mint_account) = create_mint(&TOKEN_PROGRAM_ID, &authority_key);
        let (pool_token_key, mut pool_token_account) = mint_token(
            &TOKEN_PROGRAM_ID,
            &pool_mint_key,
            &mut pool_mint_account,
            &authority_key,
            0,
        );

        // StakePool Init
        do_process_instruction(
            initialize(
                &STAKE_POOL_PROGRAM_ID,
                &stake_pool_key,
                &owner_key,
                &pool_mint_key,
                &pool_token_key,
                &TOKEN_PROGRAM_ID,
                InitArgs {
                    fee: Fee {
                        denominator: 10,
                        numerator: 2,
                    },
                },
            )
            .unwrap(),
            vec![
                &mut stake_pool_account,
                &mut owner_account,
                &mut pool_mint_account,
                &mut pool_token_account,
                &mut Account::default(),
            ],
        )
        .unwrap();
    }
}
