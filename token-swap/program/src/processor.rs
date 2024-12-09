//! Program state processor

use {
    crate::{
        constraints::{SwapConstraints, SWAP_CONSTRAINTS},
        curve::{
            base::SwapCurve,
            calculator::{RoundDirection, TradeDirection},
            fees::Fees,
        },
        error::SwapError,
        instruction::{
            DepositAllTokenTypes, DepositSingleTokenTypeExactAmountIn, Initialize, Swap,
            SwapInstruction, WithdrawAllTokenTypes, WithdrawSingleTokenTypeExactAmountOut,
        },
        state::{SwapState, SwapV1, SwapVersion},
    },
    num_traits::FromPrimitive,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        clock::Clock,
        decode_error::DecodeError,
        entrypoint::ProgramResult,
        instruction::Instruction,
        msg,
        program::invoke_signed,
        program_error::{PrintProgramError, ProgramError},
        program_option::COption,
        pubkey::Pubkey,
        sysvar::Sysvar,
    },
    spl_token_2022::{
        check_spl_token_program_account,
        error::TokenError,
        extension::{
            mint_close_authority::MintCloseAuthority, transfer_fee::TransferFeeConfig,
            BaseStateWithExtensions, StateWithExtensions,
        },
        state::{Account, Mint},
    },
    std::{convert::TryInto, error::Error},
};

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Unpacks a spl_token `Account`.
    pub fn unpack_token_account(
        account_info: &AccountInfo,
        token_program_id: &Pubkey,
    ) -> Result<Account, SwapError> {
        if account_info.owner != token_program_id
            && check_spl_token_program_account(account_info.owner).is_err()
        {
            Err(SwapError::IncorrectTokenProgramId)
        } else {
            StateWithExtensions::<Account>::unpack(&account_info.data.borrow())
                .map(|a| a.base)
                .map_err(|_| SwapError::ExpectedAccount)
        }
    }

    /// Unpacks a spl_token `Mint`.
    pub fn unpack_mint(
        account_info: &AccountInfo,
        token_program_id: &Pubkey,
    ) -> Result<Mint, SwapError> {
        if account_info.owner != token_program_id
            && check_spl_token_program_account(account_info.owner).is_err()
        {
            Err(SwapError::IncorrectTokenProgramId)
        } else {
            StateWithExtensions::<Mint>::unpack(&account_info.data.borrow())
                .map(|m| m.base)
                .map_err(|_| SwapError::ExpectedMint)
        }
    }

    /// Unpacks a spl_token `Mint` with extension data
    pub fn unpack_mint_with_extensions<'a>(
        account_data: &'a [u8],
        owner: &Pubkey,
        token_program_id: &Pubkey,
    ) -> Result<StateWithExtensions<'a, Mint>, SwapError> {
        if owner != token_program_id && check_spl_token_program_account(owner).is_err() {
            Err(SwapError::IncorrectTokenProgramId)
        } else {
            StateWithExtensions::<Mint>::unpack(account_data).map_err(|_| SwapError::ExpectedMint)
        }
    }

    /// Calculates the authority id by generating a program address.
    pub fn authority_id(
        program_id: &Pubkey,
        my_info: &Pubkey,
        bump_seed: u8,
    ) -> Result<Pubkey, SwapError> {
        Pubkey::create_program_address(&[&my_info.to_bytes()[..32], &[bump_seed]], program_id)
            .or(Err(SwapError::InvalidProgramAddress))
    }

    /// Issue a spl_token `Burn` instruction.
    pub fn token_burn<'a>(
        swap: &Pubkey,
        token_program: AccountInfo<'a>,
        burn_account: AccountInfo<'a>,
        mint: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        bump_seed: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];

        let ix = spl_token_2022::instruction::burn(
            token_program.key,
            burn_account.key,
            mint.key,
            authority.key,
            &[],
            amount,
        )?;

        invoke_signed_wrapper::<TokenError>(
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
        bump_seed: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];
        let ix = spl_token_2022::instruction::mint_to(
            token_program.key,
            mint.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?;

        invoke_signed_wrapper::<TokenError>(
            &ix,
            &[mint, destination, authority, token_program],
            signers,
        )
    }

    /// Issue a spl_token `Transfer` instruction.
    #[allow(clippy::too_many_arguments)]
    pub fn token_transfer<'a>(
        swap: &Pubkey,
        token_program: AccountInfo<'a>,
        source: AccountInfo<'a>,
        mint: AccountInfo<'a>,
        destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        bump_seed: u8,
        amount: u64,
        decimals: u8,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];
        let ix = spl_token_2022::instruction::transfer_checked(
            token_program.key,
            source.key,
            mint.key,
            destination.key,
            authority.key,
            &[],
            amount,
            decimals,
        )?;
        invoke_signed_wrapper::<TokenError>(
            &ix,
            &[source, mint, destination, authority, token_program],
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
        pool_token_program_info: &AccountInfo,
        user_token_a_info: Option<&AccountInfo>,
        user_token_b_info: Option<&AccountInfo>,
        pool_fee_account_info: Option<&AccountInfo>,
    ) -> ProgramResult {
        if swap_account_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        if *authority_info.key
            != Self::authority_id(program_id, swap_account_info.key, token_swap.bump_seed())?
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
        if *pool_token_program_info.key != *token_swap.token_program_id() {
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
        fees: Fees,
        swap_curve: SwapCurve,
        accounts: &[AccountInfo],
        swap_constraints: &Option<SwapConstraints>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let token_a_info = next_account_info(account_info_iter)?;
        let token_b_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let fee_account_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let pool_token_program_info = next_account_info(account_info_iter)?;

        let token_program_id = *pool_token_program_info.key;
        if SwapVersion::is_initialized(&swap_info.data.borrow()) {
            return Err(SwapError::AlreadyInUse.into());
        }

        let (swap_authority, bump_seed) =
            Pubkey::find_program_address(&[&swap_info.key.to_bytes()], program_id);
        if *authority_info.key != swap_authority {
            return Err(SwapError::InvalidProgramAddress.into());
        }
        let token_a = Self::unpack_token_account(token_a_info, &token_program_id)?;
        let token_b = Self::unpack_token_account(token_b_info, &token_program_id)?;
        let fee_account = Self::unpack_token_account(fee_account_info, &token_program_id)?;
        let destination = Self::unpack_token_account(destination_info, &token_program_id)?;
        let pool_mint = {
            let pool_mint_data = pool_mint_info.data.borrow();
            let pool_mint = Self::unpack_mint_with_extensions(
                &pool_mint_data,
                pool_mint_info.owner,
                &token_program_id,
            )?;
            if let Ok(extension) = pool_mint.get_extension::<MintCloseAuthority>() {
                let close_authority: Option<Pubkey> = extension.close_authority.into();
                if close_authority.is_some() {
                    return Err(SwapError::InvalidCloseAuthority.into());
                }
            }
            pool_mint.base
        };
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

        if let Some(swap_constraints) = swap_constraints {
            let owner_key = swap_constraints
                .owner_key
                .unwrap()
                .parse::<Pubkey>()
                .map_err(|_| SwapError::InvalidOwner)?;
            if fee_account.owner != owner_key {
                return Err(SwapError::InvalidOwner.into());
            }
            swap_constraints.validate_curve(&swap_curve)?;
            swap_constraints.validate_fees(&fees)?;
        }
        fees.validate()?;
        swap_curve.calculator.validate()?;

        let initial_amount = swap_curve.calculator.new_pool_supply();

        Self::token_mint_to(
            swap_info.key,
            pool_token_program_info.clone(),
            pool_mint_info.clone(),
            destination_info.clone(),
            authority_info.clone(),
            bump_seed,
            to_u64(initial_amount)?,
        )?;

        let obj = SwapVersion::SwapV1(SwapV1 {
            is_initialized: true,
            bump_seed,
            token_program_id,
            token_a: *token_a_info.key,
            token_b: *token_b_info.key,
            pool_mint: *pool_mint_info.key,
            token_a_mint: token_a.mint,
            token_b_mint: token_b.mint,
            pool_fee_account: *fee_account_info.key,
            fees,
            swap_curve,
        });
        SwapVersion::pack(obj, &mut swap_info.data.borrow_mut())?;
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
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let swap_source_info = next_account_info(account_info_iter)?;
        let swap_destination_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let pool_fee_account_info = next_account_info(account_info_iter)?;
        let source_token_mint_info = next_account_info(account_info_iter)?;
        let destination_token_mint_info = next_account_info(account_info_iter)?;
        let source_token_program_info = next_account_info(account_info_iter)?;
        let destination_token_program_info = next_account_info(account_info_iter)?;
        let pool_token_program_info = next_account_info(account_info_iter)?;

        if swap_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        let token_swap = SwapVersion::unpack(&swap_info.data.borrow())?;

        if *authority_info.key
            != Self::authority_id(program_id, swap_info.key, token_swap.bump_seed())?
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
        if *pool_token_program_info.key != *token_swap.token_program_id() {
            return Err(SwapError::IncorrectTokenProgramId.into());
        }

        let source_account =
            Self::unpack_token_account(swap_source_info, token_swap.token_program_id())?;
        let dest_account =
            Self::unpack_token_account(swap_destination_info, token_swap.token_program_id())?;
        let pool_mint = Self::unpack_mint(pool_mint_info, token_swap.token_program_id())?;

        // Take transfer fees into account for actual amount transferred in
        let actual_amount_in = {
            let source_mint_data = source_token_mint_info.data.borrow();
            let source_mint = Self::unpack_mint_with_extensions(
                &source_mint_data,
                source_token_mint_info.owner,
                token_swap.token_program_id(),
            )?;

            if let Ok(transfer_fee_config) = source_mint.get_extension::<TransferFeeConfig>() {
                amount_in.saturating_sub(
                    transfer_fee_config
                        .calculate_epoch_fee(Clock::get()?.epoch, amount_in)
                        .ok_or(SwapError::FeeCalculationFailure)?,
                )
            } else {
                amount_in
            }
        };

        // Calculate the trade amounts
        let trade_direction = if *swap_source_info.key == *token_swap.token_a_account() {
            TradeDirection::AtoB
        } else {
            TradeDirection::BtoA
        };
        let result = token_swap
            .swap_curve()
            .swap(
                u128::from(actual_amount_in),
                u128::from(source_account.amount),
                u128::from(dest_account.amount),
                trade_direction,
                token_swap.fees(),
            )
            .ok_or(SwapError::ZeroTradingTokens)?;

        // Re-calculate the source amount swapped based on what the curve says
        let (source_transfer_amount, source_mint_decimals) = {
            let source_amount_swapped = to_u64(result.source_amount_swapped)?;

            let source_mint_data = source_token_mint_info.data.borrow();
            let source_mint = Self::unpack_mint_with_extensions(
                &source_mint_data,
                source_token_mint_info.owner,
                token_swap.token_program_id(),
            )?;
            let amount =
                if let Ok(transfer_fee_config) = source_mint.get_extension::<TransferFeeConfig>() {
                    source_amount_swapped.saturating_add(
                        transfer_fee_config
                            .calculate_inverse_epoch_fee(Clock::get()?.epoch, source_amount_swapped)
                            .ok_or(SwapError::FeeCalculationFailure)?,
                    )
                } else {
                    source_amount_swapped
                };
            (amount, source_mint.base.decimals)
        };

        let (destination_transfer_amount, destination_mint_decimals) = {
            let destination_mint_data = destination_token_mint_info.data.borrow();
            let destination_mint = Self::unpack_mint_with_extensions(
                &destination_mint_data,
                source_token_mint_info.owner,
                token_swap.token_program_id(),
            )?;
            let amount_out = to_u64(result.destination_amount_swapped)?;
            let amount_received = if let Ok(transfer_fee_config) =
                destination_mint.get_extension::<TransferFeeConfig>()
            {
                amount_out.saturating_sub(
                    transfer_fee_config
                        .calculate_epoch_fee(Clock::get()?.epoch, amount_out)
                        .ok_or(SwapError::FeeCalculationFailure)?,
                )
            } else {
                amount_out
            };
            if amount_received < minimum_amount_out {
                return Err(SwapError::ExceededSlippage.into());
            }
            (amount_out, destination_mint.base.decimals)
        };

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
            source_token_program_info.clone(),
            source_info.clone(),
            source_token_mint_info.clone(),
            swap_source_info.clone(),
            user_transfer_authority_info.clone(),
            token_swap.bump_seed(),
            source_transfer_amount,
            source_mint_decimals,
        )?;

        if result.owner_fee > 0 {
            let mut pool_token_amount = token_swap
                .swap_curve()
                .calculator
                .withdraw_single_token_type_exact_out(
                    result.owner_fee,
                    swap_token_a_amount,
                    swap_token_b_amount,
                    u128::from(pool_mint.supply),
                    trade_direction,
                    RoundDirection::Floor,
                )
                .ok_or(SwapError::FeeCalculationFailure)?;
            // Allow error to fall through
            if let Ok(host_fee_account_info) = next_account_info(account_info_iter) {
                let host_fee_account = Self::unpack_token_account(
                    host_fee_account_info,
                    token_swap.token_program_id(),
                )?;
                if *pool_mint_info.key != host_fee_account.mint {
                    return Err(SwapError::IncorrectPoolMint.into());
                }
                let host_fee = token_swap
                    .fees()
                    .host_fee(pool_token_amount)
                    .ok_or(SwapError::FeeCalculationFailure)?;
                if host_fee > 0 {
                    pool_token_amount = pool_token_amount
                        .checked_sub(host_fee)
                        .ok_or(SwapError::FeeCalculationFailure)?;
                    Self::token_mint_to(
                        swap_info.key,
                        pool_token_program_info.clone(),
                        pool_mint_info.clone(),
                        host_fee_account_info.clone(),
                        authority_info.clone(),
                        token_swap.bump_seed(),
                        to_u64(host_fee)?,
                    )?;
                }
            }
            if token_swap
                .check_pool_fee_info(pool_fee_account_info)
                .is_ok()
            {
                Self::token_mint_to(
                    swap_info.key,
                    pool_token_program_info.clone(),
                    pool_mint_info.clone(),
                    pool_fee_account_info.clone(),
                    authority_info.clone(),
                    token_swap.bump_seed(),
                    to_u64(pool_token_amount)?,
                )?;
            };
        }

        Self::token_transfer(
            swap_info.key,
            destination_token_program_info.clone(),
            swap_destination_info.clone(),
            destination_token_mint_info.clone(),
            destination_info.clone(),
            authority_info.clone(),
            token_swap.bump_seed(),
            destination_transfer_amount,
            destination_mint_decimals,
        )?;

        Ok(())
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
        let token_a_mint_info = next_account_info(account_info_iter)?;
        let token_b_mint_info = next_account_info(account_info_iter)?;
        let token_a_program_info = next_account_info(account_info_iter)?;
        let token_b_program_info = next_account_info(account_info_iter)?;
        let pool_token_program_info = next_account_info(account_info_iter)?;

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
            pool_token_program_info,
            Some(source_a_info),
            Some(source_b_info),
            None,
        )?;

        let token_a = Self::unpack_token_account(token_a_info, token_swap.token_program_id())?;
        let token_b = Self::unpack_token_account(token_b_info, token_swap.token_program_id())?;
        let pool_mint = Self::unpack_mint(pool_mint_info, token_swap.token_program_id())?;
        let current_pool_mint_supply = u128::from(pool_mint.supply);
        let (pool_token_amount, pool_mint_supply) = if current_pool_mint_supply > 0 {
            (u128::from(pool_token_amount), current_pool_mint_supply)
        } else {
            (calculator.new_pool_supply(), calculator.new_pool_supply())
        };

        let results = calculator
            .pool_tokens_to_trading_tokens(
                pool_token_amount,
                pool_mint_supply,
                u128::from(token_a.amount),
                u128::from(token_b.amount),
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
            token_a_program_info.clone(),
            source_a_info.clone(),
            token_a_mint_info.clone(),
            token_a_info.clone(),
            user_transfer_authority_info.clone(),
            token_swap.bump_seed(),
            token_a_amount,
            Self::unpack_mint(token_a_mint_info, token_swap.token_program_id())?.decimals,
        )?;
        Self::token_transfer(
            swap_info.key,
            token_b_program_info.clone(),
            source_b_info.clone(),
            token_b_mint_info.clone(),
            token_b_info.clone(),
            user_transfer_authority_info.clone(),
            token_swap.bump_seed(),
            token_b_amount,
            Self::unpack_mint(token_b_mint_info, token_swap.token_program_id())?.decimals,
        )?;
        Self::token_mint_to(
            swap_info.key,
            pool_token_program_info.clone(),
            pool_mint_info.clone(),
            dest_info.clone(),
            authority_info.clone(),
            token_swap.bump_seed(),
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
        let token_a_mint_info = next_account_info(account_info_iter)?;
        let token_b_mint_info = next_account_info(account_info_iter)?;
        let pool_token_program_info = next_account_info(account_info_iter)?;
        let token_a_program_info = next_account_info(account_info_iter)?;
        let token_b_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapVersion::unpack(&swap_info.data.borrow())?;
        Self::check_accounts(
            token_swap.as_ref(),
            program_id,
            swap_info,
            authority_info,
            token_a_info,
            token_b_info,
            pool_mint_info,
            pool_token_program_info,
            Some(dest_token_a_info),
            Some(dest_token_b_info),
            Some(pool_fee_account_info),
        )?;

        let token_a = Self::unpack_token_account(token_a_info, token_swap.token_program_id())?;
        let token_b = Self::unpack_token_account(token_b_info, token_swap.token_program_id())?;
        let pool_mint = Self::unpack_mint(pool_mint_info, token_swap.token_program_id())?;

        let calculator = &token_swap.swap_curve().calculator;

        let withdraw_fee = match token_swap.check_pool_fee_info(pool_fee_account_info) {
            Ok(_) => {
                if *pool_fee_account_info.key == *source_info.key {
                    // withdrawing from the fee account, don't assess withdraw fee
                    0
                } else {
                    token_swap
                        .fees()
                        .owner_withdraw_fee(u128::from(pool_token_amount))
                        .ok_or(SwapError::FeeCalculationFailure)?
                }
            }
            Err(_) => 0,
        };
        let pool_token_amount = u128::from(pool_token_amount)
            .checked_sub(withdraw_fee)
            .ok_or(SwapError::CalculationFailure)?;

        let results = calculator
            .pool_tokens_to_trading_tokens(
                pool_token_amount,
                u128::from(pool_mint.supply),
                u128::from(token_a.amount),
                u128::from(token_b.amount),
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
                pool_token_program_info.clone(),
                source_info.clone(),
                pool_mint_info.clone(),
                pool_fee_account_info.clone(),
                user_transfer_authority_info.clone(),
                token_swap.bump_seed(),
                to_u64(withdraw_fee)?,
                pool_mint.decimals,
            )?;
        }
        Self::token_burn(
            swap_info.key,
            pool_token_program_info.clone(),
            source_info.clone(),
            pool_mint_info.clone(),
            user_transfer_authority_info.clone(),
            token_swap.bump_seed(),
            to_u64(pool_token_amount)?,
        )?;

        if token_a_amount > 0 {
            Self::token_transfer(
                swap_info.key,
                token_a_program_info.clone(),
                token_a_info.clone(),
                token_a_mint_info.clone(),
                dest_token_a_info.clone(),
                authority_info.clone(),
                token_swap.bump_seed(),
                token_a_amount,
                Self::unpack_mint(token_a_mint_info, token_swap.token_program_id())?.decimals,
            )?;
        }
        if token_b_amount > 0 {
            Self::token_transfer(
                swap_info.key,
                token_b_program_info.clone(),
                token_b_info.clone(),
                token_b_mint_info.clone(),
                dest_token_b_info.clone(),
                authority_info.clone(),
                token_swap.bump_seed(),
                token_b_amount,
                Self::unpack_mint(token_b_mint_info, token_swap.token_program_id())?.decimals,
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
        let source_token_mint_info = next_account_info(account_info_iter)?;
        let source_token_program_info = next_account_info(account_info_iter)?;
        let pool_token_program_info = next_account_info(account_info_iter)?;

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
            pool_token_program_info,
            source_a_info,
            source_b_info,
            None,
        )?;

        let pool_mint = Self::unpack_mint(pool_mint_info, token_swap.token_program_id())?;
        let pool_mint_supply = u128::from(pool_mint.supply);
        let pool_token_amount = if pool_mint_supply > 0 {
            token_swap
                .swap_curve()
                .deposit_single_token_type(
                    u128::from(source_token_amount),
                    u128::from(swap_token_a.amount),
                    u128::from(swap_token_b.amount),
                    pool_mint_supply,
                    trade_direction,
                    token_swap.fees(),
                )
                .ok_or(SwapError::ZeroTradingTokens)?
        } else {
            calculator.new_pool_supply()
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
                    source_token_program_info.clone(),
                    source_info.clone(),
                    source_token_mint_info.clone(),
                    swap_token_a_info.clone(),
                    user_transfer_authority_info.clone(),
                    token_swap.bump_seed(),
                    source_token_amount,
                    Self::unpack_mint(source_token_mint_info, token_swap.token_program_id())?
                        .decimals,
                )?;
            }
            TradeDirection::BtoA => {
                Self::token_transfer(
                    swap_info.key,
                    source_token_program_info.clone(),
                    source_info.clone(),
                    source_token_mint_info.clone(),
                    swap_token_b_info.clone(),
                    user_transfer_authority_info.clone(),
                    token_swap.bump_seed(),
                    source_token_amount,
                    Self::unpack_mint(source_token_mint_info, token_swap.token_program_id())?
                        .decimals,
                )?;
            }
        }
        Self::token_mint_to(
            swap_info.key,
            pool_token_program_info.clone(),
            pool_mint_info.clone(),
            destination_info.clone(),
            authority_info.clone(),
            token_swap.bump_seed(),
            pool_token_amount,
        )?;

        Ok(())
    }

    /// Processes a
    /// [WithdrawSingleTokenTypeExactAmountOut](enum.Instruction.html).
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
        let destination_token_mint_info = next_account_info(account_info_iter)?;
        let pool_token_program_info = next_account_info(account_info_iter)?;
        let destination_token_program_info = next_account_info(account_info_iter)?;

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
            pool_token_program_info,
            destination_a_info,
            destination_b_info,
            Some(pool_fee_account_info),
        )?;

        let pool_mint = Self::unpack_mint(pool_mint_info, token_swap.token_program_id())?;
        let pool_mint_supply = u128::from(pool_mint.supply);
        let swap_token_a_amount = u128::from(swap_token_a.amount);
        let swap_token_b_amount = u128::from(swap_token_b.amount);

        let burn_pool_token_amount = token_swap
            .swap_curve()
            .withdraw_single_token_type_exact_out(
                u128::from(destination_token_amount),
                swap_token_a_amount,
                swap_token_b_amount,
                pool_mint_supply,
                trade_direction,
                token_swap.fees(),
            )
            .ok_or(SwapError::ZeroTradingTokens)?;

        let withdraw_fee = match token_swap.check_pool_fee_info(pool_fee_account_info) {
            Ok(_) => {
                if *pool_fee_account_info.key == *source_info.key {
                    // withdrawing from the fee account, don't assess withdraw fee
                    0
                } else {
                    token_swap
                        .fees()
                        .owner_withdraw_fee(burn_pool_token_amount)
                        .ok_or(SwapError::FeeCalculationFailure)?
                }
            }
            Err(_) => 0,
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
                pool_token_program_info.clone(),
                source_info.clone(),
                pool_mint_info.clone(),
                pool_fee_account_info.clone(),
                user_transfer_authority_info.clone(),
                token_swap.bump_seed(),
                to_u64(withdraw_fee)?,
                pool_mint.decimals,
            )?;
        }
        Self::token_burn(
            swap_info.key,
            pool_token_program_info.clone(),
            source_info.clone(),
            pool_mint_info.clone(),
            user_transfer_authority_info.clone(),
            token_swap.bump_seed(),
            to_u64(burn_pool_token_amount)?,
        )?;

        match trade_direction {
            TradeDirection::AtoB => {
                Self::token_transfer(
                    swap_info.key,
                    destination_token_program_info.clone(),
                    swap_token_a_info.clone(),
                    destination_token_mint_info.clone(),
                    destination_info.clone(),
                    authority_info.clone(),
                    token_swap.bump_seed(),
                    destination_token_amount,
                    Self::unpack_mint(destination_token_mint_info, token_swap.token_program_id())?
                        .decimals,
                )?;
            }
            TradeDirection::BtoA => {
                Self::token_transfer(
                    swap_info.key,
                    destination_token_program_info.clone(),
                    swap_token_b_info.clone(),
                    destination_token_mint_info.clone(),
                    destination_info.clone(),
                    authority_info.clone(),
                    token_swap.bump_seed(),
                    destination_token_amount,
                    Self::unpack_mint(destination_token_mint_info, token_swap.token_program_id())?
                        .decimals,
                )?;
            }
        }

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
            SwapInstruction::Initialize(Initialize { fees, swap_curve }) => {
                msg!("Instruction: Init");
                Self::process_initialize(program_id, fees, swap_curve, accounts, swap_constraints)
            }
            SwapInstruction::Swap(Swap {
                amount_in,
                minimum_amount_out,
            }) => {
                msg!("Instruction: Swap");
                Self::process_swap(program_id, amount_in, minimum_amount_out, accounts)
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
        }
    }
}

