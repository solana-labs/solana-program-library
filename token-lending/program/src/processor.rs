//! Program state processor

use crate::state::TokenConverter;
use crate::{
    dex_market::{DexMarket, TradeSimulator, BASE_MINT_OFFSET, QUOTE_MINT_OFFSET},
    error::LendingError,
    instruction::{init_lending_market, AmountType, LendingInstruction},
    math::{Decimal, Rate, TryAdd, TryDiv, TryMul, TrySub, WAD},
    state::{
        BorrowResult, LendingMarket, LiquidateResult, NewObligationCollateralParams,
        NewObligationLiquidityParams, NewObligationParams, NewReserveParams, Obligation,
        ObligationCollateral, ObligationLiquidity, RepayResult, Reserve, ReserveCollateral,
        ReserveConfig, ReserveLiquidity, MAX_OBLIGATION_ACCOUNTS, PROGRAM_VERSION,
    },
};
use num_traits::FromPrimitive;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Slot,
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::{PrintProgramError, ProgramError},
    program_option::COption,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::{clock::Clock, rent::Rent, Sysvar},
};
use spl_token::state::Account;
use std::convert::TryFrom;

/// Processes an instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = LendingInstruction::unpack(input)?;
    match instruction {
        LendingInstruction::InitLendingMarket {
            owner,
            loan_to_value_ratio,
            liquidation_threshold,
        } => {
            msg!("Instruction: Init Lending Market");
            process_init_lending_market(
                program_id,
                owner,
                loan_to_value_ratio,
                liquidation_threshold,
                accounts,
            )
        }
        LendingInstruction::InitReserve {
            liquidity_amount,
            config,
        } => {
            msg!("Instruction: Init Reserve");
            process_init_reserve(program_id, liquidity_amount, config, accounts)
        }
        LendingInstruction::InitObligation => {
            msg!("Instruction: Init Obligation");
            process_init_obligation(program_id, accounts)
        }
        LendingInstruction::DepositReserveLiquidity { liquidity_amount } => {
            msg!("Instruction: Deposit");
            process_deposit_reserve_liquidity(program_id, liquidity_amount, accounts)
        }
        LendingInstruction::WithdrawReserveLiquidity { collateral_amount } => {
            msg!("Instruction: Withdraw");
            process_withdraw_reserve_liquidity(program_id, collateral_amount, accounts)
        }
        LendingInstruction::BorrowObligationLiquidity {
            liquidity_amount,
            liquidity_amount_type,
        } => {
            msg!("Instruction: Borrow");
            process_borrow_obligation_liquidity(
                program_id,
                liquidity_amount,
                liquidity_amount_type,
                accounts,
            )
        }
        LendingInstruction::RepayObligationLiquidity {
            liquidity_amount,
            liquidity_amount_type,
        } => {
            msg!("Instruction: Repay");
            process_repay_obligation_liquidity(
                program_id,
                liquidity_amount,
                liquidity_amount_type,
                accounts,
            )
        }
        LendingInstruction::LiquidateObligation {
            liquidity_amount,
            liquidity_amount_type,
        } => {
            msg!("Instruction: Liquidate");
            process_liquidate_obligation(
                program_id,
                liquidity_amount,
                liquidity_amount_type,
                accounts,
            )
        }
        LendingInstruction::AccrueReserveInterest => {
            msg!("Instruction: Accrue Interest");
            process_accrue_reserve_interest(program_id, accounts)
        }
        LendingInstruction::DepositObligationCollateral { collateral_amount } => {
            msg!("Instruction: Deposit Obligation Collateral");
            process_deposit_obligation_collateral(program_id, collateral_amount, accounts)
        }
        LendingInstruction::WithdrawObligationCollateral {
            collateral_amount,
            collateral_amount_type,
        } => {
            msg!("Instruction: Withdraw Obligation Collateral");
            process_withdraw_obligation_collateral(
                program_id,
                collateral_amount,
                collateral_amount_type,
                accounts,
            )
        }
        LendingInstruction::SetLendingMarketOwner { new_owner } => {
            msg!("Instruction: Set Lending Market Owner");
            process_set_lending_market_owner(program_id, new_owner, accounts)
        }
        LendingInstruction::InitObligationCollateral => {
            msg!("Instruction: Init Obligation Collateral");
            process_init_obligation_collateral(program_id, accounts)
        }
        LendingInstruction::InitObligationLiquidity => {
            msg!("Instruction: Init Obligation Liquidity");
            process_init_obligation_liquidity(program_id, accounts)
        }
        LendingInstruction::RefreshObligationCollateral => {
            msg!("Instruction: Refresh Obligation Collateral");
            process_refresh_obligation_collateral(program_id, accounts)
        }
        LendingInstruction::RefreshObligationLiquidity => {
            msg!("Instruction: Refresh Obligation Liquidity");
            process_refresh_obligation_liquidity(program_id, accounts)
        }
        LendingInstruction::RefreshObligation => {
            msg!("Instruction: Refresh Obligation");
            process_refresh_obligation(program_id, accounts)
        }
    }
}

fn process_init_lending_market(
    program_id: &Pubkey,
    lending_market_owner: Pubkey,
    loan_to_value_ratio: u8,
    liquidation_threshold: u8,
    accounts: &[AccountInfo],
) -> ProgramResult {
    if loan_to_value_ratio >= 100 {
        msg!("Loan to value ratio must be in range [0, 100)");
        return Err(LendingError::InvalidConfig.into());
    }
    if liquidation_threshold <= loan_to_value_ratio || liquidation_threshold > 100 {
        msg!("Liquidation threshold must be in range (LTV, 100]");
        return Err(LendingError::InvalidConfig.into());
    }

    let account_info_iter = &mut accounts.iter();
    let lending_market_info = next_account_info(account_info_iter)?;
    let quote_token_mint_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_id = next_account_info(account_info_iter)?;

    unpack_mint(&quote_token_mint_info.data.borrow())?;
    if quote_token_mint_info.owner != token_program_id.key {
        return Err(LendingError::InvalidTokenOwner.into());
    }

    assert_rent_exempt(rent, lending_market_info)?;
    assert_uninitialized(lending_market_info)?;

    let mut lending_market = LendingMarket {
        version: PROGRAM_VERSION,
        bump_seed: Pubkey::find_program_address(&[lending_market_info.key.as_ref()], program_id).1,
        owner: lending_market_owner,
        quote_token_mint: *quote_token_mint_info.key,
        token_program_id: *token_program_id.key,
        loan_to_value_ratio,
        liquidation_threshold,
    };
    LendingMarket::pack(lending_market, &mut lending_market_info.data.borrow_mut())?;

    Ok(())
}

