//! Program state processor

use crate::constraints::{FeeConstraints, FEE_CONSTRAINTS};
use crate::{curve::SwapCurve, error::SwapError, instruction::SwapInstruction, state::SwapInfo};
use num_traits::FromPrimitive;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    info,
    program::invoke_signed,
    program_error::{PrintProgramError, ProgramError},
    program_option::COption,
    program_pack::Pack,
    pubkey::Pubkey,
};
use std::convert::TryInto;

/// Hardcode the number of token types in a pool, used to calculate the
/// equivalent pool tokens for the owner trading fee.
const TOKENS_IN_POOL: u64 = 2;

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Unpacks a spl_token `Account`.
    pub fn unpack_token_account(data: &[u8]) -> Result<spl_token::state::Account, SwapError> {
        spl_token::state::Account::unpack(data).map_err(|_| SwapError::ExpectedAccount)
    }

    /// Unpacks a spl_token `Mint`.
    pub fn unpack_mint(data: &[u8]) -> Result<spl_token::state::Mint, SwapError> {
        spl_token::state::Mint::unpack(data).map_err(|_| SwapError::ExpectedMint)
    }

    /// Calculates the authority id by generating a program address.
    pub fn authority_id(
        program_id: &Pubkey,
        my_info: &Pubkey,
        nonce: u8,
    ) -> Result<Pubkey, SwapError> {
        Pubkey::create_program_address(&[&my_info.to_bytes()[..32], &[nonce]], program_id)
            .or(Err(SwapError::InvalidProgramAddress))
    }

    /// Issue a spl_token `Burn` instruction.
    pub fn token_burn<'a>(
        swap: &Pubkey,
        token_program: AccountInfo<'a>,
        burn_account: AccountInfo<'a>,
        mint: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        nonce: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[nonce]];
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
    pub fn token_mint_to<'a>(
        swap: &Pubkey,
        token_program: AccountInfo<'a>,
        mint: AccountInfo<'a>,
        destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        nonce: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[nonce]];
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

    /// Issue a spl_token `Transfer` instruction.
    pub fn token_transfer<'a>(
        swap: &Pubkey,
        token_program: AccountInfo<'a>,
        source: AccountInfo<'a>,
        destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        nonce: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[nonce]];
        let signers = &[&authority_signature_seeds[..]];
        let ix = spl_token::instruction::transfer(
            token_program.key,
            source.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?;
        invoke_signed(
            &ix,
            &[source, destination, authority, token_program],
            signers,
        )
    }

    /// Processes an [Initialize](enum.Instruction.html).
    pub fn process_initialize(
        program_id: &Pubkey,
        nonce: u8,
        swap_curve: SwapCurve,
        accounts: &[AccountInfo],
        fee_constraints: &Option<FeeConstraints>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let token_a_info = next_account_info(account_info_iter)?;
        let token_b_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let fee_account_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapInfo::unpack_unchecked(&swap_info.data.borrow())?;
        if token_swap.is_initialized {
            return Err(SwapError::AlreadyInUse.into());
        }

        if *authority_info.key != Self::authority_id(program_id, swap_info.key, nonce)? {
            return Err(SwapError::InvalidProgramAddress.into());
        }
        let token_a = Self::unpack_token_account(&token_a_info.data.borrow())?;
        let token_b = Self::unpack_token_account(&token_b_info.data.borrow())?;
        let fee_account = Self::unpack_token_account(&fee_account_info.data.borrow())?;
        let destination = Self::unpack_token_account(&destination_info.data.borrow())?;
        let pool_mint = Self::unpack_mint(&pool_mint_info.data.borrow())?;
        if *authority_info.key != token_a.owner {
            return Err(SwapError::InvalidOwner.into());
        }
        if *authority_info.key != token_b.owner {
            return Err(SwapError::InvalidOwner.into());
        }
        if *authority_info.key == destination.owner {
            return Err(SwapError::InvalidOutputOwner.into());
        }
        if *authority_info.key == fee_account.owner {
            return Err(SwapError::InvalidOutputOwner.into());
        }
        if COption::Some(*authority_info.key) != pool_mint.mint_authority {
            return Err(SwapError::InvalidOwner.into());
        }

        if token_a.mint == token_b.mint {
            return Err(SwapError::RepeatedMint.into());
        }
        if token_b.amount == 0 {
            return Err(SwapError::EmptySupply.into());
        }
        if token_a.amount == 0 {
            return Err(SwapError::EmptySupply.into());
        }
        if token_a.delegate.is_some() {
            return Err(SwapError::InvalidDelegate.into());
        }
        if token_b.delegate.is_some() {
            return Err(SwapError::InvalidDelegate.into());
        }
        if token_a.close_authority.is_some() {
            return Err(SwapError::InvalidCloseAuthority.into());
        }
        if token_b.close_authority.is_some() {
            return Err(SwapError::InvalidCloseAuthority.into());
        }

        if pool_mint.supply != 0 {
            return Err(SwapError::InvalidSupply.into());
        }
        if pool_mint.freeze_authority.is_some() {
            return Err(SwapError::InvalidFreezeAuthority.into());
        }
        if *pool_mint_info.key != fee_account.mint {
            return Err(SwapError::IncorrectPoolMint.into());
        }

        if let Some(fee_constraints) = fee_constraints {
            let owner_key = fee_constraints
                .owner_key
                .parse::<Pubkey>()
                .map_err(|_| SwapError::InvalidOwner)?;
            if fee_account.owner != owner_key {
                return Err(SwapError::InvalidOwner.into());
            }
            fee_constraints.validate_curve(&swap_curve)?;
        }

        let initial_amount = swap_curve.calculator.new_pool_supply();

        Self::token_mint_to(
            swap_info.key,
            token_program_info.clone(),
            pool_mint_info.clone(),
            destination_info.clone(),
            authority_info.clone(),
            nonce,
            to_u64(initial_amount)?,
        )?;

        let obj = SwapInfo {
            is_initialized: true,
            nonce,
            token_program_id: *token_program_info.key,
            token_a: *token_a_info.key,
            token_b: *token_b_info.key,
            pool_mint: *pool_mint_info.key,
            token_a_mint: token_a.mint,
            token_b_mint: token_b.mint,
            pool_fee_account: *fee_account_info.key,
            swap_curve,
        };
        SwapInfo::pack(obj, &mut swap_info.data.borrow_mut())?;
        Ok(())
    }

    /// Processes an [Swap](enum.Instruction.html).
    pub fn process_swap(
        program_id: &Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let swap_source_info = next_account_info(account_info_iter)?;
        let swap_destination_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let pool_fee_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapInfo::unpack(&swap_info.data.borrow())?;

        if *authority_info.key != Self::authority_id(program_id, swap_info.key, token_swap.nonce)? {
            return Err(SwapError::InvalidProgramAddress.into());
        }
        if !(*swap_source_info.key == token_swap.token_a
            || *swap_source_info.key == token_swap.token_b)
        {
            return Err(SwapError::IncorrectSwapAccount.into());
        }
        if !(*swap_destination_info.key == token_swap.token_a
            || *swap_destination_info.key == token_swap.token_b)
        {
            return Err(SwapError::IncorrectSwapAccount.into());
        }
        if *swap_source_info.key == *swap_destination_info.key {
            return Err(SwapError::InvalidInput.into());
        }
        if swap_source_info.key == source_info.key {
            return Err(SwapError::InvalidInput.into());
        }
        if swap_destination_info.key == destination_info.key {
            return Err(SwapError::InvalidInput.into());
        }
        if *pool_mint_info.key != token_swap.pool_mint {
            return Err(SwapError::IncorrectPoolMint.into());
        }
        if *pool_fee_account_info.key != token_swap.pool_fee_account {
            return Err(SwapError::IncorrectFeeAccount.into());
        }

        let source_account = Self::unpack_token_account(&swap_source_info.data.borrow())?;
        let dest_account = Self::unpack_token_account(&swap_destination_info.data.borrow())?;
        let pool_mint = Self::unpack_mint(&pool_mint_info.data.borrow())?;

        let result = token_swap
            .swap_curve
            .calculator
            .swap(
                to_u128(amount_in)?,
                to_u128(source_account.amount)?,
                to_u128(dest_account.amount)?,
            )
            .ok_or(SwapError::ZeroTradingTokens)?;
        if result.amount_swapped < to_u128(minimum_amount_out)? {
            return Err(SwapError::ExceededSlippage.into());
        }
        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            source_info.clone(),
            swap_source_info.clone(),
            authority_info.clone(),
            token_swap.nonce,
            amount_in,
        )?;
        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            swap_destination_info.clone(),
            destination_info.clone(),
            authority_info.clone(),
            token_swap.nonce,
            to_u64(result.amount_swapped)?,
        )?;

        // mint pool tokens equivalent to the owner fee
        let source_account = Self::unpack_token_account(&swap_source_info.data.borrow())?;
        let mut pool_token_amount = token_swap
            .swap_curve
            .calculator
            .owner_fee_to_pool_tokens(
                result.owner_fee,
                to_u128(source_account.amount)?,
                to_u128(pool_mint.supply)?,
                to_u128(TOKENS_IN_POOL)?,
            )
            .ok_or(SwapError::FeeCalculationFailure)?;
        if pool_token_amount > 0 {
            // Allow error to fall through
            if let Ok(host_fee_account_info) = next_account_info(account_info_iter) {
                let host_fee_account =
                    Self::unpack_token_account(&host_fee_account_info.data.borrow())?;
                if *pool_mint_info.key != host_fee_account.mint {
                    return Err(SwapError::IncorrectPoolMint.into());
                }
                let host_fee = token_swap
                    .swap_curve
                    .calculator
                    .host_fee(pool_token_amount)
                    .ok_or(SwapError::FeeCalculationFailure)?;
                if host_fee > 0 {
                    pool_token_amount = pool_token_amount
                        .checked_sub(host_fee)
                        .ok_or(SwapError::FeeCalculationFailure)?;
                    Self::token_mint_to(
                        swap_info.key,
                        token_program_info.clone(),
                        pool_mint_info.clone(),
                        host_fee_account_info.clone(),
                        authority_info.clone(),
                        token_swap.nonce,
                        to_u64(host_fee)?,
                    )?;
                }
            }
            Self::token_mint_to(
                swap_info.key,
                token_program_info.clone(),
                pool_mint_info.clone(),
                pool_fee_account_info.clone(),
                authority_info.clone(),
                token_swap.nonce,
                to_u64(pool_token_amount)?,
            )?;
        }
        Ok(())
    }

    /// Processes an [Deposit](enum.Instruction.html).
    pub fn process_deposit(
        program_id: &Pubkey,
        pool_token_amount: u64,
        maximum_token_a_amount: u64,
        maximum_token_b_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let source_a_info = next_account_info(account_info_iter)?;
        let source_b_info = next_account_info(account_info_iter)?;
        let token_a_info = next_account_info(account_info_iter)?;
        let token_b_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let dest_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapInfo::unpack(&swap_info.data.borrow())?;
        if *authority_info.key != Self::authority_id(program_id, swap_info.key, token_swap.nonce)? {
            return Err(SwapError::InvalidProgramAddress.into());
        }
        if *token_a_info.key != token_swap.token_a {
            return Err(SwapError::IncorrectSwapAccount.into());
        }
        if *token_b_info.key != token_swap.token_b {
            return Err(SwapError::IncorrectSwapAccount.into());
        }
        if *pool_mint_info.key != token_swap.pool_mint {
            return Err(SwapError::IncorrectPoolMint.into());
        }
        if token_a_info.key == source_a_info.key {
            return Err(SwapError::InvalidInput.into());
        }
        if token_b_info.key == source_b_info.key {
            return Err(SwapError::InvalidInput.into());
        }

        let token_a = Self::unpack_token_account(&token_a_info.data.borrow())?;
        let token_b = Self::unpack_token_account(&token_b_info.data.borrow())?;
        let pool_mint = Self::unpack_mint(&pool_mint_info.data.borrow())?;
        let pool_token_amount = to_u128(pool_token_amount)?;
        let pool_mint_supply = to_u128(pool_mint.supply)?;

        let calculator = token_swap.swap_curve.calculator;

        let a_amount = calculator
            .pool_tokens_to_trading_tokens(
                pool_token_amount,
                pool_mint_supply,
                to_u128(token_a.amount)?,
            )
            .ok_or(SwapError::ZeroTradingTokens)?;
        if a_amount > to_u128(maximum_token_a_amount)? {
            return Err(SwapError::ExceededSlippage.into());
        }
        let b_amount = calculator
            .pool_tokens_to_trading_tokens(
                pool_token_amount,
                pool_mint_supply,
                to_u128(token_b.amount)?,
            )
            .ok_or(SwapError::ZeroTradingTokens)?;
        if b_amount > to_u128(maximum_token_b_amount)? {
            return Err(SwapError::ExceededSlippage.into());
        }

        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            source_a_info.clone(),
            token_a_info.clone(),
            authority_info.clone(),
            token_swap.nonce,
            to_u64(a_amount)?,
        )?;
        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            source_b_info.clone(),
            token_b_info.clone(),
            authority_info.clone(),
            token_swap.nonce,
            to_u64(b_amount)?,
        )?;
        Self::token_mint_to(
            swap_info.key,
            token_program_info.clone(),
            pool_mint_info.clone(),
            dest_info.clone(),
            authority_info.clone(),
            token_swap.nonce,
            to_u64(pool_token_amount)?,
        )?;

        Ok(())
    }

    /// Processes an [Withdraw](enum.Instruction.html).
    pub fn process_withdraw(
        program_id: &Pubkey,
        pool_token_amount: u64,
        minimum_token_a_amount: u64,
        minimum_token_b_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let token_a_info = next_account_info(account_info_iter)?;
        let token_b_info = next_account_info(account_info_iter)?;
        let dest_token_a_info = next_account_info(account_info_iter)?;
        let dest_token_b_info = next_account_info(account_info_iter)?;
        let pool_fee_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapInfo::unpack(&swap_info.data.borrow())?;
        if *authority_info.key != Self::authority_id(program_id, swap_info.key, token_swap.nonce)? {
            return Err(SwapError::InvalidProgramAddress.into());
        }
        if *token_a_info.key != token_swap.token_a {
            return Err(SwapError::IncorrectSwapAccount.into());
        }
        if *token_b_info.key != token_swap.token_b {
            return Err(SwapError::IncorrectSwapAccount.into());
        }
        if *pool_mint_info.key != token_swap.pool_mint {
            return Err(SwapError::IncorrectPoolMint.into());
        }
        if *pool_fee_account_info.key != token_swap.pool_fee_account {
            return Err(SwapError::IncorrectFeeAccount.into());
        }
        if token_a_info.key == dest_token_a_info.key {
            return Err(SwapError::InvalidInput.into());
        }
        if token_b_info.key == dest_token_b_info.key {
            return Err(SwapError::InvalidInput.into());
        }

        let token_a = Self::unpack_token_account(&token_a_info.data.borrow())?;
        let token_b = Self::unpack_token_account(&token_b_info.data.borrow())?;
        let pool_mint = Self::unpack_mint(&pool_mint_info.data.borrow())?;

        let calculator = token_swap.swap_curve.calculator;

        let withdraw_fee: u128 = if *pool_fee_account_info.key == *source_info.key {
            // withdrawing from the fee account, don't assess withdraw fee
            0
        } else {
            calculator
                .owner_withdraw_fee(to_u128(pool_token_amount)?)
                .ok_or(SwapError::FeeCalculationFailure)?
        };
        let pool_token_amount = to_u128(pool_token_amount)?
            .checked_sub(withdraw_fee)
            .ok_or(SwapError::CalculationFailure)?;

        let a_amount = calculator
            .pool_tokens_to_trading_tokens(
                pool_token_amount,
                to_u128(pool_mint.supply)?,
                to_u128(token_a.amount)?,
            )
            .ok_or(SwapError::ZeroTradingTokens)?;
        if a_amount < to_u128(minimum_token_a_amount)? {
            return Err(SwapError::ExceededSlippage.into());
        }
        let b_amount = calculator
            .pool_tokens_to_trading_tokens(
                pool_token_amount,
                to_u128(pool_mint.supply)?,
                to_u128(token_b.amount)?,
            )
            .ok_or(SwapError::ZeroTradingTokens)?;
        let b_amount = to_u64(b_amount)?;
        if b_amount < minimum_token_b_amount {
            return Err(SwapError::ExceededSlippage.into());
        }

        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            token_a_info.clone(),
            dest_token_a_info.clone(),
            authority_info.clone(),
            token_swap.nonce,
            to_u64(a_amount)?,
        )?;
        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            token_b_info.clone(),
            dest_token_b_info.clone(),
            authority_info.clone(),
            token_swap.nonce,
            b_amount,
        )?;
        if withdraw_fee > 0 {
            Self::token_transfer(
                swap_info.key,
                token_program_info.clone(),
                source_info.clone(),
                pool_fee_account_info.clone(),
                authority_info.clone(),
                token_swap.nonce,
                to_u64(withdraw_fee)?,
            )?;
        }
        Self::token_burn(
            swap_info.key,
            token_program_info.clone(),
            source_info.clone(),
            pool_mint_info.clone(),
            authority_info.clone(),
            token_swap.nonce,
            to_u64(pool_token_amount)?,
        )?;
        Ok(())
    }

    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        Self::process_with_fee_constraints(program_id, accounts, input, &FEE_CONSTRAINTS)
    }

    /// Processes an instruction given extra constraint
    pub fn process_with_fee_constraints(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
        fee_constraints: &Option<FeeConstraints>,
    ) -> ProgramResult {
        let instruction = SwapInstruction::unpack(input)?;
        match instruction {
            SwapInstruction::Initialize { nonce, swap_curve } => {
                info!("Instruction: Init");
                Self::process_initialize(program_id, nonce, swap_curve, accounts, fee_constraints)
            }
            SwapInstruction::Swap {
                amount_in,
                minimum_amount_out,
            } => {
                info!("Instruction: Swap");
                Self::process_swap(program_id, amount_in, minimum_amount_out, accounts)
            }
            SwapInstruction::Deposit {
                pool_token_amount,
                maximum_token_a_amount,
                maximum_token_b_amount,
            } => {
                info!("Instruction: Deposit");
                Self::process_deposit(
                    program_id,
                    pool_token_amount,
                    maximum_token_a_amount,
                    maximum_token_b_amount,
                    accounts,
                )
            }
            SwapInstruction::Withdraw {
                pool_token_amount,
                minimum_token_a_amount,
                minimum_token_b_amount,
            } => {
                info!("Instruction: Withdraw");
                Self::process_withdraw(
                    program_id,
                    pool_token_amount,
                    minimum_token_a_amount,
                    minimum_token_b_amount,
                    accounts,
                )
            }
        }
    }
}