fn to_u64(val: u128) -> Result<u64, SwapError> {
    val.try_into().map_err(|_| SwapError::ConversionFailure)
}

fn invoke_signed_wrapper<T>(
    instruction: &Instruction,
    account_infos: &[AccountInfo],
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError>
where
    T: 'static + PrintProgramError + DecodeError<T> + FromPrimitive + Error,
{
    invoke_signed(instruction, account_infos, signers_seeds).inspect_err(|err| {
        err.print::<T>();
    })
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            curve::{
                base::CurveType,
                calculator::{CurveCalculator, INITIAL_SWAP_POOL_AMOUNT},
                constant_price::ConstantPriceCurve,
                constant_product::ConstantProductCurve,
                offset::OffsetCurve,
            },
            instruction::{
                deposit_all_token_types, deposit_single_token_type_exact_amount_in, initialize,
                swap, withdraw_all_token_types, withdraw_single_token_type_exact_amount_out,
            },
        },
        solana_program::{
            clock::Clock, entrypoint::SUCCESS, instruction::Instruction, program_pack::Pack,
            program_stubs, rent::Rent,
        },
        solana_sdk::account::{
            create_account_for_test, create_is_signer_account_infos, Account as SolanaAccount,
        },
        spl_token_2022::{
            error::TokenError,
            extension::{
                transfer_fee::{instruction::initialize_transfer_fee_config, TransferFee},
                ExtensionType,
            },
            instruction::{
                approve, close_account, freeze_account, initialize_account,
                initialize_immutable_owner, initialize_mint, initialize_mint_close_authority,
                mint_to, revoke, set_authority, AuthorityType,
            },
        },
        std::sync::Arc,
        test_case::test_case,
    };

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
            if !account_infos
                .iter()
                .any(|x| *x.key == spl_token::id() || *x.key == spl_token_2022::id())
            {
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

            if instruction.program_id == spl_token::id() {
                spl_token::processor::Processor::process(
                    &instruction.program_id,
                    &new_account_infos,
                    &instruction.data,
                )
            } else if instruction.program_id == spl_token_2022::id() {
                spl_token_2022::processor::Processor::process(
                    &instruction.program_id,
                    &new_account_infos,
                    &instruction.data,
                )
            } else {
                Err(ProgramError::IncorrectProgramId)
            }
        }

        fn sol_get_clock_sysvar(&self, var_addr: *mut u8) -> u64 {
            unsafe {
                *(var_addr as *mut _ as *mut Clock) = Clock::default();
            }
            SUCCESS
        }
    }

    fn test_syscall_stubs() {
        use std::sync::Once;
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            program_stubs::set_syscall_stubs(Box::new(TestSyscallStubs {}));
        });
    }

    #[derive(Default)]
    struct SwapTransferFees {
        pool_token: TransferFee,
        token_a: TransferFee,
        token_b: TransferFee,
    }

    struct SwapAccountInfo {
        bump_seed: u8,
        authority_key: Pubkey,
        fees: Fees,
        transfer_fees: SwapTransferFees,
        swap_curve: SwapCurve,
        swap_key: Pubkey,
        swap_account: SolanaAccount,
        pool_mint_key: Pubkey,
        pool_mint_account: SolanaAccount,
        pool_fee_key: Pubkey,
        pool_fee_account: SolanaAccount,
        pool_token_key: Pubkey,
        pool_token_account: SolanaAccount,
        token_a_key: Pubkey,
        token_a_account: SolanaAccount,
        token_a_mint_key: Pubkey,
        token_a_mint_account: SolanaAccount,
        token_b_key: Pubkey,
        token_b_account: SolanaAccount,
        token_b_mint_key: Pubkey,
        token_b_mint_account: SolanaAccount,
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    }

    impl SwapAccountInfo {
        #[allow(clippy::too_many_arguments)]
        pub fn new(
            user_key: &Pubkey,
            fees: Fees,
            transfer_fees: SwapTransferFees,
            swap_curve: SwapCurve,
            token_a_amount: u64,
            token_b_amount: u64,
            pool_token_program_id: &Pubkey,
            token_a_program_id: &Pubkey,
            token_b_program_id: &Pubkey,
        ) -> Self {
            let swap_key = Pubkey::new_unique();
            let swap_account = SolanaAccount::new(0, SwapVersion::LATEST_LEN, &SWAP_PROGRAM_ID);
            let (authority_key, bump_seed) =
                Pubkey::find_program_address(&[&swap_key.to_bytes()[..]], &SWAP_PROGRAM_ID);

            let (pool_mint_key, mut pool_mint_account) = create_mint(
                pool_token_program_id,
                &authority_key,
                None,
                None,
                &transfer_fees.pool_token,
            );
            let (pool_token_key, pool_token_account) = mint_token(
                pool_token_program_id,
                &pool_mint_key,
                &mut pool_mint_account,
                &authority_key,
                user_key,
                0,
            );
            let (pool_fee_key, pool_fee_account) = mint_token(
                pool_token_program_id,
                &pool_mint_key,
                &mut pool_mint_account,
                &authority_key,
                user_key,
                0,
            );
            let (token_a_mint_key, mut token_a_mint_account) = create_mint(
                token_a_program_id,
                user_key,
                None,
                None,
                &transfer_fees.token_a,
            );
            let (token_a_key, token_a_account) = mint_token(
                token_a_program_id,
                &token_a_mint_key,
                &mut token_a_mint_account,
                user_key,
                &authority_key,
                token_a_amount,
            );
            let (token_b_mint_key, mut token_b_mint_account) = create_mint(
                token_b_program_id,
                user_key,
                None,
                None,
                &transfer_fees.token_b,
            );
            let (token_b_key, token_b_account) = mint_token(
                token_b_program_id,
                &token_b_mint_key,
                &mut token_b_mint_account,
                user_key,
                &authority_key,
                token_b_amount,
            );

            SwapAccountInfo {
                bump_seed,
                authority_key,
                fees,
                transfer_fees,
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
                pool_token_program_id: *pool_token_program_id,
                token_a_program_id: *token_a_program_id,
                token_b_program_id: *token_b_program_id,
            }
        }

        pub fn initialize_swap(&mut self) -> ProgramResult {
            do_process_instruction(
                initialize(
                    &SWAP_PROGRAM_ID,
                    &self.pool_token_program_id,
                    &self.swap_key,
                    &self.authority_key,
                    &self.token_a_key,
                    &self.token_b_key,
                    &self.pool_mint_key,
                    &self.pool_fee_key,
                    &self.pool_token_key,
                    self.fees.clone(),
                    self.swap_curve.clone(),
                )
                .unwrap(),
                vec![
                    &mut self.swap_account,
                    &mut SolanaAccount::default(),
                    &mut self.token_a_account,
                    &mut self.token_b_account,
                    &mut self.pool_mint_account,
                    &mut self.pool_fee_account,
                    &mut self.pool_token_account,
                    &mut SolanaAccount::default(),
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
        ) -> (
            Pubkey,
            SolanaAccount,
            Pubkey,
            SolanaAccount,
            Pubkey,
            SolanaAccount,
        ) {
            let (token_a_key, token_a_account) = mint_token(
                &self.token_a_program_id,
                &self.token_a_mint_key,
                &mut self.token_a_mint_account,
                mint_owner,
                account_owner,
                a_amount,
            );
            let (token_b_key, token_b_account) = mint_token(
                &self.token_b_program_id,
                &self.token_b_mint_key,
                &mut self.token_b_mint_account,
                mint_owner,
                account_owner,
                b_amount,
            );
            let (pool_key, pool_account) = mint_token(
                &self.pool_token_program_id,
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

        fn get_swap_key(&self, mint_key: &Pubkey) -> &Pubkey {
            if *mint_key == self.token_a_mint_key {
                &self.token_a_key
            } else if *mint_key == self.token_b_mint_key {
                &self.token_b_key
            } else {
                panic!("Could not find matching swap token account");
            }
        }

        fn get_token_program_id(&self, account_key: &Pubkey) -> &Pubkey {
            if *account_key == self.token_a_key {
                &self.token_a_program_id
            } else if *account_key == self.token_b_key {
                &self.token_b_program_id
            } else {
                panic!("Could not find matching swap token account");
            }
        }

        fn get_token_mint(&self, account_key: &Pubkey) -> (Pubkey, SolanaAccount) {
            if *account_key == self.token_a_key {
                (self.token_a_mint_key, self.token_a_mint_account.clone())
            } else if *account_key == self.token_b_key {
                (self.token_b_mint_key, self.token_b_mint_account.clone())
            } else {
                panic!("Could not find matching swap token account");
            }
        }

        fn get_token_account(&self, account_key: &Pubkey) -> &SolanaAccount {
            if *account_key == self.token_a_key {
                &self.token_a_account
            } else if *account_key == self.token_b_key {
                &self.token_b_account
            } else {
                panic!("Could not find matching swap token account");
            }
        }

        fn set_token_account(&mut self, account_key: &Pubkey, account: SolanaAccount) {
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
            user_source_account: &mut SolanaAccount,
            swap_source_key: &Pubkey,
            swap_destination_key: &Pubkey,
            user_destination_key: &Pubkey,
            user_destination_account: &mut SolanaAccount,
            amount_in: u64,
            minimum_amount_out: u64,
        ) -> ProgramResult {
            let user_transfer_key = Pubkey::new_unique();
            let source_token_program_id = self.get_token_program_id(swap_source_key);
            let destination_token_program_id = self.get_token_program_id(swap_destination_key);
            // approve moving from user source account
            do_process_instruction(
                approve(
                    source_token_program_id,
                    user_source_key,
                    &user_transfer_key,
                    user_key,
                    &[],
                    amount_in,
                )
                .unwrap(),
                vec![
                    user_source_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                ],
            )
            .unwrap();

            let (source_mint_key, mut source_mint_account) = self.get_token_mint(swap_source_key);
            let (destination_mint_key, mut destination_mint_account) =
                self.get_token_mint(swap_destination_key);
            let mut swap_source_account = self.get_token_account(swap_source_key).clone();
            let mut swap_destination_account = self.get_token_account(swap_destination_key).clone();

            // perform the swap
            do_process_instruction(
                swap(
                    &SWAP_PROGRAM_ID,
                    source_token_program_id,
                    destination_token_program_id,
                    &self.pool_token_program_id,
                    &self.swap_key,
                    &self.authority_key,
                    &user_transfer_key,
                    user_source_key,
                    swap_source_key,
                    swap_destination_key,
                    user_destination_key,
                    &self.pool_mint_key,
                    &self.pool_fee_key,
                    &source_mint_key,
                    &destination_mint_key,
                    None,
                    Swap {
                        amount_in,
                        minimum_amount_out,
                    },
                )
                .unwrap(),
                vec![
                    &mut self.swap_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                    user_source_account,
                    &mut swap_source_account,
                    &mut swap_destination_account,
                    user_destination_account,
                    &mut self.pool_mint_account,
                    &mut self.pool_fee_account,
                    &mut source_mint_account,
                    &mut destination_mint_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
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
            depositor_token_a_account: &mut SolanaAccount,
            depositor_token_b_key: &Pubkey,
            depositor_token_b_account: &mut SolanaAccount,
            depositor_pool_key: &Pubkey,
            depositor_pool_account: &mut SolanaAccount,
            pool_token_amount: u64,
            maximum_token_a_amount: u64,
            maximum_token_b_amount: u64,
        ) -> ProgramResult {
            let user_transfer_authority = Pubkey::new_unique();
            let token_a_program_id = depositor_token_a_account.owner;
            do_process_instruction(
                approve(
                    &token_a_program_id,
                    depositor_token_a_key,
                    &user_transfer_authority,
                    depositor_key,
                    &[],
                    maximum_token_a_amount,
                )
                .unwrap(),
                vec![
                    depositor_token_a_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                ],
            )
            .unwrap();

            let token_b_program_id = depositor_token_b_account.owner;
            do_process_instruction(
                approve(
                    &token_b_program_id,
                    depositor_token_b_key,
                    &user_transfer_authority,
                    depositor_key,
                    &[],
                    maximum_token_b_amount,
                )
                .unwrap(),
                vec![
                    depositor_token_b_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                ],
            )
            .unwrap();

            let pool_token_program_id = depositor_pool_account.owner;
            do_process_instruction(
                deposit_all_token_types(
                    &SWAP_PROGRAM_ID,
                    &token_a_program_id,
                    &token_b_program_id,
                    &pool_token_program_id,
                    &self.swap_key,
                    &self.authority_key,
                    &user_transfer_authority,
                    depositor_token_a_key,
                    depositor_token_b_key,
                    &self.token_a_key,
                    &self.token_b_key,
                    &self.pool_mint_key,
                    depositor_pool_key,
                    &self.token_a_mint_key,
                    &self.token_b_mint_key,
                    DepositAllTokenTypes {
                        pool_token_amount,
                        maximum_token_a_amount,
                        maximum_token_b_amount,
                    },
                )
                .unwrap(),
                vec![
                    &mut self.swap_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                    depositor_token_a_account,
                    depositor_token_b_account,
                    &mut self.token_a_account,
                    &mut self.token_b_account,
                    &mut self.pool_mint_account,
                    depositor_pool_account,
                    &mut self.token_a_mint_account,
                    &mut self.token_b_mint_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                ],
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn withdraw_all_token_types(
            &mut self,
            user_key: &Pubkey,
            pool_key: &Pubkey,
            pool_account: &mut SolanaAccount,
            token_a_key: &Pubkey,
            token_a_account: &mut SolanaAccount,
            token_b_key: &Pubkey,
            token_b_account: &mut SolanaAccount,
            pool_token_amount: u64,
            minimum_token_a_amount: u64,
            minimum_token_b_amount: u64,
        ) -> ProgramResult {
            let user_transfer_authority_key = Pubkey::new_unique();
            let pool_token_program_id = pool_account.owner;
            // approve user transfer authority to take out pool tokens
            do_process_instruction(
                approve(
                    &pool_token_program_id,
                    pool_key,
                    &user_transfer_authority_key,
                    user_key,
                    &[],
                    pool_token_amount,
                )
                .unwrap(),
                vec![
                    pool_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                ],
            )
            .unwrap();

            // withdraw token a and b correctly
            let token_a_program_id = token_a_account.owner;
            let token_b_program_id = token_b_account.owner;
            do_process_instruction(
                withdraw_all_token_types(
                    &SWAP_PROGRAM_ID,
                    &pool_token_program_id,
                    &token_a_program_id,
                    &token_b_program_id,
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
                    &self.token_a_mint_key,
                    &self.token_b_mint_key,
                    WithdrawAllTokenTypes {
                        pool_token_amount,
                        minimum_token_a_amount,
                        minimum_token_b_amount,
                    },
                )
                .unwrap(),
                vec![
                    &mut self.swap_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                    &mut self.pool_mint_account,
                    pool_account,
                    &mut self.token_a_account,
                    &mut self.token_b_account,
                    token_a_account,
                    token_b_account,
                    &mut self.pool_fee_account,
                    &mut self.token_a_mint_account,
                    &mut self.token_b_mint_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                ],
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn deposit_single_token_type_exact_amount_in(
            &mut self,
            depositor_key: &Pubkey,
            deposit_account_key: &Pubkey,
            deposit_token_account: &mut SolanaAccount,
            deposit_pool_key: &Pubkey,
            deposit_pool_account: &mut SolanaAccount,
            source_token_amount: u64,
            minimum_pool_token_amount: u64,
        ) -> ProgramResult {
            let user_transfer_authority_key = Pubkey::new_unique();
            let source_token_program_id = deposit_token_account.owner;
            do_process_instruction(
                approve(
                    &source_token_program_id,
                    deposit_account_key,
                    &user_transfer_authority_key,
                    depositor_key,
                    &[],
                    source_token_amount,
                )
                .unwrap(),
                vec![
                    deposit_token_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                ],
            )
            .unwrap();

            let source_mint_key =
                StateWithExtensions::<Account>::unpack(&deposit_token_account.data)
                    .unwrap()
                    .base
                    .mint;
            let swap_source_key = self.get_swap_key(&source_mint_key);
            let (source_mint_key, mut source_mint_account) = self.get_token_mint(swap_source_key);

            let pool_token_program_id = deposit_pool_account.owner;
            do_process_instruction(
                deposit_single_token_type_exact_amount_in(
                    &SWAP_PROGRAM_ID,
                    &source_token_program_id,
                    &pool_token_program_id,
                    &self.swap_key,
                    &self.authority_key,
                    &user_transfer_authority_key,
                    deposit_account_key,
                    &self.token_a_key,
                    &self.token_b_key,
                    &self.pool_mint_key,
                    deposit_pool_key,
                    &source_mint_key,
                    DepositSingleTokenTypeExactAmountIn {
                        source_token_amount,
                        minimum_pool_token_amount,
                    },
                )
                .unwrap(),
                vec![
                    &mut self.swap_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                    deposit_token_account,
                    &mut self.token_a_account,
                    &mut self.token_b_account,
                    &mut self.pool_mint_account,
                    deposit_pool_account,
                    &mut source_mint_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                ],
            )
        }

        #[allow(clippy::too_many_arguments)]
        pub fn withdraw_single_token_type_exact_amount_out(
            &mut self,
            user_key: &Pubkey,
            pool_key: &Pubkey,
            pool_account: &mut SolanaAccount,
            destination_key: &Pubkey,
            destination_account: &mut SolanaAccount,
            destination_token_amount: u64,
            maximum_pool_token_amount: u64,
        ) -> ProgramResult {
            let user_transfer_authority_key = Pubkey::new_unique();
            let pool_token_program_id = pool_account.owner;
            // approve user transfer authority to take out pool tokens
            do_process_instruction(
                approve(
                    &pool_token_program_id,
                    pool_key,
                    &user_transfer_authority_key,
                    user_key,
                    &[],
                    maximum_pool_token_amount,
                )
                .unwrap(),
                vec![
                    pool_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                ],
            )
            .unwrap();

            let destination_mint_key =
                StateWithExtensions::<Account>::unpack(&destination_account.data)
                    .unwrap()
                    .base
                    .mint;
            let swap_destination_key = self.get_swap_key(&destination_mint_key);
            let (destination_mint_key, mut destination_mint_account) =
                self.get_token_mint(swap_destination_key);

            let destination_token_program_id = destination_account.owner;
            do_process_instruction(
                withdraw_single_token_type_exact_amount_out(
                    &SWAP_PROGRAM_ID,
                    &pool_token_program_id,
                    &destination_token_program_id,
                    &self.swap_key,
                    &self.authority_key,
                    &user_transfer_authority_key,
                    &self.pool_mint_key,
                    &self.pool_fee_key,
                    pool_key,
                    &self.token_a_key,
                    &self.token_b_key,
                    destination_key,
                    &destination_mint_key,
                    WithdrawSingleTokenTypeExactAmountOut {
                        destination_token_amount,
                        maximum_pool_token_amount,
                    },
                )
                .unwrap(),
                vec![
                    &mut self.swap_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                    &mut self.pool_mint_account,
                    pool_account,
                    &mut self.token_a_account,
                    &mut self.token_b_account,
                    destination_account,
                    &mut self.pool_fee_account,
                    &mut destination_mint_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
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
        accounts: Vec<&mut SolanaAccount>,
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
        } else if instruction.program_id == spl_token::id() {
            spl_token::processor::Processor::process(
                &instruction.program_id,
                &account_infos,
                &instruction.data,
            )
        } else if instruction.program_id == spl_token_2022::id() {
            spl_token_2022::processor::Processor::process(
                &instruction.program_id,
                &account_infos,
                &instruction.data,
            )
        } else {
            Err(ProgramError::IncorrectProgramId)
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
        accounts: Vec<&mut SolanaAccount>,
    ) -> ProgramResult {
        do_process_instruction_with_fee_constraints(instruction, accounts, &SWAP_CONSTRAINTS)
    }

    fn mint_token(
        program_id: &Pubkey,
        mint_key: &Pubkey,
        mint_account: &mut SolanaAccount,
        mint_authority_key: &Pubkey,
        account_owner_key: &Pubkey,
        amount: u64,
    ) -> (Pubkey, SolanaAccount) {
        let account_key = Pubkey::new_unique();
        let space = if *program_id == spl_token_2022::id() {
            ExtensionType::try_calculate_account_len::<Account>(&[
                ExtensionType::ImmutableOwner,
                ExtensionType::TransferFeeAmount,
            ])
            .unwrap()
        } else {
            Account::get_packed_len()
        };
        let minimum_balance = Rent::default().minimum_balance(space);
        let mut account_account = SolanaAccount::new(minimum_balance, space, program_id);
        let mut mint_authority_account = SolanaAccount::default();
        let mut rent_sysvar_account = create_account_for_test(&Rent::free());

        // no-ops in normal token, so we're good to run it either way
        do_process_instruction(
            initialize_immutable_owner(program_id, &account_key).unwrap(),
            vec![&mut account_account],
        )
        .unwrap();

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

    fn create_mint(
        program_id: &Pubkey,
        authority_key: &Pubkey,
        freeze_authority: Option<&Pubkey>,
        close_authority: Option<&Pubkey>,
        fees: &TransferFee,
    ) -> (Pubkey, SolanaAccount) {
        let mint_key = Pubkey::new_unique();
        let space = if *program_id == spl_token_2022::id() {
            if close_authority.is_some() {
                ExtensionType::try_calculate_account_len::<Mint>(&[
                    ExtensionType::MintCloseAuthority,
                    ExtensionType::TransferFeeConfig,
                ])
                .unwrap()
            } else {
                ExtensionType::try_calculate_account_len::<Mint>(&[
                    ExtensionType::TransferFeeConfig,
                ])
                .unwrap()
            }
        } else {
            Mint::get_packed_len()
        };
        let minimum_balance = Rent::default().minimum_balance(space);
        let mut mint_account = SolanaAccount::new(minimum_balance, space, program_id);
        let mut rent_sysvar_account = create_account_for_test(&Rent::free());

        if *program_id == spl_token_2022::id() {
            if close_authority.is_some() {
                do_process_instruction(
                    initialize_mint_close_authority(program_id, &mint_key, close_authority)
                        .unwrap(),
                    vec![&mut mint_account],
                )
                .unwrap();
            }
            do_process_instruction(
                initialize_transfer_fee_config(
                    program_id,
                    &mint_key,
                    freeze_authority,
                    freeze_authority,
                    fees.transfer_fee_basis_points.into(),
                    fees.maximum_fee.into(),
                )
                .unwrap(),
                vec![&mut mint_account],
            )
            .unwrap();
        }
        do_process_instruction(
            initialize_mint(program_id, &mint_key, authority_key, freeze_authority, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar_account],
        )
        .unwrap();

        (mint_key, mint_account)
    }

    #[test_case(spl_token::id(); "token")]
    #[test_case(spl_token_2022::id(); "token-2022")]
    fn test_token_program_id_error(token_program_id: Pubkey) {
        test_syscall_stubs();
        let swap_key = Pubkey::new_unique();
        let mut mint = (Pubkey::new_unique(), SolanaAccount::default());
        let mut destination = (Pubkey::new_unique(), SolanaAccount::default());
        let token_program = (token_program_id, SolanaAccount::default());
        let (authority_key, bump_seed) =
            Pubkey::find_program_address(&[&swap_key.to_bytes()[..]], &SWAP_PROGRAM_ID);
        let mut authority = (authority_key, SolanaAccount::default());
        let swap_bytes = swap_key.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[bump_seed]];
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

    #[test_case(spl_token::id(); "token")]
    #[test_case(spl_token_2022::id(); "token-2022")]
    fn test_token_error(token_program_id: Pubkey) {
        test_syscall_stubs();
        let swap_key = Pubkey::new_unique();
        let mut mint = (
            Pubkey::new_unique(),
            SolanaAccount::new(
                mint_minimum_balance(),
                spl_token::state::Mint::get_packed_len(),
                &token_program_id,
            ),
        );
        let mut destination = (
            Pubkey::new_unique(),
            SolanaAccount::new(
                account_minimum_balance(),
                spl_token::state::Account::get_packed_len(),
                &token_program_id,
            ),
        );
        let mut token_program = (token_program_id, SolanaAccount::default());
        let (authority_key, bump_seed) =
            Pubkey::find_program_address(&[&swap_key.to_bytes()[..]], &SWAP_PROGRAM_ID);
        let mut authority = (authority_key, SolanaAccount::default());
        let swap_bytes = swap_key.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];
        let mut rent_sysvar = (
            Pubkey::new_unique(),
            create_account_for_test(&Rent::default()),
        );
        do_process_instruction(
            initialize_mint(
                &token_program.0,
                &mint.0,
                &authority.0,
                Some(&authority.0),
                2,
            )
            .unwrap(),
            vec![&mut mint.1, &mut rent_sysvar.1],
        )
        .unwrap();
        do_process_instruction(
            initialize_account(&token_program.0, &destination.0, &mint.0, &authority.0).unwrap(),
            vec![
                &mut destination.1,
                &mut mint.1,
                &mut authority.1,
                &mut rent_sysvar.1,
                &mut token_program.1,
            ],
        )
        .unwrap();
        do_process_instruction(
            freeze_account(&token_program.0, &destination.0, &mint.0, &authority.0, &[]).unwrap(),
            vec![
                &mut destination.1,
                &mut mint.1,
                &mut authority.1,
                &mut token_program.1,
            ],
        )
        .unwrap();
        let ix = mint_to(
            &token_program.0,
            &mint.0,
            &destination.0,
            &authority.0,
            &[],
            10,
        )
        .unwrap();
        let mint_info = (&mut mint).into();
        let destination_info = (&mut destination).into();
        let authority_info = (&mut authority).into();
        let token_program_info = (&mut token_program).into();

        let err = invoke_signed_wrapper::<TokenError>(
            &ix,
            &[
                mint_info,
                destination_info,
                authority_info,
                token_program_info,
            ],
            signers,
        )
        .unwrap_err();
        assert_eq!(err, ProgramError::Custom(TokenError::AccountFrozen as u32));
    }

    #[test_case(spl_token::id(), spl_token::id(), spl_token::id(); "all-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_initialize(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
        let user_key = Pubkey::new_unique();
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 2;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 10;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 5;
        let host_fee_numerator = 20;
        let host_fee_denominator = 100;
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };

        let token_a_amount = 1000;
        let token_b_amount = 2000;
        let pool_token_amount = 10;
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Arc::new(ConstantProductCurve {}),
        };

        let mut accounts = SwapAccountInfo::new(
            &user_key,
            fees,
            SwapTransferFees::default(),
            swap_curve,
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );

        // uninitialized token a account
        {
            let old_account = accounts.token_a_account;
            accounts.token_a_account = SolanaAccount::new(0, 0, &token_a_program_id);
            assert_eq!(
                Err(SwapError::ExpectedAccount.into()),
                accounts.initialize_swap()
            );
            accounts.token_a_account = old_account;
        }

        // uninitialized token b account
        {
            let old_account = accounts.token_b_account;
            accounts.token_b_account = SolanaAccount::new(0, 0, &token_b_program_id);
            assert_eq!(
                Err(SwapError::ExpectedAccount.into()),
                accounts.initialize_swap()
            );
            accounts.token_b_account = old_account;
        }

        // uninitialized pool mint
        {
            let old_account = accounts.pool_mint_account;
            accounts.pool_mint_account = SolanaAccount::new(0, 0, &pool_token_program_id);
            assert_eq!(
                Err(SwapError::ExpectedMint.into()),
                accounts.initialize_swap()
            );
            accounts.pool_mint_account = old_account;
        }

        // token A account owner is not swap authority
        {
            let (_token_a_key, token_a_account) = mint_token(
                &token_a_program_id,
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
                &token_b_program_id,
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
                &pool_token_program_id,
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
                &pool_token_program_id,
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
            let (_pool_mint_key, pool_mint_account) = create_mint(
                &pool_token_program_id,
                &user_key,
                None,
                None,
                &TransferFee::default(),
            );
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
            let (_pool_mint_key, pool_mint_account) = create_mint(
                &pool_token_program_id,
                &accounts.authority_key,
                Some(&user_key),
                None,
                &TransferFee::default(),
            );
            let old_mint = accounts.pool_mint_account;
            accounts.pool_mint_account = pool_mint_account;
            assert_eq!(
                Err(SwapError::InvalidFreezeAuthority.into()),
                accounts.initialize_swap()
            );
            accounts.pool_mint_account = old_mint;
        }

        // pool mint token has close authority, only available in token-2022
        if pool_token_program_id == spl_token_2022::id() {
            let (_pool_mint_key, pool_mint_account) = create_mint(
                &pool_token_program_id,
                &accounts.authority_key,
                None,
                Some(&user_key),
                &TransferFee::default(),
            );
            let old_mint = accounts.pool_mint_account;
            accounts.pool_mint_account = pool_mint_account;
            assert_eq!(
                Err(SwapError::InvalidCloseAuthority.into()),
                accounts.initialize_swap()
            );
            accounts.pool_mint_account = old_mint;
        }

        // token A account owned by wrong program
        {
            let (_token_a_key, mut token_a_account) = mint_token(
                &token_a_program_id,
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
                &token_b_program_id,
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
                &token_a_program_id,
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
                &token_b_program_id,
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

            let (_pool_mint_key, pool_mint_account) = create_mint(
                &pool_token_program_id,
                &accounts.authority_key,
                None,
                None,
                &TransferFee::default(),
            );
            accounts.pool_mint_account = pool_mint_account;

            let (_empty_pool_token_key, empty_pool_token_account) = mint_token(
                &pool_token_program_id,
                &accounts.pool_mint_key,
                &mut accounts.pool_mint_account,
                &accounts.authority_key,
                &user_key,
                0,
            );

            let (_pool_token_key, pool_token_account) = mint_token(
                &pool_token_program_id,
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
                &token_a_program_id,
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
                    &token_a_program_id,
                    &accounts.token_a_key,
                    &user_key,
                    &accounts.authority_key,
                    &[],
                    1,
                )
                .unwrap(),
                vec![
                    &mut accounts.token_a_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                ],
            )
            .unwrap();
            assert_eq!(
                Err(SwapError::InvalidDelegate.into()),
                accounts.initialize_swap()
            );

            do_process_instruction(
                revoke(
                    &token_a_program_id,
                    &accounts.token_a_key,
                    &accounts.authority_key,
                    &[],
                )
                .unwrap(),
                vec![&mut accounts.token_a_account, &mut SolanaAccount::default()],
            )
            .unwrap();
        }

        // token B account is delegated
        {
            do_process_instruction(
                approve(
                    &token_b_program_id,
                    &accounts.token_b_key,
                    &user_key,
                    &accounts.authority_key,
                    &[],
                    1,
                )
                .unwrap(),
                vec![
                    &mut accounts.token_b_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                ],
            )
            .unwrap();
            assert_eq!(
                Err(SwapError::InvalidDelegate.into()),
                accounts.initialize_swap()
            );

            do_process_instruction(
                revoke(
                    &token_b_program_id,
                    &accounts.token_b_key,
                    &accounts.authority_key,
                    &[],
                )
                .unwrap(),
                vec![&mut accounts.token_b_account, &mut SolanaAccount::default()],
            )
            .unwrap();
        }

        // token A account has close authority
        {
            do_process_instruction(
                set_authority(
                    &token_a_program_id,
                    &accounts.token_a_key,
                    Some(&user_key),
                    AuthorityType::CloseAccount,
                    &accounts.authority_key,
                    &[],
                )
                .unwrap(),
                vec![&mut accounts.token_a_account, &mut SolanaAccount::default()],
            )
            .unwrap();
            assert_eq!(
                Err(SwapError::InvalidCloseAuthority.into()),
                accounts.initialize_swap()
            );

            do_process_instruction(
                set_authority(
                    &token_a_program_id,
                    &accounts.token_a_key,
                    None,
                    AuthorityType::CloseAccount,
                    &user_key,
                    &[],
                )
                .unwrap(),
                vec![&mut accounts.token_a_account, &mut SolanaAccount::default()],
            )
            .unwrap();
        }

        // token B account has close authority
        {
            do_process_instruction(
                set_authority(
                    &token_b_program_id,
                    &accounts.token_b_key,
                    Some(&user_key),
                    AuthorityType::CloseAccount,
                    &accounts.authority_key,
                    &[],
                )
                .unwrap(),
                vec![&mut accounts.token_b_account, &mut SolanaAccount::default()],
            )
            .unwrap();
            assert_eq!(
                Err(SwapError::InvalidCloseAuthority.into()),
                accounts.initialize_swap()
            );

            do_process_instruction(
                set_authority(
                    &token_b_program_id,
                    &accounts.token_b_key,
                    None,
                    AuthorityType::CloseAccount,
                    &user_key,
                    &[],
                )
                .unwrap(),
                vec![&mut accounts.token_b_account, &mut SolanaAccount::default()],
            )
            .unwrap();
        }

        // wrong token program id
        {
            let wrong_program_id = Pubkey::new_unique();
            assert_eq!(
                Err(ProgramError::IncorrectProgramId),
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
                        accounts.fees.clone(),
                        accounts.swap_curve.clone(),
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut SolanaAccount::default(),
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.pool_token_account,
                        &mut SolanaAccount::default(),
                    ],
                )
            );
        }

        // create swap with same token A and B
        {
            let (_token_a_repeat_key, token_a_repeat_account) = mint_token(
                &token_a_program_id,
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

        // create invalid flat swap
        {
            let token_b_price = 0;
            let fees = Fees {
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
                curve_type: CurveType::ConstantPrice,
                calculator: Arc::new(ConstantPriceCurve { token_b_price }),
            };
            let mut accounts = SwapAccountInfo::new(
                &user_key,
                fees,
                SwapTransferFees::default(),
                swap_curve,
                token_a_amount,
                token_b_amount,
                &pool_token_program_id,
                &token_a_program_id,
                &token_b_program_id,
            );
            assert_eq!(
                Err(SwapError::InvalidCurve.into()),
                accounts.initialize_swap()
            );
        }

        // create valid flat swap
        {
            let fees = Fees {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
                host_fee_numerator,
                host_fee_denominator,
            };
            let token_b_price = 10_000;
            let swap_curve = SwapCurve {
                curve_type: CurveType::ConstantPrice,
                calculator: Arc::new(ConstantPriceCurve { token_b_price }),
            };
            let mut accounts = SwapAccountInfo::new(
                &user_key,
                fees,
                SwapTransferFees::default(),
                swap_curve,
                token_a_amount,
                token_b_amount,
                &pool_token_program_id,
                &token_a_program_id,
                &token_b_program_id,
            );
            accounts.initialize_swap().unwrap();
        }

        // create invalid offset swap
        {
            let token_b_offset = 0;
            let fees = Fees {
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
                curve_type: CurveType::Offset,
                calculator: Arc::new(OffsetCurve { token_b_offset }),
            };
            let mut accounts = SwapAccountInfo::new(
                &user_key,
                fees,
                SwapTransferFees::default(),
                swap_curve,
                token_a_amount,
                token_b_amount,
                &pool_token_program_id,
                &token_a_program_id,
                &token_b_program_id,
            );
            assert_eq!(
                Err(SwapError::InvalidCurve.into()),
                accounts.initialize_swap()
            );
        }

        // create valid offset swap
        {
            let token_b_offset = 10;
            let fees = Fees {
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
                curve_type: CurveType::Offset,
                calculator: Arc::new(OffsetCurve { token_b_offset }),
            };
            let mut accounts = SwapAccountInfo::new(
                &user_key,
                fees,
                SwapTransferFees::default(),
                swap_curve,
                token_a_amount,
                token_b_amount,
                &pool_token_program_id,
                &token_a_program_id,
                &token_b_program_id,
            );
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
            let fees = Fees {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
                host_fee_numerator,
                host_fee_denominator,
            };
            let curve = ConstantProductCurve {};
            let swap_curve = SwapCurve {
                curve_type: CurveType::ConstantProduct,
                calculator: Arc::new(curve),
            };
            let owner_key = new_key.to_string();
            let valid_curve_types = &[CurveType::ConstantProduct];
            let constraints = Some(SwapConstraints {
                owner_key: Some(owner_key.as_ref()),
                valid_curve_types,
                fees: &fees,
            });
            let mut accounts = SwapAccountInfo::new(
                &user_key,
                fees.clone(),
                SwapTransferFees::default(),
                swap_curve,
                token_a_amount,
                token_b_amount,
                &pool_token_program_id,
                &token_a_program_id,
                &token_b_program_id,
            );
            assert_eq!(
                Err(SwapError::InvalidOwner.into()),
                do_process_instruction_with_fee_constraints(
                    initialize(
                        &SWAP_PROGRAM_ID,
                        &pool_token_program_id,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &accounts.pool_token_key,
                        accounts.fees.clone(),
                        accounts.swap_curve.clone(),
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut SolanaAccount::default(),
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.pool_token_account,
                        &mut SolanaAccount::default(),
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
            let fees = Fees {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
                host_fee_numerator,
                host_fee_denominator,
            };
            let curve = ConstantProductCurve {};
            let swap_curve = SwapCurve {
                curve_type: CurveType::ConstantProduct,
                calculator: Arc::new(curve),
            };
            let owner_key = user_key.to_string();
            let valid_curve_types = &[CurveType::ConstantProduct];
            let constraints = Some(SwapConstraints {
                owner_key: Some(owner_key.as_ref()),
                valid_curve_types,
                fees: &fees,
            });
            let mut bad_fees = fees.clone();
            bad_fees.trade_fee_numerator = trade_fee_numerator - 1;
            let mut accounts = SwapAccountInfo::new(
                &user_key,
                bad_fees,
                SwapTransferFees::default(),
                swap_curve,
                token_a_amount,
                token_b_amount,
                &pool_token_program_id,
                &token_a_program_id,
                &token_b_program_id,
            );
            assert_eq!(
                Err(SwapError::InvalidFee.into()),
                do_process_instruction_with_fee_constraints(
                    initialize(
                        &SWAP_PROGRAM_ID,
                        &pool_token_program_id,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &accounts.pool_token_key,
                        accounts.fees.clone(),
                        accounts.swap_curve.clone(),
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut SolanaAccount::default(),
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.pool_token_account,
                        &mut SolanaAccount::default(),
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
            let fees = Fees {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
                host_fee_numerator,
                host_fee_denominator,
            };
            let curve = ConstantProductCurve {};
            let swap_curve = SwapCurve {
                curve_type: CurveType::ConstantProduct,
                calculator: Arc::new(curve),
            };
            let owner_key = user_key.to_string();
            let valid_curve_types = &[CurveType::ConstantProduct];
            let constraints = Some(SwapConstraints {
                owner_key: Some(owner_key.as_ref()),
                valid_curve_types,
                fees: &fees,
            });
            let mut accounts = SwapAccountInfo::new(
                &user_key,
                fees.clone(),
                SwapTransferFees::default(),
                swap_curve,
                token_a_amount,
                token_b_amount,
                &pool_token_program_id,
                &token_a_program_id,
                &token_b_program_id,
            );
            do_process_instruction_with_fee_constraints(
                initialize(
                    &SWAP_PROGRAM_ID,
                    &pool_token_program_id,
                    &accounts.swap_key,
                    &accounts.authority_key,
                    &accounts.token_a_key,
                    &accounts.token_b_key,
                    &accounts.pool_mint_key,
                    &accounts.pool_fee_key,
                    &accounts.pool_token_key,
                    accounts.fees,
                    accounts.swap_curve.clone(),
                )
                .unwrap(),
                vec![
                    &mut accounts.swap_account,
                    &mut SolanaAccount::default(),
                    &mut accounts.token_a_account,
                    &mut accounts.token_b_account,
                    &mut accounts.pool_mint_account,
                    &mut accounts.pool_fee_account,
                    &mut accounts.pool_token_account,
                    &mut SolanaAccount::default(),
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
        let swap_state = SwapVersion::unpack(&accounts.swap_account.data).unwrap();
        assert!(swap_state.is_initialized());
        assert_eq!(swap_state.bump_seed(), accounts.bump_seed);
        assert_eq!(
            swap_state.swap_curve().curve_type,
            accounts.swap_curve.curve_type
        );
        assert_eq!(*swap_state.token_a_account(), accounts.token_a_key);
        assert_eq!(*swap_state.token_b_account(), accounts.token_b_key);
        assert_eq!(*swap_state.pool_mint(), accounts.pool_mint_key);
        assert_eq!(*swap_state.token_a_mint(), accounts.token_a_mint_key);
        assert_eq!(*swap_state.token_b_mint(), accounts.token_b_mint_key);
        assert_eq!(*swap_state.pool_fee_account(), accounts.pool_fee_key);
        let token_a =
            StateWithExtensions::<Account>::unpack(&accounts.token_a_account.data).unwrap();
        assert_eq!(token_a.base.amount, token_a_amount);
        let token_b =
            StateWithExtensions::<Account>::unpack(&accounts.token_b_account.data).unwrap();
        assert_eq!(token_b.base.amount, token_b_amount);
        let pool_account =
            StateWithExtensions::<Account>::unpack(&accounts.pool_token_account.data).unwrap();
        let pool_mint =
            StateWithExtensions::<Mint>::unpack(&accounts.pool_mint_account.data).unwrap();
        assert_eq!(pool_mint.base.supply, pool_account.base.amount);
    }

    #[test_case(spl_token::id(), spl_token::id(), spl_token::id(); "all-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_deposit(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
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

        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };

        let token_a_amount = 1000;
        let token_b_amount = 9000;
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Arc::new(ConstantProductCurve {}),
        };

        let mut accounts = SwapAccountInfo::new(
            &user_key,
            fees,
            SwapTransferFees::default(),
            swap_curve,
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );

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

        accounts.initialize_swap().unwrap();

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
            wrong_swap_account.owner = pool_token_program_id;
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

        // wrong bump seed for authority_key
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
            let (bad_authority_key, _bump_seed) = Pubkey::find_program_address(
                &[&accounts.swap_key.to_bytes()[..]],
                &pool_token_program_id,
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
            let expected_error: ProgramError = if token_a_account.owner == token_b_account.owner {
                TokenError::MintMismatch.into()
            } else {
                ProgramError::InvalidAccountData
            };
            assert_eq!(
                Err(expected_error),
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
                pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            let expected_error: ProgramError = if token_a_account.owner == pool_account.owner {
                TokenError::MintMismatch.into()
            } else {
                SwapError::IncorrectTokenProgramId.into()
            };
            assert_eq!(
                Err(expected_error),
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
                        &token_a_program_id,
                        &token_b_program_id,
                        &pool_token_program_id,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &user_transfer_authority_key,
                        &token_a_key,
                        &token_b_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &pool_key,
                        &accounts.token_a_mint_key,
                        &accounts.token_b_mint_key,
                        DepositAllTokenTypes {
                            pool_token_amount: pool_amount.try_into().unwrap(),
                            maximum_token_a_amount: deposit_a,
                            maximum_token_b_amount: deposit_b,
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut token_a_account,
                        &mut token_b_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut pool_account,
                        &mut accounts.token_a_mint_account,
                        &mut accounts.token_b_mint_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
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
                        &wrong_key,
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
                        &accounts.token_a_mint_key,
                        &accounts.token_b_mint_key,
                        DepositAllTokenTypes {
                            pool_token_amount: pool_amount.try_into().unwrap(),
                            maximum_token_a_amount: deposit_a,
                            maximum_token_b_amount: deposit_b,
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut token_a_account,
                        &mut token_b_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut pool_account,
                        &mut accounts.token_a_mint_account,
                        &mut accounts.token_b_mint_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
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
            let (pool_mint_key, pool_mint_account) = create_mint(
                &pool_token_program_id,
                &accounts.authority_key,
                None,
                None,
                &TransferFee::default(),
            );
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

        // deposit 1 pool token fails because it equates to 0 swap tokens
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
                StateWithExtensions::<Account>::unpack(&accounts.token_a_account.data).unwrap();
            assert_eq!(swap_token_a.base.amount, deposit_a + token_a_amount);
            let swap_token_b =
                StateWithExtensions::<Account>::unpack(&accounts.token_b_account.data).unwrap();
            assert_eq!(swap_token_b.base.amount, deposit_b + token_b_amount);
            let token_a = StateWithExtensions::<Account>::unpack(&token_a_account.data).unwrap();
            assert_eq!(token_a.base.amount, 0);
            let token_b = StateWithExtensions::<Account>::unpack(&token_b_account.data).unwrap();
            assert_eq!(token_b.base.amount, 0);
            let pool_account = StateWithExtensions::<Account>::unpack(&pool_account.data).unwrap();
            let swap_pool_account =
                StateWithExtensions::<Account>::unpack(&accounts.pool_token_account.data).unwrap();
            let pool_mint =
                StateWithExtensions::<Mint>::unpack(&accounts.pool_mint_account.data).unwrap();
            assert_eq!(
                pool_mint.base.supply,
                pool_account.base.amount + swap_pool_account.base.amount
            );
        }
    }

    #[test_case(spl_token::id(), spl_token::id(), spl_token::id(); "all-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_withdraw(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
        let user_key = Pubkey::new_unique();
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 2;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 10;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 5;
        let host_fee_numerator = 7;
        let host_fee_denominator = 100;

        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
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

        let mut accounts = SwapAccountInfo::new(
            &user_key,
            fees,
            SwapTransferFees::default(),
            swap_curve,
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );

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

        accounts.initialize_swap().unwrap();

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
            wrong_swap_account.owner = pool_token_program_id;
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

        // wrong bump seed for authority_key
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
            let (bad_authority_key, _bump_seed) = Pubkey::find_program_address(
                &[&accounts.swap_key.to_bytes()[..]],
                &pool_token_program_id,
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
            let expected_error: ProgramError = if token_a_account.owner == token_b_account.owner {
                TokenError::MintMismatch.into()
            } else {
                ProgramError::InvalidAccountData
            };
            assert_eq!(
                Err(expected_error),
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
                pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                withdraw_amount.try_into().unwrap(),
                initial_b,
                withdraw_amount.try_into().unwrap(),
            );
            let expected_error: ProgramError = if token_a_account.owner == pool_account.owner {
                TokenError::MintMismatch.into()
            } else {
                SwapError::IncorrectTokenProgramId.into()
            };
            assert_eq!(
                Err(expected_error),
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
                        &pool_token_program_id,
                        &token_a_program_id,
                        &token_b_program_id,
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
                        &accounts.token_a_mint_key,
                        &accounts.token_b_mint_key,
                        WithdrawAllTokenTypes {
                            pool_token_amount: withdraw_amount.try_into().unwrap(),
                            minimum_token_a_amount,
                            minimum_token_b_amount,
                        }
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut accounts.pool_mint_account,
                        &mut pool_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut token_a_account,
                        &mut token_b_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.token_a_mint_account,
                        &mut accounts.token_b_mint_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
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
                        &wrong_key,
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
                        &accounts.token_a_mint_key,
                        &accounts.token_b_mint_key,
                        WithdrawAllTokenTypes {
                            pool_token_amount: withdraw_amount.try_into().unwrap(),
                            minimum_token_a_amount,
                            minimum_token_b_amount,
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut accounts.pool_mint_account,
                        &mut pool_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut token_a_account,
                        &mut token_b_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.token_a_mint_account,
                        &mut accounts.token_b_mint_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
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
            let (pool_mint_key, pool_mint_account) = create_mint(
                &pool_token_program_id,
                &accounts.authority_key,
                None,
                None,
                &TransferFee::default(),
            );
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
                StateWithExtensions::<Account>::unpack(&accounts.token_a_account.data).unwrap();
            let swap_token_b =
                StateWithExtensions::<Account>::unpack(&accounts.token_b_account.data).unwrap();
            let pool_mint =
                StateWithExtensions::<Mint>::unpack(&accounts.pool_mint_account.data).unwrap();
            let withdraw_fee = accounts.fees.owner_withdraw_fee(withdraw_amount).unwrap();
            let results = accounts
                .swap_curve
                .calculator
                .pool_tokens_to_trading_tokens(
                    withdraw_amount - withdraw_fee,
                    pool_mint.base.supply.into(),
                    swap_token_a.base.amount.into(),
                    swap_token_b.base.amount.into(),
                    RoundDirection::Floor,
                )
                .unwrap();
            assert_eq!(
                swap_token_a.base.amount,
                token_a_amount - to_u64(results.token_a_amount).unwrap()
            );
            assert_eq!(
                swap_token_b.base.amount,
                token_b_amount - to_u64(results.token_b_amount).unwrap()
            );
            let token_a = StateWithExtensions::<Account>::unpack(&token_a_account.data).unwrap();
            assert_eq!(
                token_a.base.amount,
                initial_a + to_u64(results.token_a_amount).unwrap()
            );
            let token_b = StateWithExtensions::<Account>::unpack(&token_b_account.data).unwrap();
            assert_eq!(
                token_b.base.amount,
                initial_b + to_u64(results.token_b_amount).unwrap()
            );
            let pool_account = StateWithExtensions::<Account>::unpack(&pool_account.data).unwrap();
            assert_eq!(
                pool_account.base.amount,
                to_u64(initial_pool - withdraw_amount).unwrap()
            );
            let fee_account =
                StateWithExtensions::<Account>::unpack(&accounts.pool_fee_account.data).unwrap();
            assert_eq!(
                fee_account.base.amount,
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
            let fee_account =
                StateWithExtensions::<Account>::unpack(&pool_fee_account.data).unwrap();
            let pool_fee_amount = fee_account.base.amount;

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
                StateWithExtensions::<Account>::unpack(&accounts.token_a_account.data).unwrap();
            let swap_token_b =
                StateWithExtensions::<Account>::unpack(&accounts.token_b_account.data).unwrap();
            let pool_mint =
                StateWithExtensions::<Mint>::unpack(&accounts.pool_mint_account.data).unwrap();
            let results = accounts
                .swap_curve
                .calculator
                .pool_tokens_to_trading_tokens(
                    pool_fee_amount.into(),
                    pool_mint.base.supply.into(),
                    swap_token_a.base.amount.into(),
                    swap_token_b.base.amount.into(),
                    RoundDirection::Floor,
                )
                .unwrap();
            let token_a = StateWithExtensions::<Account>::unpack(&token_a_account.data).unwrap();
            assert_eq!(
                token_a.base.amount,
                TryInto::<u64>::try_into(results.token_a_amount).unwrap()
            );
            let token_b = StateWithExtensions::<Account>::unpack(&token_b_account.data).unwrap();
            assert_eq!(
                token_b.base.amount,
                TryInto::<u64>::try_into(results.token_b_amount).unwrap()
            );
        }
    }

    #[test_case(spl_token::id(), spl_token::id(), spl_token::id(); "all-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_deposit_one_exact_in(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
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

        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };

        let token_a_amount = 1000;
        let token_b_amount = 9000;
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Arc::new(ConstantProductCurve {}),
        };

        let mut accounts = SwapAccountInfo::new(
            &user_key,
            fees,
            SwapTransferFees::default(),
            swap_curve,
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );

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

        accounts.initialize_swap().unwrap();

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
            wrong_swap_account.owner = pool_token_program_id;
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

        // wrong bump seed for authority_key
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
            let (bad_authority_key, _bump_seed) = Pubkey::find_program_address(
                &[&accounts.swap_key.to_bytes()[..]],
                &pool_token_program_id,
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
                pool_account,
            ) = accounts.setup_token_accounts(&user_key, &depositor_key, deposit_a, deposit_b, 0);
            let expected_error: ProgramError = if token_b_account.owner == pool_account.owner {
                TokenError::MintMismatch.into()
            } else {
                SwapError::IncorrectTokenProgramId.into()
            };
            assert_eq!(
                Err(expected_error),
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
                        &token_a_program_id,
                        &pool_token_program_id,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &user_transfer_authority_key,
                        &token_a_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &pool_key,
                        &accounts.token_a_mint_key,
                        DepositSingleTokenTypeExactAmountIn {
                            source_token_amount: deposit_a,
                            minimum_pool_token_amount: pool_amount,
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut token_a_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut pool_account,
                        &mut accounts.token_a_mint_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
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
                        &wrong_key,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.authority_key,
                        &token_a_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &pool_key,
                        &accounts.token_a_mint_key,
                        DepositSingleTokenTypeExactAmountIn {
                            source_token_amount: deposit_a,
                            minimum_pool_token_amount: pool_amount,
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut token_a_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut pool_account,
                        &mut accounts.token_a_mint_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
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
            let (pool_mint_key, pool_mint_account) = create_mint(
                &pool_token_program_id,
                &accounts.authority_key,
                None,
                None,
                &TransferFee::default(),
            );
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
                StateWithExtensions::<Account>::unpack(&accounts.token_a_account.data).unwrap();
            assert_eq!(swap_token_a.base.amount, deposit_a + token_a_amount);

            let token_a = StateWithExtensions::<Account>::unpack(&token_a_account.data).unwrap();
            assert_eq!(token_a.base.amount, 0);

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
                StateWithExtensions::<Account>::unpack(&accounts.token_b_account.data).unwrap();
            assert_eq!(swap_token_b.base.amount, deposit_b + token_b_amount);

            let token_b = StateWithExtensions::<Account>::unpack(&token_b_account.data).unwrap();
            assert_eq!(token_b.base.amount, 0);

            let pool_account = StateWithExtensions::<Account>::unpack(&pool_account.data).unwrap();
            let swap_pool_account =
                StateWithExtensions::<Account>::unpack(&accounts.pool_token_account.data).unwrap();
            let pool_mint =
                StateWithExtensions::<Mint>::unpack(&accounts.pool_mint_account.data).unwrap();
            assert_eq!(
                pool_mint.base.supply,
                pool_account.base.amount + swap_pool_account.base.amount
            );
        }
    }

    #[test_case(spl_token::id(), spl_token::id(), spl_token::id(); "all-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_withdraw_one_exact_out(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
        let user_key = Pubkey::new_unique();
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 2;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 10;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 5;
        let host_fee_numerator = 7;
        let host_fee_denominator = 100;

        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
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

        let mut accounts = SwapAccountInfo::new(
            &user_key,
            fees,
            SwapTransferFees::default(),
            swap_curve,
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );

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

        accounts.initialize_swap().unwrap();

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
            wrong_swap_account.owner = pool_token_program_id;
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

        // wrong bump seed for authority_key
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
            let (bad_authority_key, _bump_seed) = Pubkey::find_program_address(
                &[&accounts.swap_key.to_bytes()[..]],
                &pool_token_program_id,
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
                pool_account,
            ) = accounts.setup_token_accounts(
                &user_key,
                &withdrawer_key,
                maximum_pool_token_amount,
                initial_b,
                maximum_pool_token_amount,
            );
            let expected_error: ProgramError = if token_a_account.owner == pool_account.owner {
                TokenError::MintMismatch.into()
            } else {
                SwapError::IncorrectTokenProgramId.into()
            };
            assert_eq!(
                Err(expected_error),
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
                        &pool_token_program_id,
                        &token_a_program_id,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &user_transfer_authority_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &pool_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &token_a_key,
                        &accounts.token_a_mint_key,
                        WithdrawSingleTokenTypeExactAmountOut {
                            destination_token_amount: destination_a_amount,
                            maximum_pool_token_amount,
                        }
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut accounts.pool_mint_account,
                        &mut pool_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut token_a_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.token_a_mint_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
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
                        &accounts.token_a_mint_key,
                        WithdrawSingleTokenTypeExactAmountOut {
                            destination_token_amount: destination_a_amount,
                            maximum_pool_token_amount,
                        }
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut accounts.pool_mint_account,
                        &mut pool_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut token_a_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.token_a_mint_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
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
            let (pool_mint_key, pool_mint_account) = create_mint(
                &pool_token_program_id,
                &accounts.authority_key,
                None,
                None,
                &TransferFee::default(),
            );
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
                StateWithExtensions::<Account>::unpack(&accounts.token_a_account.data).unwrap();
            let swap_token_b =
                StateWithExtensions::<Account>::unpack(&accounts.token_b_account.data).unwrap();
            let pool_mint =
                StateWithExtensions::<Mint>::unpack(&accounts.pool_mint_account.data).unwrap();

            let pool_token_amount = accounts
                .swap_curve
                .withdraw_single_token_type_exact_out(
                    destination_a_amount.into(),
                    swap_token_a.base.amount.into(),
                    swap_token_b.base.amount.into(),
                    pool_mint.base.supply.into(),
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
                StateWithExtensions::<Account>::unpack(&accounts.token_a_account.data).unwrap();

            assert_eq!(
                swap_token_a.base.amount,
                token_a_amount - destination_a_amount
            );
            let token_a = StateWithExtensions::<Account>::unpack(&token_a_account.data).unwrap();
            assert_eq!(token_a.base.amount, initial_a + destination_a_amount);

            let pool_account = StateWithExtensions::<Account>::unpack(&pool_account.data).unwrap();
            assert_eq!(
                pool_account.base.amount,
                to_u64(initial_pool - pool_token_amount - withdraw_fee).unwrap()
            );
            let fee_account =
                StateWithExtensions::<Account>::unpack(&accounts.pool_fee_account.data).unwrap();
            assert_eq!(fee_account.base.amount, to_u64(withdraw_fee).unwrap());
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
            let fee_account =
                StateWithExtensions::<Account>::unpack(&pool_fee_account.data).unwrap();
            let pool_fee_amount = fee_account.base.amount;

            let swap_token_a =
                StateWithExtensions::<Account>::unpack(&accounts.token_a_account.data).unwrap();

            let token_a_amount = swap_token_a.base.amount;
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
                StateWithExtensions::<Account>::unpack(&accounts.token_a_account.data).unwrap();

            assert_eq!(swap_token_a.base.amount, token_a_amount - fee_a_amount);
            let token_a = StateWithExtensions::<Account>::unpack(&token_a_account.data).unwrap();
            assert_eq!(token_a.base.amount, initial_a + fee_a_amount);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_valid_swap_curve(
        fees: Fees,
        transfer_fees: SwapTransferFees,
        curve_type: CurveType,
        calculator: Arc<dyn CurveCalculator + Send + Sync>,
        token_a_amount: u64,
        token_b_amount: u64,
        pool_token_program_id: &Pubkey,
        token_a_program_id: &Pubkey,
        token_b_program_id: &Pubkey,
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
            transfer_fees,
            swap_curve.clone(),
            token_a_amount,
            token_b_amount,
            pool_token_program_id,
            token_a_program_id,
            token_b_program_id,
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
        let minimum_token_b_amount = 0;
        let pool_mint =
            StateWithExtensions::<Mint>::unpack(&accounts.pool_mint_account.data).unwrap();
        let initial_supply = pool_mint.base.supply;
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

        // tweak values based on transfer fees assessed
        let token_a_fee = accounts
            .transfer_fees
            .token_a
            .calculate_fee(a_to_b_amount)
            .unwrap();
        let actual_a_to_b_amount = a_to_b_amount - token_a_fee;
        let results = swap_curve
            .swap(
                actual_a_to_b_amount.into(),
                token_a_amount.into(),
                token_b_amount.into(),
                TradeDirection::AtoB,
                &fees,
            )
            .unwrap();

        let swap_token_a =
            StateWithExtensions::<Account>::unpack(&accounts.token_a_account.data).unwrap();
        let token_a_amount = swap_token_a.base.amount;
        assert_eq!(
            token_a_amount,
            TryInto::<u64>::try_into(results.new_swap_source_amount).unwrap()
        );
        let token_a = StateWithExtensions::<Account>::unpack(&token_a_account.data).unwrap();
        assert_eq!(token_a.base.amount, initial_a - a_to_b_amount);

        let swap_token_b =
            StateWithExtensions::<Account>::unpack(&accounts.token_b_account.data).unwrap();
        let token_b_amount = swap_token_b.base.amount;
        assert_eq!(
            token_b_amount,
            TryInto::<u64>::try_into(results.new_swap_destination_amount).unwrap()
        );
        let token_b = StateWithExtensions::<Account>::unpack(&token_b_account.data).unwrap();
        assert_eq!(
            token_b.base.amount,
            initial_b + to_u64(results.destination_amount_swapped).unwrap()
        );

        let first_fee = if results.owner_fee > 0 {
            swap_curve
                .calculator
                .withdraw_single_token_type_exact_out(
                    results.owner_fee,
                    token_a_amount.into(),
                    token_b_amount.into(),
                    initial_supply.into(),
                    TradeDirection::AtoB,
                    RoundDirection::Floor,
                )
                .unwrap()
        } else {
            0
        };
        let fee_account =
            StateWithExtensions::<Account>::unpack(&accounts.pool_fee_account.data).unwrap();
        assert_eq!(
            fee_account.base.amount,
            TryInto::<u64>::try_into(first_fee).unwrap()
        );

        let first_swap_amount = results.destination_amount_swapped;

        // swap the other way
        let pool_mint =
            StateWithExtensions::<Mint>::unpack(&accounts.pool_mint_account.data).unwrap();
        let initial_supply = pool_mint.base.supply;

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

        let mut results = swap_curve
            .swap(
                b_to_a_amount.into(),
                token_b_amount.into(),
                token_a_amount.into(),
                TradeDirection::BtoA,
                &fees,
            )
            .unwrap();
        // tweak values based on transfer fees assessed
        let token_a_fee = accounts
            .transfer_fees
            .token_a
            .calculate_fee(results.destination_amount_swapped.try_into().unwrap())
            .unwrap();
        results.destination_amount_swapped -= token_a_fee as u128;

        let swap_token_a =
            StateWithExtensions::<Account>::unpack(&accounts.token_a_account.data).unwrap();
        let token_a_amount = swap_token_a.base.amount;
        assert_eq!(
            token_a_amount,
            TryInto::<u64>::try_into(results.new_swap_destination_amount).unwrap()
        );
        let token_a = StateWithExtensions::<Account>::unpack(&token_a_account.data).unwrap();
        assert_eq!(
            token_a.base.amount,
            initial_a - a_to_b_amount + to_u64(results.destination_amount_swapped).unwrap()
        );

        let swap_token_b =
            StateWithExtensions::<Account>::unpack(&accounts.token_b_account.data).unwrap();
        let token_b_amount = swap_token_b.base.amount;
        assert_eq!(
            token_b_amount,
            TryInto::<u64>::try_into(results.new_swap_source_amount).unwrap()
        );
        let token_b = StateWithExtensions::<Account>::unpack(&token_b_account.data).unwrap();
        assert_eq!(
            token_b.base.amount,
            initial_b + to_u64(first_swap_amount).unwrap()
                - to_u64(results.source_amount_swapped).unwrap()
        );

        let second_fee = if results.owner_fee > 0 {
            swap_curve
                .calculator
                .withdraw_single_token_type_exact_out(
                    results.owner_fee,
                    token_a_amount.into(),
                    token_b_amount.into(),
                    initial_supply.into(),
                    TradeDirection::BtoA,
                    RoundDirection::Floor,
                )
                .unwrap()
        } else {
            0
        };
        let fee_account =
            StateWithExtensions::<Account>::unpack(&accounts.pool_fee_account.data).unwrap();
        assert_eq!(
            fee_account.base.amount,
            to_u64(first_fee + second_fee).unwrap()
        );
    }

    #[test_case(spl_token::id(), spl_token::id(), spl_token::id(); "all-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_valid_swap_curve_all_fees(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
        // All fees
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 10;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 30;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 30;
        let host_fee_numerator = 20;
        let host_fee_denominator = 100;
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };

        let token_a_amount = 10_000_000_000;
        let token_b_amount = 50_000_000_000;

        check_valid_swap_curve(
            fees.clone(),
            SwapTransferFees::default(),
            CurveType::ConstantProduct,
            Arc::new(ConstantProductCurve {}),
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );
        let token_b_price = 1;
        check_valid_swap_curve(
            fees.clone(),
            SwapTransferFees::default(),
            CurveType::ConstantPrice,
            Arc::new(ConstantPriceCurve { token_b_price }),
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );
        let token_b_offset = 10_000_000_000;
        check_valid_swap_curve(
            fees,
            SwapTransferFees::default(),
            CurveType::Offset,
            Arc::new(OffsetCurve { token_b_offset }),
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );
    }

    #[test_case(spl_token::id(), spl_token::id(), spl_token::id(); "all-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_valid_swap_curve_trade_fee_only(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 10;
        let owner_trade_fee_numerator = 0;
        let owner_trade_fee_denominator = 0;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 0;
        let host_fee_numerator = 0;
        let host_fee_denominator = 0;
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };

        let token_a_amount = 10_000_000_000;
        let token_b_amount = 50_000_000_000;

        check_valid_swap_curve(
            fees.clone(),
            SwapTransferFees::default(),
            CurveType::ConstantProduct,
            Arc::new(ConstantProductCurve {}),
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );
        let token_b_price = 10_000;
        check_valid_swap_curve(
            fees.clone(),
            SwapTransferFees::default(),
            CurveType::ConstantPrice,
            Arc::new(ConstantPriceCurve { token_b_price }),
            token_a_amount,
            token_b_amount / token_b_price,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );
        let token_b_offset = 1;
        check_valid_swap_curve(
            fees,
            SwapTransferFees::default(),
            CurveType::Offset,
            Arc::new(OffsetCurve { token_b_offset }),
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );
    }

    #[test_case(spl_token::id(), spl_token::id(), spl_token::id(); "all-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_valid_swap_with_fee_constraints(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
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

        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };

        let curve = ConstantProductCurve {};
        let swap_curve = SwapCurve {
            curve_type: CurveType::ConstantProduct,
            calculator: Arc::new(curve),
        };

        let owner_key_str = owner_key.to_string();
        let valid_curve_types = &[CurveType::ConstantProduct];
        let constraints = Some(SwapConstraints {
            owner_key: Some(owner_key_str.as_ref()),
            valid_curve_types,
            fees: &fees,
        });
        let mut accounts = SwapAccountInfo::new(
            &owner_key,
            fees.clone(),
            SwapTransferFees::default(),
            swap_curve,
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );

        // initialize swap
        do_process_instruction_with_fee_constraints(
            initialize(
                &SWAP_PROGRAM_ID,
                &pool_token_program_id,
                &accounts.swap_key,
                &accounts.authority_key,
                &accounts.token_a_key,
                &accounts.token_b_key,
                &accounts.pool_mint_key,
                &accounts.pool_fee_key,
                &accounts.pool_token_key,
                accounts.fees.clone(),
                accounts.swap_curve.clone(),
            )
            .unwrap(),
            vec![
                &mut accounts.swap_account,
                &mut SolanaAccount::default(),
                &mut accounts.token_a_account,
                &mut accounts.token_b_account,
                &mut accounts.pool_mint_account,
                &mut accounts.pool_fee_account,
                &mut accounts.pool_token_account,
                &mut SolanaAccount::default(),
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
                &token_a_program_id,
                &token_b_program_id,
                &pool_token_program_id,
                &accounts.swap_key,
                &accounts.authority_key,
                &accounts.authority_key,
                &token_a_key,
                &accounts.token_a_key,
                &accounts.token_b_key,
                &token_b_key,
                &accounts.pool_mint_key,
                &accounts.pool_fee_key,
                &accounts.token_a_mint_key,
                &accounts.token_b_mint_key,
                Some(&pool_key),
                Swap {
                    amount_in,
                    minimum_amount_out,
                },
            )
            .unwrap(),
            vec![
                &mut accounts.swap_account,
                &mut SolanaAccount::default(),
                &mut SolanaAccount::default(),
                &mut token_a_account,
                &mut accounts.token_a_account,
                &mut accounts.token_b_account,
                &mut token_b_account,
                &mut accounts.pool_mint_account,
                &mut accounts.pool_fee_account,
                &mut accounts.token_a_mint_account,
                &mut accounts.token_b_mint_account,
                &mut SolanaAccount::default(),
                &mut SolanaAccount::default(),
                &mut SolanaAccount::default(),
                &mut pool_account,
            ],
            &constraints,
        )
        .unwrap();

        // check that fees were taken in the host fee account
        let host_fee_account = StateWithExtensions::<Account>::unpack(&pool_account.data).unwrap();
        let owner_fee_account =
            StateWithExtensions::<Account>::unpack(&accounts.pool_fee_account.data).unwrap();
        let total_fee = owner_fee_account.base.amount * host_fee_denominator
            / (host_fee_denominator - host_fee_numerator);
        assert_eq!(
            total_fee,
            host_fee_account.base.amount + owner_fee_account.base.amount
        );
    }

    #[test_case(spl_token::id(), spl_token::id(), spl_token::id(); "all-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_invalid_swap(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
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
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };

        let token_a_amount = 1000;
        let token_b_amount = 5000;
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Arc::new(ConstantProductCurve {}),
        };
        let mut accounts = SwapAccountInfo::new(
            &user_key,
            fees,
            SwapTransferFees::default(),
            swap_curve,
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );

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

        accounts.initialize_swap().unwrap();

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
            wrong_swap_account.owner = pool_token_program_id;
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

        // wrong bump seed
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
            let (bad_authority_key, _bump_seed) = Pubkey::find_program_address(
                &[&accounts.swap_key.to_bytes()[..]],
                &pool_token_program_id,
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
                        &wrong_program_id,
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
                        &accounts.token_a_mint_key,
                        &accounts.token_b_mint_key,
                        None,
                        Swap {
                            amount_in: initial_a,
                            minimum_amount_out: minimum_token_b_amount,
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut token_a_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.token_a_mint_account,
                        &mut accounts.token_b_mint_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
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
                        &token_a_program_id,
                        &token_b_program_id,
                        &pool_token_program_id,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &user_transfer_key,
                        &token_a_key,
                        &token_a_key,
                        &token_b_key,
                        &token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &accounts.token_a_mint_key,
                        &accounts.token_b_mint_key,
                        None,
                        Swap {
                            amount_in: initial_a,
                            minimum_amount_out: minimum_token_b_amount,
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut token_a_account.clone(),
                        &mut token_a_account,
                        &mut token_b_account.clone(),
                        &mut token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.token_a_mint_account,
                        &mut accounts.token_b_mint_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
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
            let (pool_mint_key, pool_mint_account) = create_mint(
                &pool_token_program_id,
                &accounts.authority_key,
                None,
                None,
                &TransferFee::default(),
            );
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
                        &token_a_program_id,
                        &token_b_program_id,
                        &pool_token_program_id,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &user_transfer_key,
                        &token_a_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &accounts.token_a_mint_key,
                        &accounts.token_b_mint_key,
                        None,
                        Swap {
                            amount_in: initial_a,
                            minimum_amount_out: minimum_token_b_amount,
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut token_a_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.token_a_mint_account,
                        &mut accounts.token_b_mint_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
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
            let owner_key = swapper_key.to_string();
            let fees = Fees {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
                host_fee_numerator,
                host_fee_denominator,
            };
            let constraints = Some(SwapConstraints {
                owner_key: Some(owner_key.as_ref()),
                valid_curve_types: &[],
                fees: &fees,
            });
            do_process_instruction_with_fee_constraints(
                swap(
                    &SWAP_PROGRAM_ID,
                    &token_a_program_id,
                    &token_b_program_id,
                    &pool_token_program_id,
                    &accounts.swap_key,
                    &accounts.authority_key,
                    &accounts.authority_key,
                    &token_a_key,
                    &accounts.token_a_key,
                    &accounts.token_b_key,
                    &token_b_key,
                    &accounts.pool_mint_key,
                    &accounts.pool_fee_key,
                    &accounts.token_a_mint_key,
                    &accounts.token_b_mint_key,
                    None,
                    Swap {
                        amount_in: initial_a,
                        minimum_amount_out: minimum_token_b_amount,
                    },
                )
                .unwrap(),
                vec![
                    &mut accounts.swap_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                    &mut token_a_account,
                    &mut accounts.token_a_account,
                    &mut accounts.token_b_account,
                    &mut token_b_account,
                    &mut accounts.pool_mint_account,
                    &mut accounts.pool_fee_account,
                    &mut accounts.token_a_mint_account,
                    &mut accounts.token_b_mint_account,
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
                    &mut SolanaAccount::default(),
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
            let owner_key = swapper_key.to_string();
            let fees = Fees {
                trade_fee_numerator,
                trade_fee_denominator,
                owner_trade_fee_numerator,
                owner_trade_fee_denominator,
                owner_withdraw_fee_numerator,
                owner_withdraw_fee_denominator,
                host_fee_numerator,
                host_fee_denominator,
            };
            let constraints = Some(SwapConstraints {
                owner_key: Some(owner_key.as_ref()),
                valid_curve_types: &[],
                fees: &fees,
            });
            assert_eq!(
                Err(SwapError::IncorrectPoolMint.into()),
                do_process_instruction_with_fee_constraints(
                    swap(
                        &SWAP_PROGRAM_ID,
                        &token_a_program_id,
                        &token_b_program_id,
                        &pool_token_program_id,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.authority_key,
                        &token_a_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_fee_key,
                        &accounts.token_a_mint_key,
                        &accounts.token_b_mint_key,
                        Some(&bad_token_a_key),
                        Swap {
                            amount_in: initial_a,
                            minimum_amount_out: 0,
                        },
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut token_a_account,
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_fee_account,
                        &mut accounts.token_a_mint_account,
                        &mut accounts.token_b_mint_account,
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut SolanaAccount::default(),
                        &mut bad_token_a_account,
                    ],
                    &constraints,
                ),
            );
        }
    }

    #[test_case(spl_token::id(), spl_token::id(), spl_token::id(); "all-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_overdraw_offset_curve(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 10;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 30;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 30;
        let host_fee_numerator = 10;
        let host_fee_denominator = 100;

        let token_a_amount = 1_000_000_000;
        let token_b_amount = 0;
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };

        let token_b_offset = 2_000_000;
        let swap_curve = SwapCurve {
            curve_type: CurveType::Offset,
            calculator: Arc::new(OffsetCurve { token_b_offset }),
        };
        let user_key = Pubkey::new_unique();
        let swapper_key = Pubkey::new_unique();

        let mut accounts = SwapAccountInfo::new(
            &user_key,
            fees,
            SwapTransferFees::default(),
            swap_curve,
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );

        accounts.initialize_swap().unwrap();

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

    #[test_case(spl_token::id(), spl_token::id(), spl_token::id(); "all-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_withdraw_all_offset_curve(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 10;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 30;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 30;
        let host_fee_numerator = 10;
        let host_fee_denominator = 100;

        let token_a_amount = 1_000_000_000;
        let token_b_amount = 10;
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };

        let token_b_offset = 2_000_000;
        let swap_curve = SwapCurve {
            curve_type: CurveType::Offset,
            calculator: Arc::new(OffsetCurve { token_b_offset }),
        };
        let total_pool = swap_curve.calculator.new_pool_supply();
        let user_key = Pubkey::new_unique();
        let withdrawer_key = Pubkey::new_unique();

        let mut accounts = SwapAccountInfo::new(
            &user_key,
            fees,
            SwapTransferFees::default(),
            swap_curve,
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );

        accounts.initialize_swap().unwrap();

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

        let token_a = StateWithExtensions::<Account>::unpack(&token_a_account.data).unwrap();
        assert_eq!(token_a.base.amount, token_a_amount);
        let token_b = StateWithExtensions::<Account>::unpack(&token_b_account.data).unwrap();
        assert_eq!(token_b.base.amount, token_b_amount);
        let swap_token_a =
            StateWithExtensions::<Account>::unpack(&accounts.token_a_account.data).unwrap();
        assert_eq!(swap_token_a.base.amount, 0);
        let swap_token_b =
            StateWithExtensions::<Account>::unpack(&accounts.token_b_account.data).unwrap();
        assert_eq!(swap_token_b.base.amount, 0);
    }

    #[test_case(spl_token::id(), spl_token::id(), spl_token::id(); "all-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_withdraw_all_constant_price_curve(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 10;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 30;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 30;
        let host_fee_numerator = 10;
        let host_fee_denominator = 100;

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
            host_fee_numerator,
            host_fee_denominator,
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
            SwapTransferFees::default(),
            swap_curve,
            swap_token_a_amount,
            swap_token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );

        accounts.initialize_swap().unwrap();

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

        let token_a = StateWithExtensions::<Account>::unpack(&token_a_account.data).unwrap();
        assert_eq!(token_a.base.amount, swap_token_a_amount);
        let token_b = StateWithExtensions::<Account>::unpack(&token_b_account.data).unwrap();
        assert_eq!(token_b.base.amount, 750);
        let swap_token_a =
            StateWithExtensions::<Account>::unpack(&accounts.token_a_account.data).unwrap();
        assert_eq!(swap_token_a.base.amount, 0);
        let swap_token_b =
            StateWithExtensions::<Account>::unpack(&accounts.token_b_account.data).unwrap();
        assert_eq!(swap_token_b.base.amount, 250);

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

    #[test_case(spl_token::id(), spl_token::id(), spl_token::id(); "all-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_deposits_allowed_single_token(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 10;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 30;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 30;
        let host_fee_numerator = 10;
        let host_fee_denominator = 100;

        let token_a_amount = 1_000_000;
        let token_b_amount = 0;
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };

        let token_b_offset = 2_000_000;
        let swap_curve = SwapCurve {
            curve_type: CurveType::Offset,
            calculator: Arc::new(OffsetCurve { token_b_offset }),
        };
        let creator_key = Pubkey::new_unique();
        let depositor_key = Pubkey::new_unique();

        let mut accounts = SwapAccountInfo::new(
            &creator_key,
            fees,
            SwapTransferFees::default(),
            swap_curve,
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );

        accounts.initialize_swap().unwrap();

        let initial_a = 1_000_000;
        let initial_b = 2_000_000;
        let (
            _depositor_token_a_key,
            _depositor_token_a_account,
            depositor_token_b_key,
            mut depositor_token_b_account,
            depositor_pool_key,
            mut depositor_pool_account,
        ) = accounts.setup_token_accounts(&creator_key, &depositor_key, initial_a, initial_b, 0);

        assert_eq!(
            Err(SwapError::UnsupportedCurveOperation.into()),
            accounts.deposit_single_token_type_exact_amount_in(
                &depositor_key,
                &depositor_token_b_key,
                &mut depositor_token_b_account,
                &depositor_pool_key,
                &mut depositor_pool_account,
                initial_b,
                0,
            )
        );
    }

    #[test_case(spl_token::id(), spl_token::id(), spl_token::id(); "all-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_withdraw_with_invalid_fee_account(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
        let user_key = Pubkey::new_unique();

        let fees = Fees {
            trade_fee_numerator: 1,
            trade_fee_denominator: 2,
            owner_trade_fee_numerator: 1,
            owner_trade_fee_denominator: 10,
            owner_withdraw_fee_numerator: 1,
            owner_withdraw_fee_denominator: 5,
            host_fee_numerator: 7,
            host_fee_denominator: 100,
        };

        let token_a_amount = 1000;
        let token_b_amount = 2000;
        let swap_curve = SwapCurve {
            curve_type: CurveType::ConstantProduct,
            calculator: Arc::new(ConstantProductCurve {}),
        };

        let withdrawer_key = Pubkey::new_unique();
        let initial_a = token_a_amount / 10;
        let initial_b = token_b_amount / 10;
        let initial_pool = swap_curve.calculator.new_pool_supply() / 10;
        let withdraw_amount = initial_pool / 4;
        let minimum_token_a_amount = initial_a / 40;
        let minimum_token_b_amount = initial_b / 40;

        let mut accounts = SwapAccountInfo::new(
            &user_key,
            fees,
            SwapTransferFees::default(),
            swap_curve,
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );

        accounts.initialize_swap().unwrap();

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

        let destination_key = Pubkey::new_unique();
        let mut destination = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &withdrawer_key,
        );

        do_process_instruction(
            close_account(
                &pool_token_program_id,
                &accounts.pool_fee_key,
                &destination_key,
                &user_key,
                &[],
            )
            .unwrap(),
            vec![
                &mut accounts.pool_fee_account,
                &mut destination,
                &mut SolanaAccount::default(),
            ],
        )
        .unwrap();

        let user_transfer_authority_key = Pubkey::new_unique();
        let pool_token_amount = withdraw_amount.try_into().unwrap();

        do_process_instruction(
            approve(
                &pool_token_program_id,
                &pool_key,
                &user_transfer_authority_key,
                &withdrawer_key,
                &[],
                pool_token_amount,
            )
            .unwrap(),
            vec![
                &mut pool_account,
                &mut SolanaAccount::default(),
                &mut SolanaAccount::default(),
            ],
        )
        .unwrap();

        do_process_instruction(
            withdraw_all_token_types(
                &SWAP_PROGRAM_ID,
                &pool_token_program_id,
                &token_a_program_id,
                &token_b_program_id,
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
                &accounts.token_a_mint_key,
                &accounts.token_b_mint_key,
                WithdrawAllTokenTypes {
                    pool_token_amount,
                    minimum_token_a_amount,
                    minimum_token_b_amount,
                },
            )
            .unwrap(),
            vec![
                &mut accounts.swap_account,
                &mut SolanaAccount::default(),
                &mut SolanaAccount::default(),
                &mut accounts.pool_mint_account,
                &mut pool_account,
                &mut accounts.token_a_account,
                &mut accounts.token_b_account,
                &mut token_a_account,
                &mut token_b_account,
                &mut accounts.pool_fee_account,
                &mut accounts.token_a_mint_account,
                &mut accounts.token_b_mint_account,
                &mut SolanaAccount::default(),
                &mut SolanaAccount::default(),
                &mut SolanaAccount::default(),
            ],
        )
        .unwrap();
    }

    #[test_case(spl_token::id(), spl_token::id(), spl_token::id(); "all-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_withdraw_one_exact_out_with_invalid_fee_account(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
        let user_key = Pubkey::new_unique();

        let fees = Fees {
            trade_fee_numerator: 1,
            trade_fee_denominator: 2,
            owner_trade_fee_numerator: 1,
            owner_trade_fee_denominator: 10,
            owner_withdraw_fee_numerator: 1,
            owner_withdraw_fee_denominator: 5,
            host_fee_numerator: 7,
            host_fee_denominator: 100,
        };

        let token_a_amount = 1000;
        let token_b_amount = 2000;
        let swap_curve = SwapCurve {
            curve_type: CurveType::ConstantProduct,
            calculator: Arc::new(ConstantProductCurve {}),
        };

        let withdrawer_key = Pubkey::new_unique();
        let initial_a = token_a_amount / 10;
        let initial_b = token_b_amount / 10;
        let initial_pool = swap_curve.calculator.new_pool_supply() / 10;
        let maximum_pool_token_amount = to_u64(initial_pool / 4).unwrap();
        let destination_a_amount = initial_a / 40;

        let mut accounts = SwapAccountInfo::new(
            &user_key,
            fees,
            SwapTransferFees::default(),
            swap_curve,
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );

        accounts.initialize_swap().unwrap();

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

        let destination_key = Pubkey::new_unique();
        let mut destination = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &withdrawer_key,
        );

        do_process_instruction(
            close_account(
                &pool_token_program_id,
                &accounts.pool_fee_key,
                &destination_key,
                &user_key,
                &[],
            )
            .unwrap(),
            vec![
                &mut accounts.pool_fee_account,
                &mut destination,
                &mut SolanaAccount::default(),
            ],
        )
        .unwrap();

        let user_transfer_authority_key = Pubkey::new_unique();

        do_process_instruction(
            approve(
                &pool_token_program_id,
                &pool_key,
                &user_transfer_authority_key,
                &withdrawer_key,
                &[],
                maximum_pool_token_amount,
            )
            .unwrap(),
            vec![
                &mut pool_account,
                &mut SolanaAccount::default(),
                &mut SolanaAccount::default(),
            ],
        )
        .unwrap();

        do_process_instruction(
            withdraw_single_token_type_exact_amount_out(
                &SWAP_PROGRAM_ID,
                &pool_token_program_id,
                &token_a_program_id,
                &accounts.swap_key,
                &accounts.authority_key,
                &user_transfer_authority_key,
                &accounts.pool_mint_key,
                &accounts.pool_fee_key,
                &pool_key,
                &accounts.token_a_key,
                &accounts.token_b_key,
                &token_a_key,
                &accounts.token_a_mint_key,
                WithdrawSingleTokenTypeExactAmountOut {
                    destination_token_amount: destination_a_amount,
                    maximum_pool_token_amount,
                },
            )
            .unwrap(),
            vec![
                &mut accounts.swap_account,
                &mut SolanaAccount::default(),
                &mut SolanaAccount::default(),
                &mut accounts.pool_mint_account,
                &mut pool_account,
                &mut accounts.token_a_account,
                &mut accounts.token_b_account,
                &mut token_a_account,
                &mut accounts.pool_fee_account,
                &mut accounts.token_a_mint_account,
                &mut SolanaAccount::default(),
                &mut SolanaAccount::default(),
            ],
        )
        .unwrap();
    }

    #[test_case(spl_token::id(), spl_token::id(), spl_token::id(); "all-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_valid_swap_with_invalid_fee_account(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
        let owner_key = &Pubkey::new_unique();

        let token_a_amount = 1_000_000;
        let token_b_amount = 5_000_000;

        let fees = Fees {
            trade_fee_numerator: 1,
            trade_fee_denominator: 10,
            owner_trade_fee_numerator: 1,
            owner_trade_fee_denominator: 30,
            owner_withdraw_fee_numerator: 1,
            owner_withdraw_fee_denominator: 30,
            host_fee_numerator: 10,
            host_fee_denominator: 100,
        };

        let swap_curve = SwapCurve {
            curve_type: CurveType::ConstantProduct,
            calculator: Arc::new(ConstantProductCurve {}),
        };

        let owner_key_str = owner_key.to_string();
        let constraints = Some(SwapConstraints {
            owner_key: Some(owner_key_str.as_ref()),
            valid_curve_types: &[CurveType::ConstantProduct],
            fees: &fees,
        });
        let mut accounts = SwapAccountInfo::new(
            owner_key,
            fees.clone(),
            SwapTransferFees::default(),
            swap_curve,
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );

        do_process_instruction_with_fee_constraints(
            initialize(
                &SWAP_PROGRAM_ID,
                &pool_token_program_id,
                &accounts.swap_key,
                &accounts.authority_key,
                &accounts.token_a_key,
                &accounts.token_b_key,
                &accounts.pool_mint_key,
                &accounts.pool_fee_key,
                &accounts.pool_token_key,
                accounts.fees.clone(),
                accounts.swap_curve.clone(),
            )
            .unwrap(),
            vec![
                &mut accounts.swap_account,
                &mut SolanaAccount::default(),
                &mut accounts.token_a_account,
                &mut accounts.token_b_account,
                &mut accounts.pool_mint_account,
                &mut accounts.pool_fee_account,
                &mut accounts.pool_token_account,
                &mut SolanaAccount::default(),
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
            owner_key,
            &authority_key,
            token_a_amount,
            token_b_amount,
            0,
        );

        let destination_key = Pubkey::new_unique();
        let mut destination = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            owner_key,
        );

        do_process_instruction(
            close_account(
                &pool_token_program_id,
                &accounts.pool_fee_key,
                &destination_key,
                owner_key,
                &[],
            )
            .unwrap(),
            vec![
                &mut accounts.pool_fee_account,
                &mut destination,
                &mut SolanaAccount::default(),
            ],
        )
        .unwrap();

        do_process_instruction_with_fee_constraints(
            swap(
                &SWAP_PROGRAM_ID,
                &token_a_program_id,
                &token_b_program_id,
                &pool_token_program_id,
                &accounts.swap_key,
                &accounts.authority_key,
                &accounts.authority_key,
                &token_a_key,
                &accounts.token_a_key,
                &accounts.token_b_key,
                &token_b_key,
                &accounts.pool_mint_key,
                &accounts.pool_fee_key,
                &accounts.token_a_mint_key,
                &accounts.token_b_mint_key,
                Some(&pool_key),
                Swap {
                    amount_in: token_a_amount / 2,
                    minimum_amount_out: 0,
                },
            )
            .unwrap(),
            vec![
                &mut accounts.swap_account,
                &mut SolanaAccount::default(),
                &mut SolanaAccount::default(),
                &mut token_a_account,
                &mut accounts.token_a_account,
                &mut accounts.token_b_account,
                &mut token_b_account,
                &mut accounts.pool_mint_account,
                &mut accounts.pool_fee_account,
                &mut accounts.token_a_mint_account,
                &mut accounts.token_b_mint_account,
                &mut SolanaAccount::default(),
                &mut SolanaAccount::default(),
                &mut SolanaAccount::default(),
                &mut pool_account,
            ],
            &constraints,
        )
        .unwrap();
    }

    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token_2022::id(); "all-token-2022")]
    #[test_case(spl_token::id(), spl_token_2022::id(), spl_token_2022::id(); "mixed-pool-token")]
    #[test_case(spl_token_2022::id(), spl_token_2022::id(), spl_token::id(); "mixed-pool-token-2022")]
    fn test_swap_curve_with_transfer_fees(
        pool_token_program_id: Pubkey,
        token_a_program_id: Pubkey,
        token_b_program_id: Pubkey,
    ) {
        // All fees
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 10;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 30;
        let owner_withdraw_fee_numerator = 1;
        let owner_withdraw_fee_denominator = 30;
        let host_fee_numerator = 20;
        let host_fee_denominator = 100;
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };

        let token_a_amount = 10_000_000_000;
        let token_b_amount = 50_000_000_000;

        check_valid_swap_curve(
            fees,
            SwapTransferFees {
                pool_token: TransferFee::default(),
                token_a: TransferFee {
                    epoch: 0.into(),
                    transfer_fee_basis_points: 100.into(),
                    maximum_fee: 1_000_000_000.into(),
                },
                token_b: TransferFee::default(),
            },
            CurveType::ConstantProduct,
            Arc::new(ConstantProductCurve {}),
            token_a_amount,
            token_b_amount,
            &pool_token_program_id,
            &token_a_program_id,
            &token_b_program_id,
        );
    }
}