fn process_init_reserve(
    program_id: &Pubkey,
    liquidity_amount: u64,
    config: ReserveConfig,
    accounts: &[AccountInfo],
) -> ProgramResult {
    if liquidity_amount == 0 {
        msg!("Reserve must be initialized with liquidity");
        return Err(LendingError::InvalidAmount.into());
    }
    if config.optimal_utilization_rate > 100 {
        msg!("Optimal utilization rate must be in range [0, 100]");
        return Err(LendingError::InvalidConfig.into());
    }
    if config.liquidation_bonus > 100 {
        msg!("Liquidation bonus must be in range [0, 100]");
        return Err(LendingError::InvalidConfig.into());
    }
    if config.optimal_borrow_rate < config.min_borrow_rate {
        msg!("Optimal borrow rate must be >= min borrow rate");
        return Err(LendingError::InvalidConfig.into());
    }
    if config.optimal_borrow_rate > config.max_borrow_rate {
        msg!("Optimal borrow rate must be <= max borrow rate");
        return Err(LendingError::InvalidConfig.into());
    }
    if config.fees.borrow_fee_wad >= WAD {
        msg!("Borrow fee must be in range [0, 1_000_000_000_000_000_000)");
        return Err(LendingError::InvalidConfig.into());
    }
    if config.fees.host_fee_percentage > 100 {
        msg!("Host fee percentage must be in range [0, 100]");
        return Err(LendingError::InvalidConfig.into());
    }

    let account_info_iter = &mut accounts.iter();
    let source_liquidity_info = next_account_info(account_info_iter)?;
    let destination_collateral_info = next_account_info(account_info_iter)?;
    let reserve_info = next_account_info(account_info_iter)?;
    let reserve_liquidity_mint_info = next_account_info(account_info_iter)?;
    let reserve_liquidity_supply_info = next_account_info(account_info_iter)?;
    let reserve_liquidity_fee_receiver_info = next_account_info(account_info_iter)?;
    let reserve_collateral_mint_info = next_account_info(account_info_iter)?;
    let reserve_collateral_supply_info = next_account_info(account_info_iter)?;
    let lending_market_info = next_account_info(account_info_iter)?;
    let lending_market_owner_info = next_account_info(account_info_iter)?;
    let lending_market_authority_info = next_account_info(account_info_iter)?;
    let user_transfer_authority_info = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
    let rent_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_info)?;
    let token_program_id = next_account_info(account_info_iter)?;

    if reserve_liquidity_supply_info.key == source_liquidity_info.key {
        msg!("Invalid source liquidity account");
        return Err(LendingError::InvalidAccountInput.into());
    }

    assert_rent_exempt(rent, reserve_info)?;
    assert_uninitialized::<Reserve>(reserve_info)?;

    let lending_market = LendingMarket::unpack(&lending_market_info.data.borrow())?;
    if lending_market_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &lending_market.token_program_id != token_program_id.key {
        return Err(LendingError::InvalidTokenProgram.into());
    }
    if &lending_market.owner != lending_market_owner_info.key {
        return Err(LendingError::InvalidMarketOwner.into());
    }
    if !lending_market_owner_info.is_signer {
        return Err(LendingError::InvalidSigner.into());
    }

    let dex_market = if reserve_liquidity_mint_info.key != &lending_market.quote_token_mint {
        let dex_market_info = next_account_info(account_info_iter)?;
        // TODO: check that market state is owned by real serum dex program
        if !rent.is_exempt(dex_market_info.lamports(), dex_market_info.data_len()) {
            return Err(LendingError::NotRentExempt.into());
        }

        let dex_market_data = &dex_market_info.data.borrow();
        let market_quote_mint = DexMarket::pubkey_at_offset(&dex_market_data, QUOTE_MINT_OFFSET);
        if lending_market.quote_token_mint != market_quote_mint {
            return Err(LendingError::DexMarketMintMismatch.into());
        }
        let market_base_mint = DexMarket::pubkey_at_offset(&dex_market_data, BASE_MINT_OFFSET);
        if reserve_liquidity_mint_info.key != &market_base_mint {
            return Err(LendingError::DexMarketMintMismatch.into());
        }

        COption::Some(*dex_market_info.key)
    } else {
        COption::None
    };

    let authority_signer_seeds = &[
        lending_market_info.key.as_ref(),
        &[lending_market.bump_seed],
    ];
    let lending_market_authority_pubkey =
        Pubkey::create_program_address(authority_signer_seeds, program_id)?;
    if lending_market_authority_info.key != &lending_market_authority_pubkey {
        return Err(LendingError::InvalidMarketAuthority.into());
    }

    let reserve_liquidity_mint = unpack_mint(&reserve_liquidity_mint_info.data.borrow())?;
    if reserve_liquidity_mint_info.owner != token_program_id.key {
        return Err(LendingError::InvalidTokenOwner.into());
    }

    let reserve_liquidity_info = ReserveLiquidity::new(
        *reserve_liquidity_mint_info.key,
        reserve_liquidity_mint.decimals,
        *reserve_liquidity_supply_info.key,
        *reserve_liquidity_fee_receiver_info.key,
    );
    let reserve_collateral_info = ReserveCollateral::new(
        *reserve_collateral_mint_info.key,
        *reserve_collateral_supply_info.key,
    );
    let mut reserve = Reserve::new(NewReserveParams {
        current_slot: clock.slot,
        lending_market: *lending_market_info.key,
        collateral: reserve_collateral_info,
        liquidity: reserve_liquidity_info,
        dex_market,
        config,
    });
    let collateral_amount = reserve.deposit_liquidity(liquidity_amount)?;
    Reserve::pack(reserve, &mut reserve_info.data.borrow_mut())?;

    spl_token_init_account(TokenInitializeAccountParams {
        account: reserve_liquidity_supply_info.clone(),
        mint: reserve_liquidity_mint_info.clone(),
        owner: lending_market_authority_info.clone(),
        rent: rent_info.clone(),
        token_program: token_program_id.clone(),
    })?;

    spl_token_init_mint(TokenInitializeMintParams {
        mint: reserve_collateral_mint_info.clone(),
        authority: lending_market_authority_info.key,
        rent: rent_info.clone(),
        decimals: reserve_liquidity_mint.decimals,
        token_program: token_program_id.clone(),
    })?;

    spl_token_init_account(TokenInitializeAccountParams {
        account: reserve_liquidity_fee_receiver_info.clone(),
        mint: reserve_liquidity_mint_info.clone(),
        owner: lending_market_owner_info.clone(),
        rent: rent_info.clone(),
        token_program: token_program_id.clone(),
    })?;

    spl_token_init_account(TokenInitializeAccountParams {
        account: reserve_collateral_supply_info.clone(),
        mint: reserve_collateral_mint_info.clone(),
        owner: lending_market_authority_info.clone(),
        rent: rent_info.clone(),
        token_program: token_program_id.clone(),
    })?;

    spl_token_init_account(TokenInitializeAccountParams {
        account: destination_collateral_info.clone(),
        mint: reserve_collateral_mint_info.clone(),
        owner: user_transfer_authority_info.clone(),
        rent: rent_info.clone(),
        token_program: token_program_id.clone(),
    })?;

    spl_token_transfer(TokenTransferParams {
        source: source_liquidity_info.clone(),
        destination: reserve_liquidity_supply_info.clone(),
        amount: liquidity_amount,
        authority: user_transfer_authority_info.clone(),
        authority_signer_seeds: &[],
        token_program: token_program_id.clone(),
    })?;

    spl_token_mint_to(TokenMintToParams {
        mint: reserve_collateral_mint_info.clone(),
        destination: destination_collateral_info.clone(),
        amount: collateral_amount,
        authority: lending_market_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_id.clone(),
    })?;

    Ok(())
}

#[inline(never)] // avoid stack frame limit
fn process_init_obligation(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let obligation_info = next_account_info(account_info_iter)?;
    let lending_market_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_id = next_account_info(account_info_iter)?;

    assert_rent_exempt(rent, obligation_info)?;
    assert_uninitialized::<Obligation>(obligation_info)?;

    let lending_market = LendingMarket::unpack(&lending_market_info.data.borrow())?;
    if lending_market_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &lending_market.token_program_id != token_program_id.key {
        return Err(LendingError::InvalidTokenProgram.into());
    }

    let obligation = Obligation::new(NewObligationParams {
        lending_market: *lending_market_info.key,
        collateral: vec![],
        liquidity: vec![],
    });
    Obligation::pack(obligation, &mut obligation_info.data.borrow_mut())?;

    Ok(())
}