impl PrintProgramError for SwapError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            SwapError::AlreadyInUse => info!("Error: Swap account already in use"),
            SwapError::InvalidProgramAddress => {
                info!("Error: Invalid program address generated from nonce and key")
            }
            SwapError::InvalidOwner => {
                info!("Error: The input account owner is not the program address")
            }
            SwapError::InvalidOutputOwner => {
                info!("Error: Output pool account owner cannot be the program address")
            }
            SwapError::ExpectedMint => {
                info!("Error: Deserialized account is not an SPL Token mint")
            }
            SwapError::ExpectedAccount => {
                info!("Error: Deserialized account is not an SPL Token account")
            }
            SwapError::EmptySupply => info!("Error: Input token account empty"),
            SwapError::InvalidSupply => info!("Error: Pool token mint has a non-zero supply"),
            SwapError::RepeatedMint => info!("Error: Swap input token accounts have the same mint"),
            SwapError::InvalidDelegate => info!("Error: Token account has a delegate"),
            SwapError::InvalidInput => info!("Error: InvalidInput"),
            SwapError::IncorrectSwapAccount => {
                info!("Error: Address of the provided swap token account is incorrect")
            }
            SwapError::IncorrectPoolMint => {
                info!("Error: Address of the provided pool token mint is incorrect")
            }
            SwapError::InvalidOutput => info!("Error: InvalidOutput"),
            SwapError::CalculationFailure => info!("Error: CalculationFailure"),
            SwapError::InvalidInstruction => info!("Error: InvalidInstruction"),
            SwapError::ExceededSlippage => {
                info!("Error: Swap instruction exceeds desired slippage limit")
            }
            SwapError::InvalidCloseAuthority => info!("Error: Token account has a close authority"),
            SwapError::InvalidFreezeAuthority => {
                info!("Error: Pool token mint has a freeze authority")
            }
            SwapError::IncorrectFeeAccount => info!("Error: Pool fee token account incorrect"),
            SwapError::ZeroTradingTokens => {
                info!("Error: Given pool token amount results in zero trading tokens")
            }
            SwapError::FeeCalculationFailure => info!(
                "Error: The fee calculation failed due to overflow, underflow, or unexpected 0"
            ),
            SwapError::ConversionFailure => info!("Error: Conversion to or from u64 failed."),
            SwapError::InvalidFee => {
                info!("Error: The provided fee does not match the program owner's constraints")
            }
        }
    }
}

fn to_u128(val: u64) -> Result<u128, SwapError> {
    val.try_into().map_err(|_| SwapError::ConversionFailure)
}

