//! Program state processor

use crate::constraints::{SwapConstraints, SWAP_CONSTRAINTS};
use crate::{
    curve::{
        base::{SwapCurve, SwapResult},
        calculator::{RoundDirection, TradeDirection},
        fees::Fees,
    },
    error::SwapError,
    instruction::{
        swap_flags, DepositAllTokenTypes, DepositSingleTokenTypeExactAmountIn, DeregisterPool,
        Initialize, Swap, SwapInstruction, WithdrawAllTokenTypes,
        WithdrawSingleTokenTypeExactAmountOut,
    },
    state::{PoolRegistry, SwapState, SwapV1, SwapVersion},
};
use num_traits::FromPrimitive;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::{PrintProgramError, ProgramError},
    program_option::COption,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};
use spl_associated_token_account::get_associated_token_address;
use std::convert::TryInto;

/// For unit testing, we need to use a owner key when generating ATAs.
/// This matches the one in the unit test
#[cfg(not(feature = "production"))]
pub const TEST_OWNER_KEY: &str = "5Cebzty8iwgAUx9jyfZVAT2iMvXBECLwEVgT6T8KYmvS";

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Unpacks a spl_token `Account`.
    pub fn unpack_token_account(
        account_info: &AccountInfo,
        token_program_id: &Pubkey,
    ) -> Result<spl_token::state::Account, SwapError> {
        if account_info.owner != token_program_id {
            Err(SwapError::IncorrectTokenProgramId)
        } else {
            spl_token::state::Account::unpack(&account_info.data.borrow())
                .map_err(|_| SwapError::ExpectedAccount)
        }
    }

    /// Unpacks a spl_token `Mint`.
    pub fn unpack_mint(
        account_info: &AccountInfo,
        token_program_id: &Pubkey,
    ) -> Result<spl_token::state::Mint, SwapError> {
        if account_info.owner != token_program_id {
            Err(SwapError::IncorrectTokenProgramId)
        } else {
            spl_token::state::Mint::unpack(&account_info.data.borrow())
                .map_err(|_| SwapError::ExpectedMint)
        }
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

    #[allow(clippy::too_many_arguments)]
    fn check_accounts(
        token_swap: &dyn SwapState,
        program_id: &Pubkey,
        swap_account_info: &AccountInfo,
        authority_info: &AccountInfo,
        token_a_info: &AccountInfo,
        token_b_info: &AccountInfo,
        pool_mint_info: &AccountInfo,
        token_program_info: &AccountInfo,
        user_token_a_info: Option<&AccountInfo>,
        user_token_b_info: Option<&AccountInfo>,
        pool_fee_account_info: Option<&AccountInfo>,
    ) -> ProgramResult {
        if swap_account_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        if *authority_info.key
            != Self::authority_id(program_id, swap_account_info.key, token_swap.nonce())?
        {
            return Err(SwapError::InvalidProgramAddress.into());
        }
        if *token_a_info.key != *token_swap.token_a_account() {
            return Err(SwapError::IncorrectSwapAccount.into());
        }
        if *token_b_info.key != *token_swap.token_b_account() {
            return Err(SwapError::IncorrectSwapAccount.into());
        }
        if *pool_mint_info.key != *token_swap.pool_mint() {
            return Err(SwapError::IncorrectPoolMint.into());
        }
        if *token_program_info.key != *token_swap.token_program_id() {
            return Err(SwapError::IncorrectTokenProgramId.into());
        }
        if let Some(user_token_a_info) = user_token_a_info {
            if token_a_info.key == user_token_a_info.key {
                return Err(SwapError::InvalidInput.into());
            }
        }
        if let Some(user_token_b_info) = user_token_b_info {
            if token_b_info.key == user_token_b_info.key {
                return Err(SwapError::InvalidInput.into());
            }
        }
        if let Some(pool_fee_account_info) = pool_fee_account_info {
            if *pool_fee_account_info.key != *token_swap.pool_fee_account() {
                return Err(SwapError::IncorrectFeeAccount.into());
            }
        }
        Ok(())
    }

    /// Processes an [Initialize](enum.Instruction.html).
    pub fn process_initialize(
        program_id: &Pubkey,
        nonce: u8,
        fees: Fees,
        swap_curve: SwapCurve,
        pool_nonce: u8,
        accounts: &[AccountInfo],
        swap_constraints: &Option<SwapConstraints>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let payer_info = next_account_info(account_info_iter)?;
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let token_a_info = next_account_info(account_info_iter)?;
        let token_b_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let fee_account_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let pool_registry_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        let mut pool_registry = PoolRegistry::load(pool_registry_info, program_id)?;
        if !pool_registry.is_initialized {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        let token_program_id = *token_program_info.key;
        if SwapVersion::is_initialized(&swap_info.data.borrow()) {
            return Err(SwapError::AlreadyInUse.into());
        }

        if *authority_info.key != Self::authority_id(program_id, swap_info.key, nonce)? {
            return Err(SwapError::InvalidProgramAddress.into());
        }

        let token_a = Self::unpack_token_account(token_a_info, &token_program_id)?;
        let token_b = Self::unpack_token_account(token_b_info, &token_program_id)?;
        let fee_account = Self::unpack_token_account(fee_account_info, &token_program_id)?;
        let destination = Self::unpack_token_account(destination_info, &token_program_id)?;
        let pool_mint = Self::unpack_mint(pool_mint_info, &token_program_id)?;

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
        swap_curve
            .calculator
            .validate_supply(token_a.amount, token_b.amount)?;
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

        let mut seed_key_vec = vec![token_a.mint.to_bytes(), token_b.mint.to_bytes()];
        seed_key_vec.sort_unstable();

        // Although this is less efficient, it prevents a malicious attacker providing a nonce
        // that produces a different valid address, allowing them to inject duplicate/fake pools.
        let (pool_pda, pool_pda_nonce) = Pubkey::find_program_address(
            &[
                &seed_key_vec[0][..32],
                &seed_key_vec[1][..32],
                &[swap_curve.curve_type as u8],
            ],
            program_id,
        );

        if *swap_info.key != pool_pda || pool_nonce != pool_pda_nonce {
            return Err(SwapError::InvalidProgramAddress.into());
        }

        let pool_signer_seeds: &[&[_]] = &[&[
            &seed_key_vec[0][..32],
            &seed_key_vec[1][..32],
            &[swap_curve.curve_type as u8],
            &[pool_nonce],
        ]];

        if let Some(swap_constraints) = swap_constraints {
            let owner_key = swap_constraints
                .owner_key
                .parse::<Pubkey>()
                .map_err(|_| SwapError::InvalidOwner)?;
            if fee_account.owner != owner_key {
                return Err(SwapError::InvalidOwner.into());
            }

            let required_mint = swap_constraints
                .required_mint
                .parse::<Pubkey>()
                .map_err(|_| SwapError::InvalidMint)?;
            if token_a.mint != required_mint && token_b.mint != required_mint {
                return Err(SwapError::InvalidMint.into());
            }

            swap_constraints.validate_curve(&swap_curve)?;
            swap_constraints.validate_fees(&fees)?;
        }
        fees.validate()?;
        swap_curve.calculator.validate()?;

        invoke_signed(
            &system_instruction::create_account(
                payer_info.key,
                &pool_pda,
                1.max(rent.minimum_balance(SwapVersion::LATEST_LEN)),
                SwapVersion::LATEST_LEN as u64,
                program_id,
            ),
            &[
                payer_info.clone(),
                swap_info.clone(),
                system_program_info.clone(),
            ],
            pool_signer_seeds,
        )?;

        pool_registry.append(&pool_pda);

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

        let obj = SwapVersion::SwapV1(SwapV1 {
            is_initialized: true,
            nonce,
            token_program_id,
            token_a: *token_a_info.key,
            token_b: *token_b_info.key,
            pool_mint: *pool_mint_info.key,
            token_a_mint: token_a.mint,
            token_b_mint: token_b.mint,
            pool_fee_account: *fee_account_info.key,
            fees,
            swap_curve,
            pool_nonce,
        });
        SwapVersion::pack(obj, &mut swap_info.data.borrow_mut())?;
        Ok(())
    }

    /// Processes an [RoutedSwap](enum.Instruction.html).
    pub fn process_routed_swap(
        program_id: &Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        flags: u8,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        //we cut the owner fees when routing through pools
        const ROUTED_OWNER_FEE_NUMERATOR_MULT: u64 = 6;
        const ROUTED_OWNER_FEE_DENOMINATOR_MULT: u64 = 10;

        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let swap_source_info = next_account_info(account_info_iter)?;
        let swap_destination_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let pool_fee_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let swap_info2 = next_account_info(account_info_iter)?;
        let authority_info2 = next_account_info(account_info_iter)?;
        let source_info2 = destination_info; //source is prior destination
        let swap_source_info2 = next_account_info(account_info_iter)?;
        let swap_destination_info2 = next_account_info(account_info_iter)?;
        let destination_info2 = next_account_info(account_info_iter)?;
        let pool_mint_info2 = next_account_info(account_info_iter)?;
        let pool_fee_account_info2 = next_account_info(account_info_iter)?;
        let refund_account_info = next_account_info(account_info_iter)?;

        let token_b = Self::unpack_token_account(destination_info, token_program_info.key)?;
        if token_b.amount > 0 {
            return Err(SwapError::RoutedSwapRequiresEmptyIntermediary.into());
        }

        //we could knock the owner fee in half since its a double swap
        if swap_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        let token_swap = SwapVersion::unpack(&swap_info.data.borrow())?;
        let new_numerator = token_swap
            .fees()
            .owner_trade_fee_numerator
            .checked_mul(ROUTED_OWNER_FEE_NUMERATOR_MULT)
            .unwrap();
        let new_denominator = token_swap
            .fees()
            .owner_trade_fee_denominator
            .checked_mul(ROUTED_OWNER_FEE_DENOMINATOR_MULT)
            .unwrap();

        let swap_result1 = Self::process_swap_internal(
            true,
            flags & swap_flags::CLOSE_OUTPUT > 0,
            flags & swap_flags::CLOSE_INPUT > 0,
            program_id,
            amount_in,
            //we frankly don't care how much this swaps, we'll do the min out check on the second swap
            0,
            swap_info,
            authority_info,
            user_transfer_authority_info,
            source_info,
            swap_source_info,
            swap_destination_info,
            destination_info,
            pool_mint_info,
            pool_fee_account_info,
            Some(refund_account_info),
            token_program_info,
            Some((new_numerator, new_denominator)),
        )?;

        msg!("first swap: {:?}", swap_result1);

        //second swap

        let swap_result2 = Self::process_swap_internal(
            true,
            flags & swap_flags::CLOSE_OUTPUT_2 > 0,
            flags & swap_flags::CLOSE_INPUT_2 > 0,
            program_id,
            //amount of swap1 out becomes swap 2 in
            swap_result1.destination_amount_swapped.try_into().unwrap(),
            //this is where the slippage checks would take hold
            minimum_amount_out,
            swap_info2,
            authority_info2,
            user_transfer_authority_info,
            source_info2,
            swap_source_info2,
            swap_destination_info2,
            destination_info2,
            pool_mint_info2,
            pool_fee_account_info2,
            Some(refund_account_info),
            token_program_info,
            //None,
            Some((new_numerator, new_denominator)),
        )?;

        msg!("second swap: {:?}", swap_result2);

        Ok(())
    }

    /// Processes an [Swap](enum.Instruction.html).
    pub fn process_swap(
        program_id: &Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        flags: u8,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let swap_source_info = next_account_info(account_info_iter)?;
        let swap_destination_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let pool_fee_account_info = next_account_info(account_info_iter)?;
        let refund_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let _swap_result = Self::process_swap_internal(
            false,
            flags & swap_flags::CLOSE_OUTPUT > 0,
            flags & swap_flags::CLOSE_INPUT > 0,
            program_id,
            amount_in,
            minimum_amount_out,
            swap_info,
            authority_info,
            user_transfer_authority_info,
            source_info,
            swap_source_info,
            swap_destination_info,
            destination_info,
            pool_mint_info,
            pool_fee_account_info,
            Some(refund_account_info),
            token_program_info,
            None,
        )?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn process_swap_internal<'a>(
        collect_dust: bool,
        output_unwrap: bool,
        input_unwrap: bool,
        program_id: &Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        swap_info: &AccountInfo<'a>,
        authority_info: &AccountInfo<'a>,
        user_transfer_authority_info: &AccountInfo<'a>,
        source_info: &AccountInfo<'a>,
        swap_source_info: &AccountInfo<'a>,
        swap_destination_info: &AccountInfo<'a>,
        destination_info: &AccountInfo<'a>,
        pool_mint_info: &AccountInfo<'a>,
        pool_fee_account_info: &AccountInfo<'a>,
        refund_account_info: Option<&AccountInfo<'a>>,
        token_program_info: &AccountInfo<'a>,
        owner_trade_fee_numerator_denominator_override: Option<(u64, u64)>,
    ) -> Result<SwapResult, ProgramError> {
        if swap_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        let token_swap = SwapVersion::unpack(&swap_info.data.borrow())?;

        if *authority_info.key != Self::authority_id(program_id, swap_info.key, token_swap.nonce())?
        {
            return Err(SwapError::InvalidProgramAddress.into());
        }
        if !(*swap_source_info.key == *token_swap.token_a_account()
            || *swap_source_info.key == *token_swap.token_b_account())
        {
            return Err(SwapError::IncorrectSwapAccount.into());
        }
        if !(*swap_destination_info.key == *token_swap.token_a_account()
            || *swap_destination_info.key == *token_swap.token_b_account())
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
        if *pool_mint_info.key != *token_swap.pool_mint() {
            return Err(SwapError::IncorrectPoolMint.into());
        }
        if *pool_fee_account_info.key != *token_swap.pool_fee_account() {
            return Err(SwapError::IncorrectFeeAccount.into());
        }
        if *token_program_info.key != *token_swap.token_program_id() {
            return Err(SwapError::IncorrectTokenProgramId.into());
        }

        let mut fees = token_swap.fees().clone();
        if let Some(nd) = owner_trade_fee_numerator_denominator_override {
            fees.owner_trade_fee_numerator = nd.0;
            fees.owner_trade_fee_denominator = nd.1;
        }

        let source_account =
            Self::unpack_token_account(swap_source_info, token_swap.token_program_id())?;
        let dest_account =
            Self::unpack_token_account(swap_destination_info, token_swap.token_program_id())?;
        let pool_mint = Self::unpack_mint(pool_mint_info, token_swap.token_program_id())?;

        let trade_direction = if *swap_source_info.key == *token_swap.token_a_account() {
            TradeDirection::AtoB
        } else {
            TradeDirection::BtoA
        };
        let amount_in_u128 = to_u128(amount_in)?;
        let mut result = token_swap
            .swap_curve()
            .swap(
                amount_in_u128,
                to_u128(source_account.amount)?,
                to_u128(dest_account.amount)?,
                trade_direction,
                &fees,
            )
            .ok_or(SwapError::ZeroTradingTokens)?;

        if collect_dust {
            //note that if this was off significantly, the output slippage
            //would be triggered
            result.source_amount_swapped = amount_in_u128;
        }
        //unmut
        let result = result;

        if result.destination_amount_swapped < to_u128(minimum_amount_out)? {
            return Err(SwapError::ExceededSlippage.into());
        }

        let (swap_token_a_amount, swap_token_b_amount) = match trade_direction {
            TradeDirection::AtoB => (
                result.new_swap_source_amount,
                result.new_swap_destination_amount,
            ),
            TradeDirection::BtoA => (
                result.new_swap_destination_amount,
                result.new_swap_source_amount,
            ),
        };

        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            source_info.clone(),
            swap_source_info.clone(),
            user_transfer_authority_info.clone(),
            token_swap.nonce(),
            to_u64(result.source_amount_swapped)?,
        )?;

        let pool_token_amount = token_swap
            .swap_curve()
            .withdraw_single_token_type_exact_out(
                result.owner_fee,
                swap_token_a_amount,
                swap_token_b_amount,
                to_u128(pool_mint.supply)?,
                trade_direction,
                &fees,
            )
            .ok_or(SwapError::FeeCalculationFailure)?;

        //if the pool fee account doesn't exist, we have to resort to no fees
        let valid_fee_account =
            Self::unpack_token_account(pool_fee_account_info, token_program_info.key).is_ok();
        if !valid_fee_account {
            msg!("cannot pay fees to {}", pool_fee_account_info.key);
        //otherwise, pay fees if there are any
        } else if pool_token_amount > 0 {
            Self::token_mint_to(
                swap_info.key,
                token_program_info.clone(),
                pool_mint_info.clone(),
                pool_fee_account_info.clone(),
                authority_info.clone(),
                token_swap.nonce(),
                to_u64(pool_token_amount)?,
            )?;
        }

        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            swap_destination_info.clone(),
            destination_info.clone(),
            authority_info.clone(),
            token_swap.nonce(),
            to_u64(result.destination_amount_swapped)?,
        )?;

        //some checks to prevent stranded token and unwrap sol

        //CHECK FOR INPUT CLOSING CAPABILITY
        //this could be wrapped sol temp account, or second swap of a route
        //in both cases the token would be owned by the xfer auth, which shouldn't be the refundee
        if input_unwrap {
            let token_a = Self::unpack_token_account(source_info, token_program_info.key)?;
            let owner_is_refundee = match refund_account_info {
                None => false,
                Some(r) => *r.key == token_a.owner,
            };
            //if we have permission to close out this account
            if token_a.owner == *user_transfer_authority_info.key {
                //if the owner isn't the refundee then this must be a temp account
                //A non-native temp account on the input should always be left empty so we can close it
                //this is stopping a caller from shooting themselves in the foot
                if !owner_is_refundee && token_a.amount > 0 && !token_a.is_native() {
                    return Err(SwapError::NonRefundeeTransferAuthorityNotEmpty.into());
                }

                //if empty, we close it
                if token_a.amount == 0 || token_a.is_native() {
                    if refund_account_info.is_none() {
                        return Err(SwapError::TransferAuthorityOwnsButRefundeeNotProvided.into());
                    }

                    let refund = refund_account_info.unwrap();
                    invoke(
                        &spl_token::instruction::close_account(
                            token_program_info.key,
                            source_info.key,
                            refund.key,
                            user_transfer_authority_info.key,
                            &[],
                        )?,
                        &[
                            source_info.clone(),
                            refund.clone(),
                            user_transfer_authority_info.clone(),
                        ],
                    )?;
                }
            //we could have closed but authority was set wrong, just leave a message
            } else if token_a.is_native() {
                msg!("couldn't close input native token account automatically; user transfer authority must own");
            } else if token_a.amount == 0 {
                msg!("couldn't close input empty token account automatically; user transfer authority must own");
            }
        }

        //CHECK FOR OUTPUT CLOSING CAPABILITY
        //this is only for output accounts that are wrapped SOL
        //when that is the case, we can close them and send the SOL to the refundee
        if output_unwrap {
            let token_b = Self::unpack_token_account(destination_info, token_program_info.key)?;

            let owner_is_refundee = match refund_account_info {
                None => false,
                Some(r) => *r.key == token_b.owner,
            };

            //if we can potentially close out this account
            if token_b.owner == *user_transfer_authority_info.key {
                //this is a safety check. if the owner isn't the refundee then this
                //must be a temp authority. A temp authority on the output should ONLY
                //be used for WSOL
                if !owner_is_refundee && !token_b.is_native() {
                    return Err(SwapError::NonRefundeeTransferAuthorityOwnsNonNative.into());
                }

                if token_b.is_native() {
                    if refund_account_info.is_none() {
                        return Err(SwapError::TransferAuthorityOwnsButRefundeeNotProvided.into());
                    }
                    let refund = refund_account_info.unwrap();
                    invoke(
                        &spl_token::instruction::close_account(
                            token_program_info.key,
                            destination_info.key,
                            refund.key,
                            user_transfer_authority_info.key,
                            &[],
                        )?,
                        &[
                            destination_info.clone(),
                            refund.clone(),
                            user_transfer_authority_info.clone(),
                        ],
                    )?;
                }
            //we should have closed but authority was set wrong, just leave a message?
            } else if token_b.is_native() {
                msg!("couldn't close native token; user transfer authority must own");
            }
        }

        Ok(result)
    }

    /// Processes an [DepositAllTokenTypes](enum.Instruction.html).
    pub fn process_deposit_all_token_types(
        program_id: &Pubkey,
        pool_token_amount: u64,
        maximum_token_a_amount: u64,
        maximum_token_b_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let source_a_info = next_account_info(account_info_iter)?;
        let source_b_info = next_account_info(account_info_iter)?;
        let token_a_info = next_account_info(account_info_iter)?;
        let token_b_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let dest_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapVersion::unpack(&swap_info.data.borrow())?;
        let calculator = &token_swap.swap_curve().calculator;
        if !calculator.allows_deposits() {
            return Err(SwapError::UnsupportedCurveOperation.into());
        }
        Self::check_accounts(
            token_swap.as_ref(),
            program_id,
            swap_info,
            authority_info,
            token_a_info,
            token_b_info,
            pool_mint_info,
            token_program_info,
            Some(source_a_info),
            Some(source_b_info),
            None,
        )?;

        let token_a = Self::unpack_token_account(token_a_info, token_swap.token_program_id())?;
        let token_b = Self::unpack_token_account(token_b_info, token_swap.token_program_id())?;
        let pool_mint = Self::unpack_mint(pool_mint_info, token_swap.token_program_id())?;
        let current_pool_mint_supply = to_u128(pool_mint.supply)?;
        let (pool_token_amount, pool_mint_supply) = if current_pool_mint_supply > 0 {
            (to_u128(pool_token_amount)?, current_pool_mint_supply)
        } else {
            (calculator.new_pool_supply(), calculator.new_pool_supply())
        };

        let results = calculator
            .pool_tokens_to_trading_tokens(
                pool_token_amount,
                pool_mint_supply,
                to_u128(token_a.amount)?,
                to_u128(token_b.amount)?,
                RoundDirection::Ceiling,
            )
            .ok_or(SwapError::ZeroTradingTokens)?;
        let token_a_amount = to_u64(results.token_a_amount)?;
        if token_a_amount > maximum_token_a_amount {
            return Err(SwapError::ExceededSlippage.into());
        }
        if token_a_amount == 0 {
            return Err(SwapError::ZeroTradingTokens.into());
        }
        let token_b_amount = to_u64(results.token_b_amount)?;
        if token_b_amount > maximum_token_b_amount {
            return Err(SwapError::ExceededSlippage.into());
        }
        if token_b_amount == 0 {
            return Err(SwapError::ZeroTradingTokens.into());
        }

        let pool_token_amount = to_u64(pool_token_amount)?;

        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            source_a_info.clone(),
            token_a_info.clone(),
            user_transfer_authority_info.clone(),
            token_swap.nonce(),
            token_a_amount,
        )?;
        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            source_b_info.clone(),
            token_b_info.clone(),
            user_transfer_authority_info.clone(),
            token_swap.nonce(),
            token_b_amount,
        )?;
        Self::token_mint_to(
            swap_info.key,
            token_program_info.clone(),
            pool_mint_info.clone(),
            dest_info.clone(),
            authority_info.clone(),
            token_swap.nonce(),
            pool_token_amount,
        )?;

        Ok(())
    }

    /// Processes an [WithdrawAllTokenTypes](enum.Instruction.html).
    pub fn process_withdraw_all_token_types(
        program_id: &Pubkey,
        pool_token_amount: u64,
        minimum_token_a_amount: u64,
        minimum_token_b_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let token_a_info = next_account_info(account_info_iter)?;
        let token_b_info = next_account_info(account_info_iter)?;
        let dest_token_a_info = next_account_info(account_info_iter)?;
        let dest_token_b_info = next_account_info(account_info_iter)?;
        let pool_fee_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapVersion::unpack(&swap_info.data.borrow())?;
        Self::check_accounts(
            token_swap.as_ref(),
            program_id,
            swap_info,
            authority_info,
            token_a_info,
            token_b_info,
            pool_mint_info,
            token_program_info,
            Some(dest_token_a_info),
            Some(dest_token_b_info),
            Some(pool_fee_account_info),
        )?;

        let token_a = Self::unpack_token_account(token_a_info, token_swap.token_program_id())?;
        let token_b = Self::unpack_token_account(token_b_info, token_swap.token_program_id())?;
        let pool_mint = Self::unpack_mint(pool_mint_info, token_swap.token_program_id())?;

        let calculator = &token_swap.swap_curve().calculator;

        let withdraw_fee: u128 = if *pool_fee_account_info.key == *source_info.key {
            // withdrawing from the fee account, don't assess withdraw fee
            0
        } else {
            token_swap
                .fees()
                .owner_withdraw_fee(to_u128(pool_token_amount)?)
                .ok_or(SwapError::FeeCalculationFailure)?
        };
        let pool_token_amount = to_u128(pool_token_amount)?
            .checked_sub(withdraw_fee)
            .ok_or(SwapError::CalculationFailure)?;

        let results = calculator
            .pool_tokens_to_trading_tokens(
                pool_token_amount,
                to_u128(pool_mint.supply)?,
                to_u128(token_a.amount)?,
                to_u128(token_b.amount)?,
                RoundDirection::Floor,
            )
            .ok_or(SwapError::ZeroTradingTokens)?;
        let token_a_amount = to_u64(results.token_a_amount)?;
        let token_a_amount = std::cmp::min(token_a.amount, token_a_amount);
        if token_a_amount < minimum_token_a_amount {
            return Err(SwapError::ExceededSlippage.into());
        }
        if token_a_amount == 0 && token_a.amount != 0 {
            return Err(SwapError::ZeroTradingTokens.into());
        }
        let token_b_amount = to_u64(results.token_b_amount)?;
        let token_b_amount = std::cmp::min(token_b.amount, token_b_amount);
        if token_b_amount < minimum_token_b_amount {
            return Err(SwapError::ExceededSlippage.into());
        }
        if token_b_amount == 0 && token_b.amount != 0 {
            return Err(SwapError::ZeroTradingTokens.into());
        }

        if withdraw_fee > 0 {
            Self::token_transfer(
                swap_info.key,
                token_program_info.clone(),
                source_info.clone(),
                pool_fee_account_info.clone(),
                user_transfer_authority_info.clone(),
                token_swap.nonce(),
                to_u64(withdraw_fee)?,
            )?;
        }
        Self::token_burn(
            swap_info.key,
            token_program_info.clone(),
            source_info.clone(),
            pool_mint_info.clone(),
            user_transfer_authority_info.clone(),
            token_swap.nonce(),
            to_u64(pool_token_amount)?,
        )?;

        if token_a_amount > 0 {
            Self::token_transfer(
                swap_info.key,
                token_program_info.clone(),
                token_a_info.clone(),
                dest_token_a_info.clone(),
                authority_info.clone(),
                token_swap.nonce(),
                token_a_amount,
            )?;
        }
        if token_b_amount > 0 {
            Self::token_transfer(
                swap_info.key,
                token_program_info.clone(),
                token_b_info.clone(),
                dest_token_b_info.clone(),
                authority_info.clone(),
                token_swap.nonce(),
                token_b_amount,
            )?;
        }
        Ok(())
    }

    /// Processes DepositSingleTokenTypeExactAmountIn
    pub fn process_deposit_single_token_type_exact_amount_in(
        program_id: &Pubkey,
        source_token_amount: u64,
        minimum_pool_token_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let swap_token_a_info = next_account_info(account_info_iter)?;
        let swap_token_b_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapVersion::unpack(&swap_info.data.borrow())?;
        let calculator = &token_swap.swap_curve().calculator;
        if !calculator.allows_deposits() {
            return Err(SwapError::UnsupportedCurveOperation.into());
        }
        let source_account =
            Self::unpack_token_account(source_info, token_swap.token_program_id())?;
        let swap_token_a =
            Self::unpack_token_account(swap_token_a_info, token_swap.token_program_id())?;
        let swap_token_b =
            Self::unpack_token_account(swap_token_b_info, token_swap.token_program_id())?;

        let trade_direction = if source_account.mint == swap_token_a.mint {
            TradeDirection::AtoB
        } else if source_account.mint == swap_token_b.mint {
            TradeDirection::BtoA
        } else {
            return Err(SwapError::IncorrectSwapAccount.into());
        };

        let (source_a_info, source_b_info) = match trade_direction {
            TradeDirection::AtoB => (Some(source_info), None),
            TradeDirection::BtoA => (None, Some(source_info)),
        };

        Self::check_accounts(
            token_swap.as_ref(),
            program_id,
            swap_info,
            authority_info,
            swap_token_a_info,
            swap_token_b_info,
            pool_mint_info,
            token_program_info,
            source_a_info,
            source_b_info,
            None,
        )?;

        let pool_mint = Self::unpack_mint(pool_mint_info, token_swap.token_program_id())?;
        let pool_mint_supply = to_u128(pool_mint.supply)?;
        let pool_token_amount = if pool_mint_supply > 0 {
            token_swap
                .swap_curve()
                .deposit_single_token_type(
                    to_u128(source_token_amount)?,
                    to_u128(swap_token_a.amount)?,
                    to_u128(swap_token_b.amount)?,
                    pool_mint_supply,
                    trade_direction,
                    token_swap.fees(),
                )
                .ok_or(SwapError::ZeroTradingTokens)?
        } else {
            token_swap.swap_curve().calculator.new_pool_supply()
        };

        let pool_token_amount = to_u64(pool_token_amount)?;
        if pool_token_amount < minimum_pool_token_amount {
            return Err(SwapError::ExceededSlippage.into());
        }
        if pool_token_amount == 0 {
            return Err(SwapError::ZeroTradingTokens.into());
        }

        match trade_direction {
            TradeDirection::AtoB => {
                Self::token_transfer(
                    swap_info.key,
                    token_program_info.clone(),
                    source_info.clone(),
                    swap_token_a_info.clone(),
                    user_transfer_authority_info.clone(),
                    token_swap.nonce(),
                    source_token_amount,
                )?;
            }
            TradeDirection::BtoA => {
                Self::token_transfer(
                    swap_info.key,
                    token_program_info.clone(),
                    source_info.clone(),
                    swap_token_b_info.clone(),
                    user_transfer_authority_info.clone(),
                    token_swap.nonce(),
                    source_token_amount,
                )?;
            }
        }
        Self::token_mint_to(
            swap_info.key,
            token_program_info.clone(),
            pool_mint_info.clone(),
            destination_info.clone(),
            authority_info.clone(),
            token_swap.nonce(),
            pool_token_amount,
        )?;

        Ok(())
    }

    /// Processes a [WithdrawSingleTokenTypeExactAmountOut](enum.Instruction.html).
    pub fn process_withdraw_single_token_type_exact_amount_out(
        program_id: &Pubkey,
        destination_token_amount: u64,
        maximum_pool_token_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let swap_token_a_info = next_account_info(account_info_iter)?;
        let swap_token_b_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let pool_fee_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapVersion::unpack(&swap_info.data.borrow())?;
        let destination_account =
            Self::unpack_token_account(destination_info, token_swap.token_program_id())?;
        let swap_token_a =
            Self::unpack_token_account(swap_token_a_info, token_swap.token_program_id())?;
        let swap_token_b =
            Self::unpack_token_account(swap_token_b_info, token_swap.token_program_id())?;

        let trade_direction = if destination_account.mint == swap_token_a.mint {
            TradeDirection::AtoB
        } else if destination_account.mint == swap_token_b.mint {
            TradeDirection::BtoA
        } else {
            return Err(SwapError::IncorrectSwapAccount.into());
        };

        let (destination_a_info, destination_b_info) = match trade_direction {
            TradeDirection::AtoB => (Some(destination_info), None),
            TradeDirection::BtoA => (None, Some(destination_info)),
        };
        Self::check_accounts(
            token_swap.as_ref(),
            program_id,
            swap_info,
            authority_info,
            swap_token_a_info,
            swap_token_b_info,
            pool_mint_info,
            token_program_info,
            destination_a_info,
            destination_b_info,
            Some(pool_fee_account_info),
        )?;

        let pool_mint = Self::unpack_mint(pool_mint_info, token_swap.token_program_id())?;
        let pool_mint_supply = to_u128(pool_mint.supply)?;
        let swap_token_a_amount = to_u128(swap_token_a.amount)?;
        let swap_token_b_amount = to_u128(swap_token_b.amount)?;

        let burn_pool_token_amount = token_swap
            .swap_curve()
            .withdraw_single_token_type_exact_out(
                to_u128(destination_token_amount)?,
                swap_token_a_amount,
                swap_token_b_amount,
                pool_mint_supply,
                trade_direction,
                token_swap.fees(),
            )
            .ok_or(SwapError::ZeroTradingTokens)?;

        let withdraw_fee: u128 = if *pool_fee_account_info.key == *source_info.key {
            // withdrawing from the fee account, don't assess withdraw fee
            0
        } else {
            token_swap
                .fees()
                .owner_withdraw_fee(burn_pool_token_amount)
                .ok_or(SwapError::FeeCalculationFailure)?
        };
        let pool_token_amount = burn_pool_token_amount
            .checked_add(withdraw_fee)
            .ok_or(SwapError::CalculationFailure)?;

        if to_u64(pool_token_amount)? > maximum_pool_token_amount {
            return Err(SwapError::ExceededSlippage.into());
        }
        if pool_token_amount == 0 {
            return Err(SwapError::ZeroTradingTokens.into());
        }

        if withdraw_fee > 0 {
            Self::token_transfer(
                swap_info.key,
                token_program_info.clone(),
                source_info.clone(),
                pool_fee_account_info.clone(),
                user_transfer_authority_info.clone(),
                token_swap.nonce(),
                to_u64(withdraw_fee)?,
            )?;
        }
        Self::token_burn(
            swap_info.key,
            token_program_info.clone(),
            source_info.clone(),
            pool_mint_info.clone(),
            user_transfer_authority_info.clone(),
            token_swap.nonce(),
            to_u64(burn_pool_token_amount)?,
        )?;

        match trade_direction {
            TradeDirection::AtoB => {
                Self::token_transfer(
                    swap_info.key,
                    token_program_info.clone(),
                    swap_token_a_info.clone(),
                    destination_info.clone(),
                    authority_info.clone(),
                    token_swap.nonce(),
                    destination_token_amount,
                )?;
            }
            TradeDirection::BtoA => {
                Self::token_transfer(
                    swap_info.key,
                    token_program_info.clone(),
                    swap_token_b_info.clone(),
                    destination_info.clone(),
                    authority_info.clone(),
                    token_swap.nonce(),
                    destination_token_amount,
                )?;
            }
        }

        Ok(())
    }

    /// Processes InitializeRegistry
    pub fn process_initialize_registry(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let payer_info = next_account_info(account_info_iter)?;
        let pool_registry_account = next_account_info(account_info_iter)?;

        let pool_registry_seed = "poolregistry";
        let pool_registry_key =
            Pubkey::create_with_seed(payer_info.key, pool_registry_seed, program_id).unwrap();

        if pool_registry_key != *pool_registry_account.key {
            msg!("Error: pool registry pubkey incorrect");
            return Err(ProgramError::InvalidArgument);
        }

        let mut pool_registry = PoolRegistry::load(pool_registry_account, program_id)?;
        if pool_registry.is_initialized {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        pool_registry.is_initialized = true;

        Ok(())
    }

    /// Processes DeregisterPool
    pub fn process_deregister_pool(
        program_id: &Pubkey,
        pool_index: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let payer_info = next_account_info(account_info_iter)?;
        let pool_registry_account = next_account_info(account_info_iter)?;

        if !payer_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let pool_registry_seed = "poolregistry";
        let pool_registry_key =
            Pubkey::create_with_seed(payer_info.key, pool_registry_seed, program_id).unwrap();

        if pool_registry_key != *pool_registry_account.key {
            msg!("Error: pool registry pubkey incorrect");
            return Err(ProgramError::InvalidArgument);
        }

        let mut pool_registry = PoolRegistry::load(pool_registry_account, program_id)?;
        if !pool_registry.is_initialized {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        //we took the arg as a u64 for consistency, convert to u32
        let pool_index: u32 = pool_index.try_into().unwrap();
        pool_registry.remove(pool_index)?;

        Ok(())
    }

    /// Processes RepairClosedFeeAccount
    pub fn process_repair_closed_fee_account(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let token_swap_account = next_account_info(account_info_iter)?;
        let old_fee_account = next_account_info(account_info_iter)?;
        let new_fee_account = next_account_info(account_info_iter)?;

        //assert program account ownership
        if token_swap_account.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }

        //no point making no change
        if old_fee_account.key == new_fee_account.key {
            return Err(SwapError::InvalidInput.into());
        }

        //old fee must NOT be a token account (assumed closed, but could be some other reason?)
        //this makes sure this can only run in a truly broken scenario
        Self::unpack_token_account(old_fee_account, &spl_token::id())
            .err()
            .ok_or(SwapError::InvalidInput)?;

        //constraints must exist
        #[cfg(feature = "production")]
        let owner_key = SWAP_CONSTRAINTS
            .map(|c| c.owner_key.parse::<Pubkey>().unwrap())
            .ok_or_else(|| ProgramError::InvalidInstructionData)?;

        //integration test has no swap constraints, so no owner key - we use our hard coded key
        #[cfg(not(feature = "production"))]
        let owner_key = TEST_OWNER_KEY.parse::<Pubkey>().unwrap();

        let new_fee_token_account = Self::unpack_token_account(new_fee_account, &spl_token::id())?;

        //new fee account must be owned by the owner fee account
        if owner_key != new_fee_token_account.owner {
            return Err(SwapError::InvalidOwner.into());
        }

        //new fee account cannot have a close authority
        if new_fee_token_account.close_authority.is_some() {
            return Err(SwapError::InvalidCloseAuthority.into());
        }

        //new fee account cannot have a delegate
        if new_fee_token_account.delegate.is_some() {
            return Err(SwapError::InvalidDelegate.into());
        }

        //token swap must parse.
        //we avoid using the trait returned from SwapVersion::unpack so we have a mutable SwapV1
        let mut token_swap: SwapV1 = {
            let data = token_swap_account.data.borrow();
            let (&version, rest) = data.split_first().ok_or(ProgramError::InvalidAccountData)?;
            match version {
                1 => Ok(SwapV1::unpack(rest)?),
                _ => Err(ProgramError::UninitializedAccount),
            }
        }?;

        //old fee account key must match whats on our token swap
        if old_fee_account.key != &token_swap.pool_fee_account {
            return Err(SwapError::IncorrectFeeAccount.into());
        }

        //new fee account must be the ata of owner fee address and the pool mint
        let ata = get_associated_token_address(&owner_key, token_swap.pool_mint());
        if new_fee_account.key != &ata {
            return Err(SwapError::IncorrectFeeAccount.into());
        }

        //set the new fee account
        token_swap.pool_fee_account = *new_fee_account.key;

        //repack the token_swap account data
        SwapVersion::pack(
            SwapVersion::SwapV1(token_swap),
            &mut token_swap_account.data.borrow_mut(),
        )?;

        Ok(())
    }

    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        Self::process_with_constraints(program_id, accounts, input, &SWAP_CONSTRAINTS)
    }

    /// Processes an instruction given extra constraint
    pub fn process_with_constraints(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
        swap_constraints: &Option<SwapConstraints>,
    ) -> ProgramResult {
        let instruction = SwapInstruction::unpack(input)?;
        match instruction {
            SwapInstruction::Initialize(Initialize {
                nonce,
                fees,
                swap_curve,
                pool_nonce,
            }) => {
                msg!("Instruction: Init");
                Self::process_initialize(
                    program_id,
                    nonce,
                    fees,
                    swap_curve,
                    pool_nonce,
                    accounts,
                    swap_constraints,
                )
            }
            SwapInstruction::Swap(Swap {
                amount_in,
                minimum_amount_out,
                flags,
            }) => {
                msg!("Instruction: Swap");
                Self::process_swap(program_id, amount_in, minimum_amount_out, flags, accounts)
            }
            SwapInstruction::DepositAllTokenTypes(DepositAllTokenTypes {
                pool_token_amount,
                maximum_token_a_amount,
                maximum_token_b_amount,
            }) => {
                msg!("Instruction: DepositAllTokenTypes");
                Self::process_deposit_all_token_types(
                    program_id,
                    pool_token_amount,
                    maximum_token_a_amount,
                    maximum_token_b_amount,
                    accounts,
                )
            }
            SwapInstruction::WithdrawAllTokenTypes(WithdrawAllTokenTypes {
                pool_token_amount,
                minimum_token_a_amount,
                minimum_token_b_amount,
            }) => {
                msg!("Instruction: WithdrawAllTokenTypes");
                Self::process_withdraw_all_token_types(
                    program_id,
                    pool_token_amount,
                    minimum_token_a_amount,
                    minimum_token_b_amount,
                    accounts,
                )
            }
            SwapInstruction::DepositSingleTokenTypeExactAmountIn(
                DepositSingleTokenTypeExactAmountIn {
                    source_token_amount,
                    minimum_pool_token_amount,
                },
            ) => {
                msg!("Instruction: DepositSingleTokenTypeExactAmountIn");
                Self::process_deposit_single_token_type_exact_amount_in(
                    program_id,
                    source_token_amount,
                    minimum_pool_token_amount,
                    accounts,
                )
            }
            SwapInstruction::WithdrawSingleTokenTypeExactAmountOut(
                WithdrawSingleTokenTypeExactAmountOut {
                    destination_token_amount,
                    maximum_pool_token_amount,
                },
            ) => {
                msg!("Instruction: WithdrawSingleTokenTypeExactAmountOut");
                Self::process_withdraw_single_token_type_exact_amount_out(
                    program_id,
                    destination_token_amount,
                    maximum_pool_token_amount,
                    accounts,
                )
            }
            SwapInstruction::InitializeRegistry() => {
                msg!("Instruction: InitializeRegistry");
                Self::process_initialize_registry(program_id, accounts)
            }
            SwapInstruction::RoutedSwap(Swap {
                amount_in,
                minimum_amount_out,
                flags,
            }) => {
                msg!("Instruction: RoutedSwap");
                Self::process_routed_swap(
                    program_id,
                    amount_in,
                    minimum_amount_out,
                    flags,
                    accounts,
                )
            }
            SwapInstruction::DeregisterPool(DeregisterPool { pool_index }) => {
                msg!("Instruction: DeregisterPool");
                Self::process_deregister_pool(program_id, pool_index, accounts)
            }
            SwapInstruction::RepairClosedFeeAccount() => {
                msg!("Instruction: RepairClosedFeeAccount");
                Self::process_repair_closed_fee_account(program_id, accounts)
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
            SwapError::AlreadyInUse => msg!("Error: Swap account already in use"),
            SwapError::InvalidProgramAddress => {
                msg!("Error: Invalid program address generated from nonce and key")
            }
            SwapError::InvalidOwner => {
                msg!("Error: The input account owner is not the program address")
            }
            SwapError::InvalidOutputOwner => {
                msg!("Error: Output pool account owner cannot be the program address")
            }
            SwapError::ExpectedMint => msg!("Error: Deserialized account is not an SPL Token mint"),
            SwapError::ExpectedAccount => {
                msg!("Error: Deserialized account is not an SPL Token account")
            }
            SwapError::EmptySupply => msg!("Error: Input token account empty"),
            SwapError::InvalidSupply => msg!("Error: Pool token mint has a non-zero supply"),
            SwapError::RepeatedMint => msg!("Error: Swap input token accounts have the same mint"),
            SwapError::InvalidDelegate => msg!("Error: Token account has a delegate"),
            SwapError::InvalidInput => msg!("Error: InvalidInput"),
            SwapError::IncorrectSwapAccount => {
                msg!("Error: Address of the provided swap token account is incorrect")
            }
            SwapError::IncorrectPoolMint => {
                msg!("Error: Address of the provided pool token mint is incorrect")
            }
            SwapError::InvalidOutput => msg!("Error: InvalidOutput"),
            SwapError::CalculationFailure => msg!("Error: CalculationFailure"),
            SwapError::InvalidInstruction => msg!("Error: InvalidInstruction"),
            SwapError::ExceededSlippage => {
                msg!("Error: Swap instruction exceeds desired slippage limit")
            }
            SwapError::InvalidCloseAuthority => msg!("Error: Token account has a close authority"),
            SwapError::InvalidFreezeAuthority => {
                msg!("Error: Pool token mint has a freeze authority")
            }
            SwapError::IncorrectFeeAccount => msg!("Error: Pool fee token account incorrect"),
            SwapError::ZeroTradingTokens => {
                msg!("Error: Given pool token amount results in zero trading tokens")
            }
            SwapError::FeeCalculationFailure => msg!(
                "Error: The fee calculation failed due to overflow, underflow, or unexpected 0"
            ),
            SwapError::ConversionFailure => msg!("Error: Conversion to or from u64 failed."),
            SwapError::InvalidFee => {
                msg!("Error: The provided fee does not match the program owner's constraints")
            }
            SwapError::IncorrectTokenProgramId => {
                msg!("Error: The provided token program does not match the token program expected by the swap")
            }
            SwapError::UnsupportedCurveType => {
                msg!("Error: The provided curve type is not supported by the program owner")
            }
            SwapError::InvalidCurve => {
                msg!("Error: The provided curve parameters are invalid")
            }
            SwapError::UnsupportedCurveOperation => {
                msg!("Error: The operation cannot be performed on the given curve")
            }
            SwapError::NonRefundeeTransferAuthorityOwnsNonNative => {
                msg!("Error: Non-native token output to transfer authority owned account")
            }
            SwapError::TransferAuthorityOwnsButRefundeeNotProvided => {
                msg!("Error: A refundee is required when a non-owner transfer authority owns native output")
            }
            SwapError::InvalidMint => {
                msg!(
                    "Error: A swap must be comprised of the required mint ({})",
                    SWAP_CONSTRAINTS.unwrap().required_mint
                )
            }
            SwapError::NonRefundeeTransferAuthorityNotEmpty => {
                msg!("Error: A non-refundee transfer authority owned account was left not empty")
            }
            SwapError::RoutedSwapRequiresEmptyIntermediary => {
                msg!("Error: A routed swap requires an empty intermediary")
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
        curve::calculator::{CurveCalculator, INITIAL_SWAP_POOL_AMOUNT},
        curve::{
            base::CurveType, constant_price::ConstantPriceCurve,
            constant_product::ConstantProductCurve, offset::OffsetCurve,
        },
        instruction::{
            deposit_all_token_types, deposit_single_token_type_exact_amount_in, initialize,
            initialize_registry, swap, withdraw_all_token_types,
            withdraw_single_token_type_exact_amount_out,
        },
        state::PoolRegistry,
    };
    use solana_program::{
        account_info::IntoAccountInfo, instruction::Instruction, program_stubs, rent::Rent,
    };
    use solana_sdk::account::{create_account_for_test, create_is_signer_account_infos, Account};
    use spl_token::{
        error::TokenError,
        instruction::{
            approve, initialize_account, initialize_mint, mint_to, revoke, set_authority,
            AuthorityType,
        },
    };
    use std::sync::Arc;

    // Test program id for the swap program.
    const SWAP_PROGRAM_ID: Pubkey = Pubkey::new_from_array([2u8; 32]);

    struct TestSyscallStubs {}
    impl program_stubs::SyscallStubs for TestSyscallStubs {
        fn sol_invoke_signed(
            &self,
            instruction: &Instruction,
            account_infos: &[AccountInfo],
            signers_seeds: &[&[&[u8]]],
        ) -> ProgramResult {
            msg!("TestSyscallStubs::sol_invoke_signed()");

            let mut new_account_infos = vec![];

            // mimic check for token program in accounts
            if !account_infos.iter().any(|x| *x.key == spl_token::id()) {
                return Err(ProgramError::InvalidAccountData);
            }

            for meta in instruction.accounts.iter() {
                for account_info in account_infos.iter() {
                    if meta.pubkey == *account_info.key {
                        let mut new_account_info = account_info.clone();
                        for seeds in signers_seeds.iter() {
                            let signer =
                                Pubkey::create_program_address(seeds, &SWAP_PROGRAM_ID).unwrap();
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

    fn create_pool_registry() -> (Pubkey, Account) {
        let payer_key = Pubkey::new_unique();
        let mut payer_account = Account::new(10, 0, &SWAP_PROGRAM_ID);
        let _payer_account_info = (&payer_key, false, &mut payer_account).into_account_info();

        let pool_registry_seed = "poolregistry";
        let pool_registry_key =
            Pubkey::create_with_seed(&payer_key, pool_registry_seed, &SWAP_PROGRAM_ID).unwrap();

        let mut pool_registry_account =
            Account::new(0, std::mem::size_of::<PoolRegistry>(), &SWAP_PROGRAM_ID);
        let _pool_registry_account_info =
            (&pool_registry_key, false, &mut pool_registry_account).into_account_info();

        do_process_instruction(
            initialize_registry(&SWAP_PROGRAM_ID, &payer_key, &pool_registry_key).unwrap(),
            vec![&mut payer_account, &mut pool_registry_account],
        )
        .unwrap();

        (pool_registry_key, pool_registry_account)
    }

    struct SwapAccountInfo {
        payer_key: Pubkey,
        payer_account: Account,
        nonce: u8,
        authority_key: Pubkey,
        fees: Fees,
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
        pool_registry_key: Pubkey,
        pool_registry_account: Account,
        rent_sysvar_account: Account,
        pool_nonce: u8,
    }

    impl SwapAccountInfo {
        pub fn new(
            user_key: &Pubkey,
            fees: Fees,
            swap_curve: SwapCurve,
            token_a_amount: u64,
            token_b_amount: u64,
        ) -> Self {
            let payer_key = Pubkey::new_unique();
            let mut payer_account = Account::new(100000000000, 0, &SWAP_PROGRAM_ID);
            let _payer_account_info = (&payer_key, false, &mut payer_account).into_account_info();

            let rent_sysvar_account = create_account_for_test(&Rent::free());

            let (token_a_mint_key, mut token_a_mint_account) =
                create_mint(&spl_token::id(), user_key, None);
            let (token_b_mint_key, mut token_b_mint_account) =
                create_mint(&spl_token::id(), user_key, None);

            let mut seed_key_vec = vec![token_a_mint_key.to_bytes(), token_b_mint_key.to_bytes()];
            seed_key_vec.sort_unstable();

            let (swap_key, pool_nonce) = Pubkey::find_program_address(
                &[
                    &seed_key_vec[0][..32],
                    &seed_key_vec[1][..32],
                    &[swap_curve.curve_type as u8],
                ],
                &SWAP_PROGRAM_ID,
            );

            let mut swap_account = Account::new(0, SwapVersion::LATEST_LEN, &SWAP_PROGRAM_ID);
            let _swap_account_info = (&swap_key, false, &mut swap_account).into_account_info();
            let (authority_key, nonce) =
                Pubkey::find_program_address(&[&swap_key.to_bytes()[..]], &SWAP_PROGRAM_ID);

            let (pool_mint_key, mut pool_mint_account) =
                create_mint(&spl_token::id(), &authority_key, None);
            let (pool_token_key, pool_token_account) = mint_token(
                &spl_token::id(),
                &pool_mint_key,
                &mut pool_mint_account,
                &authority_key,
                user_key,
                0,
            );
            let (pool_fee_key, pool_fee_account) = mint_token(
                &spl_token::id(),
                &pool_mint_key,
                &mut pool_mint_account,
                &authority_key,
                user_key,
                0,
            );

            let (token_a_key, token_a_account) = mint_token(
                &spl_token::id(),
                &token_a_mint_key,
                &mut token_a_mint_account,
                user_key,
                &authority_key,
                token_a_amount,
            );

            let (token_b_key, token_b_account) = mint_token(
                &spl_token::id(),
                &token_b_mint_key,
                &mut token_b_mint_account,
                user_key,
                &authority_key,
                token_b_amount,
            );

            let (pool_registry_key, pool_registry_account) = create_pool_registry();

            SwapAccountInfo {
                payer_key,
                payer_account,
                nonce,
                authority_key,
                fees,
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
                pool_registry_key,
                pool_registry_account,
                rent_sysvar_account,
                pool_nonce,
            }
        }

        pub fn initialize_swap(&mut self) -> ProgramResult {
            do_process_instruction(
                initialize(
                    &SWAP_PROGRAM_ID,
                    &spl_token::id(),
                    &self.payer_key,
                    &self.swap_key,
                    &self.authority_key,
                    &self.token_a_key,
                    &self.token_b_key,
                    &self.pool_mint_key,
                    &self.pool_fee_key,
                    &self.pool_token_key,
                    self.nonce,
                    self.fees.clone(),
                    self.swap_curve.clone(),
                    &self.pool_registry_key,
                    self.pool_nonce,
                )
                .unwrap(),
                vec![
                    &mut self.payer_account,
                    &mut self.swap_account,
                    &mut Account::default(),
                    &mut self.token_a_account,
                    &mut self.token_b_account,
                    &mut self.pool_mint_account,
                    &mut self.pool_fee_account,
                    &mut self.pool_token_account,
                    &mut Account::default(),
                    &mut self.pool_registry_account,
                    &mut Account::default(),
                    &mut self.rent_sysvar_account,
                ],
            )
        }

        //mimic an initialization with NO checks in order to unit test on an initialized swap
        //initialize itself is tested in functional tests
        pub fn initialize_swap_mock_for_testing(&mut self) -> Result<(), ProgramError> {
            //mint pool tokens
            let initial_amount = self.swap_curve.calculator.new_pool_supply();
            mint_token_to_existing(
                &spl_token::id(),
                &self.pool_mint_key,
                &mut self.pool_mint_account,
                &self.authority_key,
                &self.pool_token_key,
                &mut self.pool_token_account,
                to_u64(initial_amount)?,
            );

            //mock swap account
            let obj = SwapVersion::SwapV1(SwapV1 {
                is_initialized: true,
                nonce: self.nonce,
                token_program_id: spl_token::id(),
                token_a: self.token_a_key,
                token_b: self.token_b_key,
                pool_mint: self.pool_mint_key,
                token_a_mint: self.token_a_mint_key,
                token_b_mint: self.token_b_mint_key,
                pool_fee_account: self.pool_fee_key,
                fees: self.fees.clone(),
                swap_curve: self.swap_curve.clone(),
                pool_nonce: self.pool_nonce,
            });
            self.swap_account = Account::new(0, SwapVersion::LATEST_LEN, &SWAP_PROGRAM_ID);
            let swap_info = (&self.swap_key, false, &mut self.swap_account).into_account_info();

            let x = SwapVersion::pack(obj, &mut swap_info.data.borrow_mut());
            x
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
                &spl_token::id(),
                &self.token_a_mint_key,
                &mut self.token_a_mint_account,
                mint_owner,
                account_owner,
                a_amount,
            );
            let (token_b_key, token_b_account) = mint_token(
                &spl_token::id(),
                &self.token_b_mint_key,
                &mut self.token_b_mint_account,
                mint_owner,
                account_owner,
                b_amount,
            );
            let (pool_key, pool_account) = mint_token(
                &spl_token::id(),
                &self.pool_mint_key,
                &mut self.pool_mint_account,
                &self.authority_key,
                account_owner,
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
            user_source_account: &mut Account,
            swap_source_key: &Pubkey,
            swap_destination_key: &Pubkey,
            user_destination_key: &Pubkey,
            user_destination_account: &mut Account,
            amount_in: u64,
            minimum_amount_out: u64,
        ) -> ProgramResult {
            let user_transfer_key = Pubkey::new_unique();
            // approve moving from user source account
            do_process_instruction(
                approve(
                    &spl_token::id(),
                    user_source_key,
                    &user_transfer_key,
                    user_key,
                    &[],
                    amount_in,
                )
                .unwrap(),
                vec![
                    user_source_account,
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
                    &spl_token::id(),
                    &self.swap_key,
                    &self.authority_key,
                    &user_transfer_key,
                    user_source_key,
                    swap_source_key,
                    swap_destination_key,
                    user_destination_key,
                    &self.pool_mint_key,
                    &self.pool_fee_key,
                    None,
                    user_key,
                    Swap {
                        amount_in,
                        minimum_amount_out,
                        flags: swap_flags::default(),
                    },
                )
                .unwrap(),
                vec![
                    &mut self.swap_account,
                    &mut Account::default(),
                    &mut Account::default(),
                    user_source_account,
                    &mut swap_source_account,
                    &mut swap_destination_account,
                    user_destination_account,
                    &mut self.pool_mint_account,
                    &mut self.pool_fee_account,
                    &mut Account::default(),
                    &mut Account::default(),
                ],
            )?;

            self.set_token_account(swap_source_key, swap_source_account);
            self.set_token_account(swap_destination_key, swap_destination_account);

            Ok(())
        }

        #[allow(clippy::too_many_arguments)]
        pub fn deposit_all_token_types(
            &mut self,
            depositor_key: &Pubkey,
            depositor_token_a_key: &Pubkey,
            depositor_token_a_account: &mut Account,
            depositor_token_b_key: &Pubkey,
            depositor_token_b_account: &mut Account,
            depositor_pool_key: &Pubkey,
            depositor_pool_account: &mut Account,
            pool_token_amount: u64,
            maximum_token_a_amount: u64,
            maximum_token_b_amount: u64,
        ) -> ProgramResult {
            let user_transfer_authority = Pubkey::new_unique();
            do_process_instruction(
                approve(
                    &spl_token::id(),
                    depositor_token_a_key,
                    &user_transfer_authority,
                    depositor_key,
                    &[],
                    maximum_token_a_amount,
                )
                .unwrap(),
                vec![
                    depositor_token_a_account,
                    &mut Account::default(),
                    &mut Account::default(),
                ],
            )
            .unwrap();

            do_process_instruction(
                approve(
                    &spl_token::id(),
                    depositor_token_b_key,
                    &user_transfer_authority,
                    depositor_key,
                    &[],
                    maximum_token_b_amount,
                )
                .unwrap(),
                vec![
                    depositor_token_b_account,
                    &mut Account::default(),
                    &mut Account::default(),
                ],
            )
            .unwrap();

            do_process_instruction(
                deposit_all_token_types(
                    &SWAP_PROGRAM_ID,
                    &spl_token::id(),
                    &self.swap_key,
                    &self.authority_key,
                    &user_transfer_authority,
                    depositor_token_a_key,
                    depositor_token_b_key,
                    &self.token_a_key,
                    &self.token_b_key,
                    &self.pool_mint_key,
                    depositor_pool_key,
                    DepositAllTokenTypes {
                        pool_token_amount,
                        maximum_token_a_amount,
                        maximum_token_b_amount,
                    },
                )
                .unwrap(),
                vec![
                    &mut self.swap_account,
                    &mut Account::default(),
                    &mut Account::default(),
                    depositor_token_a_account,
                    depositor_token_b_account,
                    &mut self.token_a_account,
                    &mut self.token_b_account,
                    &mut self.pool_mint_account,
                    depositor_pool_account,
                    &mut Account::default(),
                ],
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn withdraw_all_token_types(
            &mut self,
            user_key: &Pubkey,
            pool_key: &Pubkey,
            pool_account: &mut Account,
            token_a_key: &Pubkey,
            token_a_account: &mut Account,
            token_b_key: &Pubkey,
            token_b_account: &mut Account,
            pool_token_amount: u64,
            minimum_token_a_amount: u64,
            minimum_token_b_amount: u64,
        ) -> ProgramResult {
            let user_transfer_authority_key = Pubkey::new_unique();
            // approve user transfer authority to take out pool tokens
            do_process_instruction(
                approve(
                    &spl_token::id(),
                    pool_key,
                    &user_transfer_authority_key,
                    user_key,
                    &[],
                    pool_token_amount,
                )
                .unwrap(),
                vec![
                    pool_account,
                    &mut Account::default(),
                    &mut Account::default(),
                ],
            )
            .unwrap();

            // withdraw token a and b correctly
            do_process_instruction(
                withdraw_all_token_types(
                    &SWAP_PROGRAM_ID,
                    &spl_token::id(),
                    &self.swap_key,
                    &self.authority_key,
                    &user_transfer_authority_key,
                    &self.pool_mint_key,
                    &self.pool_fee_key,
                    pool_key,
                    &self.token_a_key,
                    &self.token_b_key,
                    token_a_key,
                    token_b_key,
                    WithdrawAllTokenTypes {
                        pool_token_amount,
                        minimum_token_a_amount,
                        minimum_token_b_amount,
                    },
                )
                .unwrap(),
                vec![
                    &mut self.swap_account,
                    &mut Account::default(),
                    &mut Account::default(),
                    &mut self.pool_mint_account,
                    pool_account,
                    &mut self.token_a_account,
                    &mut self.token_b_account,
                    token_a_account,
                    token_b_account,
                    &mut self.pool_fee_account,
                    &mut Account::default(),
                ],
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn deposit_single_token_type_exact_amount_in(
            &mut self,
            depositor_key: &Pubkey,
            deposit_account_key: &Pubkey,
            deposit_token_account: &mut Account,
            deposit_pool_key: &Pubkey,
            deposit_pool_account: &mut Account,
            source_token_amount: u64,
            minimum_pool_token_amount: u64,
        ) -> ProgramResult {
            let user_transfer_authority_key = Pubkey::new_unique();
            do_process_instruction(
                approve(
                    &spl_token::id(),
                    deposit_account_key,
                    &user_transfer_authority_key,
                    depositor_key,
                    &[],
                    source_token_amount,
                )
                .unwrap(),
                vec![
                    deposit_token_account,
                    &mut Account::default(),
                    &mut Account::default(),
                ],
            )
            .unwrap();

            do_process_instruction(
                deposit_single_token_type_exact_amount_in(
                    &SWAP_PROGRAM_ID,
                    &spl_token::id(),
                    &self.swap_key,
                    &self.authority_key,
                    &user_transfer_authority_key,
                    deposit_account_key,
                    &self.token_a_key,
                    &self.token_b_key,
                    &self.pool_mint_key,
                    deposit_pool_key,
                    DepositSingleTokenTypeExactAmountIn {
                        source_token_amount,
                        minimum_pool_token_amount,
                    },
                )
                .unwrap(),
                vec![
                    &mut self.swap_account,
                    &mut Account::default(),
                    &mut Account::default(),
                    deposit_token_account,
                    &mut self.token_a_account,
                    &mut self.token_b_account,
                    &mut self.pool_mint_account,
                    deposit_pool_account,
                    &mut Account::default(),
                ],
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn withdraw_single_token_type_exact_amount_out(
            &mut self,
            user_key: &Pubkey,
            pool_key: &Pubkey,
            pool_account: &mut Account,
            destination_key: &Pubkey,
            destination_account: &mut Account,
            destination_token_amount: u64,
            maximum_pool_token_amount: u64,
        ) -> ProgramResult {
            let user_transfer_authority_key = Pubkey::new_unique();
            // approve user transfer authority to take out pool tokens
            do_process_instruction(
                approve(
                    &spl_token::id(),
                    pool_key,
                    &user_transfer_authority_key,
                    user_key,
                    &[],
                    maximum_pool_token_amount,
                )
                .unwrap(),
                vec![
                    pool_account,
                    &mut Account::default(),
                    &mut Account::default(),
                ],
            )
            .unwrap();

            do_process_instruction(
                withdraw_single_token_type_exact_amount_out(
                    &SWAP_PROGRAM_ID,
                    &spl_token::id(),
                    &self.swap_key,
                    &self.authority_key,
                    &user_transfer_authority_key,
                    &self.pool_mint_key,
                    &self.pool_fee_key,
                    pool_key,
                    &self.token_a_key,
                    &self.token_b_key,
                    destination_key,
                    WithdrawSingleTokenTypeExactAmountOut {
                        destination_token_amount,
                        maximum_pool_token_amount,
                    },
                )
                .unwrap(),
                vec![
                    &mut self.swap_account,
                    &mut Account::default(),
                    &mut Account::default(),
                    &mut self.pool_mint_account,
                    pool_account,
                    &mut self.token_a_account,
                    &mut self.token_b_account,
                    destination_account,
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
        swap_constraints: &Option<SwapConstraints>,
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
            Processor::process_with_constraints(
                &instruction.program_id,
                &account_infos,
                &instruction.data,
                swap_constraints,
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
        do_process_instruction_with_fee_constraints(instruction, accounts, &SWAP_CONSTRAINTS)
    }

    fn mint_token(
        program_id: &Pubkey,
        mint_key: &Pubkey,
        mint_account: &mut Account,
        mint_authority_key: &Pubkey,
        account_owner_key: &Pubkey,
        amount: u64,
    ) -> (Pubkey, Account) {
        let account_key = Pubkey::new_unique();
        let mut account_account = Account::new(
            account_minimum_balance(),
            spl_token::state::Account::get_packed_len(),
            program_id,
        );
        let mut mint_authority_account = Account::default();
        let mut rent_sysvar_account = create_account_for_test(&Rent::free());

        do_process_instruction(
            initialize_account(program_id, &account_key, mint_key, account_owner_key).unwrap(),
            vec![
                &mut account_account,
                mint_account,
                &mut mint_authority_account,
                &mut rent_sysvar_account,
            ],
        )
        .unwrap();

        if amount > 0 {
            do_process_instruction(
                mint_to(
                    program_id,
                    mint_key,
                    &account_key,
                    mint_authority_key,
                    &[],
                    amount,
                )
                .unwrap(),
                vec![
                    mint_account,
                    &mut account_account,
                    &mut mint_authority_account,
                ],
            )
            .unwrap();
        }

        (account_key, account_account)
    }

    fn mint_token_to_existing(
        program_id: &Pubkey,
        mint_key: &Pubkey,
        mint_account: &mut Account,
        mint_authority_key: &Pubkey,
        destination_key: &Pubkey,
        destination_account: &mut Account,
        amount: u64,
    ) {
        let mut mint_authority_account = Account::default();
        do_process_instruction(
            mint_to(
                program_id,
                mint_key,
                destination_key,
                mint_authority_key,
                &[],
                amount,
            )
            .unwrap(),
            vec![
                mint_account,
                destination_account,
                &mut mint_authority_account,
            ],
        )
        .unwrap();
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
            program_id,
        );
        let mut rent_sysvar_account = create_account_for_test(&Rent::free());

        do_process_instruction(
            initialize_mint(program_id, &mint_key, authority_key, freeze_authority, 2).unwrap(),
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
        let token_program = (spl_token::id(), Account::default());
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
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };

        let token_a_amount = 1000;
        let token_b_amount = 2000;
        let pool_token_amount = 10;
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Arc::new(ConstantProductCurve {}),
        };

        let mut accounts =
            SwapAccountInfo::new(&user_key, fees, swap_curve, token_a_amount, token_b_amount);

        // wrong pda for swap account
        {
            let old_key = accounts.swap_key;
            accounts.swap_key = Pubkey::new_unique();
            assert_eq!(
                Err(SwapError::InvalidProgramAddress.into()),
                accounts.initialize_swap()
            );
            accounts.swap_key = old_key;
        }

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
            accounts.token_a_account = Account::new(0, 0, &spl_token::id());
            assert_eq!(
                Err(SwapError::ExpectedAccount.into()),
                accounts.initialize_swap()
            );
            accounts.token_a_account = old_account;
        }

        // uninitialized token b account
        {
            let old_account = accounts.token_b_account;
            accounts.token_b_account = Account::new(0, 0, &spl_token::id());
            assert_eq!(
                Err(SwapError::ExpectedAccount.into()),
                accounts.initialize_swap()
            );
            accounts.token_b_account = old_account;
        }

        // uninitialized pool mint
        {
            let old_account = accounts.pool_mint_account;
            accounts.pool_mint_account = Account::new(0, 0, &spl_token::id());
            assert_eq!(
                Err(SwapError::ExpectedMint.into()),
                accounts.initialize_swap()
            );
            accounts.pool_mint_account = old_account;
        }

        // token A account owner is not swap authority
        {
            let (_token_a_key, token_a_account) = mint_token(
                &spl_token::id(),
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
                &spl_token::id(),
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
                &spl_token::id(),
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
                &spl_token::id(),
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
                create_mint(&spl_token::id(), &user_key, None);
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
                create_mint(&spl_token::id(), &accounts.authority_key, Some(&user_key));
            let old_mint = accounts.pool_mint_account;
            accounts.pool_mint_account = pool_mint_account;
            assert_eq!(
                Err(SwapError::InvalidFreezeAuthority.into()),
                accounts.initialize_swap()
            );
            accounts.pool_mint_account = old_mint;
        }

        // token A account owned by wrong program
        {
            let (_token_a_key, mut token_a_account) = mint_token(
                &spl_token::id(),
                &accounts.token_a_mint_key,
                &mut accounts.token_a_mint_account,
                &user_key,
                &accounts.authority_key,
                token_a_amount,
            );
            token_a_account.owner = SWAP_PROGRAM_ID;
            let old_account = accounts.token_a_account;
            accounts.token_a_account = token_a_account;
            assert_eq!(
                Err(SwapError::IncorrectTokenProgramId.into()),
                accounts.initialize_swap()
            );
            accounts.token_a_account = old_account;
        }

        // token B account owned by wrong program
        {
            let (_token_b_key, mut token_b_account) = mint_token(
                &spl_token::id(),
                &accounts.token_b_mint_key,
                &mut accounts.token_b_mint_account,
                &user_key,
                &accounts.authority_key,
                token_b_amount,
            );
            token_b_account.owner = SWAP_PROGRAM_ID;
            let old_account = accounts.token_b_account;
            accounts.token_b_account = token_b_account;
            assert_eq!(
                Err(SwapError::IncorrectTokenProgramId.into()),
                accounts.initialize_swap()
            );
            accounts.token_b_account = old_account;
        }

        // empty token A account
        {
            let (_token_a_key, token_a_account) = mint_token(
                &spl_token::id(),
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
                &spl_token::id(),
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
                create_mint(&spl_token::id(), &accounts.authority_key, None);
            accounts.pool_mint_account = pool_mint_account;

            let (_empty_pool_token_key, empty_pool_token_account) = mint_token(
                &spl_token::id(),
                &accounts.pool_mint_key,
                &mut accounts.pool_mint_account,
                &accounts.authority_key,
                &user_key,
                0,
            );

            let (_pool_token_key, pool_token_account) = mint_token(
                &spl_token::id(),
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
                &spl_token::id(),
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
                    &spl_token::id(),
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
                    &spl_token::id(),
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
                    &spl_token::id(),
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
                    &spl_token::id(),
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
                    &spl_token::id(),
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
                    &spl_token::id(),
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
                    &spl_token::id(),
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
                    &spl_token::id(),
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
                Err(SwapError::IncorrectTokenProgramId.into()),
                do_process_instruction(
                    initialize(
                        &SWAP_PROGRAM_ID,
                        &wrong_program_id,
                        &accounts.payer_key,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &accounts.pool_token_key,
                        accounts.nonce,
                        accounts.fees.clone(),
                        accounts.swap_curve.clone(),
                        &accounts.pool_registry_key,
                        accounts.pool_nonce
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.payer_account,
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.pool_token_account,
                        &mut Account::default(),
                        &mut accounts.pool_registry_account,
                        &mut Account::default(),
                        &mut accounts.rent_sysvar_account,
                    ],
                )
            );
        }

        // create swap with same token A and B
        {
            let (_token_a_repeat_key, token_a_repeat_account) = mint_token(
                &spl_token::id(),
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

        // right required mint in constraint
        {
            let trade_fee_numerator = 25;
            let trade_fee_denominator = 10000;
            let owner_trade_fee_numerator = 5;
            let owner_trade_fee_denominator = 10000;
            let fees = Fees {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
            };
            let curve = ConstantProductCurve {};
            let swap_curve = SwapCurve {
                curve_type: CurveType::ConstantProduct,
                calculator: Arc::new(curve),
            };

            let mut accounts = SwapAccountInfo::new(
                &user_key,
                fees.clone(),
                swap_curve,
                token_a_amount,
                token_b_amount,
            );

            //use proper required mint
            let required_mint = accounts.token_a_mint_key.to_string();

            let valid_curve_types = &[CurveType::ConstantProduct];

            //use proper owner
            let fee_account =
                spl_token::state::Account::unpack(&accounts.pool_fee_account.data).unwrap();
            let owner_as_str = fee_account.owner.to_string();
            let constraints = Some(SwapConstraints {
                owner_key: &owner_as_str,
                valid_curve_types,
                fees: &fees,
                required_mint: &required_mint,
            });

            assert_eq!(
                //this err is a side affect of getting past the point in the code where the invalid
                //mint is checked its what we expect because this is what happens if you try and swap
                //without using banks_client.  It's the best we can do for now.
                //tl;dr; it's not an InvalidMint err.
                Err(ProgramError::InvalidAccountData),
                do_process_instruction_with_fee_constraints(
                    initialize(
                        &SWAP_PROGRAM_ID,
                        &spl_token::id(),
                        &accounts.payer_key,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &accounts.pool_token_key,
                        accounts.nonce,
                        accounts.fees.clone(),
                        accounts.swap_curve.clone(),
                        &accounts.pool_registry_key,
                        accounts.pool_nonce
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.payer_account,
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.pool_token_account,
                        &mut Account::default(),
                        &mut accounts.pool_registry_account,
                        &mut Account::default(),
                        &mut accounts.rent_sysvar_account,
                    ],
                    &constraints,
                )
            );
        }

        // wrong required mint in constraint
        {
            let trade_fee_numerator = 25;
            let trade_fee_denominator = 10000;
            let owner_trade_fee_numerator = 5;
            let owner_trade_fee_denominator = 10000;
            let fees = Fees {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
            };
            let curve = ConstantProductCurve {};
            let swap_curve = SwapCurve {
                curve_type: CurveType::ConstantProduct,
                calculator: Arc::new(curve),
            };
            let new_key = Pubkey::new_unique();
            let required_mint = &new_key.to_string();
            let valid_curve_types = &[CurveType::ConstantProduct];
            //use proper owner
            let fee_account =
                spl_token::state::Account::unpack(&accounts.pool_fee_account.data).unwrap();
            let owner_as_str = bs58::encode(fee_account.owner).into_string();
            let constraints = Some(SwapConstraints {
                owner_key: &owner_as_str,
                valid_curve_types,
                fees: &fees,
                required_mint,
            });
            let mut accounts = SwapAccountInfo::new(
                &user_key,
                fees.clone(),
                swap_curve,
                token_a_amount,
                token_b_amount,
            );
            assert_eq!(
                Err(SwapError::InvalidMint.into()),
                do_process_instruction_with_fee_constraints(
                    initialize(
                        &SWAP_PROGRAM_ID,
                        &spl_token::id(),
                        &accounts.payer_key,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &accounts.pool_token_key,
                        accounts.nonce,
                        accounts.fees.clone(),
                        accounts.swap_curve.clone(),
                        &accounts.pool_registry_key,
                        accounts.pool_nonce
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.payer_account,
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.pool_token_account,
                        &mut Account::default(),
                        &mut accounts.pool_registry_account,
                        &mut Account::default(),
                        &mut accounts.rent_sysvar_account,
                    ],
                    &constraints,
                )
            );
        }

        // wrong owner key in constraint
        {
            let new_key = Pubkey::new_unique();
            let trade_fee_numerator = 25;
            let trade_fee_denominator = 10000;
            let owner_trade_fee_numerator = 5;
            let owner_trade_fee_denominator = 10000;
            let fees = Fees {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
            };
            let curve = ConstantProductCurve {};
            let swap_curve = SwapCurve {
                curve_type: CurveType::ConstantProduct,
                calculator: Arc::new(curve),
            };
            let owner_key = &new_key.to_string();
            let required_mint = &new_key.to_string();
            let valid_curve_types = &[CurveType::ConstantProduct];
            let constraints = Some(SwapConstraints {
                owner_key,
                valid_curve_types,
                fees: &fees,
                required_mint,
            });
            let mut accounts = SwapAccountInfo::new(
                &user_key,
                fees.clone(),
                swap_curve,
                token_a_amount,
                token_b_amount,
            );
            assert_eq!(
                Err(SwapError::InvalidOwner.into()),
                do_process_instruction_with_fee_constraints(
                    initialize(
                        &SWAP_PROGRAM_ID,
                        &spl_token::id(),
                        &accounts.payer_key,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &accounts.pool_token_key,
                        accounts.nonce,
                        accounts.fees.clone(),
                        accounts.swap_curve.clone(),
                        &accounts.pool_registry_key,
                        accounts.pool_nonce
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.payer_account,
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.pool_token_account,
                        &mut Account::default(),
                        &mut accounts.pool_registry_account,
                        &mut Account::default(),
                        &mut accounts.rent_sysvar_account,
                    ],
                    &constraints,
                )
            );
        }
    }

    #[test]
    fn test_initialize_registry() {
        let user_key = Pubkey::new_unique();
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 2;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 10;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 5;
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };

        let token_a_amount = 1000;
        let token_b_amount = 2000;
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Arc::new(ConstantProductCurve {}),
        };

        let mut _accounts =
            SwapAccountInfo::new(&user_key, fees, swap_curve, token_a_amount, token_b_amount);

        let (_pool_registry_key, _pool_registry_account) = create_pool_registry();
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

        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };

        let token_a_amount = 1000;
        let token_b_amount = 9000;
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Arc::new(ConstantProductCurve {}),
        };

        let mut accounts =
            SwapAccountInfo::new(&user_key, fees, swap_curve, token_a_amount, token_b_amount);

        // depositing 10% of the current pool amount in token A and B means
        // that our pool tokens will be worth 1 / 10 of the current pool amount
        let pool_amount = INITIAL_SWAP_POOL_AMOUNT / 10;
        let deposit_a = token_a_amount / 10;
        let deposit_b = token_b_amount / 10;

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
                accounts.deposit_all_token_types(
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

        //fake init
        accounts.initialize_swap_mock_for_testing().unwrap();

        // wrong owner for swap account
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            let old_swap_account = accounts.swap_account;
            let mut wrong_swap_account = old_swap_account.clone();
            wrong_swap_account.owner = spl_token::id();
            accounts.swap_account = wrong_swap_account;
            assert_eq!(
                Err(ProgramError::IncorrectProgramId),
                accounts.deposit_all_token_types(
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
            accounts.swap_account = old_swap_account;
        }

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
                &spl_token::id(),
            );
            accounts.authority_key = bad_authority_key;
            assert_eq!(
                Err(SwapError::InvalidProgramAddress.into()),
                accounts.deposit_all_token_types(
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
                accounts.deposit_all_token_types(
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
                accounts.deposit_all_token_types(
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
                accounts.deposit_all_token_types(
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
                accounts.deposit_all_token_types(
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
            let user_transfer_authority_key = Pubkey::new_unique();
            assert_eq!(
                Err(TokenError::OwnerMismatch.into()),
                do_process_instruction(
                    deposit_all_token_types(
                        &SWAP_PROGRAM_ID,
                        &spl_token::id(),
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &user_transfer_authority_key,
                        &token_a_key,
                        &token_b_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &pool_key,
                        DepositAllTokenTypes {
                            pool_token_amount: pool_amount.try_into().unwrap(),
                            maximum_token_a_amount: deposit_a,
                            maximum_token_b_amount: deposit_b,
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
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
                Err(SwapError::IncorrectTokenProgramId.into()),
                do_process_instruction(
                    deposit_all_token_types(
                        &SWAP_PROGRAM_ID,
                        &wrong_key,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.authority_key,
                        &token_a_key,
                        &token_b_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &pool_key,
                        DepositAllTokenTypes {
                            pool_token_amount: pool_amount.try_into().unwrap(),
                            maximum_token_a_amount: deposit_a,
                            maximum_token_b_amount: deposit_b,
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
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
                accounts.deposit_all_token_types(
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
                accounts.deposit_all_token_types(
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
                create_mint(&spl_token::id(), &accounts.authority_key, None);
            let old_pool_key = accounts.pool_mint_key;
            let old_pool_account = accounts.pool_mint_account;
            accounts.pool_mint_key = pool_mint_key;
            accounts.pool_mint_account = pool_mint_account;

            assert_eq!(
                Err(SwapError::IncorrectPoolMint.into()),
                accounts.deposit_all_token_types(
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
                accounts.deposit_all_token_types(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    &pool_key,
                    &mut pool_account,
                    1,
                    deposit_a,
                    deposit_b,
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
                accounts.deposit_all_token_types(
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
                accounts.deposit_all_token_types(
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
                accounts.deposit_all_token_types(
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
                .deposit_all_token_types(
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
                spl_token::state::Account::unpack(&accounts.token_a_account.data).unwrap();
            assert_eq!(swap_token_a.amount, deposit_a + token_a_amount);
            let swap_token_b =
                spl_token::state::Account::unpack(&accounts.token_b_account.data).unwrap();
            assert_eq!(swap_token_b.amount, deposit_b + token_b_amount);
            let token_a = spl_token::state::Account::unpack(&token_a_account.data).unwrap();
            assert_eq!(token_a.amount, 0);
            let token_b = spl_token::state::Account::unpack(&token_b_account.data).unwrap();
            assert_eq!(token_b.amount, 0);
            let pool_account = spl_token::state::Account::unpack(&pool_account.data).unwrap();
            let swap_pool_account =
                spl_token::state::Account::unpack(&accounts.pool_token_account.data).unwrap();
            let pool_mint =
                spl_token::state::Mint::unpack(&accounts.pool_mint_account.data).unwrap();

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

        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };

        let token_a_amount = 1000;
        let token_b_amount = 2000;
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Arc::new(ConstantProductCurve {}),
        };

        let withdrawer_key = Pubkey::new_unique();
        let initial_a = token_a_amount / 10;
        let initial_b = token_b_amount / 10;
        let initial_pool = swap_curve.calculator.new_pool_supply() / 10;
        let withdraw_amount = initial_pool / 4;
        let minimum_token_a_amount = initial_a / 40;
        let minimum_token_b_amount = initial_b / 40;

        let mut accounts =
            SwapAccountInfo::new(&user_key, fees, swap_curve, token_a_amount, token_b_amount);

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
                accounts.withdraw_all_token_types(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_token_a_amount,
                    minimum_token_b_amount,
                )
            );
        }

        //fake init
        accounts.initialize_swap_mock_for_testing().unwrap();

        // wrong owner for swap account
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &withdrawer_key, initial_a, initial_b, 0);
            let old_swap_account = accounts.swap_account;
            let mut wrong_swap_account = old_swap_account.clone();
            wrong_swap_account.owner = spl_token::id();
            accounts.swap_account = wrong_swap_account;
            assert_eq!(
                Err(ProgramError::IncorrectProgramId),
                accounts.withdraw_all_token_types(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_token_a_amount,
                    minimum_token_b_amount,
                )
            );
            accounts.swap_account = old_swap_account;
        }

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
                &spl_token::id(),
            );
            accounts.authority_key = bad_authority_key;
            assert_eq!(
                Err(SwapError::InvalidProgramAddress.into()),
                accounts.withdraw_all_token_types(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_token_a_amount,
                    minimum_token_b_amount,
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
                accounts.withdraw_all_token_types(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_token_a_amount / 2,
                    minimum_token_b_amount / 2,
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
                accounts.withdraw_all_token_types(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_b_key,
                    &mut token_b_account,
                    &token_a_key,
                    &mut token_a_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_token_a_amount,
                    minimum_token_b_amount,
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
                accounts.withdraw_all_token_types(
                    &withdrawer_key,
                    &wrong_token_a_key,
                    &mut wrong_token_a_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_token_a_amount,
                    minimum_token_b_amount,
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
                accounts.withdraw_all_token_types(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_token_a_amount,
                    minimum_token_b_amount,
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
            let user_transfer_authority_key = Pubkey::new_unique();
            assert_eq!(
                Err(TokenError::OwnerMismatch.into()),
                do_process_instruction(
                    withdraw_all_token_types(
                        &SWAP_PROGRAM_ID,
                        &spl_token::id(),
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &user_transfer_authority_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &pool_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &token_a_key,
                        &token_b_key,
                        WithdrawAllTokenTypes {
                            pool_token_amount: withdraw_amount.try_into().unwrap(),
                            minimum_token_a_amount,
                            minimum_token_b_amount,
                        }
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
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
                Err(SwapError::IncorrectTokenProgramId.into()),
                do_process_instruction(
                    withdraw_all_token_types(
                        &SWAP_PROGRAM_ID,
                        &wrong_key,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.authority_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &pool_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &token_a_key,
                        &token_b_key,
                        WithdrawAllTokenTypes {
                            pool_token_amount: withdraw_amount.try_into().unwrap(),
                            minimum_token_a_amount,
                            minimum_token_b_amount,
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
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
                accounts.withdraw_all_token_types(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_token_a_amount,
                    minimum_token_b_amount,
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
                accounts.withdraw_all_token_types(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_token_a_amount,
                    minimum_token_b_amount,
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
                create_mint(&spl_token::id(), &accounts.authority_key, None);
            let old_pool_key = accounts.pool_mint_key;
            let old_pool_account = accounts.pool_mint_account;
            accounts.pool_mint_key = pool_mint_key;
            accounts.pool_mint_account = pool_mint_account;

            assert_eq!(
                Err(SwapError::IncorrectPoolMint.into()),
                accounts.withdraw_all_token_types(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_token_a_amount,
                    minimum_token_b_amount,
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
                accounts.withdraw_all_token_types(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    1,
                    0,
                    0,
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
                accounts.withdraw_all_token_types(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_token_a_amount * 10,
                    minimum_token_b_amount,
                )
            );
            // minimum B amount out too high
            assert_eq!(
                Err(SwapError::ExceededSlippage.into()),
                accounts.withdraw_all_token_types(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_token_a_amount,
                    minimum_token_b_amount * 10,
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
                accounts.withdraw_all_token_types(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &swap_token_a_key,
                    &mut swap_token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_token_a_amount,
                    minimum_token_b_amount,
                )
            );
            let swap_token_b_key = accounts.token_b_key;
            let mut swap_token_b_account = accounts.get_token_account(&swap_token_b_key).clone();
            assert_eq!(
                Err(SwapError::InvalidInput.into()),
                accounts.withdraw_all_token_types(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &swap_token_b_key,
                    &mut swap_token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_token_a_amount,
                    minimum_token_b_amount,
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
                .withdraw_all_token_types(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    withdraw_amount.try_into().unwrap(),
                    minimum_token_a_amount,
                    minimum_token_b_amount,
                )
                .unwrap();

            let swap_token_a =
                spl_token::state::Account::unpack(&accounts.token_a_account.data).unwrap();
            let swap_token_b =
                spl_token::state::Account::unpack(&accounts.token_b_account.data).unwrap();
            let pool_mint =
                spl_token::state::Mint::unpack(&accounts.pool_mint_account.data).unwrap();
            let withdraw_fee = accounts.fees.owner_withdraw_fee(withdraw_amount).unwrap();
            let results = accounts
                .swap_curve
                .calculator
                .pool_tokens_to_trading_tokens(
                    withdraw_amount - withdraw_fee,
                    pool_mint.supply.try_into().unwrap(),
                    swap_token_a.amount.try_into().unwrap(),
                    swap_token_b.amount.try_into().unwrap(),
                    RoundDirection::Floor,
                )
                .unwrap();
            assert_eq!(
                swap_token_a.amount,
                token_a_amount - to_u64(results.token_a_amount).unwrap()
            );
            assert_eq!(
                swap_token_b.amount,
                token_b_amount - to_u64(results.token_b_amount).unwrap()
            );
            let token_a = spl_token::state::Account::unpack(&token_a_account.data).unwrap();
            assert_eq!(
                token_a.amount,
                initial_a + to_u64(results.token_a_amount).unwrap()
            );
            let token_b = spl_token::state::Account::unpack(&token_b_account.data).unwrap();
            assert_eq!(
                token_b.amount,
                initial_b + to_u64(results.token_b_amount).unwrap()
            );
            let pool_account = spl_token::state::Account::unpack(&pool_account.data).unwrap();
            assert_eq!(
                pool_account.amount,
                to_u64(initial_pool - withdraw_amount).unwrap()
            );
            let fee_account =
                spl_token::state::Account::unpack(&accounts.pool_fee_account.data).unwrap();
            assert_eq!(
                fee_account.amount,
                TryInto::<u64>::try_into(withdraw_fee).unwrap()
            );
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
            let fee_account = spl_token::state::Account::unpack(&pool_fee_account.data).unwrap();
            let pool_fee_amount = fee_account.amount;

            accounts
                .withdraw_all_token_types(
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
                spl_token::state::Account::unpack(&accounts.token_a_account.data).unwrap();
            let swap_token_b =
                spl_token::state::Account::unpack(&accounts.token_b_account.data).unwrap();
            let pool_mint =
                spl_token::state::Mint::unpack(&accounts.pool_mint_account.data).unwrap();
            let results = accounts
                .swap_curve
                .calculator
                .pool_tokens_to_trading_tokens(
                    pool_fee_amount.try_into().unwrap(),
                    pool_mint.supply.try_into().unwrap(),
                    swap_token_a.amount.try_into().unwrap(),
                    swap_token_b.amount.try_into().unwrap(),
                    RoundDirection::Floor,
                )
                .unwrap();
            let token_a = spl_token::state::Account::unpack(&token_a_account.data).unwrap();
            assert_eq!(
                token_a.amount,
                TryInto::<u64>::try_into(results.token_a_amount).unwrap()
            );
            let token_b = spl_token::state::Account::unpack(&token_b_account.data).unwrap();
            assert_eq!(
                token_b.amount,
                TryInto::<u64>::try_into(results.token_b_amount).unwrap()
            );
        }
    }

    #[test]
    fn test_deposit_one_exact_in() {
        let user_key = Pubkey::new_unique();
        let depositor_key = Pubkey::new_unique();
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 2;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 10;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 5;

        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };

        let token_a_amount = 1000;
        let token_b_amount = 9000;
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Arc::new(ConstantProductCurve {}),
        };

        let mut accounts =
            SwapAccountInfo::new(&user_key, fees, swap_curve, token_a_amount, token_b_amount);

        let deposit_a = token_a_amount / 10;
        let deposit_b = token_b_amount / 10;
        let pool_amount = to_u64(INITIAL_SWAP_POOL_AMOUNT / 100).unwrap();

        // swap not initialized
        {
            let (
                token_a_key,
                mut token_a_account,
                _token_b_key,
                _token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            assert_eq!(
                Err(ProgramError::UninitializedAccount),
                accounts.deposit_single_token_type_exact_amount_in(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &pool_key,
                    &mut pool_account,
                    deposit_a,
                    pool_amount,
                )
            );
        }

        //fake init
        accounts.initialize_swap_mock_for_testing().unwrap();

        // wrong owner for swap account
        {
            let (
                token_a_key,
                mut token_a_account,
                _token_b_key,
                _token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            let old_swap_account = accounts.swap_account;
            let mut wrong_swap_account = old_swap_account.clone();
            wrong_swap_account.owner = spl_token::id();
            accounts.swap_account = wrong_swap_account;
            assert_eq!(
                Err(ProgramError::IncorrectProgramId),
                accounts.deposit_single_token_type_exact_amount_in(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &pool_key,
                    &mut pool_account,
                    deposit_a,
                    pool_amount,
                )
            );
            accounts.swap_account = old_swap_account;
        }

        // wrong nonce for authority_key
        {
            let (
                token_a_key,
                mut token_a_account,
                _token_b_key,
                _token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            let old_authority = accounts.authority_key;
            let (bad_authority_key, _nonce) = Pubkey::find_program_address(
                &[&accounts.swap_key.to_bytes()[..]],
                &spl_token::id(),
            );
            accounts.authority_key = bad_authority_key;
            assert_eq!(
                Err(SwapError::InvalidProgramAddress.into()),
                accounts.deposit_single_token_type_exact_amount_in(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &pool_key,
                    &mut pool_account,
                    deposit_a,
                    pool_amount,
                )
            );
            accounts.authority_key = old_authority;
        }

        // not enough token A / B
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
                deposit_b / 2,
                0,
            );
            assert_eq!(
                Err(TokenError::InsufficientFunds.into()),
                accounts.deposit_single_token_type_exact_amount_in(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &pool_key,
                    &mut pool_account,
                    deposit_a,
                    0,
                )
            );
            assert_eq!(
                Err(TokenError::InsufficientFunds.into()),
                accounts.deposit_single_token_type_exact_amount_in(
                    &depositor_key,
                    &token_b_key,
                    &mut token_b_account,
                    &pool_key,
                    &mut pool_account,
                    deposit_b,
                    0,
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
            assert_eq!(
                Err(TokenError::MintMismatch.into()),
                accounts.deposit_single_token_type_exact_amount_in(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    deposit_a,
                    pool_amount,
                )
            );
        }

        // no approval
        {
            let (
                token_a_key,
                mut token_a_account,
                _token_b_key,
                _token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            let user_transfer_authority_key = Pubkey::new_unique();
            assert_eq!(
                Err(TokenError::OwnerMismatch.into()),
                do_process_instruction(
                    deposit_single_token_type_exact_amount_in(
                        &SWAP_PROGRAM_ID,
                        &spl_token::id(),
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &user_transfer_authority_key,
                        &token_a_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &pool_key,
                        DepositSingleTokenTypeExactAmountIn {
                            source_token_amount: deposit_a,
                            minimum_pool_token_amount: pool_amount,
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut Account::default(),
                        &mut token_a_account,
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
                _token_b_key,
                _token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            let wrong_key = Pubkey::new_unique();
            assert_eq!(
                Err(SwapError::IncorrectTokenProgramId.into()),
                do_process_instruction(
                    deposit_single_token_type_exact_amount_in(
                        &SWAP_PROGRAM_ID,
                        &wrong_key,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.authority_key,
                        &token_a_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &pool_key,
                        DepositSingleTokenTypeExactAmountIn {
                            source_token_amount: deposit_a,
                            minimum_pool_token_amount: pool_amount,
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut Account::default(),
                        &mut token_a_account,
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
                token_b_account,
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
                accounts.deposit_single_token_type_exact_amount_in(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &pool_key,
                    &mut pool_account,
                    deposit_a,
                    pool_amount,
                )
            );

            accounts.token_a_key = old_a_key;
            accounts.token_a_account = old_a_account;

            let old_b_key = accounts.token_b_key;
            let old_b_account = accounts.token_b_account;

            accounts.token_b_key = token_b_key;
            accounts.token_b_account = token_b_account;

            // wrong swap token b account
            assert_eq!(
                Err(SwapError::IncorrectSwapAccount.into()),
                accounts.deposit_single_token_type_exact_amount_in(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &pool_key,
                    &mut pool_account,
                    deposit_a,
                    pool_amount,
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
                _token_b_key,
                _token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            let (pool_mint_key, pool_mint_account) =
                create_mint(&spl_token::id(), &accounts.authority_key, None);
            let old_pool_key = accounts.pool_mint_key;
            let old_pool_account = accounts.pool_mint_account;
            accounts.pool_mint_key = pool_mint_key;
            accounts.pool_mint_account = pool_mint_account;

            assert_eq!(
                Err(SwapError::IncorrectPoolMint.into()),
                accounts.deposit_single_token_type_exact_amount_in(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &pool_key,
                    &mut pool_account,
                    deposit_a,
                    pool_amount,
                )
            );

            accounts.pool_mint_key = old_pool_key;
            accounts.pool_mint_account = old_pool_account;
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
            // minimum pool amount too high
            assert_eq!(
                Err(SwapError::ExceededSlippage.into()),
                accounts.deposit_single_token_type_exact_amount_in(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &pool_key,
                    &mut pool_account,
                    deposit_a / 10,
                    pool_amount,
                )
            );
            // minimum pool amount too high
            assert_eq!(
                Err(SwapError::ExceededSlippage.into()),
                accounts.deposit_single_token_type_exact_amount_in(
                    &depositor_key,
                    &token_b_key,
                    &mut token_b_account,
                    &pool_key,
                    &mut pool_account,
                    deposit_b / 10,
                    pool_amount,
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
                accounts.deposit_single_token_type_exact_amount_in(
                    &authority_key,
                    &swap_token_a_key,
                    &mut swap_token_a_account,
                    &pool_key,
                    &mut pool_account,
                    deposit_a,
                    pool_amount,
                )
            );
            assert_eq!(
                Err(SwapError::InvalidInput.into()),
                accounts.deposit_single_token_type_exact_amount_in(
                    &authority_key,
                    &swap_token_b_key,
                    &mut swap_token_b_account,
                    &pool_key,
                    &mut pool_account,
                    deposit_b,
                    pool_amount,
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
                .deposit_single_token_type_exact_amount_in(
                    &depositor_key,
                    &token_a_key,
                    &mut token_a_account,
                    &pool_key,
                    &mut pool_account,
                    deposit_a,
                    pool_amount,
                )
                .unwrap();

            let swap_token_a =
                spl_token::state::Account::unpack(&accounts.token_a_account.data).unwrap();
            assert_eq!(swap_token_a.amount, deposit_a + token_a_amount);

            let token_a = spl_token::state::Account::unpack(&token_a_account.data).unwrap();
            assert_eq!(token_a.amount, 0);

            accounts
                .deposit_single_token_type_exact_amount_in(
                    &depositor_key,
                    &token_b_key,
                    &mut token_b_account,
                    &pool_key,
                    &mut pool_account,
                    deposit_b,
                    pool_amount,
                )
                .unwrap();
            let swap_token_b =
                spl_token::state::Account::unpack(&accounts.token_b_account.data).unwrap();
            assert_eq!(swap_token_b.amount, deposit_b + token_b_amount);

            let token_b = spl_token::state::Account::unpack(&token_b_account.data).unwrap();
            assert_eq!(token_b.amount, 0);

            let pool_account = spl_token::state::Account::unpack(&pool_account.data).unwrap();
            let swap_pool_account =
                spl_token::state::Account::unpack(&accounts.pool_token_account.data).unwrap();
            let pool_mint =
                spl_token::state::Mint::unpack(&accounts.pool_mint_account.data).unwrap();
            assert_eq!(
                pool_mint.supply,
                pool_account.amount + swap_pool_account.amount
            );
        }
    }

    #[test]
    fn test_withdraw_one_exact_out() {
        let user_key = Pubkey::new_unique();
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 2;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 10;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 5;

        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };

        let token_a_amount = 100_000;
        let token_b_amount = 200_000;
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Arc::new(ConstantProductCurve {}),
        };

        let withdrawer_key = Pubkey::new_unique();
        let initial_a = token_a_amount / 10;
        let initial_b = token_b_amount / 10;
        let initial_pool = swap_curve.calculator.new_pool_supply() / 10;
        let maximum_pool_token_amount = to_u64(initial_pool / 4).unwrap();
        let destination_a_amount = initial_a / 40;
        let destination_b_amount = initial_b / 40;

        let mut accounts =
            SwapAccountInfo::new(&user_key, fees, swap_curve, token_a_amount, token_b_amount);

        // swap not initialized
        {
            let (
                token_a_key,
                mut token_a_account,
                _token_b_key,
                _token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &withdrawer_key, initial_a, initial_b, 0);
            assert_eq!(
                Err(ProgramError::UninitializedAccount),
                accounts.withdraw_single_token_type_exact_amount_out(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    destination_a_amount,
                    maximum_pool_token_amount,
                )
            );
        }

        //fake init
        accounts.initialize_swap_mock_for_testing().unwrap();

        // wrong owner for swap account
        {
            let (
                token_a_key,
                mut token_a_account,
                _token_b_key,
                _token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &withdrawer_key, initial_a, initial_b, 0);
            let old_swap_account = accounts.swap_account;
            let mut wrong_swap_account = old_swap_account.clone();
            wrong_swap_account.owner = spl_token::id();
            accounts.swap_account = wrong_swap_account;
            assert_eq!(
                Err(ProgramError::IncorrectProgramId),
                accounts.withdraw_single_token_type_exact_amount_out(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    destination_a_amount,
                    maximum_pool_token_amount,
                )
            );
            accounts.swap_account = old_swap_account;
        }

        // wrong nonce for authority_key
        {
            let (
                _token_a_key,
                _token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &withdrawer_key, initial_a, initial_b, 0);
            let old_authority = accounts.authority_key;
            let (bad_authority_key, _nonce) = Pubkey::find_program_address(
                &[&accounts.swap_key.to_bytes()[..]],
                &spl_token::id(),
            );
            accounts.authority_key = bad_authority_key;
            assert_eq!(
                Err(SwapError::InvalidProgramAddress.into()),
                accounts.withdraw_single_token_type_exact_amount_out(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_b_key,
                    &mut token_b_account,
                    destination_b_amount,
                    maximum_pool_token_amount,
                )
            );
            accounts.authority_key = old_authority;
        }

        // not enough pool tokens
        {
            let (
                _token_a_key,
                _token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                initial_a,
                initial_b,
                maximum_pool_token_amount / 1000,
            );
            assert_eq!(
                Err(TokenError::InsufficientFunds.into()),
                accounts.withdraw_single_token_type_exact_amount_out(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_b_key,
                    &mut token_b_account,
                    destination_b_amount,
                    maximum_pool_token_amount,
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
                maximum_pool_token_amount,
                initial_b,
                maximum_pool_token_amount,
            );
            assert_eq!(
                Err(TokenError::MintMismatch.into()),
                accounts.withdraw_single_token_type_exact_amount_out(
                    &withdrawer_key,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    destination_b_amount,
                    maximum_pool_token_amount,
                )
            );
        }

        // wrong pool fee account
        {
            let (
                token_a_key,
                mut token_a_account,
                _token_b_key,
                _token_b_account,
                wrong_pool_key,
                wrong_pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                initial_a,
                initial_b,
                maximum_pool_token_amount,
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
                maximum_pool_token_amount,
            );
            let old_pool_fee_account = accounts.pool_fee_account;
            let old_pool_fee_key = accounts.pool_fee_key;
            accounts.pool_fee_account = wrong_pool_account;
            accounts.pool_fee_key = wrong_pool_key;
            assert_eq!(
                Err(SwapError::IncorrectFeeAccount.into()),
                accounts.withdraw_single_token_type_exact_amount_out(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    destination_a_amount,
                    maximum_pool_token_amount,
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
                _token_b_key,
                _token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                0,
                0,
                maximum_pool_token_amount,
            );
            let user_transfer_authority_key = Pubkey::new_unique();
            assert_eq!(
                Err(TokenError::OwnerMismatch.into()),
                do_process_instruction(
                    withdraw_single_token_type_exact_amount_out(
                        &SWAP_PROGRAM_ID,
                        &spl_token::id(),
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &user_transfer_authority_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &pool_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &token_a_key,
                        WithdrawSingleTokenTypeExactAmountOut {
                            destination_token_amount: destination_a_amount,
                            maximum_pool_token_amount,
                        }
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut Account::default(),
                        &mut accounts.pool_mint_account,
                        &mut pool_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut token_a_account,
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
                _token_b_key,
                _token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                initial_a,
                initial_b,
                maximum_pool_token_amount,
            );
            let wrong_key = Pubkey::new_unique();
            assert_eq!(
                Err(SwapError::IncorrectTokenProgramId.into()),
                do_process_instruction(
                    withdraw_single_token_type_exact_amount_out(
                        &SWAP_PROGRAM_ID,
                        &wrong_key,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.authority_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &pool_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &token_a_key,
                        WithdrawSingleTokenTypeExactAmountOut {
                            destination_token_amount: destination_a_amount,
                            maximum_pool_token_amount,
                        }
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut Account::default(),
                        &mut accounts.pool_mint_account,
                        &mut pool_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut token_a_account,
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
                accounts.withdraw_single_token_type_exact_amount_out(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    destination_a_amount,
                    maximum_pool_token_amount,
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
                accounts.withdraw_single_token_type_exact_amount_out(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_b_key,
                    &mut token_b_account,
                    destination_b_amount,
                    maximum_pool_token_amount,
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
                _token_b_key,
                _token_b_account,
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
                create_mint(&spl_token::id(), &accounts.authority_key, None);
            let old_pool_key = accounts.pool_mint_key;
            let old_pool_account = accounts.pool_mint_account;
            accounts.pool_mint_key = pool_mint_key;
            accounts.pool_mint_account = pool_mint_account;

            assert_eq!(
                Err(SwapError::IncorrectPoolMint.into()),
                accounts.withdraw_single_token_type_exact_amount_out(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    destination_a_amount,
                    maximum_pool_token_amount,
                )
            );

            accounts.pool_mint_key = old_pool_key;
            accounts.pool_mint_account = old_pool_account;
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
                maximum_pool_token_amount,
            );

            // maximum pool token amount too low
            assert_eq!(
                Err(SwapError::ExceededSlippage.into()),
                accounts.withdraw_single_token_type_exact_amount_out(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    destination_a_amount,
                    maximum_pool_token_amount / 1000,
                )
            );
            assert_eq!(
                Err(SwapError::ExceededSlippage.into()),
                accounts.withdraw_single_token_type_exact_amount_out(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_b_key,
                    &mut token_b_account,
                    destination_b_amount,
                    maximum_pool_token_amount / 1000,
                )
            );
        }

        // invalid input: can't use swap pool tokens as destination
        {
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
                maximum_pool_token_amount,
            );
            let swap_token_a_key = accounts.token_a_key;
            let mut swap_token_a_account = accounts.get_token_account(&swap_token_a_key).clone();
            assert_eq!(
                Err(SwapError::InvalidInput.into()),
                accounts.withdraw_single_token_type_exact_amount_out(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &swap_token_a_key,
                    &mut swap_token_a_account,
                    destination_a_amount,
                    maximum_pool_token_amount,
                )
            );
            let swap_token_b_key = accounts.token_b_key;
            let mut swap_token_b_account = accounts.get_token_account(&swap_token_b_key).clone();
            assert_eq!(
                Err(SwapError::InvalidInput.into()),
                accounts.withdraw_single_token_type_exact_amount_out(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &swap_token_b_key,
                    &mut swap_token_b_account,
                    destination_b_amount,
                    maximum_pool_token_amount,
                )
            );
        }

        // correct withdrawal
        {
            let (
                token_a_key,
                mut token_a_account,
                _token_b_key,
                _token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                initial_a,
                initial_b,
                initial_pool.try_into().unwrap(),
            );

            let swap_token_a =
                spl_token::state::Account::unpack(&accounts.token_a_account.data).unwrap();
            let swap_token_b =
                spl_token::state::Account::unpack(&accounts.token_b_account.data).unwrap();
            let pool_mint =
                spl_token::state::Mint::unpack(&accounts.pool_mint_account.data).unwrap();

            let pool_token_amount = accounts
                .swap_curve
                .withdraw_single_token_type_exact_out(
                    destination_a_amount.try_into().unwrap(),
                    swap_token_a.amount.try_into().unwrap(),
                    swap_token_b.amount.try_into().unwrap(),
                    pool_mint.supply.try_into().unwrap(),
                    TradeDirection::AtoB,
                    &accounts.fees,
                )
                .unwrap();
            let withdraw_fee = accounts.fees.owner_withdraw_fee(pool_token_amount).unwrap();

            accounts
                .withdraw_single_token_type_exact_amount_out(
                    &withdrawer_key,
                    &pool_key,
                    &mut pool_account,
                    &token_a_key,
                    &mut token_a_account,
                    destination_a_amount,
                    maximum_pool_token_amount,
                )
                .unwrap();

            let swap_token_a =
                spl_token::state::Account::unpack(&accounts.token_a_account.data).unwrap();

            assert_eq!(swap_token_a.amount, token_a_amount - destination_a_amount);
            let token_a = spl_token::state::Account::unpack(&token_a_account.data).unwrap();
            assert_eq!(token_a.amount, initial_a + destination_a_amount);

            let pool_account = spl_token::state::Account::unpack(&pool_account.data).unwrap();
            assert_eq!(
                pool_account.amount,
                to_u64(initial_pool - pool_token_amount - withdraw_fee).unwrap()
            );
            let fee_account =
                spl_token::state::Account::unpack(&accounts.pool_fee_account.data).unwrap();
            assert_eq!(fee_account.amount, to_u64(withdraw_fee).unwrap());
        }

        // correct withdrawal from fee account
        {
            let (
                token_a_key,
                mut token_a_account,
                _token_b_key,
                _token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &withdrawer_key, initial_a, initial_b, 0);

            let fee_a_amount = 2;
            let pool_fee_key = accounts.pool_fee_key;
            let mut pool_fee_account = accounts.pool_fee_account.clone();
            let fee_account = spl_token::state::Account::unpack(&pool_fee_account.data).unwrap();
            let pool_fee_amount = fee_account.amount;

            let swap_token_a =
                spl_token::state::Account::unpack(&accounts.token_a_account.data).unwrap();

            let token_a_amount = swap_token_a.amount;
            accounts
                .withdraw_single_token_type_exact_amount_out(
                    &user_key,
                    &pool_fee_key,
                    &mut pool_fee_account,
                    &token_a_key,
                    &mut token_a_account,
                    fee_a_amount,
                    pool_fee_amount,
                )
                .unwrap();

            let swap_token_a =
                spl_token::state::Account::unpack(&accounts.token_a_account.data).unwrap();

            assert_eq!(swap_token_a.amount, token_a_amount - fee_a_amount);
            let token_a = spl_token::state::Account::unpack(&token_a_account.data).unwrap();
            assert_eq!(token_a.amount, initial_a + fee_a_amount);
        }
    }

    fn check_valid_swap_curve(
        fees: Fees,
        curve_type: CurveType,
        calculator: Arc<dyn CurveCalculator + Send + Sync>,
        token_a_amount: u64,
        token_b_amount: u64,
    ) {
        let user_key = Pubkey::new_unique();
        let swapper_key = Pubkey::new_unique();

        let swap_curve = SwapCurve {
            curve_type,
            calculator,
        };

        let mut accounts = SwapAccountInfo::new(
            &user_key,
            fees.clone(),
            swap_curve.clone(),
            token_a_amount,
            token_b_amount,
        );
        let initial_a = token_a_amount / 5;
        let initial_b = token_b_amount / 5;

        //fake init
        accounts.initialize_swap_mock_for_testing().unwrap();

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
        let minimum_token_b_amount = 0;
        let pool_mint = spl_token::state::Mint::unpack(&accounts.pool_mint_account.data).unwrap();
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
                minimum_token_b_amount,
            )
            .unwrap();

        let results = swap_curve
            .swap(
                a_to_b_amount.try_into().unwrap(),
                token_a_amount.try_into().unwrap(),
                token_b_amount.try_into().unwrap(),
                TradeDirection::AtoB,
                &fees,
            )
            .unwrap();

        let swap_token_a =
            spl_token::state::Account::unpack(&accounts.token_a_account.data).unwrap();
        let token_a_amount = swap_token_a.amount;
        assert_eq!(
            token_a_amount,
            TryInto::<u64>::try_into(results.new_swap_source_amount).unwrap()
        );
        let token_a = spl_token::state::Account::unpack(&token_a_account.data).unwrap();
        assert_eq!(token_a.amount, initial_a - a_to_b_amount);

        let swap_token_b =
            spl_token::state::Account::unpack(&accounts.token_b_account.data).unwrap();
        let token_b_amount = swap_token_b.amount;
        assert_eq!(
            token_b_amount,
            TryInto::<u64>::try_into(results.new_swap_destination_amount).unwrap()
        );
        let token_b = spl_token::state::Account::unpack(&token_b_account.data).unwrap();
        assert_eq!(
            token_b.amount,
            initial_b + to_u64(results.destination_amount_swapped).unwrap()
        );

        let first_fee = swap_curve
            .withdraw_single_token_type_exact_out(
                results.owner_fee,
                token_a_amount.try_into().unwrap(),
                token_b_amount.try_into().unwrap(),
                initial_supply.try_into().unwrap(),
                TradeDirection::AtoB,
                &fees,
            )
            .unwrap();
        let fee_account =
            spl_token::state::Account::unpack(&accounts.pool_fee_account.data).unwrap();
        assert_eq!(
            fee_account.amount,
            TryInto::<u64>::try_into(first_fee).unwrap()
        );

        let first_swap_amount = results.destination_amount_swapped;

        // swap the other way
        let pool_mint = spl_token::state::Mint::unpack(&accounts.pool_mint_account.data).unwrap();
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
            .swap(
                b_to_a_amount.try_into().unwrap(),
                token_b_amount.try_into().unwrap(),
                token_a_amount.try_into().unwrap(),
                TradeDirection::BtoA,
                &fees,
            )
            .unwrap();

        let swap_token_a =
            spl_token::state::Account::unpack(&accounts.token_a_account.data).unwrap();
        let token_a_amount = swap_token_a.amount;
        assert_eq!(
            token_a_amount,
            TryInto::<u64>::try_into(results.new_swap_destination_amount).unwrap()
        );
        let token_a = spl_token::state::Account::unpack(&token_a_account.data).unwrap();
        assert_eq!(
            token_a.amount,
            initial_a - a_to_b_amount + to_u64(results.destination_amount_swapped).unwrap()
        );

        let swap_token_b =
            spl_token::state::Account::unpack(&accounts.token_b_account.data).unwrap();
        let token_b_amount = swap_token_b.amount;
        assert_eq!(
            token_b_amount,
            TryInto::<u64>::try_into(results.new_swap_source_amount).unwrap()
        );
        let token_b = spl_token::state::Account::unpack(&token_b_account.data).unwrap();
        assert_eq!(
            token_b.amount,
            initial_b + to_u64(first_swap_amount).unwrap()
                - to_u64(results.source_amount_swapped).unwrap()
        );

        let second_fee = swap_curve
            .withdraw_single_token_type_exact_out(
                results.owner_fee,
                token_a_amount.try_into().unwrap(),
                token_b_amount.try_into().unwrap(),
                initial_supply.try_into().unwrap(),
                TradeDirection::BtoA,
                &fees,
            )
            .unwrap();
        let fee_account =
            spl_token::state::Account::unpack(&accounts.pool_fee_account.data).unwrap();
        assert_eq!(fee_account.amount, to_u64(first_fee + second_fee).unwrap());
    }

    #[test]
    fn test_valid_swap_curves_all_fees() {
        // All fees
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 10;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 30;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 30;
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };

        let token_a_amount = 10_000_000_000;
        let token_b_amount = 50_000_000_000;

        check_valid_swap_curve(
            fees.clone(),
            CurveType::ConstantProduct,
            Arc::new(ConstantProductCurve {}),
            token_a_amount,
            token_b_amount,
        );
        let token_b_price = 1;
        check_valid_swap_curve(
            fees.clone(),
            CurveType::ConstantPrice,
            Arc::new(ConstantPriceCurve { token_b_price }),
            token_a_amount,
            token_b_amount,
        );
        let token_b_offset = 10_000_000_000;
        check_valid_swap_curve(
            fees,
            CurveType::Offset,
            Arc::new(OffsetCurve { token_b_offset }),
            token_a_amount,
            token_b_amount,
        );
    }

    #[test]
    fn test_valid_swap_curves_trade_fee_only() {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 10;
        let owner_trade_fee_numerator = 0;
        let owner_trade_fee_denominator = 0;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 0;
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };

        let token_a_amount = 10_000_000_000;
        let token_b_amount = 50_000_000_000;

        check_valid_swap_curve(
            fees.clone(),
            CurveType::ConstantProduct,
            Arc::new(ConstantProductCurve {}),
            token_a_amount,
            token_b_amount,
        );
        let token_b_price = 10_000;
        check_valid_swap_curve(
            fees.clone(),
            CurveType::ConstantPrice,
            Arc::new(ConstantPriceCurve { token_b_price }),
            token_a_amount,
            token_b_amount / token_b_price,
        );
        let token_b_offset = 1;
        check_valid_swap_curve(
            fees,
            CurveType::Offset,
            Arc::new(OffsetCurve { token_b_offset }),
            token_a_amount,
            token_b_amount,
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
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };

        let token_a_amount = 1000;
        let token_b_amount = 5000;
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Arc::new(ConstantProductCurve {}),
        };
        let mut accounts =
            SwapAccountInfo::new(&user_key, fees, swap_curve, token_a_amount, token_b_amount);

        let initial_a = token_a_amount / 5;
        let initial_b = token_b_amount / 5;
        let minimum_token_b_amount = initial_b / 2;

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
                    minimum_token_b_amount,
                )
            );
        }

        //fake init
        accounts.initialize_swap_mock_for_testing().unwrap();

        // wrong swap account program id
        {
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                _pool_key,
                _pool_account,
            ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);
            let old_swap_account = accounts.swap_account;
            let mut wrong_swap_account = old_swap_account.clone();
            wrong_swap_account.owner = spl_token::id();
            accounts.swap_account = wrong_swap_account;
            assert_eq!(
                Err(ProgramError::IncorrectProgramId),
                accounts.swap(
                    &swapper_key,
                    &token_a_key,
                    &mut token_a_account,
                    &swap_token_a_key,
                    &swap_token_b_key,
                    &token_b_key,
                    &mut token_b_account,
                    initial_a,
                    minimum_token_b_amount,
                )
            );
            accounts.swap_account = old_swap_account;
        }

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
                &spl_token::id(),
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
                    minimum_token_b_amount,
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
                Err(SwapError::IncorrectTokenProgramId.into()),
                do_process_instruction(
                    swap(
                        &SWAP_PROGRAM_ID,
                        &wrong_program_id,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.authority_key,
                        &token_a_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        None,
                        &user_key,
                        Swap {
                            amount_in: initial_a,
                            minimum_amount_out: minimum_token_b_amount,
                            flags: swap_flags::default(),
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut Account::default(),
                        &mut token_a_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut Account::default(),
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
                    minimum_token_b_amount * 2,
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
            let user_transfer_key = Pubkey::new_unique();
            assert_eq!(
                Err(SwapError::IncorrectSwapAccount.into()),
                do_process_instruction(
                    swap(
                        &SWAP_PROGRAM_ID,
                        &spl_token::id(),
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &user_transfer_key,
                        &token_a_key,
                        &token_a_key,
                        &token_b_key,
                        &token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        None,
                        &user_key,
                        Swap {
                            amount_in: initial_a,
                            minimum_amount_out: minimum_token_b_amount,
                            flags: swap_flags::default(),
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut Account::default(),
                        &mut token_a_account.clone(),
                        &mut token_a_account,
                        &mut token_b_account.clone(),
                        &mut token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut Account::default(),
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
                    minimum_token_b_amount,
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
                    minimum_token_b_amount,
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
                create_mint(&spl_token::id(), &accounts.authority_key, None);
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
                    minimum_token_b_amount,
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
                    minimum_token_b_amount,
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
            let user_transfer_key = Pubkey::new_unique();
            assert_eq!(
                Err(TokenError::OwnerMismatch.into()),
                do_process_instruction(
                    swap(
                        &SWAP_PROGRAM_ID,
                        &spl_token::id(),
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &user_transfer_key,
                        &token_a_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        None,
                        &user_key,
                        Swap {
                            amount_in: initial_a,
                            minimum_amount_out: minimum_token_b_amount,
                            flags: swap_flags::default(),
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut Account::default(),
                        &mut token_a_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut Account::default(),
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
                    minimum_token_b_amount * 2,
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
                    minimum_token_b_amount,
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
                    minimum_token_b_amount,
                )
            );
        }
    }

    #[test]
    fn test_overdraw_offset_curve() {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 10;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 30;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 30;

        let token_a_amount = 1_000_000_000;
        let token_b_amount = 0;
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };

        let token_b_offset = 2_000_000;
        let swap_curve = SwapCurve {
            curve_type: CurveType::Offset,
            calculator: Arc::new(OffsetCurve { token_b_offset }),
        };
        let user_key = Pubkey::new_unique();
        let swapper_key = Pubkey::new_unique();

        let mut accounts =
            SwapAccountInfo::new(&user_key, fees, swap_curve, token_a_amount, token_b_amount);

        //fake init
        accounts.initialize_swap_mock_for_testing().unwrap();

        let swap_token_a_key = accounts.token_a_key;
        let swap_token_b_key = accounts.token_b_key;
        let initial_a = 500_000;
        let initial_b = 1_000;

        let (
            token_a_key,
            mut token_a_account,
            token_b_key,
            mut token_b_account,
            _pool_key,
            _pool_account,
        ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);

        // swap a to b way, fails, there's no liquidity
        let a_to_b_amount = initial_a;
        let minimum_token_b_amount = 0;

        assert_eq!(
            Err(SwapError::ZeroTradingTokens.into()),
            accounts.swap(
                &swapper_key,
                &token_a_key,
                &mut token_a_account,
                &swap_token_a_key,
                &swap_token_b_key,
                &token_b_key,
                &mut token_b_account,
                a_to_b_amount,
                minimum_token_b_amount,
            )
        );

        // swap b to a, succeeds at offset price
        let b_to_a_amount = initial_b;
        let minimum_token_a_amount = 0;
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
                minimum_token_a_amount,
            )
            .unwrap();

        // try a to b again, succeeds due to new liquidity
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
                minimum_token_b_amount,
            )
            .unwrap();

        // try a to b again, fails due to no more liquidity
        assert_eq!(
            Err(SwapError::ZeroTradingTokens.into()),
            accounts.swap(
                &swapper_key,
                &token_a_key,
                &mut token_a_account,
                &swap_token_a_key,
                &swap_token_b_key,
                &token_b_key,
                &mut token_b_account,
                a_to_b_amount,
                minimum_token_b_amount,
            )
        );

        // Try to deposit, fails because deposits are not allowed for offset
        // curve swaps
        {
            let initial_a = 100;
            let initial_b = 100;
            let pool_amount = 100;
            let (
                token_a_key,
                mut token_a_account,
                token_b_key,
                mut token_b_account,
                pool_key,
                mut pool_account,
            ) = accounts.setup_token_accounts(&user_key, &swapper_key, initial_a, initial_b, 0);
            assert_eq!(
                Err(SwapError::UnsupportedCurveOperation.into()),
                accounts.deposit_all_token_types(
                    &swapper_key,
                    &token_a_key,
                    &mut token_a_account,
                    &token_b_key,
                    &mut token_b_account,
                    &pool_key,
                    &mut pool_account,
                    pool_amount,
                    initial_a,
                    initial_b,
                )
            );
        }
    }

    #[test]
    fn test_withdraw_all_offset_curve() {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 10;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 30;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 30;

        let token_a_amount = 1_000_000_000;
        let token_b_amount = 10;
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };

        let token_b_offset = 2_000_000;
        let swap_curve = SwapCurve {
            curve_type: CurveType::Offset,
            calculator: Arc::new(OffsetCurve { token_b_offset }),
        };
        let total_pool = swap_curve.calculator.new_pool_supply();
        let user_key = Pubkey::new_unique();
        let withdrawer_key = Pubkey::new_unique();

        let mut accounts =
            SwapAccountInfo::new(&user_key, fees, swap_curve, token_a_amount, token_b_amount);

        //fake init
        accounts.initialize_swap_mock_for_testing().unwrap();

        let (
            token_a_key,
            mut token_a_account,
            token_b_key,
            mut token_b_account,
            _pool_key,
            _pool_account,
        ) = accounts.setup_token_accounts(&user_key, &withdrawer_key, 0, 0, 0);

        let pool_key = accounts.pool_token_key;
        let mut pool_account = accounts.pool_token_account.clone();

        // WithdrawAllTokenTypes takes all tokens for A and B.
        // The curve's calculation for token B will say to transfer
        // `token_b_offset + token_b_amount`, but only `token_b_amount` will be
        // moved.
        accounts
            .withdraw_all_token_types(
                &user_key,
                &pool_key,
                &mut pool_account,
                &token_a_key,
                &mut token_a_account,
                &token_b_key,
                &mut token_b_account,
                total_pool.try_into().unwrap(),
                0,
                0,
            )
            .unwrap();

        let token_a = spl_token::state::Account::unpack(&token_a_account.data).unwrap();
        assert_eq!(token_a.amount, token_a_amount);
        let token_b = spl_token::state::Account::unpack(&token_b_account.data).unwrap();
        assert_eq!(token_b.amount, token_b_amount);
        let swap_token_a =
            spl_token::state::Account::unpack(&accounts.token_a_account.data).unwrap();
        assert_eq!(swap_token_a.amount, 0);
        let swap_token_b =
            spl_token::state::Account::unpack(&accounts.token_b_account.data).unwrap();
        assert_eq!(swap_token_b.amount, 0);
    }

    #[test]
    fn test_withdraw_all_constant_price_curve() {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 10;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 30;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 30;

        // initialize "unbalanced", so that withdrawing all will have some issues
        // A: 1_000_000_000
        // B: 2_000_000_000 (1_000 * 2_000_000)
        let swap_token_a_amount = 1_000_000_000;
        let swap_token_b_amount = 1_000;
        let token_b_price = 2_000_000;
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };

        let swap_curve = SwapCurve {
            curve_type: CurveType::ConstantPrice,
            calculator: Arc::new(ConstantPriceCurve { token_b_price }),
        };
        let total_pool = swap_curve.calculator.new_pool_supply();
        let user_key = Pubkey::new_unique();
        let withdrawer_key = Pubkey::new_unique();

        let mut accounts = SwapAccountInfo::new(
            &user_key,
            fees,
            swap_curve,
            swap_token_a_amount,
            swap_token_b_amount,
        );

        //fake init
        accounts.initialize_swap_mock_for_testing().unwrap();

        let (
            token_a_key,
            mut token_a_account,
            token_b_key,
            mut token_b_account,
            _pool_key,
            _pool_account,
        ) = accounts.setup_token_accounts(&user_key, &withdrawer_key, 0, 0, 0);

        let pool_key = accounts.pool_token_key;
        let mut pool_account = accounts.pool_token_account.clone();

        // WithdrawAllTokenTypes will not take all token A and B, since their
        // ratio is unbalanced.  It will try to take 1_500_000_000 worth of
        // each token, which means 1_500_000_000 token A, and 750 token B.
        // With no slippage, this will leave 250 token B in the pool.
        assert_eq!(
            Err(SwapError::ExceededSlippage.into()),
            accounts.withdraw_all_token_types(
                &user_key,
                &pool_key,
                &mut pool_account,
                &token_a_key,
                &mut token_a_account,
                &token_b_key,
                &mut token_b_account,
                total_pool.try_into().unwrap(),
                swap_token_a_amount,
                swap_token_b_amount,
            )
        );

        accounts
            .withdraw_all_token_types(
                &user_key,
                &pool_key,
                &mut pool_account,
                &token_a_key,
                &mut token_a_account,
                &token_b_key,
                &mut token_b_account,
                total_pool.try_into().unwrap(),
                0,
                0,
            )
            .unwrap();

        let token_a = spl_token::state::Account::unpack(&token_a_account.data).unwrap();
        assert_eq!(token_a.amount, swap_token_a_amount);
        let token_b = spl_token::state::Account::unpack(&token_b_account.data).unwrap();
        assert_eq!(token_b.amount, 750);
        let swap_token_a =
            spl_token::state::Account::unpack(&accounts.token_a_account.data).unwrap();
        assert_eq!(swap_token_a.amount, 0);
        let swap_token_b =
            spl_token::state::Account::unpack(&accounts.token_b_account.data).unwrap();
        assert_eq!(swap_token_b.amount, 250);

        // deposit now, not enough to cover the tokens already in there
        let token_b_amount = 10;
        let token_a_amount = token_b_amount * token_b_price;
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
            token_a_amount,
            token_b_amount,
            0,
        );

        assert_eq!(
            Err(SwapError::ExceededSlippage.into()),
            accounts.deposit_all_token_types(
                &withdrawer_key,
                &token_a_key,
                &mut token_a_account,
                &token_b_key,
                &mut token_b_account,
                &pool_key,
                &mut pool_account,
                1, // doesn't matter
                token_a_amount,
                token_b_amount,
            )
        );

        // deposit enough tokens, success!
        let token_b_amount = 125;
        let token_a_amount = token_b_amount * token_b_price;
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
            token_a_amount,
            token_b_amount,
            0,
        );

        accounts
            .deposit_all_token_types(
                &withdrawer_key,
                &token_a_key,
                &mut token_a_account,
                &token_b_key,
                &mut token_b_account,
                &pool_key,
                &mut pool_account,
                1, // doesn't matter
                token_a_amount,
                token_b_amount,
            )
            .unwrap();
    }
}