fn process_deposit_reserve_liquidity(
    program_id: &Pubkey,
    liquidity_amount: u64,
    accounts: &[AccountInfo],
) -> ProgramResult {
    if liquidity_amount == 0 {
        return Err(LendingError::InvalidAmount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let source_liquidity_info = next_account_info(account_info_iter)?;
    let destination_collateral_info = next_account_info(account_info_iter)?;
    let reserve_info = next_account_info(account_info_iter)?;
    let reserve_liquidity_supply_info = next_account_info(account_info_iter)?;
    let reserve_collateral_mint_info = next_account_info(account_info_iter)?;
    let lending_market_info = next_account_info(account_info_iter)?;
    let lending_market_authority_info = next_account_info(account_info_iter)?;
    let user_transfer_authority_info = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_id = next_account_info(account_info_iter)?;

    let lending_market = LendingMarket::unpack(&lending_market_info.data.borrow())?;
    if lending_market_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &lending_market.token_program_id != token_program_id.key {
        return Err(LendingError::InvalidTokenProgram.into());
    }

    let mut reserve = Reserve::unpack(&reserve_info.data.borrow())?;
    if reserve_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &reserve.lending_market != lending_market_info.key {
        msg!("Invalid reserve lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &reserve.liquidity.supply_pubkey != reserve_liquidity_supply_info.key {
        msg!("Invalid reserve liquidity supply account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &reserve.collateral.mint_pubkey != reserve_collateral_mint_info.key {
        msg!("Invalid reserve collateral mint account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &reserve.liquidity.supply_pubkey == source_liquidity_info.key {
        msg!("Invalid source liquidity account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &reserve.collateral.supply_pubkey == destination_collateral_info.key {
        msg!("Invalid destination collateral account");
        return Err(LendingError::InvalidAccountInput.into());
    }

    let authority_signer_seeds = &[
        lending_market_info.key.as_ref(),
        &[lending_market.bump_seed],
    ];
    let lending_market_authority_pubkey =
        Pubkey::create_program_address(authority_signer_seeds, program_id)?;
    if lending_market_authority_info.key != &lending_market_authority_pubkey {
        return Err(LendingError::InvalidMarketAuthority.into());
    }

    assert_last_update_slot(&reserve, clock.slot)?;

    let collateral_amount = reserve.deposit_liquidity(liquidity_amount)?;
    Reserve::pack(reserve, &mut reserve_info.data.borrow_mut())?;

    spl_token_transfer(TokenTransferParams {
        source: source_liquidity_info.clone(),
        destination: reserve_liquidity_supply_info.clone(),
        amount: liquidity_amount,
        authority: user_transfer_authority_info.clone(),
        authority_signer_seeds: &[],
        token_program: token_program_id.clone(),
    })?;

    spl_token_mint_to(TokenMintToParams {
        mint: reserve_collateral_mint_info.clone(),
        destination: destination_collateral_info.clone(),
        amount: collateral_amount,
        authority: lending_market_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_id.clone(),
    })?;

    Ok(())
}

fn process_withdraw_reserve_liquidity(
    program_id: &Pubkey,
    collateral_amount: u64,
    accounts: &[AccountInfo],
) -> ProgramResult {
    if collateral_amount == 0 {
        return Err(LendingError::InvalidAmount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let source_collateral_info = next_account_info(account_info_iter)?;
    let destination_liquidity_info = next_account_info(account_info_iter)?;
    let reserve_info = next_account_info(account_info_iter)?;
    let reserve_collateral_mint_info = next_account_info(account_info_iter)?;
    let reserve_liquidity_supply_info = next_account_info(account_info_iter)?;
    let lending_market_info = next_account_info(account_info_iter)?;
    let lending_market_authority_info = next_account_info(account_info_iter)?;
    let user_transfer_authority_info = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_id = next_account_info(account_info_iter)?;

    let lending_market = LendingMarket::unpack(&lending_market_info.data.borrow())?;
    if lending_market_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &lending_market.token_program_id != token_program_id.key {
        return Err(LendingError::InvalidTokenProgram.into());
    }

    let mut reserve = Reserve::unpack(&reserve_info.data.borrow())?;
    if reserve_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &reserve.lending_market != lending_market_info.key {
        msg!("Invalid reserve lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &reserve.liquidity.supply_pubkey != reserve_liquidity_supply_info.key {
        msg!("Invalid reserve liquidity supply account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &reserve.collateral.mint_pubkey != reserve_collateral_mint_info.key {
        msg!("Invalid reserve collateral mint account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &reserve.liquidity.supply_pubkey == destination_liquidity_info.key {
        msg!("Invalid destination liquidity account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &reserve.collateral.supply_pubkey == source_collateral_info.key {
        msg!("Invalid source collateral account");
        return Err(LendingError::InvalidAccountInput.into());
    }

    let authority_signer_seeds = &[
        lending_market_info.key.as_ref(),
        &[lending_market.bump_seed],
    ];
    let lending_market_authority_pubkey =
        Pubkey::create_program_address(authority_signer_seeds, program_id)?;
    if lending_market_authority_info.key != &lending_market_authority_pubkey {
        return Err(LendingError::InvalidMarketAuthority.into());
    }

    assert_last_update_slot(&reserve, clock.slot)?;

    let liquidity_amount = reserve.redeem_collateral(collateral_amount)?;
    Reserve::pack(reserve, &mut reserve_info.data.borrow_mut())?;

    spl_token_burn(TokenBurnParams {
        mint: reserve_collateral_mint_info.clone(),
        source: source_collateral_info.clone(),
        amount: collateral_amount,
        authority: user_transfer_authority_info.clone(),
        authority_signer_seeds: &[],
        token_program: token_program_id.clone(),
    })?;

    spl_token_transfer(TokenTransferParams {
        source: reserve_liquidity_supply_info.clone(),
        destination: destination_liquidity_info.clone(),
        amount: liquidity_amount,
        authority: lending_market_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_id.clone(),
    })?;

    Ok(())
}

#[inline(never)] // avoid stack frame limit
fn process_borrow_obligation_liquidity(
    program_id: &Pubkey,
    liquidity_amount: u64,
    liquidity_amount_type: AmountType,
    accounts: &[AccountInfo],
) -> ProgramResult {
    if liquidity_amount == 0 {
        return Err(LendingError::InvalidAmount.into());
    }
    if let AmountType::PercentAmount = liquidity_amount_type {
        if liquidity_amount > 100 {
            msg!("Liquidity amount must be in range (0, 100]");
            return Err(LendingError::InvalidAmount.into());
        }
    }

    let account_info_iter = &mut accounts.iter();
    let source_liquidity_info = next_account_info(account_info_iter)?;
    let destination_liquidity_info = next_account_info(account_info_iter)?;
    let borrow_reserve_info = next_account_info(account_info_iter)?;
    let borrow_reserve_liquidity_fee_receiver_info = next_account_info(account_info_iter)?;
    let obligation_info = next_account_info(account_info_iter)?;
    let obligation_liquidity_info = next_account_info(account_info_iter)?;
    let lending_market_info = next_account_info(account_info_iter)?;
    let lending_market_authority_info = next_account_info(account_info_iter)?;
    let dex_market_info = next_account_info(account_info_iter)?;
    let dex_market_orders_info = next_account_info(account_info_iter)?;
    let memory = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_id = next_account_info(account_info_iter)?;

    // Ensure memory is owned by this program so that we don't have to zero it out
    if memory.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }

    let lending_market = LendingMarket::unpack(&lending_market_info.data.borrow())?;
    if lending_market_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &lending_market.token_program_id != token_program_id.key {
        return Err(LendingError::InvalidTokenProgram.into());
    }

    let mut borrow_reserve = Reserve::unpack(&borrow_reserve_info.data.borrow())?;
    if borrow_reserve_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &borrow_reserve.lending_market != lending_market_info.key {
        return Err(LendingError::LendingMarketMismatch.into());
    }
    if &borrow_reserve.liquidity.supply_pubkey != source_liquidity_info.key {
        msg!("Invalid source liquidity account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &borrow_reserve.liquidity.supply_pubkey == destination_liquidity_info.key {
        msg!("Invalid destination liquidity account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &borrow_reserve.liquidity.fee_receiver != borrow_reserve_liquidity_fee_receiver_info.key {
        msg!("Invalid borrow reserve liquidity fee receiver account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if borrow_reserve.dex_market.is_none() {
        msg!("Borrow reserve must have a dex market");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if let COption::Some(dex_market_pubkey) = borrow_reserve.dex_market {
        if &dex_market_pubkey != dex_market_info.key {
            msg!("Invalid dex market account");
            return Err(LendingError::InvalidAccountInput.into());
        }
    }

    let mut obligation = Obligation::unpack(&obligation_info.data.borrow())?;
    if obligation_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation.lending_market != lending_market_info.key {
        msg!("Invalid obligation lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if obligation.is_stale(clock.slot)? {
        return Err(LendingError::ObligationStale.into());
    }
    // @TODO: is this enough? other reserves could have been updated that we don't check here, and
    //          they all affect the market value. need to think about when interest may be accrued
    if obligation.last_update_slot < borrow_reserve.last_update_slot {
        return Err(LendingError::ObligationStale.into());
    }

    let mut obligation_liquidity =
        ObligationLiquidity::unpack(&obligation_liquidity_info.data.borrow())?;
    if obligation_liquidity_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation_liquidity.obligation != obligation_info.key {
        msg!("Invalid obligation account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &obligation_liquidity.borrow_reserve != borrow_reserve_info.key {
        msg!("Invalid borrow reserve account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if !obligation.liquidity.contains(obligation_liquidity_info.key) {
        return Err(LendingError::ObligationAccountNotFound.into());
    }
    // @TODO: is this enough? other collateral/liquidity could have been updated that we don't
    //          check here. we could mark the obligation stale on every refresh of
    //          collateral/liquidity, but this means they can't be refreshed in parallel
    if obligation.last_update_slot < obligation_liquidity.last_update_slot {
        return Err(LendingError::ObligationStale.into());
    }
    // @TODO: is this necessary if checking obligation.last_update_slot < obligation_liquidity.last_update_slot above?
    if obligation_liquidity.is_stale(clock.slot)? {
        return Err(LendingError::ObligationLiquidityStale.into());
    }

    let authority_signer_seeds = &[
        lending_market_info.key.as_ref(),
        &[lending_market.bump_seed],
    ];
    let lending_market_authority_pubkey =
        Pubkey::create_program_address(authority_signer_seeds, program_id)?;
    if lending_market_authority_info.key != &lending_market_authority_pubkey {
        return Err(LendingError::InvalidMarketAuthority.into());
    }

    // @TODO: is this necessary?
    assert_last_update_slot(&borrow_reserve, clock.slot)?;

    // @TODO: is this necessary?
    obligation_liquidity.accrue_interest(borrow_reserve.cumulative_borrow_rate_wads)?;

    let lending_market_ltv = Rate::from_percent(lending_market.loan_to_value_ratio);
    let obligation_ltv = obligation.loan_to_value()?;
    if obligation_ltv > lending_market_ltv {
        return Err(LendingError::ObligationLTVAboveReserveLTV.into());
    }
    if obligation_ltv == lending_market_ltv {
        return Err(LendingError::ObligationLTVCannotGoAboveReserveLTV.into());
    }

    let trade_simulator = TradeSimulator::new(
        dex_market_info,
        dex_market_orders_info,
        memory,
        &lending_market.quote_token_mint,
        // @TODO: check these
        &borrow_reserve.liquidity.mint_pubkey,
        &lending_market.quote_token_mint,
    )?;

    let max_borrow_value = obligation
        .collateral_value
        .try_mul(lending_market_ltv)?
        .try_sub(obligation.liquidity_value)?;

    let BorrowResult {
        total_amount,
        borrow_amount,
        origination_fee,
        host_fee,
    } = borrow_reserve.borrow(
        liquidity_amount,
        liquidity_amount_type,
        max_borrow_value,
        trade_simulator,
        &lending_market.quote_token_mint,
    )?;

    // @TODO: will this need further adjustment for fees?
    borrow_reserve
        .liquidity
        .borrow(total_amount, borrow_amount)?;
    // @TODO: will this need further adjustment for fees?
    obligation_liquidity.borrow(borrow_amount);
    obligation_liquidity.mark_stale();
    obligation.mark_stale();

    ObligationLiquidity::pack(
        obligation_liquidity,
        &mut obligation_liquidity_info.data.borrow_mut(),
    )?;
    Obligation::pack(obligation, &mut obligation_info.data.borrow_mut())?;
    Reserve::pack(borrow_reserve, &mut borrow_reserve_info.data.borrow_mut())?;

    let mut owner_fee = origination_fee;
    if let Ok(host_fee_receiver_info) = next_account_info(account_info_iter) {
        if host_fee > 0 {
            owner_fee = owner_fee
                .checked_sub(host_fee)
                .ok_or(LendingError::MathOverflow)?;

            spl_token_transfer(TokenTransferParams {
                source: source_liquidity_info.clone(),
                destination: host_fee_receiver_info.clone(),
                amount: host_fee,
                authority: lending_market_authority_info.clone(),
                authority_signer_seeds,
                token_program: token_program_id.clone(),
            })?;
        }
    }
    if owner_fee > 0 {
        spl_token_transfer(TokenTransferParams {
            source: source_liquidity_info.clone(),
            destination: borrow_reserve_liquidity_fee_receiver_info.clone(),
            amount: owner_fee,
            authority: lending_market_authority_info.clone(),
            authority_signer_seeds,
            token_program: token_program_id.clone(),
        })?;
    }

    spl_token_transfer(TokenTransferParams {
        source: source_liquidity_info.clone(),
        destination: destination_liquidity_info.clone(),
        amount: borrow_amount,
        authority: lending_market_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_id.clone(),
    })?;

    Ok(())
}

#[inline(never)] // avoid stack frame limit
fn process_repay_obligation_liquidity(
    program_id: &Pubkey,
    liquidity_amount: u64,
    liquidity_amount_type: AmountType,
    accounts: &[AccountInfo],
) -> ProgramResult {
    if liquidity_amount == 0 {
        return Err(LendingError::InvalidAmount.into());
    }
    if let AmountType::PercentAmount = liquidity_amount_type {
        if liquidity_amount > 100 {
            msg!("Liquidity amount must be in range (0, 100]");
            return Err(LendingError::InvalidAmount.into());
        }
    }

    let account_info_iter = &mut accounts.iter();
    let source_liquidity_info = next_account_info(account_info_iter)?;
    let destination_liquidity_info = next_account_info(account_info_iter)?;
    let repay_reserve_info = next_account_info(account_info_iter)?;
    let repay_reserve_liquidity_supply_info = next_account_info(account_info_iter)?;
    let obligation_info = next_account_info(account_info_iter)?;
    let obligation_liquidity_info = next_account_info(account_info_iter)?;
    let lending_market_info = next_account_info(account_info_iter)?;
    let lending_market_authority_info = next_account_info(account_info_iter)?;
    let user_transfer_authority_info = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_id = next_account_info(account_info_iter)?;

    let lending_market = LendingMarket::unpack(&lending_market_info.data.borrow())?;
    if lending_market_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &lending_market.token_program_id != token_program_id.key {
        return Err(LendingError::InvalidTokenProgram.into());
    }

    let mut repay_reserve = Reserve::unpack(&repay_reserve_info.data.borrow())?;
    if repay_reserve_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &repay_reserve.lending_market != lending_market_info.key {
        msg!("Invalid reserve lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    // @TODO: how is the currency/mint of the liquidity known?
    if &repay_reserve.liquidity.supply_pubkey == source_liquidity_info.key {
        msg!("Invalid source liquidity account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &repay_reserve.liquidity.supply_pubkey != destination_liquidity_info.key {
        msg!("Invalid destination liquidity account");
        return Err(LendingError::InvalidAccountInput.into());
    }

    let mut obligation = Obligation::unpack(&obligation_info.data.borrow())?;
    if obligation_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation.lending_market != lending_market_info.key {
        msg!("Invalid obligation lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if obligation.is_stale(clock.slot)? {
        return Err(LendingError::ObligationStale.into());
    }
    // @TODO: is this enough? other reserves could have been updated that we don't check here, and
    //          they all affect the market value. need to think about when interest may be accrued
    if obligation.last_update_slot < repay_reserve.last_update_slot {
        return Err(LendingError::ObligationStale.into());
    }

    let mut obligation_liquidity =
        ObligationLiquidity::unpack(&obligation_liquidity_info.data.borrow())?;
    if obligation_liquidity_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation_liquidity.obligation != obligation_info.key {
        msg!("Invalid obligation account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &obligation_liquidity.borrow_reserve != repay_reserve_info.key {
        msg!("Invalid repay reserve account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if !obligation.liquidity.contains(obligation_liquidity_info.key) {
        return Err(LendingError::ObligationAccountNotFound.into());
    }
    // @TODO: is this enough? other collateral/liquidity could have been updated that we don't
    //          check here. we could mark the obligation stale on every refresh of
    //          collateral/liquidity, but this means they can't be refreshed in parallel
    if obligation.last_update_slot < obligation_liquidity.last_update_slot {
        return Err(LendingError::ObligationStale.into());
    }
    // @TODO: is this necessary if checking obligation.last_update_slot < obligation_liquidity.last_update_slot above?
    if obligation_liquidity.is_stale(clock.slot)? {
        return Err(LendingError::ObligationLiquidityStale.into());
    }

    let authority_signer_seeds = &[
        lending_market_info.key.as_ref(),
        &[lending_market.bump_seed],
    ];
    let lending_market_authority_pubkey =
        Pubkey::create_program_address(authority_signer_seeds, program_id)?;
    if lending_market_authority_info.key != &lending_market_authority_pubkey {
        return Err(LendingError::InvalidMarketAuthority.into());
    }

    // @TODO: is this necessary?
    assert_last_update_slot(&repay_reserve, clock.slot)?;

    // @TODO: is this necessary?
    obligation_liquidity.accrue_interest(repay_reserve.cumulative_borrow_rate_wads)?;

    let settle_amount = match liquidity_amount_type {
        AmountType::ExactAmount => {
            Decimal::from(liquidity_amount).min(obligation_liquidity.borrowed_wads)
        }
        AmountType::PercentAmount => Decimal::from_percent(u8::try_from(liquidity_amount)?)
            .try_mul(obligation_liquidity.borrowed_wads)?,
    };
    let repay_amount = settle_amount.try_floor_u64()?;

    repay_reserve.liquidity.repay(repay_amount, settle_amount)?;
    obligation_liquidity.repay(settle_amount);
    obligation_liquidity.mark_stale();
    obligation.mark_stale();

    ObligationLiquidity::pack(
        obligation_liquidity,
        &mut obligation_liquidity_info.data.borrow_mut(),
    )?;
    Obligation::pack(obligation, &mut obligation_info.data.borrow_mut())?;
    Reserve::pack(repay_reserve, &mut repay_reserve_info.data.borrow_mut())?;

    spl_token_transfer(TokenTransferParams {
        source: source_liquidity_info.clone(),
        destination: repay_reserve_liquidity_supply_info.clone(),
        amount: repay_amount,
        authority: user_transfer_authority_info.clone(),
        authority_signer_seeds: &[],
        token_program: token_program_id.clone(),
    })?;

    Ok(())
}

// @FIXME
#[inline(never)] // avoid stack frame limit
fn process_liquidate_obligation(
    program_id: &Pubkey,
    liquidity_amount: u64,
    liquidity_amount_type: AmountType,
    accounts: &[AccountInfo],
) -> ProgramResult {
    if liquidity_amount == 0 {
        return Err(LendingError::InvalidAmount.into());
    }
    if let AmountType::PercentAmount = liquidity_amount_type {
        if liquidity_amount > 100 {
            msg!("Liquidity amount must be in range (0, 100]");
            return Err(LendingError::InvalidAmount.into());
        }
    }

    let account_info_iter = &mut accounts.iter();
    let source_liquidity_info = next_account_info(account_info_iter)?;
    let destination_collateral_info = next_account_info(account_info_iter)?;
    let repay_reserve_info = next_account_info(account_info_iter)?;
    let repay_reserve_liquidity_supply_info = next_account_info(account_info_iter)?;
    let withdraw_reserve_info = next_account_info(account_info_iter)?;
    let withdraw_reserve_collateral_supply_info = next_account_info(account_info_iter)?;
    let obligation_info = next_account_info(account_info_iter)?;
    let lending_market_info = next_account_info(account_info_iter)?;
    let lending_market_authority_info = next_account_info(account_info_iter)?;
    let user_transfer_authority_info = next_account_info(account_info_iter)?;
    let dex_market_info = next_account_info(account_info_iter)?;
    let dex_market_orders_info = next_account_info(account_info_iter)?;
    let memory = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_id = next_account_info(account_info_iter)?;

    // Ensure memory is owned by this program so that we don't have to zero it out
    if memory.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }

    let lending_market = LendingMarket::unpack(&lending_market_info.data.borrow())?;
    if lending_market_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &lending_market.token_program_id != token_program_id.key {
        return Err(LendingError::InvalidTokenProgram.into());
    }

    let mut obligation = Obligation::unpack(&obligation_info.data.borrow())?;
    if obligation_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation.borrow_reserve != repay_reserve_info.key {
        msg!("Invalid repay reserve account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &obligation.collateral_reserve != withdraw_reserve_info.key {
        msg!("Invalid withdraw reserve account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if obligation.deposited_collateral_tokens == 0 {
        return Err(LendingError::ObligationEmpty.into());
    }

    let mut repay_reserve = Reserve::unpack(&repay_reserve_info.data.borrow())?;
    if repay_reserve_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &repay_reserve.lending_market != lending_market_info.key {
        msg!("Invalid reserve lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }

    let withdraw_reserve = Reserve::unpack(&withdraw_reserve_info.data.borrow())?;
    if withdraw_reserve_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if withdraw_reserve.lending_market != repay_reserve.lending_market {
        return Err(LendingError::LendingMarketMismatch.into());
    }

    if repay_reserve_info.key == withdraw_reserve_info.key {
        return Err(LendingError::DuplicateReserve.into());
    }
    if repay_reserve.liquidity.mint_pubkey == withdraw_reserve.liquidity.mint_pubkey {
        return Err(LendingError::DuplicateReserveMint.into());
    }
    if &repay_reserve.liquidity.supply_pubkey != repay_reserve_liquidity_supply_info.key {
        msg!("Invalid repay reserve liquidity supply account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &withdraw_reserve.collateral.supply_pubkey != withdraw_reserve_collateral_supply_info.key {
        msg!("Invalid withdraw reserve collateral supply account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &repay_reserve.liquidity.supply_pubkey == source_liquidity_info.key {
        msg!("Cannot use repay reserve liquidity supply as source account input");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &withdraw_reserve.collateral.supply_pubkey == destination_collateral_info.key {
        msg!("Cannot use withdraw reserve collateral supply as destination account input");
        return Err(LendingError::InvalidAccountInput.into());
    }

    // TODO: handle case when neither reserve is the quote currency
    if repay_reserve.dex_market.is_none() && withdraw_reserve.dex_market.is_none() {
        msg!("One reserve must have a dex market");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if let COption::Some(dex_market_pubkey) = repay_reserve.dex_market {
        if &dex_market_pubkey != dex_market_info.key {
            msg!("Invalid dex market account");
            return Err(LendingError::InvalidAccountInput.into());
        }
    }
    if let COption::Some(dex_market_pubkey) = withdraw_reserve.dex_market {
        if &dex_market_pubkey != dex_market_info.key {
            msg!("Invalid dex market account");
            return Err(LendingError::InvalidAccountInput.into());
        }
    }

    let authority_signer_seeds = &[
        lending_market_info.key.as_ref(),
        &[lending_market.bump_seed],
    ];
    let lending_market_authority_pubkey =
        Pubkey::create_program_address(authority_signer_seeds, program_id)?;
    if lending_market_authority_info.key != &lending_market_authority_pubkey {
        return Err(LendingError::InvalidMarketAuthority.into());
    }

    // accrue interest and update rates
    assert_last_update_slot(&repay_reserve, clock.slot)?;
    assert_last_update_slot(&withdraw_reserve, clock.slot)?;

    obligation.accrue_interest(repay_reserve.cumulative_borrow_rate_wads)?;

    let trade_simulator = TradeSimulator::new(
        dex_market_info,
        dex_market_orders_info,
        memory,
        &lending_market.quote_token_mint,
        &withdraw_reserve.liquidity.mint_pubkey,
        &repay_reserve.liquidity.mint_pubkey,
    )?;

    // @FIXME: use liquidity_amount_type

    let LiquidateResult {
        withdraw_amount,
        repay_amount,
        settle_amount,
    } = withdraw_reserve.liquidate_obligation(
        &obligation,
        liquidity_amount,
        &repay_reserve.liquidity.mint_pubkey,
        trade_simulator,
    )?;

    repay_reserve.liquidity.repay(repay_amount, settle_amount)?;
    Reserve::pack(repay_reserve, &mut repay_reserve_info.data.borrow_mut())?;

    obligation.liquidate(settle_amount, withdraw_amount)?;
    Obligation::pack(obligation, &mut obligation_info.data.borrow_mut())?;

    // deposit repaid liquidity
    spl_token_transfer(TokenTransferParams {
        source: source_liquidity_info.clone(),
        destination: repay_reserve_liquidity_supply_info.clone(),
        amount: repay_amount,
        authority: user_transfer_authority_info.clone(),
        authority_signer_seeds: &[],
        token_program: token_program_id.clone(),
    })?;

    // withdraw collateral
    spl_token_transfer(TokenTransferParams {
        source: withdraw_reserve_collateral_supply_info.clone(),
        destination: destination_collateral_info.clone(),
        amount: withdraw_amount,
        authority: lending_market_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_id.clone(),
    })?;

    Ok(())
}

#[inline(never)] // avoid stack frame limit
fn process_accrue_reserve_interest(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
    for reserve_info in account_info_iter {
        let mut reserve = Reserve::unpack(&reserve_info.data.borrow())?;
        if reserve_info.owner != program_id {
            return Err(LendingError::InvalidAccountOwner.into());
        }

        reserve.accrue_interest(clock.slot)?;
        Reserve::pack(reserve, &mut reserve_info.data.borrow_mut())?;
    }

    Ok(())
}

#[inline(never)] // avoid stack frame limit
fn process_deposit_obligation_collateral(
    program_id: &Pubkey,
    collateral_amount: u64,
    accounts: &[AccountInfo],
) -> ProgramResult {
    if collateral_amount == 0 {
        return Err(LendingError::InvalidAmount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let source_collateral_info = next_account_info(account_info_iter)?;
    let destination_collateral_info = next_account_info(account_info_iter)?;
    let deposit_reserve_info = next_account_info(account_info_iter)?;
    let obligation_info = next_account_info(account_info_iter)?;
    let obligation_collateral_info = next_account_info(account_info_iter)?;
    let obligation_token_mint_info = next_account_info(account_info_iter)?;
    let obligation_token_output_info = next_account_info(account_info_iter)?;
    let lending_market_info = next_account_info(account_info_iter)?;
    let lending_market_authority_info = next_account_info(account_info_iter)?;
    let user_transfer_authority_info = next_account_info(account_info_iter)?;
    let token_program_id = next_account_info(account_info_iter)?;

    let lending_market = LendingMarket::unpack(&lending_market_info.data.borrow())?;
    if lending_market_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &lending_market.token_program_id != token_program_id.key {
        return Err(LendingError::InvalidTokenProgram.into());
    }

    let deposit_reserve = Reserve::unpack(&deposit_reserve_info.data.borrow())?;
    if deposit_reserve_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &deposit_reserve.lending_market != lending_market_info.key {
        msg!("Invalid reserve lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    // @FIXME: moved to lending market; does per-reserve collateral enable still make sense?
    if deposit_reserve.config.loan_to_value_ratio == 0 {
        return Err(LendingError::ReserveCollateralDisabled.into());
    }
    if &deposit_reserve.collateral.supply_pubkey != destination_collateral_info.key {
        msg!("Invalid destination collateral account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &deposit_reserve.collateral.supply_pubkey == source_collateral_info.key {
        msg!("Invalid source collateral account");
        return Err(LendingError::InvalidAccountInput.into());
    }

    let mut obligation = Obligation::unpack(&obligation_info.data.borrow())?;
    if obligation_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation.lending_market != lending_market_info.key {
        msg!("Invalid obligation lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }

    let mut obligation_collateral =
        ObligationCollateral::unpack(&obligation_collateral_info.data.borrow())?;
    if obligation_collateral_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation_collateral.obligation != obligation_info.key {
        msg!("Invalid obligation account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &obligation_collateral.deposit_reserve != deposit_reserve_info.key {
        msg!("Invalid deposit reserve account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &obligation_collateral.token_mint != obligation_token_mint_info.key {
        msg!("Invalid obligation token mint");
        return Err(LendingError::InvalidTokenMint.into());
    }
    if !obligation
        .collateral
        .contains(obligation_collateral_info.key)
    {
        return Err(LendingError::ObligationAccountNotFound.into());
    }

    unpack_mint(&obligation_token_mint_info.data.borrow())?;
    if obligation_token_mint_info.owner != token_program_id.key {
        return Err(LendingError::InvalidTokenOwner.into());
    }

    let obligation_token_output = Account::unpack(&obligation_token_output_info.data.borrow())?;
    if obligation_token_output_info.owner != token_program_id.key {
        return Err(LendingError::InvalidTokenOwner.into());
    }
    if &obligation_token_output.mint != obligation_token_mint_info.key {
        return Err(LendingError::InvalidTokenMint.into());
    }

    let authority_signer_seeds = &[
        lending_market_info.key.as_ref(),
        &[lending_market.bump_seed],
    ];
    let lending_market_authority_pubkey =
        Pubkey::create_program_address(authority_signer_seeds, program_id)?;
    if lending_market_authority_info.key != &lending_market_authority_pubkey {
        return Err(LendingError::InvalidMarketAuthority.into());
    }

    obligation_collateral.deposit(collateral_amount)?;
    obligation_collateral.mark_stale();
    obligation.mark_stale();

    ObligationCollateral::pack(
        obligation_collateral,
        &mut obligation_collateral_info.data.borrow_mut(),
    )?;
    Obligation::pack(obligation, &mut obligation_info.data.borrow_mut())?;

    spl_token_transfer(TokenTransferParams {
        source: source_collateral_info.clone(),
        destination: destination_collateral_info.clone(),
        amount: collateral_amount,
        authority: user_transfer_authority_info.clone(),
        authority_signer_seeds: &[],
        token_program: token_program_id.clone(),
    })?;

    spl_token_mint_to(TokenMintToParams {
        mint: obligation_token_mint_info.clone(),
        destination: obligation_token_output_info.clone(),
        amount: collateral_amount,
        authority: lending_market_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_id.clone(),
    })?;

    Ok(())
}

#[inline(never)] // avoid stack frame limit
fn process_withdraw_obligation_collateral(
    program_id: &Pubkey,
    collateral_amount: u64,
    collateral_amount_type: AmountType,
    accounts: &[AccountInfo],
) -> ProgramResult {
    if collateral_amount == 0 {
        return Err(LendingError::InvalidAmount.into());
    }
    if let AmountType::PercentAmount = collateral_amount_type {
        if collateral_amount > 100 {
            msg!("Collateral amount must be in range (0, 100]");
            return Err(LendingError::InvalidAmount.into());
        }
    }

    let account_info_iter = &mut accounts.iter();
    let source_collateral_info = next_account_info(account_info_iter)?;
    let destination_collateral_info = next_account_info(account_info_iter)?;
    let withdraw_reserve_info = next_account_info(account_info_iter)?;
    let obligation_info = next_account_info(account_info_iter)?;
    let obligation_collateral_info = next_account_info(account_info_iter)?;
    let obligation_token_mint_info = next_account_info(account_info_iter)?;
    let obligation_token_input_info = next_account_info(account_info_iter)?;
    let lending_market_info = next_account_info(account_info_iter)?;
    let lending_market_authority_info = next_account_info(account_info_iter)?;
    let user_transfer_authority_info = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_id = next_account_info(account_info_iter)?;

    let lending_market = LendingMarket::unpack(&lending_market_info.data.borrow())?;
    if lending_market_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &lending_market.token_program_id != token_program_id.key {
        return Err(LendingError::InvalidTokenProgram.into());
    }

    let withdraw_reserve = Reserve::unpack(&withdraw_reserve_info.data.borrow())?;
    if withdraw_reserve_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &withdraw_reserve.lending_market != lending_market_info.key {
        msg!("Invalid reserve lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &withdraw_reserve.collateral.supply_pubkey != source_collateral_info.key {
        msg!("Invalid source collateral account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &withdraw_reserve.collateral.supply_pubkey == destination_collateral_info.key {
        msg!("Invalid destination collateral account");
        return Err(LendingError::InvalidAccountInput.into());
    }

    let mut obligation = Obligation::unpack(&obligation_info.data.borrow())?;
    if obligation_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation.lending_market != lending_market_info.key {
        msg!("Invalid obligation lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if obligation.is_stale(clock.slot)? {
        return Err(LendingError::ObligationStale.into());
    }
    // @TODO: is this enough? other reserves could have been updated that we don't check here, and
    //          they all affect the market value. need to think about when interest may be accrued
    if obligation.last_update_slot < withdraw_reserve.last_update_slot {
        return Err(LendingError::ObligationStale.into());
    }

    let mut obligation_collateral =
        ObligationCollateral::unpack(&obligation_collateral_info.data.borrow())?;
    if obligation_collateral_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation_collateral.obligation != obligation_info.key {
        msg!("Invalid obligation account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &obligation_collateral.deposit_reserve != withdraw_reserve_info.key {
        msg!("Invalid withdraw reserve account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &obligation_collateral.token_mint != obligation_token_mint_info.key {
        msg!("Invalid obligation token mint");
        return Err(LendingError::InvalidTokenMint.into());
    }
    if !obligation
        .collateral
        .contains(obligation_collateral_info.key)
    {
        return Err(LendingError::ObligationAccountNotFound.into());
    }
    // @TODO: is this enough? other collateral/liquidity could have been updated that we don't
    //          check here. we could mark the obligation stale on every refresh of
    //          collateral/liquidity, but this means they can't be refreshed in parallel
    if obligation.last_update_slot < obligation_collateral.last_update_slot {
        return Err(LendingError::ObligationCollateralStale.into());
    }
    // @TODO: is this necessary if checking obligation.last_update_slot < obligation_liquidity.last_update_slot above?
    if obligation_collateral.is_stale(clock.slot)? {
        return Err(LendingError::ObligationCollateralStale.into());
    }
    if obligation_collateral.deposited_tokens == 0 {
        return Err(LendingError::ObligationEmpty.into());
    }

    let obligation_token_mint = unpack_mint(&obligation_token_mint_info.data.borrow())?;
    if obligation_token_mint_info.owner != token_program_id.key {
        return Err(LendingError::InvalidTokenOwner.into());
    }

    let obligation_token_input = Account::unpack(&obligation_token_input_info.data.borrow())?;
    if obligation_token_input_info.owner != token_program_id.key {
        return Err(LendingError::InvalidTokenOwner.into());
    }
    if &obligation_token_input.mint != obligation_token_mint_info.key {
        return Err(LendingError::InvalidTokenMint.into());
    }

    let authority_signer_seeds = &[
        lending_market_info.key.as_ref(),
        &[lending_market.bump_seed],
    ];
    let lending_market_authority_pubkey =
        Pubkey::create_program_address(authority_signer_seeds, program_id)?;
    if lending_market_authority_info.key != &lending_market_authority_pubkey {
        return Err(LendingError::InvalidMarketAuthority.into());
    }

    let lending_market_ltv = Rate::from_percent(lending_market.loan_to_value_ratio);
    let obligation_ltv = obligation.loan_to_value()?;
    if obligation_ltv > lending_market_ltv {
        return Err(LendingError::ObligationLTVAboveReserveLTV.into());
    }
    if obligation_ltv == lending_market_ltv {
        return Err(LendingError::ObligationLTVCannotGoAboveReserveLTV.into());
    }

    let min_collateral_value = obligation.liquidity_value.try_div(lending_market_ltv)?;
    let max_withdraw_value = obligation.collateral_value.try_sub(min_collateral_value)?;

    let withdraw_amount = match collateral_amount_type {
        AmountType::ExactAmount => {
            let withdraw_amount = collateral_amount.min(obligation_collateral.deposited_tokens);
            let withdraw_pct =
                Decimal::from(withdraw_amount).try_div(obligation_collateral.deposited_tokens)?;
            let withdraw_value = obligation.collateral_value.try_mul(withdraw_pct)?;
            if withdraw_value > max_withdraw_value {
                return Err(LendingError::ObligationCollateralWithdrawBelowRequired.into());
            }

            withdraw_amount
        }
        AmountType::PercentAmount => {
            let withdraw_pct = Decimal::from_percent(u8::try_from(collateral_amount)?);
            let withdraw_value = max_withdraw_value
                .try_mul(withdraw_pct)?
                .min(obligation_collateral.value);
            let withdraw_amount = withdraw_value
                .try_div(obligation_collateral.value)?
                .try_mul(obligation_collateral.deposited_tokens)?
                .try_floor_u64()?;

            withdraw_amount
        }
    };

    let obligation_token_amount = obligation_collateral
        .collateral_to_obligation_token_amount(withdraw_amount, obligation_token_mint.supply)?;

    obligation_collateral.withdraw(withdraw_amount)?;
    obligation_collateral.mark_stale();
    obligation.mark_stale();

    ObligationCollateral::pack(
        obligation_collateral,
        &mut obligation_collateral_info.data.borrow_mut(),
    )?;
    Obligation::pack(obligation, &mut obligation_info.data.borrow_mut())?;

    spl_token_burn(TokenBurnParams {
        mint: obligation_token_mint_info.clone(),
        source: obligation_token_input_info.clone(),
        amount: obligation_token_amount,
        authority: user_transfer_authority_info.clone(),
        authority_signer_seeds: &[],
        token_program: token_program_id.clone(),
    })?;

    spl_token_transfer(TokenTransferParams {
        source: source_collateral_info.clone(),
        destination: destination_collateral_info.clone(),
        amount: withdraw_amount,
        authority: lending_market_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_id.clone(),
    })?;

    Ok(())
}

#[inline(never)] // avoid stack frame limit
fn process_set_lending_market_owner(
    program_id: &Pubkey,
    new_owner: Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let lending_market_info = next_account_info(account_info_iter)?;
    let lending_market_owner_info = next_account_info(account_info_iter)?;

    let mut lending_market = LendingMarket::unpack(&lending_market_info.data.borrow())?;
    if lending_market_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &lending_market.owner != lending_market_owner_info.key {
        return Err(LendingError::InvalidMarketOwner.into());
    }
    if !lending_market_owner_info.is_signer {
        return Err(LendingError::InvalidSigner.into());
    }

    lending_market.owner = new_owner;
    LendingMarket::pack(lending_market, &mut lending_market_info.data.borrow_mut())?;

    Ok(())
}

#[inline(never)] // avoid stack frame limit
fn process_init_obligation_collateral(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let obligation_info = next_account_info(account_info_iter)?;
    let obligation_collateral_info = next_account_info(account_info_iter)?;
    let deposit_reserve_info = next_account_info(account_info_iter)?;
    let obligation_token_mint_info = next_account_info(account_info_iter)?;
    let obligation_token_output_info = next_account_info(account_info_iter)?;
    let obligation_token_owner_info = next_account_info(account_info_iter)?;
    let lending_market_info = next_account_info(account_info_iter)?;
    let lending_market_authority_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_info)?;
    let token_program_id = next_account_info(account_info_iter)?;

    assert_rent_exempt(rent, obligation_collateral_info)?;
    assert_uninitialized::<ObligationCollateral>(obligation_collateral_info)?;

    let lending_market = LendingMarket::unpack(&lending_market_info.data.borrow())?;
    if lending_market_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &lending_market.token_program_id != token_program_id.key {
        return Err(LendingError::InvalidTokenProgram.into());
    }

    let deposit_reserve = Reserve::unpack(&deposit_reserve_info.data.borrow())?;
    if deposit_reserve_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &deposit_reserve.lending_market != lending_market_info.key {
        msg!("Invalid reserve lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    // @FIXME: moved to lending market; does per-reserve collateral enable still make sense?
    if deposit_reserve.config.loan_to_value_ratio == 0 {
        return Err(LendingError::ReserveCollateralDisabled.into());
    }

    let mut obligation = Obligation::unpack(&obligation_info.data.borrow())?;
    if obligation_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation.lending_market != lending_market_info.key {
        msg!("Invalid obligation lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if obligation.collateral.len() + obligation.liquidity.len() + 1 > MAX_OBLIGATION_ACCOUNTS {
        return Err(LendingError::ObligationAccountLimit.into());
    }
    if obligation.collateral.contains(deposit_reserve_info.key) {
        return Err(LendingError::ObligationAccountDuplicate.into());
    }

    let authority_signer_seeds = &[
        lending_market_info.key.as_ref(),
        &[lending_market.bump_seed],
    ];
    let lending_market_authority_pubkey =
        Pubkey::create_program_address(authority_signer_seeds, program_id)?;
    if lending_market_authority_info.key != &lending_market_authority_pubkey {
        return Err(LendingError::InvalidMarketAuthority.into());
    }

    let obligation_collateral = ObligationCollateral::new(NewObligationCollateralParams {
        obligation: *obligation_info.key,
        deposit_reserve: *deposit_reserve_info.key,
        token_mint: obligation_token_mint_info.key(),
    });
    obligation.collateral.push(*obligation_collateral_info.key);
    obligation.mark_stale();

    ObligationCollateral::pack(
        obligation_collateral,
        &mut obligation_collateral_info.data.borrow_mut(),
    )?;
    Obligation::pack(obligation, &mut obligation_info.data.borrow_mut())?;

    spl_token_init_mint(TokenInitializeMintParams {
        mint: obligation_token_mint_info.clone(),
        authority: lending_market_authority_info.key,
        rent: rent_info.clone(),
        decimals: deposit_reserve.liquidity.mint_decimals,
        token_program: token_program_id.clone(),
    })?;

    spl_token_init_account(TokenInitializeAccountParams {
        account: obligation_token_output_info.clone(),
        mint: obligation_token_mint_info.clone(),
        owner: obligation_token_owner_info.clone(),
        rent: rent_info.clone(),
        token_program: token_program_id.clone(),
    })?;

    Ok(())
}

#[inline(never)] // avoid stack frame limit
fn process_init_obligation_liquidity(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let obligation_info = next_account_info(account_info_iter)?;
    let obligation_liquidity_info = next_account_info(account_info_iter)?;
    let borrow_reserve_info = next_account_info(account_info_iter)?;
    let lending_market_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_info)?;
    let token_program_id = next_account_info(account_info_iter)?;

    assert_rent_exempt(rent, obligation_liquidity_info)?;
    assert_uninitialized::<ObligationLiquidity>(obligation_liquidity_info)?;

    let lending_market = LendingMarket::unpack(&lending_market_info.data.borrow())?;
    if lending_market_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &lending_market.token_program_id != token_program_id.key {
        return Err(LendingError::InvalidTokenProgram.into());
    }

    let borrow_reserve = Reserve::unpack(&borrow_reserve_info.data.borrow())?;
    if borrow_reserve_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &borrow_reserve.lending_market != lending_market_info.key {
        msg!("Invalid reserve lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }

    let mut obligation = Obligation::unpack(&obligation_info.data.borrow())?;
    if obligation_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation.lending_market != lending_market_info.key {
        msg!("Invalid obligation lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if obligation.collateral.len() + obligation.liquidity.len() + 1 > MAX_OBLIGATION_ACCOUNTS {
        return Err(LendingError::ObligationAccountLimit.into());
    }
    if obligation.liquidity.contains(borrow_reserve_info.key) {
        return Err(LendingError::ObligationAccountDuplicate.into());
    }

    let obligation_liquidity = ObligationLiquidity::new(NewObligationLiquidityParams {
        obligation: *obligation_info.key,
        borrow_reserve: *borrow_reserve_info.key,
    });
    obligation.liquidity.push(*obligation_liquidity_info.key);
    obligation.mark_stale();

    ObligationLiquidity::pack(
        obligation_liquidity,
        &mut obligation_liquidity_info.data.borrow_mut(),
    )?;
    Obligation::pack(obligation, &mut obligation_info.data.borrow_mut())?;

    Ok(())
}

fn process_refresh_obligation_collateral(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let obligation_collateral_info = next_account_info(account_info_iter)?;
    let deposit_reserve_info = next_account_info(account_info_iter)?;
    let lending_market_info = next_account_info(account_info_iter)?;
    let lending_market_authority_info = next_account_info(account_info_iter)?;
    let dex_market_info = next_account_info(account_info_iter)?;
    let dex_market_orders_info = next_account_info(account_info_iter)?;
    let memory = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_id = next_account_info(account_info_iter)?;

    // Ensure memory is owned by this program so that we don't have to zero it out
    if memory.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }

    let lending_market = LendingMarket::unpack(&lending_market_info.data.borrow())?;
    if lending_market_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &lending_market.token_program_id != token_program_id.key {
        return Err(LendingError::InvalidTokenProgram.into());
    }

    let deposit_reserve = Reserve::unpack(&deposit_reserve_info.data.borrow())?;
    if deposit_reserve_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &deposit_reserve.lending_market != lending_market_info.key {
        msg!("Invalid reserve lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if deposit_reserve.dex_market.is_none() {
        msg!("Deposit reserve must have a dex market");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if let COption::Some(dex_market_pubkey) = deposit_reserve.dex_market {
        if &dex_market_pubkey != dex_market_info.key {
            msg!("Invalid dex market account");
            return Err(LendingError::InvalidAccountInput.into());
        }
    }

    let mut obligation_collateral =
        ObligationCollateral::unpack(&obligation_collateral_info.data.borrow())?;
    if obligation_collateral_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation_collateral.deposit_reserve != deposit_reserve_info.key {
        msg!("Invalid deposit reserve account");
        return Err(LendingError::InvalidAccountInput.into());
    }

    let authority_signer_seeds = &[
        lending_market_info.key.as_ref(),
        &[lending_market.bump_seed],
    ];
    let lending_market_authority_pubkey =
        Pubkey::create_program_address(authority_signer_seeds, program_id)?;
    if lending_market_authority_info.key != &lending_market_authority_pubkey {
        return Err(LendingError::InvalidMarketAuthority.into());
    }

    // @TODO: is this necessary? collateral exchange rate can change, but interest doesn't accrue
    assert_last_update_slot(&deposit_reserve, clock.slot)?;

    let trade_simulator = TradeSimulator::new(
        dex_market_info,
        dex_market_orders_info,
        memory,
        &lending_market.quote_token_mint,
        // @TODO: check these
        &lending_market.quote_token_mint,
        &deposit_reserve.liquidity.mint_pubkey,
    )?;

    obligation_collateral.update_value(
        deposit_reserve.collateral_exchange_rate()?,
        trade_simulator,
        &deposit_reserve.liquidity.mint_pubkey,
    )?;
    obligation_collateral.update_slot(clock.slot)?;
    ObligationCollateral::pack(
        obligation_collateral,
        &mut obligation_collateral_info.data.borrow_mut(),
    )?;
    // @TODO: should we mark the obligation stale here? could also iteratively update
    //          obligation.collateral_value

    Ok(())
}

fn process_refresh_obligation_liquidity(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let obligation_liquidity_info = next_account_info(account_info_iter)?;
    let borrow_reserve_info = next_account_info(account_info_iter)?;
    let lending_market_info = next_account_info(account_info_iter)?;
    let lending_market_authority_info = next_account_info(account_info_iter)?;
    let dex_market_info = next_account_info(account_info_iter)?;
    let dex_market_orders_info = next_account_info(account_info_iter)?;
    let memory = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_id = next_account_info(account_info_iter)?;

    // Ensure memory is owned by this program so that we don't have to zero it out
    if memory.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }

    let lending_market = LendingMarket::unpack(&lending_market_info.data.borrow())?;
    if lending_market_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &lending_market.token_program_id != token_program_id.key {
        return Err(LendingError::InvalidTokenProgram.into());
    }

    let borrow_reserve = Reserve::unpack(&borrow_reserve_info.data.borrow())?;
    if borrow_reserve_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &borrow_reserve.lending_market != lending_market_info.key {
        msg!("Invalid reserve lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if borrow_reserve.dex_market.is_none() {
        msg!("Borrow reserve must have a dex market");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if let COption::Some(dex_market_pubkey) = borrow_reserve.dex_market {
        if &dex_market_pubkey != dex_market_info.key {
            msg!("Invalid dex market account");
            return Err(LendingError::InvalidAccountInput.into());
        }
    }

    let mut obligation_liquidity =
        ObligationLiquidity::unpack(&obligation_liquidity_info.data.borrow())?;
    if obligation_liquidity_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation_liquidity.borrow_reserve != borrow_reserve_info.key {
        msg!("Invalid borrow reserve account");
        return Err(LendingError::InvalidAccountInput.into());
    }

    let authority_signer_seeds = &[
        lending_market_info.key.as_ref(),
        &[lending_market.bump_seed],
    ];
    let lending_market_authority_pubkey =
        Pubkey::create_program_address(authority_signer_seeds, program_id)?;
    if lending_market_authority_info.key != &lending_market_authority_pubkey {
        return Err(LendingError::InvalidMarketAuthority.into());
    }

    // @TODO: is this necessary? what if we accrue interest here if it's not the current slot?
    assert_last_update_slot(&borrow_reserve, clock.slot)?;

    let trade_simulator = TradeSimulator::new(
        dex_market_info,
        dex_market_orders_info,
        memory,
        &lending_market.quote_token_mint,
        // @TODO: check these
        &lending_market.quote_token_mint,
        &borrow_reserve.liquidity.mint_pubkey,
    )?;

    obligation_liquidity.accrue_interest(borrow_reserve.cumulative_borrow_rate_wads)?;
    obligation_liquidity.update_value(trade_simulator, &borrow_reserve.liquidity.mint_pubkey)?;
    obligation_liquidity.update_slot(clock.slot)?;
    ObligationLiquidity::pack(
        obligation_liquidity,
        &mut obligation_liquidity_info.data.borrow_mut(),
    )?;
    // @TODO: should we mark the obligation stale here? could also iteratively update
    //          obligation.liquidity_value

    Ok(())
}

fn process_refresh_obligation(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let obligation_info = next_account_info(account_info_iter)?;
    let lending_market_info = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_id = next_account_info(account_info_iter)?;

    let lending_market = LendingMarket::unpack(&lending_market_info.data.borrow())?;
    if lending_market_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &lending_market.token_program_id != token_program_id.key {
        return Err(LendingError::InvalidTokenProgram.into());
    }

    let mut obligation = Obligation::unpack(&obligation_info.data.borrow())?;
    if obligation_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation.lending_market != lending_market_info.key {
        msg!("Invalid obligation lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }

    let mut collateral_value = Decimal::zero();
    for pubkey in obligation.collateral {
        let obligation_collateral_info = next_account_info(account_info_iter)?;
        if obligation_collateral_info.owner != program_id {
            return Err(LendingError::InvalidAccountOwner.into());
        }
        if pubkey != obligation_collateral_info.key {
            msg!("Invalid obligation collateral account");
            return Err(LendingError::InvalidAccountInput.into());
        }

        let obligation_collateral =
            ObligationCollateral::unpack(&obligation_collateral_info.data.borrow())?;
        if obligation_collateral.obligation != obligation_info.key {
            msg!("Invalid obligation account");
            return Err(LendingError::InvalidAccountInput.into());
        }
        if obligation_collateral.is_stale(clock.slot)? {
            return Err(LendingError::ObligationCollateralStale.into());
        }

        collateral_value = collateral_value.try_add(obligation_collateral.value)?;
    }

    let mut liquidity_value = Decimal::zero();
    for pubkey in obligation.liquidity {
        let obligation_liquidity_info = next_account_info(account_info_iter)?;
        if obligation_liquidity_info.owner != program_id {
            return Err(LendingError::InvalidAccountOwner.into());
        }
        if pubkey != obligation_liquidity_info.key {
            msg!("Invalid obligation liquidity account");
            return Err(LendingError::InvalidAccountInput.into());
        }

        let obligation_liquidity =
            ObligationLiquidity::unpack(&obligation_liquidity_info.data.borrow())?;
        if obligation_liquidity.obligation != obligation_info.key {
            msg!("Invalid obligation account");
            return Err(LendingError::InvalidAccountInput.into());
        }
        if obligation_liquidity.is_stale(clock.slot)? {
            return Err(LendingError::ObligationLiquidityStale.into());
        }

        liquidity_value = liquidity_value.try_add(obligation_liquidity.value)?;
    }

    // @TODO: check this
    if account_info_iter.count() > 0 {
        msg!("Too many obligation collateral or liquidity accounts");
        return Err(LendingError::InvalidAccountInput.into());
    }

    obligation.collateral_value = collateral_value;
    obligation.liquidity_value = liquidity_value;
    obligation.update_slot(clock.slot)?;
    Obligation::pack(obligation, &mut obligation_info.data.borrow_mut())?;

    Ok(())
}

fn assert_rent_exempt(rent: &Rent, account_info: &AccountInfo) -> ProgramResult {
    if !rent.is_exempt(account_info.lamports(), account_info.data_len()) {
        msg!(&rent.minimum_balance(account_info.data_len()).to_string());
        Err(LendingError::NotRentExempt.into())
    } else {
        Ok(())
    }
}

fn assert_last_update_slot(reserve: &Reserve, slot: Slot) -> ProgramResult {
    if !reserve.last_update_slot == slot {
        Err(LendingError::ReserveStale.into())
    } else {
        Ok(())
    }
}

fn assert_uninitialized<T: Pack + IsInitialized>(
    account_info: &AccountInfo,
) -> Result<T, ProgramError> {
    let account: T = T::unpack_unchecked(&account_info.data.borrow())?;
    if account.is_initialized() {
        Err(LendingError::AlreadyInitialized.into())
    } else {
        Ok(account)
    }
}

/// Unpacks a spl_token `Mint`.
fn unpack_mint(data: &[u8]) -> Result<spl_token::state::Mint, LendingError> {
    spl_token::state::Mint::unpack(data).map_err(|_| LendingError::InvalidTokenMint)
}

/// Issue a spl_token `InitializeMint` instruction.
#[inline(always)]
fn spl_token_init_mint(params: TokenInitializeMintParams<'_, '_>) -> ProgramResult {
    let TokenInitializeMintParams {
        mint,
        rent,
        authority,
        token_program,
        decimals,
    } = params;
    let ix = spl_token::instruction::initialize_mint(
        token_program.key,
        mint.key,
        authority,
        None,
        decimals,
    )?;
    let result = invoke(&ix, &[mint, rent, token_program]);
    result.map_err(|_| LendingError::TokenInitializeMintFailed.into())
}

/// Issue a spl_token `InitializeAccount` instruction.
#[inline(always)]
fn spl_token_init_account(params: TokenInitializeAccountParams<'_>) -> ProgramResult {
    let TokenInitializeAccountParams {
        account,
        mint,
        owner,
        rent,
        token_program,
    } = params;
    let ix = spl_token::instruction::initialize_account(
        token_program.key,
        account.key,
        mint.key,
        owner.key,
    )?;
    let result = invoke(&ix, &[account, mint, owner, rent, token_program]);
    result.map_err(|_| LendingError::TokenInitializeAccountFailed.into())
}

/// Issue a spl_token `Transfer` instruction.
#[inline(always)]
fn spl_token_transfer(params: TokenTransferParams<'_, '_>) -> ProgramResult {
    let TokenTransferParams {
        source,
        destination,
        authority,
        token_program,
        amount,
        authority_signer_seeds,
    } = params;
    let result = invoke_signed(
        &spl_token::instruction::transfer(
            token_program.key,
            source.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?,
        &[source, destination, authority, token_program],
        &[authority_signer_seeds],
    );
    result.map_err(|_| LendingError::TokenTransferFailed.into())
}

/// Issue a spl_token `MintTo` instruction.
fn spl_token_mint_to(params: TokenMintToParams<'_, '_>) -> ProgramResult {
    let TokenMintToParams {
        mint,
        destination,
        authority,
        token_program,
        amount,
        authority_signer_seeds,
    } = params;
    let result = invoke_signed(
        &spl_token::instruction::mint_to(
            token_program.key,
            mint.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?,
        &[mint, destination, authority, token_program],
        &[authority_signer_seeds],
    );
    result.map_err(|_| LendingError::TokenMintToFailed.into())
}

/// Issue a spl_token `Burn` instruction.
#[inline(always)]
fn spl_token_burn(params: TokenBurnParams<'_, '_>) -> ProgramResult {
    let TokenBurnParams {
        mint,
        source,
        authority,
        token_program,
        amount,
        authority_signer_seeds,
    } = params;
    let result = invoke_signed(
        &spl_token::instruction::burn(
            token_program.key,
            source.key,
            mint.key,
            authority.key,
            &[],
            amount,
        )?,
        &[source, mint, authority, token_program],
        &[authority_signer_seeds],
    );
    result.map_err(|_| LendingError::TokenBurnFailed.into())
}

struct TokenInitializeMintParams<'a: 'b, 'b> {
    mint: AccountInfo<'a>,
    rent: AccountInfo<'a>,
    authority: &'b Pubkey,
    decimals: u8,
    token_program: AccountInfo<'a>,
}

struct TokenInitializeAccountParams<'a> {
    account: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    owner: AccountInfo<'a>,
    rent: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
}

struct TokenTransferParams<'a: 'b, 'b> {
    source: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    amount: u64,
    authority: AccountInfo<'a>,
    authority_signer_seeds: &'b [&'b [u8]],
    token_program: AccountInfo<'a>,
}

struct TokenMintToParams<'a: 'b, 'b> {
    mint: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    amount: u64,
    authority: AccountInfo<'a>,
    authority_signer_seeds: &'b [&'b [u8]],
    token_program: AccountInfo<'a>,
}

struct TokenBurnParams<'a: 'b, 'b> {
    mint: AccountInfo<'a>,
    source: AccountInfo<'a>,
    amount: u64,
    authority: AccountInfo<'a>,
    authority_signer_seeds: &'b [&'b [u8]],
    token_program: AccountInfo<'a>,
}

impl PrintProgramError for LendingError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        msg!(&self.to_string());
    }
}