fn to_u64(val: u128) -> Result<u64, SwapError> {
    val.try_into().map_err(|_| SwapError::ConversionFailure)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        curve::{
            ConstantProductCurve, CurveCalculator, CurveType, FlatCurve, INITIAL_SWAP_POOL_AMOUNT,
        },
        instruction::{deposit, initialize, swap, withdraw},
    };
    use solana_program::{
        account::Account, account_info::create_is_signer_account_infos, instruction::Instruction,
        program_stubs, rent::Rent, sysvar::rent,
    };
    use spl_token::{
        error::TokenError,
        instruction::{
            approve, initialize_account, initialize_mint, mint_to, revoke, set_authority,
            AuthorityType,
        },
    };

    // Test program id for the swap program.
    const SWAP_PROGRAM_ID: Pubkey = Pubkey::new_from_array([2u8; 32]);
    // Test program id for the token program.
    const TOKEN_PROGRAM_ID: Pubkey = Pubkey::new_from_array([1u8; 32]);

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

            // mimic check for token program in accounts
            if !account_infos.iter().any(|x| *x.key == TOKEN_PROGRAM_ID) {
                return Err(ProgramError::InvalidAccountData);
            }

            for meta in instruction.accounts.iter() {
                for account_info in account_infos.iter() {
                    if meta.pubkey == *account_info.key {
                        let mut new_account_info = account_info.clone();
                        for seeds in signers_seeds.iter() {
                            let signer =
                                Pubkey::create_program_address(&seeds, &SWAP_PROGRAM_ID).unwrap();
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
    }

    fn test_syscall_stubs() {
        use std::sync::Once;
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            program_stubs::set_syscall_stubs(Box::new(TestSyscallStubs {}));
        });
    }

    struct SwapAccountInfo {
        nonce: u8,
        authority_key: Pubkey,
        swap_curve: SwapCurve,
        swap_key: Pubkey,
        swap_account: Account,
        pool_mint_key: Pubkey,
        pool_mint_account: Account,
        pool_fee_key: Pubkey,
        pool_fee_account: Account,
        pool_token_key: Pubkey,
        pool_token_account: Account,
        token_a_key: Pubkey,
        token_a_account: Account,
        token_a_mint_key: Pubkey,
        token_a_mint_account: Account,
        token_b_key: Pubkey,
        token_b_account: Account,
        token_b_mint_key: Pubkey,
        token_b_mint_account: Account,
    }

    impl SwapAccountInfo {
        pub fn new(
            user_key: &Pubkey,
            swap_curve: SwapCurve,
            token_a_amount: u64,
            token_b_amount: u64,
        ) -> Self {
            let swap_key = Pubkey::new_unique();
            let swap_account = Account::new(0, SwapInfo::get_packed_len(), &SWAP_PROGRAM_ID);
            let (authority_key, nonce) =
                Pubkey::find_program_address(&[&swap_key.to_bytes()[..]], &SWAP_PROGRAM_ID);

            let (pool_mint_key, mut pool_mint_account) =
                create_mint(&TOKEN_PROGRAM_ID, &authority_key, None);
            let (pool_token_key, pool_token_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &pool_mint_key,
                &mut pool_mint_account,
                &authority_key,
                &user_key,
                0,
            );
            let (pool_fee_key, pool_fee_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &pool_mint_key,
                &mut pool_mint_account,
                &authority_key,
                &user_key,
                0,
            );
            let (token_a_mint_key, mut token_a_mint_account) =
                create_mint(&TOKEN_PROGRAM_ID, &user_key, None);
            let (token_a_key, token_a_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &token_a_mint_key,
                &mut token_a_mint_account,
                &user_key,
                &authority_key,
                token_a_amount,
            );
            let (token_b_mint_key, mut token_b_mint_account) =
                create_mint(&TOKEN_PROGRAM_ID, &user_key, None);
            let (token_b_key, token_b_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &token_b_mint_key,
                &mut token_b_mint_account,
                &user_key,
                &authority_key,
                token_b_amount,
            );

            SwapAccountInfo {
                nonce,
                authority_key,
                swap_curve,
                swap_key,
                swap_account,
                pool_mint_key,
                pool_mint_account,
                pool_fee_key,
                pool_fee_account,
                pool_token_key,
                pool_token_account,
                token_a_key,
                token_a_account,
                token_a_mint_key,
                token_a_mint_account,
                token_b_key,
                token_b_account,
                token_b_mint_key,
                token_b_mint_account,
            }
        }

        pub fn initialize_swap(&mut self) -> ProgramResult {
            do_process_instruction(
                initialize(
                    &SWAP_PROGRAM_ID,
                    &TOKEN_PROGRAM_ID,
                    &self.swap_key,
                    &self.authority_key,
                    &self.token_a_key,
                    &self.token_b_key,
                    &self.pool_mint_key,
                    &self.pool_fee_key,
                    &self.pool_token_key,
                    self.nonce,
                    self.swap_curve.clone(),
                )
                .unwrap(),
                vec![
                    &mut self.swap_account,
                    &mut Account::default(),
                    &mut self.token_a_account,
                    &mut self.token_b_account,
                    &mut self.pool_mint_account,
                    &mut self.pool_fee_account,
                    &mut self.pool_token_account,
                    &mut Account::default(),
                ],
            )
        }

        pub fn setup_token_accounts(
            &mut self,
            mint_owner: &Pubkey,
            account_owner: &Pubkey,
            a_amount: u64,
            b_amount: u64,
            pool_amount: u64,
        ) -> (Pubkey, Account, Pubkey, Account, Pubkey, Account) {
            let (token_a_key, token_a_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &self.token_a_mint_key,
                &mut self.token_a_mint_account,
                &mint_owner,
                &account_owner,
                a_amount,
            );
            let (token_b_key, token_b_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &self.token_b_mint_key,
                &mut self.token_b_mint_account,
                &mint_owner,
                &account_owner,
                b_amount,
            );
            let (pool_key, pool_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &self.pool_mint_key,
                &mut self.pool_mint_account,
                &self.authority_key,
                &account_owner,
                pool_amount,
            );
            (
                token_a_key,
                token_a_account,
                token_b_key,
                token_b_account,
                pool_key,
                pool_account,
            )
        }

        fn get_token_account(&self, account_key: &Pubkey) -> &Account {
            if *account_key == self.token_a_key {
                return &self.token_a_account;
            } else if *account_key == self.token_b_key {
                return &self.token_b_account;
            }
            panic!("Could not find matching swap token account");
        }

        fn set_token_account(&mut self, account_key: &Pubkey, account: Account) {
            if *account_key == self.token_a_key {
                self.token_a_account = account;
                return;
            } else if *account_key == self.token_b_key {
                self.token_b_account = account;
                return;
            }
            panic!("Could not find matching swap token account");
        }

        #[allow(clippy::too_many_arguments)]
        pub fn swap(
            &mut self,
            user_key: &Pubkey,
            user_source_key: &Pubkey,
            mut user_source_account: &mut Account,
            swap_source_key: &Pubkey,
            swap_destination_key: &Pubkey,
            user_destination_key: &Pubkey,
            mut user_destination_account: &mut Account,
            amount_in: u64,
            minimum_amount_out: u64,
        ) -> ProgramResult {
            // approve moving from user source account
            do_process_instruction(
                approve(
                    &TOKEN_PROGRAM_ID,
                    &user_source_key,
                    &self.authority_key,
                    &user_key,
                    &[],
                    amount_in,
                )
                .unwrap(),
                vec![
                    &mut user_source_account,
                    &mut Account::default(),
                    &mut Account::default(),
                ],
            )
            .unwrap();

            let mut swap_source_account = self.get_token_account(swap_source_key).clone();
            let mut swap_destination_account = self.get_token_account(swap_destination_key).clone();

            // perform the swap
            do_process_instruction(
                swap(
                    &SWAP_PROGRAM_ID,
                    &TOKEN_PROGRAM_ID,
                    &self.swap_key,
                    &self.authority_key,
                    &user_source_key,
                    &swap_source_key,
                    &swap_destination_key,
                    &user_destination_key,
                    &self.pool_mint_key,
                    &self.pool_fee_key,
                    None,
                    amount_in,
                    minimum_amount_out,
                )
                .unwrap(),
                vec![
                    &mut self.swap_account,
                    &mut Account::default(),
                    &mut user_source_account,
                    &mut swap_source_account,
                    &mut swap_destination_account,
                    &mut user_destination_account,
                    &mut self.pool_mint_account,
                    &mut self.pool_fee_account,
                    &mut Account::default(),
                ],
            )?;

            self.set_token_account(swap_source_key, swap_source_account);
            self.set_token_account(swap_destination_key, swap_destination_account);

            Ok(())
        }

        #[allow(clippy::too_many_arguments)]
        pub fn deposit(
            &mut self,
            depositor_key: &Pubkey,
            depositor_token_a_key: &Pubkey,
            mut depositor_token_a_account: &mut Account,
            depositor_token_b_key: &Pubkey,
            mut depositor_token_b_account: &mut Account,
            depositor_pool_key: &Pubkey,
            mut depositor_pool_account: &mut Account,
            pool_amount: u64,
            amount_a: u64,
            amount_b: u64,
        ) -> ProgramResult {
            do_process_instruction(
                approve(
                    &TOKEN_PROGRAM_ID,
                    &depositor_token_a_key,
                    &self.authority_key,
                    &depositor_key,
                    &[],
                    amount_a,
                )
                .unwrap(),
                vec![
                    &mut depositor_token_a_account,
                    &mut Account::default(),
                    &mut Account::default(),
                ],
            )
            .unwrap();

            do_process_instruction(
                approve(
                    &TOKEN_PROGRAM_ID,
                    &depositor_token_b_key,
                    &self.authority_key,
                    &depositor_key,
                    &[],
                    amount_b,
                )
                .unwrap(),
                vec![
                    &mut depositor_token_b_account,
                    &mut Account::default(),
                    &mut Account::default(),
                ],
            )
            .unwrap();

            do_process_instruction(
                deposit(
                    &SWAP_PROGRAM_ID,
                    &TOKEN_PROGRAM_ID,
                    &self.swap_key,
                    &self.authority_key,
                    &depositor_token_a_key,
                    &depositor_token_b_key,
                    &self.token_a_key,
                    &self.token_b_key,
                    &self.pool_mint_key,
                    &depositor_pool_key,
                    pool_amount,
                    amount_a,
                    amount_b,
                )
                .unwrap(),
                vec![
                    &mut self.swap_account,
                    &mut Account::default(),
                    &mut depositor_token_a_account,
                    &mut depositor_token_b_account,
                    &mut self.token_a_account,
                    &mut self.token_b_account,
                    &mut self.pool_mint_account,
                    &mut depositor_pool_account,
                    &mut Account::default(),
                ],
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn withdraw(
            &mut self,
            user_key: &Pubkey,
            pool_key: &Pubkey,
            mut pool_account: &mut Account,
            token_a_key: &Pubkey,
            mut token_a_account: &mut Account,
            token_b_key: &Pubkey,
            mut token_b_account: &mut Account,
            pool_amount: u64,
            minimum_a_amount: u64,
            minimum_b_amount: u64,
        ) -> ProgramResult {
            // approve swap program to take out pool tokens
            do_process_instruction(
                approve(
                    &TOKEN_PROGRAM_ID,
                    &pool_key,
                    &self.authority_key,
                    &user_key,
                    &[],
                    pool_amount,
                )
                .unwrap(),
                vec![
                    &mut pool_account,
                    &mut Account::default(),
                    &mut Account::default(),
                ],
            )
            .unwrap();

            // withdraw token a and b correctly
            do_process_instruction(
                withdraw(
                    &SWAP_PROGRAM_ID,
                    &TOKEN_PROGRAM_ID,
                    &self.swap_key,
                    &self.authority_key,
                    &self.pool_mint_key,
                    &self.pool_fee_key,
                    &pool_key,
                    &self.token_a_key,
                    &self.token_b_key,
                    &token_a_key,
                    &token_b_key,
                    pool_amount,
                    minimum_a_amount,
                    minimum_b_amount,
                )
                .unwrap(),
                vec![
                    &mut self.swap_account,
                    &mut Account::default(),
                    &mut self.pool_mint_account,
                    &mut pool_account,
                    &mut self.token_a_account,
                    &mut self.token_b_account,
                    &mut token_a_account,
                    &mut token_b_account,
                    &mut self.pool_fee_account,
                    &mut Account::default(),
                ],
            )
        }
    }

    fn mint_minimum_balance() -> u64 {
        Rent::default().minimum_balance(spl_token::state::Mint::get_packed_len())
    }

    fn account_minimum_balance() -> u64 {
        Rent::default().minimum_balance(spl_token::state::Account::get_packed_len())
    }

    fn do_process_instruction_with_fee_constraints(
        instruction: Instruction,
        accounts: Vec<&mut Account>,
        fee_constraints: &Option<FeeConstraints>,
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
        let res = if instruction.program_id == SWAP_PROGRAM_ID {
            Processor::process_with_fee_constraints(
                &instruction.program_id,
                &account_infos,
                &instruction.data,
                fee_constraints,
            )
        } else {
            spl_token::processor::Processor::process(
                &instruction.program_id,
                &account_infos,
                &instruction.data,
            )
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

    fn do_process_instruction(
        instruction: Instruction,
        accounts: Vec<&mut Account>,
    ) -> ProgramResult {
        do_process_instruction_with_fee_constraints(instruction, accounts, &FEE_CONSTRAINTS)
    }

    fn mint_token(
        program_id: &Pubkey,
        mint_key: &Pubkey,
        mut mint_account: &mut Account,
        mint_authority_key: &Pubkey,
        account_owner_key: &Pubkey,
        amount: u64,
    ) -> (Pubkey, Account) {
        let account_key = Pubkey::new_unique();
        let mut account_account = Account::new(
            account_minimum_balance(),
            spl_token::state::Account::get_packed_len(),
            &program_id,
        );
        let mut mint_authority_account = Account::default();
        let mut rent_sysvar_account = rent::create_account(1, &Rent::free());

        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, account_owner_key).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut mint_authority_account,
                &mut rent_sysvar_account,
            ],
        )
        .unwrap();

        if amount > 0 {
            do_process_instruction(
                mint_to(
                    &program_id,
                    &mint_key,
                    &account_key,
                    &mint_authority_key,
                    &[],
                    amount,
                )
                .unwrap(),
                vec![
                    &mut mint_account,
                    &mut account_account,
                    &mut mint_authority_account,
                ],
            )
            .unwrap();
        }

        (account_key, account_account)
    }

    fn create_mint(
        program_id: &Pubkey,
        authority_key: &Pubkey,
        freeze_authority: Option<&Pubkey>,
    ) -> (Pubkey, Account) {
        let mint_key = Pubkey::new_unique();
        let mut mint_account = Account::new(
            mint_minimum_balance(),
            spl_token::state::Mint::get_packed_len(),
            &program_id,
        );
        let mut rent_sysvar_account = rent::create_account(1, &Rent::free());

        do_process_instruction(
            initialize_mint(&program_id, &mint_key, authority_key, freeze_authority, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar_account],
        )
        .unwrap();

        (mint_key, mint_account)
    }

    #[test]
    fn test_token_program_id_error() {
        test_syscall_stubs();
        let swap_key = Pubkey::new_unique();
        let mut mint = (Pubkey::new_unique(), Account::default());
        let mut destination = (Pubkey::new_unique(), Account::default());
        let token_program = (TOKEN_PROGRAM_ID, Account::default());
        let (authority_key, nonce) =
            Pubkey::find_program_address(&[&swap_key.to_bytes()[..]], &SWAP_PROGRAM_ID);
        let mut authority = (authority_key, Account::default());
        let swap_bytes = swap_key.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[nonce]];
        let signers = &[&authority_signature_seeds[..]];
        let ix = mint_to(
            &token_program.0,
            &mint.0,
            &destination.0,
            &authority.0,
            &[],
            10,
        )
        .unwrap();
        let mint = (&mut mint).into();
        let destination = (&mut destination).into();
        let authority = (&mut authority).into();

        let err = invoke_signed(&ix, &[mint, destination, authority], signers).unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);
    }

    #[test]
    fn test_initialize() {
        let user_key = Pubkey::new_unique();
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 2;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 10;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 5;
        let host_fee_numerator = 20;
        let host_fee_denominator = 100;

        let token_a_amount = 1000;
        let token_b_amount = 2000;
        let pool_token_amount = 10;
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Box::new(ConstantProductCurve {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
                host_fee_numerator,
                host_fee_denominator,
            }),
        };

        let mut accounts =
            SwapAccountInfo::new(&user_key, swap_curve, token_a_amount, token_b_amount);

        // wrong nonce for authority_key
        {
            let old_nonce = accounts.nonce;
            accounts.nonce = old_nonce - 1;
            assert_eq!(
                Err(SwapError::InvalidProgramAddress.into()),
                accounts.initialize_swap()
            );
            accounts.nonce = old_nonce;
        }

        // uninitialized token a account
        {
            let old_account = accounts.token_a_account;
            accounts.token_a_account = Account::default();
            assert_eq!(
                Err(SwapError::ExpectedAccount.into()),
                accounts.initialize_swap()
            );
            accounts.token_a_account = old_account;
        }

        // uninitialized token b account
        {
            let old_account = accounts.token_b_account;
            accounts.token_b_account = Account::default();
            assert_eq!(
                Err(SwapError::ExpectedAccount.into()),
                accounts.initialize_swap()
            );
            accounts.token_b_account = old_account;
        }

        // uninitialized pool mint
        {
            let old_account = accounts.pool_mint_account;
            accounts.pool_mint_account = Account::default();
            assert_eq!(
                Err(SwapError::ExpectedMint.into()),
                accounts.initialize_swap()
            );
            accounts.pool_mint_account = old_account;
        }

        // token A account owner is not swap authority
        {
            let (_token_a_key, token_a_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.token_a_mint_key,
                &mut accounts.token_a_mint_account,
                &user_key,
                &user_key,
                0,
            );
            let old_account = accounts.token_a_account;
            accounts.token_a_account = token_a_account;
            assert_eq!(
                Err(SwapError::InvalidOwner.into()),
                accounts.initialize_swap()
            );
            accounts.token_a_account = old_account;
        }

        // token B account owner is not swap authority
        {
            let (_token_b_key, token_b_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.token_b_mint_key,
                &mut accounts.token_b_mint_account,
                &user_key,
                &user_key,
                0,
            );
            let old_account = accounts.token_b_account;
            accounts.token_b_account = token_b_account;
            assert_eq!(
                Err(SwapError::InvalidOwner.into()),
                accounts.initialize_swap()
            );
            accounts.token_b_account = old_account;
        }

        // pool token account owner is swap authority
        {
            let (_pool_token_key, pool_token_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.pool_mint_key,
                &mut accounts.pool_mint_account,
                &accounts.authority_key,
                &accounts.authority_key,
                0,
            );
            let old_account = accounts.pool_token_account;
            accounts.pool_token_account = pool_token_account;
            assert_eq!(
                Err(SwapError::InvalidOutputOwner.into()),
                accounts.initialize_swap()
            );
            accounts.pool_token_account = old_account;
        }

        // pool fee account owner is swap authority
        {
            let (_pool_fee_key, pool_fee_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.pool_mint_key,
                &mut accounts.pool_mint_account,
                &accounts.authority_key,
                &accounts.authority_key,
                0,
            );
            let old_account = accounts.pool_fee_account;
            accounts.pool_fee_account = pool_fee_account;
            assert_eq!(
                Err(SwapError::InvalidOutputOwner.into()),
                accounts.initialize_swap()
            );
            accounts.pool_fee_account = old_account;
        }

        // pool mint authority is not swap authority
        {
            let (_pool_mint_key, pool_mint_account) =
                create_mint(&TOKEN_PROGRAM_ID, &user_key, None);
            let old_mint = accounts.pool_mint_account;
            accounts.pool_mint_account = pool_mint_account;
            assert_eq!(
                Err(SwapError::InvalidOwner.into()),
                accounts.initialize_swap()
            );
            accounts.pool_mint_account = old_mint;
        }

        // pool mint token has freeze authority
        {
            let (_pool_mint_key, pool_mint_account) =
                create_mint(&TOKEN_PROGRAM_ID, &accounts.authority_key, Some(&user_key));
            let old_mint = accounts.pool_mint_account;
            accounts.pool_mint_account = pool_mint_account;
            assert_eq!(
                Err(SwapError::InvalidFreezeAuthority.into()),
                accounts.initialize_swap()
            );
            accounts.pool_mint_account = old_mint;
        }

        // empty token A account
        {
            let (_token_a_key, token_a_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.token_a_mint_key,
                &mut accounts.token_a_mint_account,
                &user_key,
                &accounts.authority_key,
                0,
            );
            let old_account = accounts.token_a_account;
            accounts.token_a_account = token_a_account;
            assert_eq!(
                Err(SwapError::EmptySupply.into()),
                accounts.initialize_swap()
            );
            accounts.token_a_account = old_account;
        }

        // empty token B account
        {
            let (_token_b_key, token_b_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.token_b_mint_key,
                &mut accounts.token_b_mint_account,
                &user_key,
                &accounts.authority_key,
                0,
            );
            let old_account = accounts.token_b_account;
            accounts.token_b_account = token_b_account;
            assert_eq!(
                Err(SwapError::EmptySupply.into()),
                accounts.initialize_swap()
            );
            accounts.token_b_account = old_account;
        }

        // invalid pool tokens
        {
            let old_mint = accounts.pool_mint_account;
            let old_pool_account = accounts.pool_token_account;

            let (_pool_mint_key, pool_mint_account) =
                create_mint(&TOKEN_PROGRAM_ID, &accounts.authority_key, None);
            accounts.pool_mint_account = pool_mint_account;

            let (_empty_pool_token_key, empty_pool_token_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.pool_mint_key,
                &mut accounts.pool_mint_account,
                &accounts.authority_key,
                &user_key,
                0,
            );

            let (_pool_token_key, pool_token_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.pool_mint_key,
                &mut accounts.pool_mint_account,
                &accounts.authority_key,
                &user_key,
                pool_token_amount,
            );

            // non-empty pool token account
            accounts.pool_token_account = pool_token_account;
            assert_eq!(
                Err(SwapError::InvalidSupply.into()),
                accounts.initialize_swap()
            );

            // pool tokens already in circulation
            accounts.pool_token_account = empty_pool_token_account;
            assert_eq!(
                Err(SwapError::InvalidSupply.into()),
                accounts.initialize_swap()
            );

            accounts.pool_mint_account = old_mint;
            accounts.pool_token_account = old_pool_account;
        }

        // pool fee account has wrong mint
        {
            let (_pool_fee_key, pool_fee_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.token_a_mint_key,
                &mut accounts.token_a_mint_account,
                &user_key,
                &user_key,
                0,
            );
            let old_account = accounts.pool_fee_account;
            accounts.pool_fee_account = pool_fee_account;
            assert_eq!(
                Err(SwapError::IncorrectPoolMint.into()),
                accounts.initialize_swap()
            );
            accounts.pool_fee_account = old_account;
        }

        // token A account is delegated
        {
            do_process_instruction(
                approve(
                    &TOKEN_PROGRAM_ID,
                    &accounts.token_a_key,
                    &user_key,
                    &accounts.authority_key,
                    &[],
                    1,
                )
                .unwrap(),
                vec![
                    &mut accounts.token_a_account,
                    &mut Account::default(),
                    &mut Account::default(),
                ],
            )
            .unwrap();
            assert_eq!(
                Err(SwapError::InvalidDelegate.into()),
                accounts.initialize_swap()
            );

            do_process_instruction(
                revoke(
                    &TOKEN_PROGRAM_ID,
                    &accounts.token_a_key,
                    &accounts.authority_key,
                    &[],
                )
                .unwrap(),
                vec![&mut accounts.token_a_account, &mut Account::default()],
            )
            .unwrap();
        }

        // token B account is delegated
        {
            do_process_instruction(
                approve(
                    &TOKEN_PROGRAM_ID,
                    &accounts.token_b_key,
                    &user_key,
                    &accounts.authority_key,
                    &[],
                    1,
                )
                .unwrap(),
                vec![
                    &mut accounts.token_b_account,
                    &mut Account::default(),
                    &mut Account::default(),
                ],
            )
            .unwrap();
            assert_eq!(
                Err(SwapError::InvalidDelegate.into()),
                accounts.initialize_swap()
            );

            do_process_instruction(
                revoke(
                    &TOKEN_PROGRAM_ID,
                    &accounts.token_b_key,
                    &accounts.authority_key,
                    &[],
                )
                .unwrap(),
                vec![&mut accounts.token_b_account, &mut Account::default()],
            )
            .unwrap();
        }

        // token A account has close authority
        {
            do_process_instruction(
                set_authority(
                    &TOKEN_PROGRAM_ID,
                    &accounts.token_a_key,
                    Some(&user_key),
                    AuthorityType::CloseAccount,
                    &accounts.authority_key,
                    &[],
                )
                .unwrap(),
                vec![&mut accounts.token_a_account, &mut Account::default()],
            )
            .unwrap();
            assert_eq!(
                Err(SwapError::InvalidCloseAuthority.into()),
                accounts.initialize_swap()
            );

            do_process_instruction(
                set_authority(
                    &TOKEN_PROGRAM_ID,
                    &accounts.token_a_key,
                    None,
                    AuthorityType::CloseAccount,
                    &user_key,
                    &[],
                )
                .unwrap(),
                vec![&mut accounts.token_a_account, &mut Account::default()],
            )
            .unwrap();
        }

        // token B account has close authority
        {
            do_process_instruction(
                set_authority(
                    &TOKEN_PROGRAM_ID,
                    &accounts.token_b_key,
                    Some(&user_key),
                    AuthorityType::CloseAccount,
                    &accounts.authority_key,
                    &[],
                )
                .unwrap(),
                vec![&mut accounts.token_b_account, &mut Account::default()],
            )
            .unwrap();
            assert_eq!(
                Err(SwapError::InvalidCloseAuthority.into()),
                accounts.initialize_swap()
            );

            do_process_instruction(
                set_authority(
                    &TOKEN_PROGRAM_ID,
                    &accounts.token_b_key,
                    None,
                    AuthorityType::CloseAccount,
                    &user_key,
                    &[],
                )
                .unwrap(),
                vec![&mut accounts.token_b_account, &mut Account::default()],
            )
            .unwrap();
        }

        // wrong token program id
        {
            let wrong_program_id = Pubkey::new_unique();
            assert_eq!(
                Err(ProgramError::InvalidAccountData),
                do_process_instruction(
                    initialize(
                        &SWAP_PROGRAM_ID,
                        &wrong_program_id,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &accounts.pool_token_key,
                        accounts.nonce,
                        accounts.swap_curve.clone(),
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.pool_token_account,
                        &mut Account::default(),
                    ],
                )
            );
        }

        // create swap with same token A and B
        {
            let (_token_a_repeat_key, token_a_repeat_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.token_a_mint_key,
                &mut accounts.token_a_mint_account,
                &user_key,
                &accounts.authority_key,
                10,
            );
            let old_account = accounts.token_b_account;
            accounts.token_b_account = token_a_repeat_account;
            assert_eq!(
                Err(SwapError::RepeatedMint.into()),
                accounts.initialize_swap()
            );
            accounts.token_b_account = old_account;
        }

        // create valid swap
        accounts.initialize_swap().unwrap();

        // create valid flat swap
        {
            let swap_curve = SwapCurve {
                curve_type: CurveType::Flat,
                calculator: Box::new(FlatCurve {
                    trade_fee_numerator,
                    trade_fee_denominator,
                    owner_trade_fee_numerator,
                    owner_trade_fee_denominator,
                    owner_withdraw_fee_numerator,
                    owner_withdraw_fee_denominator,
                    host_fee_numerator,
                    host_fee_denominator,
                }),
            };
            let mut accounts =
                SwapAccountInfo::new(&user_key, swap_curve, token_a_amount, token_b_amount);
            accounts.initialize_swap().unwrap();
        }

        // wrong owner key in constraint
        {
            let new_key = Pubkey::new_unique();
            let trade_fee_numerator = 25;
            let trade_fee_denominator = 10000;
            let owner_trade_fee_numerator = 5;
            let owner_trade_fee_denominator = 10000;
            let host_fee_numerator = 20;
            let host_fee_denominator = 100;
            let curve = ConstantProductCurve {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
                host_fee_numerator,
                host_fee_denominator,
            };
            let swap_curve = SwapCurve {
                curve_type: CurveType::ConstantProduct,
                calculator: Box::new(curve.clone()),
            };
            let owner_key = &new_key.to_string();
            let valid_constant_product_curves = &[curve];
            let constraints = Some(FeeConstraints {
                owner_key,
                valid_constant_product_curves,
                valid_flat_curves: &[],
            });
            let mut accounts =
                SwapAccountInfo::new(&user_key, swap_curve, token_a_amount, token_b_amount);
            assert_eq!(
                Err(SwapError::InvalidOwner.into()),
                do_process_instruction_with_fee_constraints(
                    initialize(
                        &SWAP_PROGRAM_ID,
                        &TOKEN_PROGRAM_ID,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &accounts.pool_token_key,
                        accounts.nonce,
                        accounts.swap_curve.clone(),
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.pool_token_account,
                        &mut Account::default(),
                    ],
                    &constraints,
                )
            );
        }

        // wrong fee in constraint
        {
            let trade_fee_numerator = 25;
            let trade_fee_denominator = 10000;
            let owner_trade_fee_numerator = 5;
            let owner_trade_fee_denominator = 10000;
            let host_fee_numerator = 20;
            let host_fee_denominator = 100;
            let mut curve = ConstantProductCurve {
                trade_fee_numerator: trade_fee_numerator - 1,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
                host_fee_numerator,
                host_fee_denominator,
            };
            let swap_curve = SwapCurve {
                curve_type: CurveType::ConstantProduct,
                calculator: Box::new(curve.clone()),
            };
            curve.trade_fee_numerator = trade_fee_numerator;
            let owner_key = &user_key.to_string();
            let valid_constant_product_curves = &[curve];
            let constraints = Some(FeeConstraints {
                owner_key,
                valid_constant_product_curves,
                valid_flat_curves: &[],
            });
            let mut accounts =
                SwapAccountInfo::new(&user_key, swap_curve, token_a_amount, token_b_amount);
            assert_eq!(
                Err(SwapError::InvalidFee.into()),
                do_process_instruction_with_fee_constraints(
                    initialize(
                        &SWAP_PROGRAM_ID,
                        &TOKEN_PROGRAM_ID,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &accounts.pool_token_key,
                        accounts.nonce,
                        accounts.swap_curve.clone(),
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.pool_token_account,
                        &mut Account::default(),
                    ],
                    &constraints,
                )
            );
        }

        // create valid swap with constraints
        {
            let trade_fee_numerator = 25;
            let trade_fee_denominator = 10000;
            let owner_trade_fee_numerator = 5;
            let owner_trade_fee_denominator = 10000;
            let host_fee_numerator = 20;
            let host_fee_denominator = 100;
            let curve = ConstantProductCurve {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
                host_fee_numerator,
                host_fee_denominator,
            };
            let swap_curve = SwapCurve {
                curve_type: CurveType::ConstantProduct,
                calculator: Box::new(curve.clone()),
            };
            let owner_key = &user_key.to_string();
            let valid_constant_product_curves = &[curve];
            let constraints = Some(FeeConstraints {
                owner_key,
                valid_constant_product_curves,
                valid_flat_curves: &[],
            });
            let mut accounts =
                SwapAccountInfo::new(&user_key, swap_curve, token_a_amount, token_b_amount);
            do_process_instruction_with_fee_constraints(
                initialize(
                    &SWAP_PROGRAM_ID,
                    &TOKEN_PROGRAM_ID,
                    &accounts.swap_key,
                    &accounts.authority_key,
                    &accounts.token_a_key,
                    &accounts.token_b_key,
                    &accounts.pool_mint_key,
                    &accounts.pool_fee_key,
                    &accounts.pool_token_key,
                    accounts.nonce,
                    accounts.swap_curve.clone(),
                )
                .unwrap(),
                vec![
                    &mut accounts.swap_account,
                    &mut Account::default(),
                    &mut accounts.token_a_account,
                    &mut accounts.token_b_account,
                    &mut accounts.pool_mint_account,
                    &mut accounts.pool_fee_account,
                    &mut accounts.pool_token_account,
                    &mut Account::default(),
                ],
                &constraints,
            )
            .unwrap();
        }

        // create again
        {
            assert_eq!(
                Err(SwapError::AlreadyInUse.into()),
                accounts.initialize_swap()
            );
        }
        let swap_info = SwapInfo::unpack(&accounts.swap_account.data).unwrap();
        assert_eq!(swap_info.is_initialized, true);
        assert_eq!(swap_info.nonce, accounts.nonce);
        assert_eq!(
            swap_info.swap_curve.curve_type,
            accounts.swap_curve.curve_type
        );
        assert_eq!(swap_info.token_a, accounts.token_a_key);
        assert_eq!(swap_info.token_b, accounts.token_b_key);
        assert_eq!(swap_info.pool_mint, accounts.pool_mint_key);
        assert_eq!(swap_info.token_a_mint, accounts.token_a_mint_key);
        assert_eq!(swap_info.token_b_mint, accounts.token_b_mint_key);
        assert_eq!(swap_info.pool_fee_account, accounts.pool_fee_key);
        let token_a = Processor::unpack_token_account(&accounts.token_a_account.data).unwrap();
        assert_eq!(token_a.amount, token_a_amount);
        let token_b = Processor::unpack_token_account(&accounts.token_b_account.data).unwrap();
        assert_eq!(token_b.amount, token_b_amount);
        let pool_account =
            Processor::unpack_token_account(&accounts.pool_token_account.data).unwrap();
        let pool_mint = Processor::unpack_mint(&accounts.pool_mint_account.data).unwrap();
        assert_eq!(pool_mint.supply, pool_account.amount);
    }

    #[test]
    fn test_deposit() {
        let user_key = Pubkey::new_unique();
        let depositor_key = Pubkey::new_unique();
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 2;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 10;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 5;
        let host_fee_numerator = 20;
        let host_fee_denominator = 100;

        let token_a_amount = 1000;
        let token_b_amount = 9000;
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Box::new(ConstantProductCurve {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
                host_fee_numerator,
                host_fee_denominator,
            }),
        };

        let mut accounts =
            SwapAccountInfo::new(&user_key, swap_curve, token_a_amount, token_b_amount);

        let deposit_a = token_a_amount / 10;
        let deposit_b = token_b_amount / 10;
        let pool_amount = INITIAL_SWAP_POOL_AMOUNT / 10;

        // swap not initialized
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            assert_eq!(
                Err(ProgramError::UninitializedAccount),
                accounts.deposit(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    &pool_key,
                    &mut pool_account,
                    pool_amount.try_into().unwrap(),
                    deposit_a,
                    deposit_b,
                )
            );
        }

        accounts.initialize_swap().unwrap();

        // wrong nonce for authority_key
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            let old_authority = accounts.authority_key;
            let (bad_authority_key, _nonce) = Pubkey::find_program_address(
                &[&accounts.swap_key.to_bytes()[..]],
                &TOKEN_PROGRAM_ID,
            );
            accounts.authority_key = bad_authority_key;
            assert_eq!(
                Err(SwapError::InvalidProgramAddress.into()),
                accounts.deposit(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    &pool_key,
                    &mut pool_account,
                    pool_amount.try_into().unwrap(),
                    deposit_a,
                    deposit_b,
                )
            );
            accounts.authority_key = old_authority;
        }

        // not enough token A
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &depositor_key,
                deposit_a / 2,
                deposit_b,
                0,
            );
            assert_eq!(
                Err(TokenError::InsufficientFunds.into()),
                accounts.deposit(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    &pool_key,
                    &mut pool_account,
                    pool_amount.try_into().unwrap(),
                    deposit_a,
                    deposit_b,
                )
            );
        }

        // not enough token B
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &depositor_key,
                deposit_a,
                deposit_b / 2,
                0,
            );
            assert_eq!(
                Err(TokenError::InsufficientFunds.into()),
                accounts.deposit(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    &pool_key,
                    &mut pool_account,
                    pool_amount.try_into().unwrap(),
                    deposit_a,
                    deposit_b,
                )
            );
        }

        // wrong swap token accounts
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            assert_eq!(
                Err(TokenError::MintMismatch.into()),
                accounts.deposit(
                    &depositor_key,
                    &token_b_key,
                    &mut token_b_account,
                    &token_a_key,
                    &mut token_a_account,
                    &pool_key,
                    &mut pool_account,
                    pool_amount.try_into().unwrap(),
                    deposit_a,
                    deposit_b,
                )
            );
        }

        // wrong pool token account
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                mut _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            let (
                wrong_token_key,
                mut wrong_token_account,
                _token_b_key,
                mut _token_b_account,
                _pool_key,
                mut _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            assert_eq!(
                Err(TokenError::MintMismatch.into()),
                accounts.deposit(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    &wrong_token_key,
                    &mut wrong_token_account,
                    pool_amount.try_into().unwrap(),
                    deposit_a,
                    deposit_b,
                )
            );
        }

        // no approval
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            assert_eq!(
                Err(TokenError::OwnerMismatch.into()),
                do_process_instruction(
                    deposit(
                        &SWAP_PROGRAM_ID,
                        &TOKEN_PROGRAM_ID,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &token_a_key,
                        &token_b_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &pool_key,
                        pool_amount.try_into().unwrap(),
                        deposit_a,
                        deposit_b,
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut token_a_account,
                        &mut token_b_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut pool_account,
                        &mut Account::default(),
                    ],
                )
            );
        }

        // wrong token program id
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            let wrong_key = Pubkey::new_unique();
            assert_eq!(
                Err(ProgramError::InvalidAccountData),
                do_process_instruction(
                    deposit(
                        &SWAP_PROGRAM_ID,
                        &wrong_key,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &token_a_key,
                        &token_b_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &pool_key,
                        pool_amount.try_into().unwrap(),
                        deposit_a,
                        deposit_b,
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut token_a_account,
                        &mut token_b_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut pool_account,
                        &mut Account::default(),
                    ],
                )
            );
        }

        // wrong swap token accounts
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);

            let old_a_key = accounts.token_a_key;
            let old_a_account = accounts.token_a_account;

            accounts.token_a_key = token_a_key;
            accounts.token_a_account = token_a_account.clone();

            // wrong swap token a account
            assert_eq!(
                Err(SwapError::IncorrectSwapAccount.into()),
                accounts.deposit(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    &pool_key,
                    &mut pool_account,
                    pool_amount.try_into().unwrap(),
                    deposit_a,
                    deposit_b,
                )
            );

            accounts.token_a_key = old_a_key;
            accounts.token_a_account = old_a_account;

            let old_b_key = accounts.token_b_key;
            let old_b_account = accounts.token_b_account;

            accounts.token_b_key = token_b_key;
            accounts.token_b_account = token_b_account.clone();

            // wrong swap token b account
            assert_eq!(
                Err(SwapError::IncorrectSwapAccount.into()),
                accounts.deposit(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    &pool_key,
                    &mut pool_account,
                    pool_amount.try_into().unwrap(),
                    deposit_a,
                    deposit_b,
                )
            );

            accounts.token_b_key = old_b_key;
            accounts.token_b_account = old_b_account;
        }

        // wrong mint
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            let (pool_mint_key, pool_mint_account) =
                create_mint(&TOKEN_PROGRAM_ID, &accounts.authority_key, None);
            let old_pool_key = accounts.pool_mint_key;
            let old_pool_account = accounts.pool_mint_account;
            accounts.pool_mint_key = pool_mint_key;
            accounts.pool_mint_account = pool_mint_account;

            assert_eq!(
                Err(SwapError::IncorrectPoolMint.into()),
                accounts.deposit(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    &pool_key,
                    &mut pool_account,
                    pool_amount.try_into().unwrap(),
                    deposit_a,
                    deposit_b,
                )
            );

            accounts.pool_mint_key = old_pool_key;
            accounts.pool_mint_account = old_pool_account;
        }

        // deposit 1 pool token fails beacuse it equates to 0 swap tokens
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            assert_eq!(
                Err(SwapError::ZeroTradingTokens.into()),
                accounts.deposit(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    &pool_key,
                    &mut pool_account,
                    1,
                    deposit_a,
                    deposit_b / 10,
                )
            );
        }

        // slippage exceeded
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            // maximum A amount in too low
            assert_eq!(
                Err(SwapError::ExceededSlippage.into()),
                accounts.deposit(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    &pool_key,
                    &mut pool_account,
                    pool_amount.try_into().unwrap(),
                    deposit_a / 10,
                    deposit_b,
                )
            );
            // maximum B amount in too low
            assert_eq!(
                Err(SwapError::ExceededSlippage.into()),
                accounts.deposit(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    &pool_key,
                    &mut pool_account,
                    pool_amount.try_into().unwrap(),
                    deposit_a,
                    deposit_b / 10,
                )
            );
        }

        // invalid input: can't use swap pool tokens as source
        {
            let (
                _token_a_key,
                _token_a_account,
                _token_b_key,
                _token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            let swap_token_a_key = accounts.token_a_key;
            let mut swap_token_a_account = accounts.get_token_account(&swap_token_a_key).clone();
            let swap_token_b_key = accounts.token_b_key;
            let mut swap_token_b_account = accounts.get_token_account(&swap_token_b_key).clone();
            let authority_key = accounts.authority_key;
            assert_eq!(
                Err(SwapError::InvalidInput.into()),
                accounts.deposit(
                    &authority_key,
                    &swap_token_a_key,
                    &mut swap_token_a_account,
                    &swap_token_b_key,
                    &mut swap_token_b_account,
                    &pool_key,
                    &mut pool_account,
                    pool_amount.try_into().unwrap(),
                    deposit_a,
                    deposit_b,
                )
            );
        }
        // correctly deposit
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            accounts
                .deposit(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    &pool_key,
                    &mut pool_account,
                    pool_amount.try_into().unwrap(),
                    deposit_a,
                    deposit_b,
                )
                .unwrap();

            let swap_token_a =
                Processor::unpack_token_account(&accounts.token_a_account.data).unwrap();
            assert_eq!(swap_token_a.amount, deposit_a + token_a_amount);
            let swap_token_b =
                Processor::unpack_token_account(&accounts.token_b_account.data).unwrap();
            assert_eq!(swap_token_b.amount, deposit_b + token_b_amount);
            let token_a = Processor::unpack_token_account(&token_a_account.data).unwrap();
            assert_eq!(token_a.amount, 0);
            let token_b = Processor::unpack_token_account(&token_b_account.data).unwrap();
            assert_eq!(token_b.amount, 0);
            let pool_account = Processor::unpack_token_account(&pool_account.data).unwrap();
            let swap_pool_account =
                Processor::unpack_token_account(&accounts.pool_token_account.data).unwrap();
            let pool_mint = Processor::unpack_mint(&accounts.pool_mint_account.data).unwrap();
            assert_eq!(
                pool_mint.supply,
                pool_account.amount + swap_pool_account.amount
            );
        }
    }

    #[test]
    fn test_withdraw() {
        let user_key = Pubkey::new_unique();
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 2;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 10;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 5;
        let host_fee_numerator = 7;
        let host_fee_denominator = 100;

        let token_a_amount = 1000;
        let token_b_amount = 2000;
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Box::new(ConstantProductCurve {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
                host_fee_numerator,
                host_fee_denominator,
            }),
        };

        let withdrawer_key = Pubkey::new_unique();
        let initial_a = token_a_amount / 10;
        let initial_b = token_b_amount / 10;
        let initial_pool = swap_curve.calculator.new_pool_supply() / 10;
        let withdraw_amount = initial_pool / 4;
        let minimum_a_amount = initial_a / 40;
        let minimum_b_amount = initial_b / 40;

        let mut accounts =
            SwapAccountInfo::new(&user_key, swap_curve, token_a_amount, token_b_amount);

        // swap not initialized
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &withdrawer_key, initial_a, initial_b, 0);
            assert_eq!(
                Err(ProgramError::UninitializedAccount),
                accounts.withdraw(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_a_amount,
                    minimum_b_amount,
                )
            );
        }

        accounts.initialize_swap().unwrap();

        // wrong nonce for authority_key
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &withdrawer_key, initial_a, initial_b, 0);
            let old_authority = accounts.authority_key;
            let (bad_authority_key, _nonce) = Pubkey::find_program_address(
                &[&accounts.swap_key.to_bytes()[..]],
                &TOKEN_PROGRAM_ID,
            );
            accounts.authority_key = bad_authority_key;
            assert_eq!(
                Err(SwapError::InvalidProgramAddress.into()),
                accounts.withdraw(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_a_amount,
                    minimum_b_amount,
                )
            );
            accounts.authority_key = old_authority;
        }

        // not enough pool tokens
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                initial_a,
                initial_b,
                to_u64(withdraw_amount).unwrap() / 2u64,
            );
            assert_eq!(
                Err(TokenError::InsufficientFunds.into()),
                accounts.withdraw(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_a_amount / 2,
                    minimum_b_amount / 2,
                )
            );
        }

        // wrong token a / b accounts
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                initial_a,
                initial_b,
                withdraw_amount.try_into().unwrap(),
            );
            assert_eq!(
                Err(TokenError::MintMismatch.into()),
                accounts.withdraw(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_b_key,
                    &mut token_b_account,
                    &token_a_key,
                    &mut token_a_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_a_amount,
                    minimum_b_amount,
                )
            );
        }

        // wrong pool token account
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                initial_a,
                initial_b,
                withdraw_amount.try_into().unwrap(),
            );
            let (
                wrong_token_a_key,
                mut wrong_token_a_account,
                _token_b_key,
                _token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                withdraw_amount.try_into().unwrap(),
                initial_b,
                withdraw_amount.try_into().unwrap(),
            );
            assert_eq!(
                Err(TokenError::MintMismatch.into()),
                accounts.withdraw(
                    &withdrawer_key,
                    &wrong_token_a_key,
                    &mut wrong_token_a_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_a_amount,
                    minimum_b_amount,
                )
            );
        }

        // wrong pool fee account
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                wrong_pool_key,
                wrong_pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                initial_a,
                initial_b,
                withdraw_amount.try_into().unwrap(),
            );
            let (
                _token_a_key,
                _token_a_account,
                _token_b_key,
                _token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                initial_a,
                initial_b,
                withdraw_amount.try_into().unwrap(),
            );
            let old_pool_fee_account = accounts.pool_fee_account;
            let old_pool_fee_key = accounts.pool_fee_key;
            accounts.pool_fee_account = wrong_pool_account;
            accounts.pool_fee_key = wrong_pool_key;
            assert_eq!(
                Err(SwapError::IncorrectFeeAccount.into()),
                accounts.withdraw(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_a_amount,
                    minimum_b_amount,
                ),
            );
            accounts.pool_fee_account = old_pool_fee_account;
            accounts.pool_fee_key = old_pool_fee_key;
        }

        // no approval
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                0,
                0,
                withdraw_amount.try_into().unwrap(),
            );
            assert_eq!(
                Err(TokenError::OwnerMismatch.into()),
                do_process_instruction(
                    withdraw(
                        &SWAP_PROGRAM_ID,
                        &TOKEN_PROGRAM_ID,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &pool_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &token_a_key,
                        &token_b_key,
                        withdraw_amount.try_into().unwrap(),
                        minimum_a_amount,
                        minimum_b_amount,
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut accounts.pool_mint_account,
                        &mut pool_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut token_a_account,
                        &mut token_b_account,
                        &mut accounts.pool_fee_account,
                        &mut Account::default(),
                    ],
                )
            );
        }

        // wrong token program id
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                initial_a,
                initial_b,
                withdraw_amount.try_into().unwrap(),
            );
            let wrong_key = Pubkey::new_unique();
            assert_eq!(
                Err(ProgramError::InvalidAccountData),
                do_process_instruction(
                    withdraw(
                        &SWAP_PROGRAM_ID,
                        &wrong_key,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &pool_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &token_a_key,
                        &token_b_key,
                        withdraw_amount.try_into().unwrap(),
                        minimum_a_amount,
                        minimum_b_amount,
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut accounts.pool_mint_account,
                        &mut pool_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut token_a_account,
                        &mut token_b_account,
                        &mut accounts.pool_fee_account,
                        &mut Account::default(),
                    ],
                )
            );
        }

        // wrong swap token accounts
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                initial_a,
                initial_b,
                initial_pool.try_into().unwrap(),
            );

            let old_a_key = accounts.token_a_key;
            let old_a_account = accounts.token_a_account;

            accounts.token_a_key = token_a_key;
            accounts.token_a_account = token_a_account.clone();

            // wrong swap token a account
            assert_eq!(
                Err(SwapError::IncorrectSwapAccount.into()),
                accounts.withdraw(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_a_amount,
                    minimum_b_amount,
                )
            );

            accounts.token_a_key = old_a_key;
            accounts.token_a_account = old_a_account;

            let old_b_key = accounts.token_b_key;
            let old_b_account = accounts.token_b_account;

            accounts.token_b_key = token_b_key;
            accounts.token_b_account = token_b_account.clone();

            // wrong swap token b account
            assert_eq!(
                Err(SwapError::IncorrectSwapAccount.into()),
                accounts.withdraw(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_a_amount,
                    minimum_b_amount,
                )
            );

            accounts.token_b_key = old_b_key;
            accounts.token_b_account = old_b_account;
        }

        // wrong mint
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                initial_a,
                initial_b,
                initial_pool.try_into().unwrap(),
            );
            let (pool_mint_key, pool_mint_account) =
                create_mint(&TOKEN_PROGRAM_ID, &accounts.authority_key, None);
            let old_pool_key = accounts.pool_mint_key;
            let old_pool_account = accounts.pool_mint_account;
            accounts.pool_mint_key = pool_mint_key;
            accounts.pool_mint_account = pool_mint_account;

            assert_eq!(
                Err(SwapError::IncorrectPoolMint.into()),
                accounts.withdraw(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_a_amount,
                    minimum_b_amount,
                )
            );

            accounts.pool_mint_key = old_pool_key;
            accounts.pool_mint_account = old_pool_account;
        }

        // withdrawing 1 pool token fails because it equates to 0 output tokens
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                initial_a,
                initial_b,
                initial_pool.try_into().unwrap(),
            );
            assert_eq!(
                Err(SwapError::ZeroTradingTokens.into()),
                accounts.withdraw(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    1,
                    minimum_a_amount * 10,
                    minimum_b_amount,
                )
            );
        }

        // slippage exceeded
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                initial_a,
                initial_b,
                initial_pool.try_into().unwrap(),
            );
            // minimum A amount out too high
            assert_eq!(
                Err(SwapError::ExceededSlippage.into()),
                accounts.withdraw(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_a_amount * 10,
                    minimum_b_amount,
                )
            );
            // minimum B amount out too high
            assert_eq!(
                Err(SwapError::ExceededSlippage.into()),
                accounts.withdraw(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_a_amount,
                    minimum_b_amount * 10,
                )
            );
        }

        // invalid input: can't use swap pool tokens as destination
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                initial_a,
                initial_b,
                initial_pool.try_into().unwrap(),
            );
            let swap_token_a_key = accounts.token_a_key;
            let mut swap_token_a_account = accounts.get_token_account(&swap_token_a_key).clone();
            assert_eq!(
                Err(SwapError::InvalidInput.into()),
                accounts.withdraw(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &swap_token_a_key,
                    &mut swap_token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_a_amount,
                    minimum_b_amount,
                )
            );
            let swap_token_b_key = accounts.token_b_key;
            let mut swap_token_b_account = accounts.get_token_account(&swap_token_b_key).clone();
            assert_eq!(
                Err(SwapError::InvalidInput.into()),
                accounts.withdraw(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &swap_token_b_key,
                    &mut swap_token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_a_amount,
                    minimum_b_amount,
                )
            );
        }

        // correct withdrawal
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                initial_a,
                initial_b,
                initial_pool.try_into().unwrap(),
            );

            accounts
                .withdraw(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_a_amount,
                    minimum_b_amount,
                )
                .unwrap();

            let swap_token_a =
                Processor::unpack_token_account(&accounts.token_a_account.data).unwrap();
            let swap_token_b =
                Processor::unpack_token_account(&accounts.token_b_account.data).unwrap();
            let pool_mint = Processor::unpack_mint(&accounts.pool_mint_account.data).unwrap();
            let withdraw_fee = accounts
                .swap_curve
                .calculator
                .owner_withdraw_fee(withdraw_amount)
                .unwrap();
            let withdrawn_a = accounts
                .swap_curve
                .calculator
                .pool_tokens_to_trading_tokens(
                    withdraw_amount - withdraw_fee,
                    pool_mint.supply.try_into().unwrap(),
                    swap_token_a.amount.try_into().unwrap(),
                )
                .unwrap();
            assert_eq!(
                swap_token_a.amount,
                token_a_amount - to_u64(withdrawn_a).unwrap()
            );
            let withdrawn_b = accounts
                .swap_curve
                .calculator
                .pool_tokens_to_trading_tokens(
                    withdraw_amount - withdraw_fee,
                    pool_mint.supply.try_into().unwrap(),
                    swap_token_b.amount.try_into().unwrap(),
                )
                .unwrap();
            assert_eq!(
                swap_token_b.amount,
                token_b_amount - to_u64(withdrawn_b).unwrap()
            );
            let token_a = Processor::unpack_token_account(&token_a_account.data).unwrap();
            assert_eq!(token_a.amount, initial_a + to_u64(withdrawn_a).unwrap());
            let token_b = Processor::unpack_token_account(&token_b_account.data).unwrap();
            assert_eq!(token_b.amount, initial_b + to_u64(withdrawn_b).unwrap());
            let pool_account = Processor::unpack_token_account(&pool_account.data).unwrap();
            assert_eq!(
                pool_account.amount,
                to_u64(initial_pool - withdraw_amount).unwrap()
            );
            let fee_account =
                Processor::unpack_token_account(&accounts.pool_fee_account.data).unwrap();
            assert_eq!(fee_account.amount, withdraw_fee.try_into().unwrap());
        }

        // correct withdrawal from fee account
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                mut _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &withdrawer_key, 0, 0, 0);

            let pool_fee_key = accounts.pool_fee_key;
            let mut pool_fee_account = accounts.pool_fee_account.clone();
            let fee_account = Processor::unpack_token_account(&pool_fee_account.data).unwrap();
            let pool_fee_amount = fee_account.amount;

            accounts
                .withdraw(
                    &user_key,
                    &pool_fee_key,
                    &mut pool_fee_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    pool_fee_amount,
                    0,
                    0,
                )
                .unwrap();

            let swap_token_a =
                Processor::unpack_token_account(&accounts.token_a_account.data).unwrap();
            let swap_token_b =
                Processor::unpack_token_account(&accounts.token_b_account.data).unwrap();
            let pool_mint = Processor::unpack_mint(&accounts.pool_mint_account.data).unwrap();
            let withdrawn_a = accounts
                .swap_curve
                .calculator
                .pool_tokens_to_trading_tokens(
                    pool_fee_amount.try_into().unwrap(),
                    pool_mint.supply.try_into().unwrap(),
                    swap_token_a.amount.try_into().unwrap(),
                )
                .unwrap();
            let token_a = Processor::unpack_token_account(&token_a_account.data).unwrap();
            assert_eq!(token_a.amount, withdrawn_a.try_into().unwrap());
            let withdrawn_b = accounts
                .swap_curve
                .calculator
                .pool_tokens_to_trading_tokens(
                    pool_fee_amount.try_into().unwrap(),
                    pool_mint.supply.try_into().unwrap(),
                    swap_token_b.amount.try_into().unwrap(),
                )
                .unwrap();
            let token_b = Processor::unpack_token_account(&token_b_account.data).unwrap();
            assert_eq!(token_b.amount, withdrawn_b.try_into().unwrap());
        }
    }

    fn check_valid_swap_curve(curve_type: CurveType, calculator: Box<dyn CurveCalculator>) {
        let user_key = Pubkey::new_unique();
        let swapper_key = Pubkey::new_unique();
        let token_a_amount = 10_000_000_000u64;
        let token_b_amount = 50_000_000_000u64;

        let swap_curve = SwapCurve {
            curve_type,
            calculator,
        };

        let mut accounts = SwapAccountInfo::new(
            &user_key,
            swap_curve.clone(),
            token_a_amount,
            token_b_amount,
        );
        let initial_a = token_a_amount / 5;
        let initial_b = token_b_amount / 5;
        accounts.initialize_swap().unwrap();

        let swap_token_a_key = accounts.token_a_key;
        let swap_token_b_key = accounts.token_b_key;

        let (
            token_a_key,
            mut token_a_account,
            token_b_key,
            mut token_b_account,
            _pool_key,
            _pool_account,
        ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);
        // swap one way
        let a_to_b_amount = initial_a / 10;
        let minimum_b_amount = 0;
        let pool_mint = Processor::unpack_mint(&accounts.pool_mint_account.data).unwrap();
        let initial_supply = pool_mint.supply;
        accounts
            .swap(
                &swapper_key,
                &token_a_key,
                &mut token_a_account,
                &swap_token_a_key,
                &swap_token_b_key,
                &token_b_key,
                &mut token_b_account,
                a_to_b_amount,
                minimum_b_amount,
            )
            .unwrap();

        let results = swap_curve
            .calculator
            .swap(
                a_to_b_amount.try_into().unwrap(),
                token_a_amount.try_into().unwrap(),
                token_b_amount.try_into().unwrap(),
            )
            .unwrap();

        let swap_token_a = Processor::unpack_token_account(&accounts.token_a_account.data).unwrap();
        let token_a_amount = swap_token_a.amount;
        assert_eq!(
            token_a_amount,
            results.new_source_amount.try_into().unwrap()
        );
        let token_a = Processor::unpack_token_account(&token_a_account.data).unwrap();
        assert_eq!(token_a.amount, initial_a - a_to_b_amount);

        let swap_token_b = Processor::unpack_token_account(&accounts.token_b_account.data).unwrap();
        let token_b_amount = swap_token_b.amount;
        assert_eq!(
            token_b_amount,
            results.new_destination_amount.try_into().unwrap()
        );
        let token_b = Processor::unpack_token_account(&token_b_account.data).unwrap();
        assert_eq!(
            token_b.amount,
            initial_b + to_u64(results.amount_swapped).unwrap()
        );

        let first_fee = swap_curve
            .calculator
            .owner_fee_to_pool_tokens(
                results.owner_fee,
                token_a_amount.try_into().unwrap(),
                initial_supply.try_into().unwrap(),
                TOKENS_IN_POOL.try_into().unwrap(),
            )
            .unwrap();
        let fee_account = Processor::unpack_token_account(&accounts.pool_fee_account.data).unwrap();
        assert_eq!(fee_account.amount, first_fee.try_into().unwrap());

        let first_swap_amount = results.amount_swapped;

        // swap the other way
        let pool_mint = Processor::unpack_mint(&accounts.pool_mint_account.data).unwrap();
        let initial_supply = pool_mint.supply;

        let b_to_a_amount = initial_b / 10;
        let minimum_a_amount = 0;
        accounts
            .swap(
                &swapper_key,
                &token_b_key,
                &mut token_b_account,
                &swap_token_b_key,
                &swap_token_a_key,
                &token_a_key,
                &mut token_a_account,
                b_to_a_amount,
                minimum_a_amount,
            )
            .unwrap();

        let results = swap_curve
            .calculator
            .swap(
                b_to_a_amount.try_into().unwrap(),
                token_b_amount.try_into().unwrap(),
                token_a_amount.try_into().unwrap(),
            )
            .unwrap();

        let swap_token_a = Processor::unpack_token_account(&accounts.token_a_account.data).unwrap();
        let token_a_amount = swap_token_a.amount;
        assert_eq!(
            token_a_amount,
            results.new_destination_amount.try_into().unwrap()
        );
        let token_a = Processor::unpack_token_account(&token_a_account.data).unwrap();
        assert_eq!(
            token_a.amount,
            initial_a - a_to_b_amount + to_u64(results.amount_swapped).unwrap()
        );

        let swap_token_b = Processor::unpack_token_account(&accounts.token_b_account.data).unwrap();
        let token_b_amount = swap_token_b.amount;
        assert_eq!(
            token_b_amount,
            results.new_source_amount.try_into().unwrap()
        );
        let token_b = Processor::unpack_token_account(&token_b_account.data).unwrap();
        assert_eq!(
            token_b.amount,
            initial_b + to_u64(first_swap_amount).unwrap() - b_to_a_amount
        );

        let second_fee = swap_curve
            .calculator
            .owner_fee_to_pool_tokens(
                results.owner_fee,
                token_b_amount.try_into().unwrap(),
                initial_supply.try_into().unwrap(),
                TOKENS_IN_POOL.try_into().unwrap(),
            )
            .unwrap();
        let fee_account = Processor::unpack_token_account(&accounts.pool_fee_account.data).unwrap();
        assert_eq!(fee_account.amount, to_u64(first_fee + second_fee).unwrap());
    }

    #[test]
    fn test_valid_swap_curves() {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 10;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 30;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 30;
        let host_fee_numerator = 20;
        let host_fee_denominator = 100;
        check_valid_swap_curve(
            CurveType::ConstantProduct,
            Box::new(ConstantProductCurve {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
                host_fee_numerator,
                host_fee_denominator,
            }),
        );
        check_valid_swap_curve(
            CurveType::Flat,
            Box::new(FlatCurve {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
                host_fee_numerator,
                host_fee_denominator,
            }),
        );
    }

    #[test]
    fn test_valid_swap_with_fee_constraints() {
        let owner_key = Pubkey::new_unique();

        let trade_fee_numerator = 1;
        let trade_fee_denominator = 10;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 30;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 30;
        let host_fee_numerator = 10;
        let host_fee_denominator = 100;

        let token_a_amount = 1_000_000;
        let token_b_amount = 5_000_000;

        let curve = ConstantProductCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };
        let swap_curve = SwapCurve {
            curve_type: CurveType::ConstantProduct,
            calculator: Box::new(curve.clone()),
        };

        let owner_key_str = &owner_key.to_string();
        let valid_constant_product_curves = &[curve];
        let constraints = Some(FeeConstraints {
            owner_key: owner_key_str,
            valid_constant_product_curves,
            valid_flat_curves: &[],
        });
        let mut accounts =
            SwapAccountInfo::new(&owner_key, swap_curve, token_a_amount, token_b_amount);

        // initialize swap
        do_process_instruction_with_fee_constraints(
            initialize(
                &SWAP_PROGRAM_ID,
                &TOKEN_PROGRAM_ID,
                &accounts.swap_key,
                &accounts.authority_key,
                &accounts.token_a_key,
                &accounts.token_b_key,
                &accounts.pool_mint_key,
                &accounts.pool_fee_key,
                &accounts.pool_token_key,
                accounts.nonce,
                accounts.swap_curve.clone(),
            )
            .unwrap(),
            vec![
                &mut accounts.swap_account,
                &mut Account::default(),
                &mut accounts.token_a_account,
                &mut accounts.token_b_account,
                &mut accounts.pool_mint_account,
                &mut accounts.pool_fee_account,
                &mut accounts.pool_token_account,
                &mut Account::default(),
            ],
            &constraints,
        )
        .unwrap();

        let authority_key = accounts.authority_key;

        let (
            token_a_key,
            mut token_a_account,
            token_b_key,
            mut token_b_account,
            pool_key,
            mut pool_account,
        ) = accounts.setup_token_accounts(
            &owner_key,
            &authority_key,
            token_a_amount,
            token_b_amount,
            0,
        );

        let amount_in = token_a_amount / 2;
        let minimum_amount_out = 0;

        // perform the swap
        do_process_instruction_with_fee_constraints(
            swap(
                &SWAP_PROGRAM_ID,
                &TOKEN_PROGRAM_ID,
                &accounts.swap_key,
                &accounts.authority_key,
                &token_a_key,
                &accounts.token_a_key,
                &accounts.token_b_key,
                &token_b_key,
                &accounts.pool_mint_key,
                &accounts.pool_fee_key,
                Some(&pool_key),
                amount_in,
                minimum_amount_out,
            )
            .unwrap(),
            vec![
                &mut accounts.swap_account,
                &mut Account::default(),
                &mut token_a_account,
                &mut accounts.token_a_account,
                &mut accounts.token_b_account,
                &mut token_b_account,
                &mut accounts.pool_mint_account,
                &mut accounts.pool_fee_account,
                &mut Account::default(),
                &mut pool_account,
            ],
            &constraints,
        )
        .unwrap();

        // check that fees were taken in the host fee account
        let host_fee_account = Processor::unpack_token_account(&pool_account.data).unwrap();
        let owner_fee_account =
            Processor::unpack_token_account(&accounts.pool_fee_account.data).unwrap();
        let total_fee = host_fee_account.amount * host_fee_denominator / host_fee_numerator;
        assert_eq!(
            total_fee,
            host_fee_account.amount + owner_fee_account.amount
        );
    }

    #[test]
    fn test_invalid_swap() {
        let user_key = Pubkey::new_unique();
        let swapper_key = Pubkey::new_unique();
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 4;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 10;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 5;
        let host_fee_numerator = 9;
        let host_fee_denominator = 100;

        let token_a_amount = 1000;
        let token_b_amount = 5000;
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Box::new(ConstantProductCurve {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
                host_fee_numerator,
                host_fee_denominator,
            }),
        };
        let mut accounts =
            SwapAccountInfo::new(&user_key, swap_curve, token_a_amount, token_b_amount);

        let initial_a = token_a_amount / 5;
        let initial_b = token_b_amount / 5;
        let minimum_b_amount = initial_b / 2;

        let swap_token_a_key = accounts.token_a_key;
        let swap_token_b_key = accounts.token_b_key;

        // swap not initialized
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);
            assert_eq!(
                Err(ProgramError::UninitializedAccount),
                accounts.swap(
                    &swapper_key,
                    &token_a_key,
                    &mut token_a_account,
                    &swap_token_a_key,
                    &swap_token_b_key,
                    &token_b_key,
                    &mut token_b_account,
                    initial_a,
                    minimum_b_amount,
                )
            );
        }

        accounts.initialize_swap().unwrap();

        // wrong nonce
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);
            let old_authority = accounts.authority_key;
            let (bad_authority_key, _nonce) = Pubkey::find_program_address(
                &[&accounts.swap_key.to_bytes()[..]],
                &TOKEN_PROGRAM_ID,
            );
            accounts.authority_key = bad_authority_key;
            assert_eq!(
                Err(SwapError::InvalidProgramAddress.into()),
                accounts.swap(
                    &swapper_key,
                    &token_a_key,
                    &mut token_a_account,
                    &swap_token_a_key,
                    &swap_token_b_key,
                    &token_b_key,
                    &mut token_b_account,
                    initial_a,
                    minimum_b_amount,
                )
            );
            accounts.authority_key = old_authority;
        }

        // wrong token program id
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);
            let wrong_program_id = Pubkey::new_unique();
            assert_eq!(
                Err(ProgramError::InvalidAccountData),
                do_process_instruction(
                    swap(
                        &SWAP_PROGRAM_ID,
                        &wrong_program_id,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &token_a_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        None,
                        initial_a,
                        minimum_b_amount,
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut token_a_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut Account::default(),
                    ],
                ),
            );
        }

        // not enough token a to swap
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);
            assert_eq!(
                Err(TokenError::InsufficientFunds.into()),
                accounts.swap(
                    &swapper_key,
                    &token_a_key,
                    &mut token_a_account,
                    &swap_token_a_key,
                    &swap_token_b_key,
                    &token_b_key,
                    &mut token_b_account,
                    initial_a * 2,
                    minimum_b_amount * 2,
                )
            );
        }

        // wrong swap token A / B accounts
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);
            assert_eq!(
                Err(SwapError::IncorrectSwapAccount.into()),
                do_process_instruction(
                    swap(
                        &SWAP_PROGRAM_ID,
                        &TOKEN_PROGRAM_ID,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &token_a_key,
                        &token_a_key,
                        &token_b_key,
                        &token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        None,
                        initial_a,
                        minimum_b_amount,
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut token_a_account.clone(),
                        &mut token_a_account,
                        &mut token_b_account.clone(),
                        &mut token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut Account::default(),
                    ],
                ),
            );
        }

        // wrong user token A / B accounts
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);
            assert_eq!(
                Err(TokenError::MintMismatch.into()),
                accounts.swap(
                    &swapper_key,
                    &token_b_key,
                    &mut token_b_account,
                    &swap_token_a_key,
                    &swap_token_b_key,
                    &token_a_key,
                    &mut token_a_account,
                    initial_a,
                    minimum_b_amount,
                )
            );
        }

        // swap from a to a
        {
            let (
                token_a_key,
                mut token_a_account,
                _token_b_key,
                _token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);
            assert_eq!(
                Err(SwapError::InvalidInput.into()),
                accounts.swap(
                    &swapper_key,
                    &token_a_key,
                    &mut token_a_account.clone(),
                    &swap_token_a_key,
                    &swap_token_a_key,
                    &token_a_key,
                    &mut token_a_account,
                    initial_a,
                    minimum_b_amount,
                )
            );
        }

        // incorrect mint provided
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);
            let (pool_mint_key, pool_mint_account) =
                create_mint(&TOKEN_PROGRAM_ID, &accounts.authority_key, None);
            let old_pool_key = accounts.pool_mint_key;
            let old_pool_account = accounts.pool_mint_account;
            accounts.pool_mint_key = pool_mint_key;
            accounts.pool_mint_account = pool_mint_account;

            assert_eq!(
                Err(SwapError::IncorrectPoolMint.into()),
                accounts.swap(
                    &swapper_key,
                    &token_a_key,
                    &mut token_a_account,
                    &swap_token_a_key,
                    &swap_token_b_key,
                    &token_b_key,
                    &mut token_b_account,
                    initial_a,
                    minimum_b_amount,
                )
            );

            accounts.pool_mint_key = old_pool_key;
            accounts.pool_mint_account = old_pool_account;
        }

        // incorrect fee account provided
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                wrong_pool_key,
                wrong_pool_account,
            ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);
            let old_pool_fee_account = accounts.pool_fee_account;
            let old_pool_fee_key = accounts.pool_fee_key;
            accounts.pool_fee_account = wrong_pool_account;
            accounts.pool_fee_key = wrong_pool_key;
            assert_eq!(
                Err(SwapError::IncorrectFeeAccount.into()),
                accounts.swap(
                    &swapper_key,
                    &token_a_key,
                    &mut token_a_account,
                    &swap_token_a_key,
                    &swap_token_b_key,
                    &token_b_key,
                    &mut token_b_account,
                    initial_a,
                    minimum_b_amount,
                )
            );
            accounts.pool_fee_account = old_pool_fee_account;
            accounts.pool_fee_key = old_pool_fee_key;
        }

        // no approval
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);
            assert_eq!(
                Err(TokenError::OwnerMismatch.into()),
                do_process_instruction(
                    swap(
                        &SWAP_PROGRAM_ID,
                        &TOKEN_PROGRAM_ID,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &token_a_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        None,
                        initial_a,
                        minimum_b_amount,
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut token_a_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut Account::default(),
                    ],
                ),
            );
        }

        // output token value 0
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);
            assert_eq!(
                Err(SwapError::ZeroTradingTokens.into()),
                accounts.swap(
                    &swapper_key,
                    &token_b_key,
                    &mut token_b_account,
                    &swap_token_b_key,
                    &swap_token_a_key,
                    &token_a_key,
                    &mut token_a_account,
                    1,
                    1,
                )
            );
        }

        // slippage exceeded: minimum out amount too high
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);
            assert_eq!(
                Err(SwapError::ExceededSlippage.into()),
                accounts.swap(
                    &swapper_key,
                    &token_a_key,
                    &mut token_a_account,
                    &swap_token_a_key,
                    &swap_token_b_key,
                    &token_b_key,
                    &mut token_b_account,
                    initial_a,
                    minimum_b_amount * 2,
                )
            );
        }

        // invalid input: can't use swap pool as user source / dest
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);
            let mut swap_token_a_account = accounts.get_token_account(&swap_token_a_key).clone();
            let authority_key = accounts.authority_key;
            assert_eq!(
                Err(SwapError::InvalidInput.into()),
                accounts.swap(
                    &authority_key,
                    &swap_token_a_key,
                    &mut swap_token_a_account,
                    &swap_token_a_key,
                    &swap_token_b_key,
                    &token_b_key,
                    &mut token_b_account,
                    initial_a,
                    minimum_b_amount,
                )
            );
            let mut swap_token_b_account = accounts.get_token_account(&swap_token_b_key).clone();
            assert_eq!(
                Err(SwapError::InvalidInput.into()),
                accounts.swap(
                    &swapper_key,
                    &token_a_key,
                    &mut token_a_account,
                    &swap_token_a_key,
                    &swap_token_b_key,
                    &swap_token_b_key,
                    &mut swap_token_b_account,
                    initial_a,
                    minimum_b_amount,
                )
            );
        }

        // still correct: constraint specified, no host fee account
        {
            let authority_key = accounts.authority_key;
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &authority_key, initial_a, initial_b, 0);
            let owner_key = &swapper_key.to_string();
            let constraints = Some(FeeConstraints {
                owner_key,
                valid_constant_product_curves: &[],
                valid_flat_curves: &[],
            });
            do_process_instruction_with_fee_constraints(
                swap(
                    &SWAP_PROGRAM_ID,
                    &TOKEN_PROGRAM_ID,
                    &accounts.swap_key,
                    &accounts.authority_key,
                    &token_a_key,
                    &accounts.token_a_key,
                    &accounts.token_b_key,
                    &token_b_key,
                    &accounts.pool_mint_key,
                    &accounts.pool_fee_key,
                    None,
                    initial_a,
                    minimum_b_amount,
                )
                .unwrap(),
                vec![
                    &mut accounts.swap_account,
                    &mut Account::default(),
                    &mut token_a_account,
                    &mut accounts.token_a_account,
                    &mut accounts.token_b_account,
                    &mut token_b_account,
                    &mut accounts.pool_mint_account,
                    &mut accounts.pool_fee_account,
                    &mut Account::default(),
                ],
                &constraints,
            )
            .unwrap();
        }

        // invalid mint for host fee account
        {
            let authority_key = accounts.authority_key;
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &authority_key, initial_a, initial_b, 0);
            let (
                bad_token_a_key,
                mut bad_token_a_account,
                _token_b_key,
                mut _token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &authority_key, initial_a, initial_b, 0);
            let owner_key = &swapper_key.to_string();
            let constraints = Some(FeeConstraints {
                owner_key,
                valid_constant_product_curves: &[],
                valid_flat_curves: &[],
            });
            assert_eq!(
                Err(SwapError::IncorrectPoolMint.into()),
                do_process_instruction_with_fee_constraints(
                    swap(
                        &SWAP_PROGRAM_ID,
                        &TOKEN_PROGRAM_ID,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &token_a_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        Some(&bad_token_a_key),
                        initial_a,
                        0,
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut token_a_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut Account::default(),
                        &mut bad_token_a_account,
                    ],
                    &constraints,
                ),
            );
        }
    }
}
