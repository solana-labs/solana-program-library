//! Program state processor

use crate::{
    error::Error,
    instruction::{InitArgs, StakePoolInstruction},
    stake,
    state::{StakePool, State},
};
use num_traits::FromPrimitive;
use solana_program::{
    account_info::next_account_info, account_info::AccountInfo, decode_error::DecodeError,
    entrypoint::ProgramResult, info, program::invoke_signed, program_error::PrintProgramError,
    program_error::ProgramError, pubkey::Pubkey,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::{deposit, initialize, Fee, InitArgs};
    use solana_program::{
        instruction::Instruction, native_token::sol_to_lamports, program_pack::Pack, program_stubs,
        rent::Rent,
    };
    use solana_sdk::account::{create_account, create_is_signer_account_infos, Account};
    use spl_token::{
        instruction::{initialize_account, initialize_mint},
        processor::Processor as TokenProcessor,
        state::{Account as SplAccount, Mint as SplMint},
    };

    /// Test program id for the stake-pool program.
    const STAKE_POOL_PROGRAM_ID: Pubkey = Pubkey::new_from_array([2u8; 32]);

    /// Test program id for the token program.
    const TOKEN_PROGRAM_ID: Pubkey = Pubkey::new_from_array([1u8; 32]);

    /// Actual stake account program id, used for tests
    fn stake_program_id() -> Pubkey {
        "Stake11111111111111111111111111111111111111"
            .parse::<Pubkey>()
            .unwrap()
    }

    struct TestSyscallStubs {}
    impl program_stubs::SyscallStubs for TestSyscallStubs {
        fn sol_invoke_signed(
            &self,
            instruction: &Instruction,
            account_infos: &[AccountInfo],
            signers_seeds: &[&[&[u8]]],
        ) -> ProgramResult {
            info!("TestSyscallStubs::sol_invoke_signed()");

            let mut new_account_infos = vec![];
            for meta in instruction.accounts.iter() {
                for account_info in account_infos.iter() {
                    if meta.pubkey == *account_info.key {
                        let mut new_account_info = account_info.clone();
                        for seeds in signers_seeds.iter() {
                            let signer =
                                Pubkey::create_program_address(seeds, &STAKE_POOL_PROGRAM_ID)
                                    .unwrap();
                            if *account_info.key == signer {
                                new_account_info.is_signer = true;
                            }
                        }
                        new_account_infos.push(new_account_info);
                    }
                }
            }

            match instruction.program_id {
                TOKEN_PROGRAM_ID => invoke_token(&new_account_infos, &instruction.data),
                pubkey => {
                    if pubkey == stake_program_id() {
                        invoke_stake(&new_account_infos, &instruction.data)
                    } else {
                        Err(ProgramError::IncorrectProgramId)
                    }
                }
            }
        }
    }

    /// Mocks token instruction invocation
    pub fn invoke_token<'a>(account_infos: &[AccountInfo<'a>], input: &[u8]) -> ProgramResult {
        spl_token::processor::Processor::process(&TOKEN_PROGRAM_ID, &account_infos, &input)
    }

    /// Mocks stake account instruction invocation
    pub fn invoke_stake<'a>(_account_infos: &[AccountInfo<'a>], _input: &[u8]) -> ProgramResult {
        // For now always return ok
        Ok(())
    }

    fn test_syscall_stubs() {
        use std::sync::Once;
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            program_stubs::set_syscall_stubs(Box::new(TestSyscallStubs {}));
        });
    }

    struct StakePoolInfo {
        pub pool_key: Pubkey,
        pub pool_account: Account,
        pub deposit_bump_seed: u8,
        pub withdraw_bump_seed: u8,
        pub deposit_authority_key: Pubkey,
        pub withdraw_authority_key: Pubkey,
        pub fee: Fee,
        pub owner_key: Pubkey,
        pub owner_fee_key: Pubkey,
        pub owner_fee_account: Account,
        pub mint_key: Pubkey,
        pub mint_account: Account,
    }

    fn do_process_instruction(
        instruction: Instruction,
        accounts: Vec<&mut Account>,
    ) -> ProgramResult {
        test_syscall_stubs();

        // approximate the logic in the actual runtime which runs the instruction
        // and only updates accounts if the instruction is successful
        let mut account_clones = accounts.iter().map(|x| (*x).clone()).collect::<Vec<_>>();
        let mut meta = instruction
            .accounts
            .iter()
            .zip(account_clones.iter_mut())
            .map(|(account_meta, account)| (&account_meta.pubkey, account_meta.is_signer, account))
            .collect::<Vec<_>>();
        let mut account_infos = create_is_signer_account_infos(&mut meta);
        let res = if instruction.program_id == STAKE_POOL_PROGRAM_ID {
            Processor::process(&instruction.program_id, &account_infos, &instruction.data)
        } else {
            TokenProcessor::process(&instruction.program_id, &account_infos, &instruction.data)
        };

        if res.is_ok() {
            let mut account_metas = instruction
                .accounts
                .iter()
                .zip(accounts)
                .map(|(account_meta, account)| (&account_meta.pubkey, account))
                .collect::<Vec<_>>();
            for account_info in account_infos.iter_mut() {
                for account_meta in account_metas.iter_mut() {
                    if account_info.key == account_meta.0 {
                        let account = &mut account_meta.1;
                        account.owner = *account_info.owner;
                        account.lamports = **account_info.lamports.borrow();
                        account.data = account_info.data.borrow().to_vec();
                    }
                }
            }
        }
        res
    }

    fn account_minimum_balance() -> u64 {
        Rent::default().minimum_balance(SplAccount::get_packed_len())
    }

    fn mint_minimum_balance() -> u64 {
        Rent::default().minimum_balance(SplMint::get_packed_len())
    }

    fn create_token_account(
        program_id: &Pubkey,
        mint_key: &Pubkey,
        mint_account: &mut Account,
    ) -> (Pubkey, Account) {
        let account_key = Pubkey::new_unique();
        let mut account_account = Account::new(
            account_minimum_balance(),
            SplAccount::get_packed_len(),
            &program_id,
        );
        let mut rent_sysvar_account = create_account(&Rent::free(), 1);
        let owner_key = Pubkey::new_unique();
        let mut owner_account = Account::default();

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account_account,
                mint_account,
                &mut owner_account,
                &mut rent_sysvar_account,
            ],
        )
        .unwrap();

        (account_key, account_account)
    }

    fn _mint_token(
        program_id: &Pubkey,
        mint_key: &Pubkey,
        mut mint_account: &mut Account,
        authority_key: &Pubkey,
        amount: u64,
    ) -> (Pubkey, Account) {
        let (account_key, mut account_account) =
            create_token_account(program_id, mint_key, mint_account);
        let mut authority_account = Account::default();

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
        let mint_key = Pubkey::new_unique();
        let mut mint_account = Account::new(
            mint_minimum_balance(),
            SplMint::get_packed_len(),
            &program_id,
        );
        let mut rent_sysvar_account = create_account(&Rent::free(), 1);

        // create token mint
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, authority_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar_account],
        )
        .unwrap();

        (mint_key, mint_account)
    }

    fn create_stake_pool(fee: Fee) -> StakePoolInfo {
        let stake_pool_key = Pubkey::new_unique();
        let owner_key = Pubkey::new_unique();

        let mut stake_pool_account = Account::new(0, State::LEN, &STAKE_POOL_PROGRAM_ID);
        let mut owner_account = Account::default();

        // Calculate authority addresses
        let (deposit_authority_key, deposit_bump_seed) = Pubkey::find_program_address(
            &[&stake_pool_key.to_bytes()[..32], b"deposit"],
            &STAKE_POOL_PROGRAM_ID,
        );
        let (withdraw_authority_key, withdraw_bump_seed) = Pubkey::find_program_address(
            &[&stake_pool_key.to_bytes()[..32], b"withdraw"],
            &STAKE_POOL_PROGRAM_ID,
        );

        let (mint_key, mut mint_account) = create_mint(&TOKEN_PROGRAM_ID, &withdraw_authority_key);
        let (owner_fee_key, mut owner_fee_account) =
            create_token_account(&TOKEN_PROGRAM_ID, &mint_key, &mut mint_account);

        // StakePool Init
        let _result = do_process_instruction(
            initialize(
                &STAKE_POOL_PROGRAM_ID,
                &stake_pool_key,
                &owner_key,
                &mint_key,
                &owner_fee_key,
                &TOKEN_PROGRAM_ID,
                InitArgs { fee },
            )
            .unwrap(),
            vec![
                &mut stake_pool_account,
                &mut owner_account,
                &mut mint_account,
                &mut owner_fee_account,
                &mut Account::default(),
            ],
        )
        .expect("Error on stake pool initialize");

        StakePoolInfo {
            pool_key: stake_pool_key,
            pool_account: stake_pool_account,
            deposit_bump_seed,
            withdraw_bump_seed,
            deposit_authority_key,
            withdraw_authority_key,
            fee,
            owner_key,
            owner_fee_key,
            owner_fee_account,
            mint_key,
            mint_account,
        }
    }

    #[test]
    fn test_initialize() {
        let fee = Fee {
            denominator: 10,
            numerator: 2,
        };
        let pool_info = create_stake_pool(fee);
        // Read account data
        let state = State::deserialize(&pool_info.pool_account.data).unwrap();
        match state {
            State::Unallocated => panic!("Stake pool state is not initialized after init"),
            State::Init(stake_pool) => {
                assert_eq!(stake_pool.deposit_bump_seed, pool_info.deposit_bump_seed);
                assert_eq!(stake_pool.withdraw_bump_seed, pool_info.withdraw_bump_seed);
                assert_eq!(stake_pool.fee.numerator, pool_info.fee.numerator);
                assert_eq!(stake_pool.fee.denominator, pool_info.fee.denominator);

                assert_eq!(stake_pool.owner, pool_info.owner_key);
                assert_eq!(stake_pool.pool_mint, pool_info.mint_key);
                assert_eq!(stake_pool.owner_fee_account, pool_info.owner_fee_key);
                assert_eq!(stake_pool.token_program_id, TOKEN_PROGRAM_ID);

                assert_eq!(stake_pool.stake_total, 0);
                assert_eq!(stake_pool.pool_total, 0);
            }
        }
    }

    #[test]
    fn test_deposit() {
        let fee = Fee {
            denominator: 100,
            numerator: 2,
        };
        let stake_balance: u64 = sol_to_lamports(10.0);
        let user_token_balance: u64 = sol_to_lamports(9.8);
        let fee_token_balance: u64 = sol_to_lamports(0.2);
        assert_eq!(stake_balance, user_token_balance + fee_token_balance);

        // Create stake account
        let mut pool_info = create_stake_pool(fee);

        let stake_account_key = Pubkey::new_unique();
        let mut stake_account_account = Account::new(stake_balance, 100, &stake_program_id());
        // TODO: Set stake account Withdrawer authority to pool_info.deposit_authority_key

        // Create account to receive minted tokens
        let (token_receiver_key, mut token_receiver_account) = create_token_account(
            &TOKEN_PROGRAM_ID,
            &pool_info.mint_key,
            &mut pool_info.mint_account,
        );

        // Call deposit
        let _result = do_process_instruction(
            deposit(
                &STAKE_POOL_PROGRAM_ID,
                &pool_info.pool_key,
                &pool_info.deposit_authority_key,
                &pool_info.withdraw_authority_key,
                &stake_account_key,
                &token_receiver_key,
                &pool_info.owner_fee_key,
                &pool_info.mint_key,
                &TOKEN_PROGRAM_ID,
                &stake_program_id(),
            )
            .unwrap(),
            vec![
                &mut pool_info.pool_account,
                &mut Account::default(),
                &mut Account::default(),
                &mut stake_account_account,
                &mut token_receiver_account,
                &mut pool_info.owner_fee_account,
                &mut pool_info.mint_account,
                &mut Account::default(),
                &mut Account::default(),
                &mut Account::default(),
            ],
        )
        .expect("Error on stake pool deposit");

        // Test stake pool balance
        let state = State::deserialize(&pool_info.pool_account.data).unwrap();
        match state {
            State::Unallocated => panic!("Stake pool state is not initialized after deposit"),
            State::Init(stake_pool) => {
                assert_eq!(stake_pool.stake_total, stake_balance);
                assert_eq!(stake_pool.pool_total, stake_balance);
            }
        }

        // Test token balances
        let user_token_state = SplAccount::unpack_from_slice(&token_receiver_account.data)
            .expect("User token account is not initialized after deposit");
        assert_eq!(user_token_state.amount, user_token_balance);
        let fee_token_state = SplAccount::unpack_from_slice(&pool_info.owner_fee_account.data)
            .expect("Fee token account is not initialized after deposit");
        assert_eq!(fee_token_state.amount, fee_token_balance);

        // Test mint total issued tokens
        let mint_state = SplMint::unpack_from_slice(&pool_info.mint_account.data)
            .expect("Mint account is not initialized after deposit");
        assert_eq!(mint_state.supply, stake_balance);

        // TODO: Test stake account Withdrawer authority
    }
}
