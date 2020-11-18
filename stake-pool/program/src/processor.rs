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
        let stake_pool = State::Init(StakePool {
            owner: *owner_info.key,
            deposit_bump_seed,
            withdraw_bump_seed,
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
            Error::WrongAccountMint => info!("Error: WrongAccountMint"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::*;
    use solana_program::{
        instruction::AccountMeta, instruction::Instruction, native_token::sol_to_lamports,
        program_pack::Pack, program_stubs, rent::Rent, sysvar,
    };
    use solana_sdk::account::{create_account, create_is_signer_account_infos, Account};
    use spl_token::{
        error::TokenError,
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

    const STAKE_ACCOUNT_LEN: usize = 100;

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
        pub owner_account: Account,
        pub owner_fee_key: Pubkey,
        pub owner_fee_account: Account,
        pub mint_key: Pubkey,
        pub mint_account: Account,
    }

    struct DepositInfo {
        result: ProgramResult,
        stake_account_key: Pubkey,
        stake_account_account: Account,
    }

    struct Deposit {
        stake_balance: u64,
        tokens_to_issue: u64,
        user_token_balance: u64,
        fee_token_balance: u64,
        pool_info: StakePoolInfo,
        pool_token_receiver: TokenInfo,
    }

    struct WithdrawInfo {
        result: ProgramResult,
    }

    struct Withdraw {
        stake_balance: u64,
        tokens_to_issue: u64,
        withdraw_amount: u64,
        tokens_to_burn: u64,
        pool_info: StakePoolInfo,
        user_withdrawer_key: Pubkey,
        pool_token_receiver: TokenInfo,
        deposit_info: DepositInfo,
    }

    struct ClaimInfo {
        result: ProgramResult,
    }

    struct Claim {
        tokens_to_issue: u64,
        pool_info: StakePoolInfo,
        user_withdrawer_key: Pubkey,
        pool_token_receiver: TokenInfo,
        deposit_info: DepositInfo,
        allow_burn_to: Pubkey,
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

    struct TokenInfo {
        key: Pubkey,
        account: Account,
        owner: Pubkey,
    }

    fn create_token_account(
        program_id: &Pubkey,
        mint_key: &Pubkey,
        mint_account: &mut Account,
    ) -> TokenInfo {
        let mut token = TokenInfo {
            key: Pubkey::new_unique(),
            account: Account::new(
                account_minimum_balance(),
                SplAccount::get_packed_len(),
                &program_id,
            ),
            owner: Pubkey::new_unique(),
        };
        let mut rent_sysvar_account = create_account(&Rent::free(), 1);
        let mut owner_account = Account::default();

        // create account
        do_process_instruction(
            initialize_account(&program_id, &token.key, &mint_key, &token.owner).unwrap(),
            vec![
                &mut token.account,
                mint_account,
                &mut owner_account,
                &mut rent_sysvar_account,
            ],
        )
        .unwrap();

        token
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

    fn approve_token(
        program_id: &Pubkey,
        token_account_pubkey: &Pubkey,
        mut token_account_account: &mut Account,
        delegate_pubkey: &Pubkey,
        owner_pubkey: &Pubkey,
        amount: u64,
    ) {
        do_process_instruction(
            spl_token::instruction::approve(
                &program_id,
                token_account_pubkey,
                delegate_pubkey,
                owner_pubkey,
                &[],
                amount,
            )
            .unwrap(),
            vec![
                &mut token_account_account,
                &mut Account::default(),
                &mut Account::default(),
            ],
        )
        .unwrap();
    }

    const FEE_DEFAULT: Fee = Fee {
        denominator: 100,
        numerator: 5,
    };

    fn create_stake_pool_default() -> StakePoolInfo {
        create_stake_pool(FEE_DEFAULT)
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
        let mut token = create_token_account(&TOKEN_PROGRAM_ID, &mint_key, &mut mint_account);

        // StakePool Init
        let _result = do_process_instruction(
            initialize(
                &STAKE_POOL_PROGRAM_ID,
                &stake_pool_key,
                &owner_key,
                &mint_key,
                &token.key,
                &TOKEN_PROGRAM_ID,
                InitArgs { fee },
            )
            .unwrap(),
            vec![
                &mut stake_pool_account,
                &mut owner_account,
                &mut mint_account,
                &mut token.account,
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
            owner_account,
            owner_fee_key: token.key,
            owner_fee_account: token.account,
            mint_key,
            mint_account,
        }
    }

    fn do_deposit(
        pool_info: &mut StakePoolInfo,
        stake_balance: u64,
        token: &mut TokenInfo,
    ) -> DepositInfo {
        let stake_account_key = Pubkey::new_unique();
        let mut stake_account_account =
            Account::new(stake_balance, STAKE_ACCOUNT_LEN, &stake_program_id());
        // TODO: Set stake account Withdrawer authority to pool_info.deposit_authority_key

        // Call deposit
        let result = do_process_instruction(
            deposit(
                &STAKE_POOL_PROGRAM_ID,
                &pool_info.pool_key,
                &pool_info.deposit_authority_key,
                &pool_info.withdraw_authority_key,
                &stake_account_key,
                &token.key,
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
                &mut token.account,
                &mut pool_info.owner_fee_account,
                &mut pool_info.mint_account,
                &mut Account::default(),
                &mut Account::default(),
                &mut Account::default(),
            ],
        );

        DepositInfo {
            result,
            stake_account_key,
            stake_account_account,
        }
    }

    fn do_withdraw(test_data: &mut Withdraw) -> WithdrawInfo {
        approve_token(
            &TOKEN_PROGRAM_ID,
            &test_data.pool_token_receiver.key,
            &mut test_data.pool_token_receiver.account,
            &test_data.pool_info.withdraw_authority_key,
            &test_data.pool_token_receiver.owner,
            test_data.tokens_to_burn,
        );

        let stake_to_receive_key = Pubkey::new_unique();
        let mut stake_to_receive_account = Account::new(
            test_data.stake_balance,
            STAKE_ACCOUNT_LEN,
            &stake_program_id(),
        );

        let result = do_process_instruction(
            withdraw(
                &STAKE_POOL_PROGRAM_ID,
                &test_data.pool_info.pool_key,
                &test_data.pool_info.withdraw_authority_key,
                &test_data.deposit_info.stake_account_key,
                &stake_to_receive_key,
                &test_data.user_withdrawer_key,
                &test_data.pool_token_receiver.key,
                &test_data.pool_info.mint_key,
                &TOKEN_PROGRAM_ID,
                &stake_program_id(),
                test_data.withdraw_amount,
            )
            .unwrap(),
            vec![
                &mut test_data.pool_info.pool_account,
                &mut Account::default(),
                &mut test_data.deposit_info.stake_account_account,
                &mut stake_to_receive_account,
                &mut Account::default(),
                &mut test_data.pool_token_receiver.account,
                &mut test_data.pool_info.mint_account,
                &mut Account::default(),
                &mut Account::default(),
                &mut Account::default(),
            ],
        );

        WithdrawInfo { result }
    }

    fn do_claim(test_data: &mut Claim) -> ClaimInfo {
        approve_token(
            &TOKEN_PROGRAM_ID,
            &test_data.pool_token_receiver.key,
            &mut test_data.pool_token_receiver.account,
            &test_data.allow_burn_to,
            &test_data.pool_token_receiver.owner,
            test_data.tokens_to_issue,
        );

        let result = do_process_instruction(
            claim(
                &STAKE_POOL_PROGRAM_ID,
                &test_data.pool_info.pool_key,
                &test_data.pool_info.withdraw_authority_key,
                &test_data.deposit_info.stake_account_key,
                &test_data.user_withdrawer_key,
                &test_data.pool_token_receiver.key,
                &test_data.pool_info.mint_key,
                &TOKEN_PROGRAM_ID,
                &stake_program_id(),
            )
            .unwrap(),
            vec![
                &mut test_data.pool_info.pool_account,
                &mut Account::default(),
                &mut test_data.deposit_info.stake_account_account,
                &mut Account::default(),
                &mut test_data.pool_token_receiver.account,
                &mut test_data.pool_info.mint_account,
                &mut Account::default(),
                &mut Account::default(),
                &mut Account::default(),
            ],
        );
        ClaimInfo { result }
    }

    fn set_staking_authority_without_signer(
        program_id: &Pubkey,
        stake_pool: &Pubkey,
        stake_pool_owner: &Pubkey,
        stake_pool_withdraw: &Pubkey,
        stake_account_to_update: &Pubkey,
        stake_account_new_authority: &Pubkey,
        stake_program_id: &Pubkey,
    ) -> Result<Instruction, ProgramError> {
        let args = StakePoolInstruction::SetStakingAuthority;
        let data = args.serialize()?;
        let accounts = vec![
            AccountMeta::new(*stake_pool, false),
            AccountMeta::new_readonly(*stake_pool_owner, false),
            AccountMeta::new_readonly(*stake_pool_withdraw, false),
            AccountMeta::new(*stake_account_to_update, false),
            AccountMeta::new_readonly(*stake_account_new_authority, false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(*stake_program_id, false),
        ];
        Ok(Instruction {
            program_id: *program_id,
            accounts,
            data,
        })
    }

    fn set_owner_without_signer(
        program_id: &Pubkey,
        stake_pool: &Pubkey,
        stake_pool_owner: &Pubkey,
        stake_pool_new_owner: &Pubkey,
        stake_pool_new_fee_receiver: &Pubkey,
    ) -> Result<Instruction, ProgramError> {
        let args = StakePoolInstruction::SetOwner;
        let data = args.serialize()?;
        let accounts = vec![
            AccountMeta::new(*stake_pool, false),
            AccountMeta::new_readonly(*stake_pool_owner, false),
            AccountMeta::new_readonly(*stake_pool_new_owner, false),
            AccountMeta::new_readonly(*stake_pool_new_fee_receiver, false),
        ];
        Ok(Instruction {
            program_id: *program_id,
            accounts,
            data,
        })
    }

    #[test]
    fn test_initialize() {
        let pool_info = create_stake_pool_default();
        // Read account data
        let state = State::deserialize(&pool_info.pool_account.data).unwrap();
        match state {
            State::Unallocated => panic!("Stake pool state is not initialized after init"),
            State::Init(stake_pool) => {
                assert_eq!(stake_pool.deposit_bump_seed, pool_info.deposit_bump_seed);
                assert_eq!(stake_pool.withdraw_bump_seed, pool_info.withdraw_bump_seed);
                assert_eq!(stake_pool.fee.numerator, FEE_DEFAULT.numerator);
                assert_eq!(stake_pool.fee.denominator, FEE_DEFAULT.denominator);

                assert_eq!(stake_pool.owner, pool_info.owner_key);
                assert_eq!(stake_pool.pool_mint, pool_info.mint_key);
                assert_eq!(stake_pool.owner_fee_account, pool_info.owner_fee_key);
                assert_eq!(stake_pool.token_program_id, TOKEN_PROGRAM_ID);

                assert_eq!(stake_pool.stake_total, 0);
                assert_eq!(stake_pool.pool_total, 0);
            }
        }
    }

    fn initialize_deposit_test() -> Deposit {
        let stake_balance: u64 = sol_to_lamports(10.0);
        let tokens_to_issue: u64 = 10_000_000_000;
        let user_token_balance: u64 = 9_800_000_000;
        let fee_token_balance: u64 = 200_000_000;
        assert_eq!(tokens_to_issue, user_token_balance + fee_token_balance);

        // Create stake account
        let mut pool_info = create_stake_pool(Fee {
            denominator: 100,
            numerator: 2,
        });

        let pool_token_receiver = create_token_account(
            &TOKEN_PROGRAM_ID,
            &pool_info.mint_key,
            &mut pool_info.mint_account,
        );

        Deposit {
            stake_balance,
            tokens_to_issue,
            user_token_balance,
            fee_token_balance,
            pool_info,
            pool_token_receiver,
        }
    }
    #[test]
    fn test_deposit() {
        let mut test_data = initialize_deposit_test();

        let deposit_info = do_deposit(
            &mut test_data.pool_info,
            test_data.stake_balance,
            &mut test_data.pool_token_receiver,
        );

        deposit_info.result.expect("Fail on deposit");
        // Test stake pool balance
        let state = State::deserialize(&test_data.pool_info.pool_account.data).unwrap();
        assert!(
            matches!(state, State::Init(stake_pool) if stake_pool.stake_total == test_data.stake_balance && stake_pool.pool_total == test_data.tokens_to_issue)
        );

        // Test token balances
        let user_token_state =
            SplAccount::unpack_from_slice(&test_data.pool_token_receiver.account.data)
                .expect("User token account is not initialized after deposit");
        assert_eq!(user_token_state.amount, test_data.user_token_balance);
        let fee_token_state =
            SplAccount::unpack_from_slice(&test_data.pool_info.owner_fee_account.data)
                .expect("Fee token account is not initialized after deposit");
        assert_eq!(fee_token_state.amount, test_data.fee_token_balance);

        // Test mint total issued tokens
        let mint_state = SplMint::unpack_from_slice(&test_data.pool_info.mint_account.data)
            .expect("Mint account is not initialized after deposit");
        assert_eq!(mint_state.supply, test_data.stake_balance);

        // TODO: Check stake account Withdrawer to match stake pool withdraw authority
    }
    #[test]
    fn negative_test_deposit_wrong_withdraw_authority() {
        let mut test_data = initialize_deposit_test();
        test_data.pool_info.withdraw_authority_key = Pubkey::new_unique();

        let deposit_info = do_deposit(
            &mut test_data.pool_info,
            test_data.stake_balance,
            &mut test_data.pool_token_receiver,
        );
        assert_eq!(
            deposit_info.result,
            Err(Error::InvalidProgramAddress.into())
        );
    }
    #[test]
    fn negative_test_deposit_wrong_deposit_authority() {
        let mut test_data = initialize_deposit_test();
        test_data.pool_info.deposit_authority_key = Pubkey::new_unique();

        let deposit_info = do_deposit(
            &mut test_data.pool_info,
            test_data.stake_balance,
            &mut test_data.pool_token_receiver,
        );

        assert_eq!(
            deposit_info.result,
            Err(Error::InvalidProgramAddress.into())
        );
    }
    #[test]
    fn negative_test_deposit_wrong_owner_fee_account() {
        let mut test_data = initialize_deposit_test();
        test_data.pool_info.owner_fee_account = Account::default();

        let deposit_info = do_deposit(
            &mut test_data.pool_info,
            test_data.stake_balance,
            &mut test_data.pool_token_receiver,
        );

        assert_eq!(deposit_info.result, Err(ProgramError::InvalidAccountData));
    }

    fn initialize_withdraw_test() -> Withdraw {
        let stake_balance = sol_to_lamports(20.0);
        let tokens_to_issue = 20_000_000_000;
        let withdraw_amount = sol_to_lamports(5.0);
        let tokens_to_burn = 5_000_000_000;

        let mut pool_info = create_stake_pool_default();

        let user_withdrawer_key = Pubkey::new_unique();

        let mut pool_token_receiver = create_token_account(
            &TOKEN_PROGRAM_ID,
            &pool_info.mint_key,
            &mut pool_info.mint_account,
        );
        let deposit_info = do_deposit(&mut pool_info, stake_balance, &mut pool_token_receiver);

        Withdraw {
            stake_balance,
            tokens_to_issue,
            withdraw_amount,
            tokens_to_burn,
            pool_info,
            user_withdrawer_key,
            pool_token_receiver,
            deposit_info,
        }
    }
    #[test]
    fn test_withdraw() {
        let mut test_data = initialize_withdraw_test();
        let withdraw_info = do_withdraw(&mut test_data);

        withdraw_info.result.expect("Fail on deposit");
        let fee_amount = test_data.stake_balance * FEE_DEFAULT.numerator / FEE_DEFAULT.denominator;

        let user_token_state =
            SplAccount::unpack_from_slice(&test_data.pool_token_receiver.account.data)
                .expect("User token account is not initialized after withdraw");
        assert_eq!(
            user_token_state.amount,
            test_data.stake_balance - fee_amount - test_data.withdraw_amount
        );

        // Check stake pool token amounts
        let state = State::deserialize(&test_data.pool_info.pool_account.data).unwrap();
        assert!(
            matches!(state, State::Init(stake_pool) if stake_pool.stake_total == test_data.stake_balance - test_data.withdraw_amount && stake_pool.pool_total == test_data.tokens_to_issue - test_data.tokens_to_burn)
        );
    }
    #[test]
    fn negative_test_withdraw_wrong_withdraw_authority() {
        let mut test_data = initialize_withdraw_test();

        test_data.pool_info.withdraw_authority_key = Pubkey::new_unique();

        let withdraw_info = do_withdraw(&mut test_data);

        assert_eq!(
            withdraw_info.result,
            Err(Error::InvalidProgramAddress.into())
        );
    }
    #[test]
    fn negative_test_withdraw_all() {
        let mut test_data = initialize_withdraw_test();

        test_data.withdraw_amount = test_data.stake_balance;

        let withdraw_info = do_withdraw(&mut test_data);

        assert_eq!(
            withdraw_info.result,
            Err(Error::InvalidProgramAddress.into())
        );
    }
    #[test]
    fn negative_test_withdraw_excess_amount() {
        let mut test_data = initialize_withdraw_test();

        test_data.withdraw_amount *= 2;

        let withdraw_info = do_withdraw(&mut test_data);

        assert_eq!(
            withdraw_info.result,
            Err(Error::InvalidProgramAddress.into())
        );
    }

    fn initialize_claim_test() -> Claim {
        let mut pool_info = create_stake_pool_default();

        let user_withdrawer_key = Pubkey::new_unique();

        let stake_balance = sol_to_lamports(20.0);
        let tokens_to_issue = 20_000_000_000;

        let mut pool_token_receiver = create_token_account(
            &TOKEN_PROGRAM_ID,
            &pool_info.mint_key,
            &mut pool_info.mint_account,
        );
        let deposit_info = do_deposit(&mut pool_info, stake_balance, &mut pool_token_receiver);

        // Need to deposit more to cover deposit fee
        let fee_amount = stake_balance * FEE_DEFAULT.numerator / FEE_DEFAULT.denominator;
        let extra_deposit = (fee_amount * FEE_DEFAULT.denominator)
            / (FEE_DEFAULT.denominator - FEE_DEFAULT.numerator);

        let _extra_deposit_info =
            do_deposit(&mut pool_info, extra_deposit, &mut pool_token_receiver);

        Claim {
            tokens_to_issue,
            allow_burn_to: pool_info.withdraw_authority_key,
            pool_info,
            user_withdrawer_key,
            pool_token_receiver,
            deposit_info,
        }
    }

    #[test]
    fn test_claim() {
        let mut test_data = initialize_claim_test();
        let claim_info = do_claim(&mut test_data);

        assert_eq!(claim_info.result, Ok(()));

        let user_token_state =
            SplAccount::unpack_from_slice(&test_data.pool_token_receiver.account.data)
                .expect("User token account is not initialized after withdraw");
        assert_eq!(user_token_state.amount, 0);

        // TODO: Check deposit_info.stake_account_account Withdrawer to change to user_withdrawer_key
    }
    #[test]
    fn negative_test_claim_not_enough_approved() {
        let mut test_data = initialize_claim_test();
        test_data.tokens_to_issue /= 2; // Approve less tokens for burning than required
        let claim_info = do_claim(&mut test_data);

        assert_eq!(claim_info.result, Err(TokenError::InsufficientFunds.into()));
    }
    #[test]
    fn negative_test_claim_approve_to_wrong_account() {
        let mut test_data = initialize_claim_test();
        test_data.allow_burn_to = test_data.pool_info.deposit_authority_key; // Change token burn authority
        let claim_info = do_claim(&mut test_data);

        assert_eq!(claim_info.result, Err(TokenError::OwnerMismatch.into()));
    }
    #[test]
    fn negative_test_claim_twice() {
        let mut test_data = initialize_claim_test();
        let claim_info = do_claim(&mut test_data);

        assert_eq!(claim_info.result, Ok(()));

        let result = do_process_instruction(
            claim(
                &STAKE_POOL_PROGRAM_ID,
                &test_data.pool_info.pool_key,
                &test_data.pool_info.withdraw_authority_key,
                &test_data.deposit_info.stake_account_key,
                &test_data.user_withdrawer_key,
                &test_data.pool_token_receiver.key,
                &test_data.pool_info.mint_key,
                &TOKEN_PROGRAM_ID,
                &stake_program_id(),
            )
            .unwrap(),
            vec![
                &mut test_data.pool_info.pool_account,
                &mut Account::default(),
                &mut test_data.deposit_info.stake_account_account,
                &mut Account::default(),
                &mut test_data.pool_token_receiver.account,
                &mut test_data.pool_info.mint_account,
                &mut Account::default(),
                &mut Account::default(),
                &mut Account::default(),
            ],
        );

        assert_eq!(result, Err(Error::InvalidProgramAddress.into()));
    }

    #[test]
    fn test_set_staking_authority() {
        let mut pool_info = create_stake_pool_default();
        let stake_balance = sol_to_lamports(10.0);

        let stake_key = Pubkey::new_unique();
        let mut stake_account = Account::new(stake_balance, STAKE_ACCOUNT_LEN, &stake_program_id());
        let new_authority_key = Pubkey::new_unique();
        let mut new_authority_account =
            Account::new(stake_balance, STAKE_ACCOUNT_LEN, &stake_program_id());

        let _result = do_process_instruction(
            set_staking_authority(
                &STAKE_POOL_PROGRAM_ID,
                &pool_info.pool_key,
                &pool_info.owner_key,
                &pool_info.withdraw_authority_key,
                &stake_key,
                &new_authority_key,
                &stake_program_id(),
            )
            .unwrap(),
            vec![
                &mut pool_info.pool_account,
                &mut pool_info.owner_fee_account,
                &mut Account::default(),
                &mut stake_account,
                &mut new_authority_account,
                &mut Account::default(),
                &mut Account::default(),
            ],
        )
        .expect("Error on set_owner");
    }
    #[test]
    fn negative_test_set_staking_authority_owner() {
        let mut pool_info = create_stake_pool_default();
        let stake_balance = sol_to_lamports(10.0);

        let stake_key = Pubkey::new_unique();
        let mut stake_account = Account::new(stake_balance, STAKE_ACCOUNT_LEN, &stake_program_id());
        let new_authority_key = Pubkey::new_unique();
        let mut new_authority_account =
            Account::new(stake_balance, STAKE_ACCOUNT_LEN, &stake_program_id());

        let result = do_process_instruction(
            set_staking_authority(
                &STAKE_POOL_PROGRAM_ID,
                &pool_info.pool_key,
                &Pubkey::new_unique(),
                &pool_info.withdraw_authority_key,
                &stake_key,
                &new_authority_key,
                &stake_program_id(),
            )
            .unwrap(),
            vec![
                &mut pool_info.pool_account,
                &mut Account::default(),
                &mut Account::default(),
                &mut stake_account,
                &mut new_authority_account,
                &mut Account::default(),
                &mut Account::default(),
            ],
        );

        assert_eq!(result, Err(Error::InvalidInput.into()));
    }
    #[test]
    fn negative_test_set_staking_authority_signer() {
        let mut pool_info = create_stake_pool_default();
        let stake_balance = sol_to_lamports(10.0);

        let stake_key = Pubkey::new_unique();
        let mut stake_account = Account::new(stake_balance, STAKE_ACCOUNT_LEN, &stake_program_id());
        let new_authority_key = Pubkey::new_unique();
        let mut new_authority_account =
            Account::new(stake_balance, STAKE_ACCOUNT_LEN, &stake_program_id());

        let result = do_process_instruction(
            set_staking_authority_without_signer(
                &STAKE_POOL_PROGRAM_ID,
                &pool_info.pool_key,
                &Pubkey::new_unique(),
                &pool_info.withdraw_authority_key,
                &stake_key,
                &new_authority_key,
                &stake_program_id(),
            )
            .unwrap(),
            vec![
                &mut pool_info.pool_account,
                &mut pool_info.owner_fee_account,
                &mut Account::default(),
                &mut stake_account,
                &mut new_authority_account,
                &mut Account::default(),
                &mut Account::default(),
            ],
        );
        assert_eq!(result, Err(Error::InvalidInput.into()));
    }

    #[test]
    fn test_set_owner() {
        let mut pool_info = create_stake_pool_default();

        let new_owner_key = Pubkey::new_unique();
        let mut new_owner_account = Account::default();

        let mut new_owner_fee = create_token_account(
            &TOKEN_PROGRAM_ID,
            &pool_info.mint_key,
            &mut pool_info.mint_account,
        );

        let _result = do_process_instruction(
            set_owner(
                &STAKE_POOL_PROGRAM_ID,
                &pool_info.pool_key,
                &pool_info.owner_key,
                &new_owner_key,
                &new_owner_fee.key,
            )
            .unwrap(),
            vec![
                &mut pool_info.pool_account,
                &mut pool_info.owner_account,
                &mut new_owner_account,
                &mut new_owner_fee.account,
            ],
        )
        .expect("Error on set_owner");

        let state = State::deserialize(&pool_info.pool_account.data).unwrap();
        assert!(
            matches!(state, State::Init(stake_pool) if stake_pool.owner == new_owner_key && stake_pool.owner_fee_account == new_owner_fee.key)
        );
    }
    #[test]
    fn negative_test_set_owner_owner() {
        let mut pool_info = create_stake_pool_default();

        let new_owner_key = Pubkey::new_unique();
        let mut new_owner_account = Account::default();

        let mut new_owner_fee = create_token_account(
            &TOKEN_PROGRAM_ID,
            &pool_info.mint_key,
            &mut pool_info.mint_account,
        );

        let result = do_process_instruction(
            set_owner(
                &STAKE_POOL_PROGRAM_ID,
                &pool_info.pool_key,
                &Pubkey::new_unique(),
                &new_owner_key,
                &new_owner_fee.key,
            )
            .unwrap(),
            vec![
                &mut pool_info.pool_account,
                &mut Account::default(),
                &mut new_owner_account,
                &mut new_owner_fee.account,
            ],
        );

        assert_eq!(result, Err(Error::InvalidInput.into()));
    }
    #[test]
    fn negative_test_set_owner_signer() {
        let mut pool_info = create_stake_pool_default();

        let new_owner_key = Pubkey::new_unique();
        let mut new_owner_account = Account::default();

        let mut new_owner_fee = create_token_account(
            &TOKEN_PROGRAM_ID,
            &pool_info.mint_key,
            &mut pool_info.mint_account,
        );

        let result = do_process_instruction(
            set_owner_without_signer(
                &STAKE_POOL_PROGRAM_ID,
                &pool_info.pool_key,
                &pool_info.owner_key,
                &new_owner_key,
                &new_owner_fee.key,
            )
            .unwrap(),
            vec![
                &mut pool_info.pool_account,
                &mut pool_info.owner_account,
                &mut new_owner_account,
                &mut new_owner_fee.account,
            ],
        );

        assert_eq!(result, Err(Error::InvalidInput.into()));
    }
}
