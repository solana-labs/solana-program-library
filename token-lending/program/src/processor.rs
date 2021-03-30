//! Program state processor

use crate::{
    error::LendingError,
    instruction::LendingInstruction,
    math::{Decimal, Rate, TryDiv, TryMul, WAD},
    state::{
        BorrowLiquidityResult, InitLendingMarketParams, InitObligationParams, InitReserveParams,
        LendingMarket, LiquidateObligationResult, NewReserveCollateralParams,
        NewReserveLiquidityParams, Obligation, RepayLiquidityResult, Reserve, ReserveCollateral,
        ReserveConfig, ReserveLiquidity, PROGRAM_VERSION,
    },
};
use flux_aggregator::read_median;
use num_traits::FromPrimitive;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
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
use spl_token::state::{Account, Mint};

/// Processes an instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = LendingInstruction::unpack(input)?;
    match instruction {
        LendingInstruction::InitLendingMarket { owner } => {
            msg!("Instruction: Init Lending Market");
            process_init_lending_market(program_id, owner, accounts)
        }
        LendingInstruction::SetLendingMarketOwner { new_owner } => {
            msg!("Instruction: Set Lending Market Owner");
            process_set_lending_market_owner(program_id, new_owner, accounts)
        }
        LendingInstruction::InitReserve {
            liquidity_amount,
            config,
        } => {
            msg!("Instruction: Init Reserve");
            process_init_reserve(program_id, liquidity_amount, config, accounts)
        }
        LendingInstruction::RefreshReserve => {
            msg!("Instruction: Refresh Reserve");
            process_refresh_reserve(program_id, accounts)
        }
        LendingInstruction::DepositReserveLiquidity { liquidity_amount } => {
            msg!("Instruction: Deposit Reserve Liquidity");
            process_deposit_reserve_liquidity(program_id, liquidity_amount, accounts)
        }
        LendingInstruction::RedeemReserveCollateral { collateral_amount } => {
            msg!("Instruction: Redeem Reserve Collateral");
            process_redeem_reserve_collateral(program_id, collateral_amount, accounts)
        }
        LendingInstruction::InitObligation => {
            msg!("Instruction: Init Obligation");
            process_init_obligation(program_id, accounts)
        }
        LendingInstruction::RefreshObligation => {
            msg!("Instruction: Refresh Obligation");
            process_refresh_obligation(program_id, accounts)
        }
        LendingInstruction::DepositObligationCollateral { collateral_amount } => {
            msg!("Instruction: Deposit Obligation Collateral");
            process_deposit_obligation_collateral(program_id, collateral_amount, accounts)
        }
        LendingInstruction::WithdrawObligationCollateral { collateral_amount } => {
            msg!("Instruction: Withdraw Obligation Collateral");
            process_withdraw_obligation_collateral(program_id, collateral_amount, accounts)
        }
        LendingInstruction::BorrowObligationLiquidity { liquidity_amount } => {
            msg!("Instruction: Borrow Obligation Liquidity");
            process_borrow_obligation_liquidity(program_id, liquidity_amount, accounts)
        }
        LendingInstruction::RepayObligationLiquidity { liquidity_amount } => {
            msg!("Instruction: Repay Obligation Liquidity");
            process_repay_obligation_liquidity(program_id, liquidity_amount, accounts)
        }
        LendingInstruction::LiquidateObligation { liquidity_amount } => {
            msg!("Instruction: Liquidate Obligation");
            process_liquidate_obligation(program_id, liquidity_amount, accounts)
        }
    }
}

fn process_init_lending_market(
    program_id: &Pubkey,
    owner: Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let lending_market_info = next_account_info(account_info_iter)?;
    let quote_token_mint_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_id = next_account_info(account_info_iter)?;

    assert_rent_exempt(rent, lending_market_info)?;
    let mut lending_market = assert_uninitialized::<LendingMarket>(lending_market_info)?;

    unpack_mint(&quote_token_mint_info.data.borrow())?;
    if quote_token_mint_info.owner != token_program_id.key {
        return Err(LendingError::InvalidTokenOwner.into());
    }

    lending_market.init(InitLendingMarketParams {
        bump_seed: Pubkey::find_program_address(&[lending_market_info.key.as_ref()], program_id).1,
        owner,
        quote_token_mint: *quote_token_mint_info.key,
        token_program_id: *token_program_id.key,
    });
    LendingMarket::pack(lending_market, &mut lending_market_info.data.borrow_mut())?;

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
    if config.optimal_borrow_rate < config.min_borrow_rate {
        msg!("Optimal borrow rate must be >= min borrow rate");
        return Err(LendingError::InvalidConfig.into());
    }
    if config.optimal_borrow_rate > config.max_borrow_rate {
        msg!("Optimal borrow rate must be <= max borrow rate");
        return Err(LendingError::InvalidConfig.into());
    }
    if config.loan_to_value_ratio >= 100 {
        msg!("Loan to value ratio must be in range [0, 100)");
        return Err(LendingError::InvalidConfig.into());
    }
    if config.liquidation_threshold <= config.loan_to_value_ratio
        || config.liquidation_threshold > 100
    {
        msg!("Liquidation threshold must be in range (LTV, 100]");
        return Err(LendingError::InvalidConfig.into());
    }
    if config.liquidation_bonus > 100 {
        msg!("Liquidation bonus must be in range [0, 100]");
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

    let account_info_iter = &mut accounts.iter().peekable();
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

    assert_rent_exempt(rent, reserve_info)?;
    let mut reserve = assert_uninitialized::<Reserve>(reserve_info)?;
    assert_uninitialized::<Account>(reserve_liquidity_supply_info)?;
    assert_uninitialized::<Account>(reserve_liquidity_fee_receiver_info)?;
    assert_uninitialized::<Mint>(reserve_collateral_mint_info)?;
    assert_uninitialized::<Account>(reserve_collateral_supply_info)?;
    assert_uninitialized::<Account>(destination_collateral_info)?;

    if reserve_liquidity_supply_info.key == source_liquidity_info.key {
        msg!("Invalid source liquidity account");
        return Err(LendingError::InvalidAccountInput.into());
    }

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

    let (reserve_liquidity_aggregator, reserve_liquidity_median_price) =
        if reserve_liquidity_mint_info.key == &lending_market.quote_token_mint {
            if account_info_iter.peek().is_some() {
                msg!("Invalid reserve liquidity aggregator account");
                return Err(LendingError::InvalidAccountInput.into());
            }
            // 1 because quote token price is equal to itself
            (COption::None, 1)
        } else {
            let aggregator_info = next_account_info(account_info_iter)?;
            assert_rent_exempt(rent, aggregator_info)?;

            // @TODO: is there a way to check that aggregator_info represents the base:quote pair?
            (
                COption::Some(*aggregator_info.key),
                read_median(aggregator_info)?.median,
            )
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

    reserve.version = PROGRAM_VERSION;
    reserve.init(InitReserveParams {
        current_slot: clock.slot,
        lending_market: *lending_market_info.key,
        liquidity: ReserveLiquidity::new(NewReserveLiquidityParams {
            mint_pubkey: *reserve_liquidity_mint_info.key,
            mint_decimals: reserve_liquidity_mint.decimals,
            supply_pubkey: *reserve_liquidity_supply_info.key,
            fee_receiver: *reserve_liquidity_fee_receiver_info.key,
            aggregator: reserve_liquidity_aggregator,
            median_price: reserve_liquidity_median_price,
        }),
        collateral: ReserveCollateral::new(NewReserveCollateralParams {
            mint_pubkey: *reserve_collateral_mint_info.key,
            supply_pubkey: *reserve_collateral_supply_info.key,
        }),
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

fn process_refresh_reserve(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter().peekable();
    let reserve_info = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;

    let mut reserve = Reserve::unpack(&reserve_info.data.borrow())?;
    if reserve_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }

    if let COption::Some(reserve_liquidity_aggregator) = reserve.liquidity.aggregator {
        let reserve_liquidity_aggregator_info = next_account_info(account_info_iter)?;
        if &reserve_liquidity_aggregator != reserve_liquidity_aggregator_info.key {
            msg!("Invalid reserve liquidity aggregator account");
            return Err(LendingError::InvalidAccountInput.into());
        }

        reserve.liquidity.median_price = read_median(reserve_liquidity_aggregator_info)?.median;
    } else if account_info_iter.peek().is_some() {
        msg!("Invalid reserve liquidity aggregator account");
        return Err(LendingError::InvalidAccountInput.into());
    }

    reserve.accrue_interest(clock.slot)?;
    Reserve::pack(reserve, &mut reserve_info.data.borrow_mut())?;

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
    if reserve.last_update.is_stale(clock.slot)? {
        return Err(LendingError::ReserveStale.into());
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

fn process_redeem_reserve_collateral(
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
    if reserve.last_update.is_stale(clock.slot)? {
        return Err(LendingError::ReserveStale.into());
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
fn process_init_obligation(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let obligation_info = next_account_info(account_info_iter)?;
    let lending_market_info = next_account_info(account_info_iter)?;
    let obligation_owner_info = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
    let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_id = next_account_info(account_info_iter)?;

    assert_rent_exempt(rent, obligation_info)?;
    let mut obligation = assert_uninitialized::<Obligation>(obligation_info)?;

    let lending_market = LendingMarket::unpack(&lending_market_info.data.borrow())?;
    if lending_market_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &lending_market.token_program_id != token_program_id.key {
        return Err(LendingError::InvalidTokenProgram.into());
    }

    if !obligation_owner_info.is_signer {
        return Err(LendingError::InvalidSigner.into());
    }

    obligation.init(InitObligationParams {
        current_slot: clock.slot,
        lending_market: *lending_market_info.key,
        owner: *obligation_owner_info.key,
        deposits: Vec::with_capacity(0),
        borrows: Vec::with_capacity(0),
    });
    Obligation::pack(obligation, &mut obligation_info.data.borrow_mut())?;

    Ok(())
}

fn process_refresh_obligation(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter().peekable();
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

    for collateral in &mut obligation.deposits {
        let deposit_reserve_info = next_account_info(account_info_iter)?;
        if deposit_reserve_info.owner != program_id {
            return Err(LendingError::InvalidAccountOwner.into());
        }
        if collateral.deposit_reserve != *deposit_reserve_info.key {
            msg!("Invalid deposit reserve");
            return Err(LendingError::InvalidAccountInput.into());
        }

        let deposit_reserve = Reserve::unpack(&deposit_reserve_info.data.borrow())?;
        if &deposit_reserve.lending_market != lending_market_info.key {
            msg!("Invalid deposit reserve lending market account");
            return Err(LendingError::InvalidAccountInput.into());
        }
        if deposit_reserve.last_update.is_stale(clock.slot)? {
            return Err(LendingError::ReserveStale.into());
        }

        collateral.market_value = deposit_reserve
            .collateral_exchange_rate()?
            .decimal_collateral_to_liquidity(collateral.deposited_amount.into())?
            .try_mul(deposit_reserve.liquidity.median_price)?;
    }

    for liquidity in &mut obligation.borrows {
        let borrow_reserve_info = next_account_info(account_info_iter)?;
        if borrow_reserve_info.owner != program_id {
            return Err(LendingError::InvalidAccountOwner.into());
        }
        if liquidity.borrow_reserve != *borrow_reserve_info.key {
            msg!("Invalid borrow reserve");
            return Err(LendingError::InvalidAccountInput.into());
        }

        let borrow_reserve = Reserve::unpack(&borrow_reserve_info.data.borrow())?;
        if &borrow_reserve.lending_market != lending_market_info.key {
            msg!("Invalid borrow reserve lending market account");
            return Err(LendingError::InvalidAccountInput.into());
        }
        if borrow_reserve.last_update.is_stale(clock.slot)? {
            return Err(LendingError::ReserveStale.into());
        }

        liquidity.accrue_interest(borrow_reserve.liquidity.cumulative_borrow_rate_wads)?;
        liquidity.market_value = liquidity
            .borrowed_amount_wads
            .try_mul(borrow_reserve.liquidity.median_price)?;
    }

    if account_info_iter.peek().is_some() {
        msg!("Too many obligation collateral or liquidity accounts");
        return Err(LendingError::InvalidAccountInput.into());
    }

    obligation.last_update.update_slot(clock.slot);
    Obligation::pack(obligation, &mut obligation_info.data.borrow_mut())?;

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
    let obligation_token_mint_info = next_account_info(account_info_iter)?;
    let obligation_token_output_info = next_account_info(account_info_iter)?;
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

    let deposit_reserve = Reserve::unpack(&deposit_reserve_info.data.borrow())?;
    if deposit_reserve_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &deposit_reserve.lending_market != lending_market_info.key {
        msg!("Invalid reserve lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &deposit_reserve.collateral.supply_pubkey != destination_collateral_info.key {
        msg!("Invalid destination collateral account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &deposit_reserve.collateral.supply_pubkey == source_collateral_info.key {
        msg!("Invalid source collateral account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    // @TODO: is this necessary? we don't care about market price or interest here yet.
    //        however, we will if we add the ability to deposit and borrow in one transaction.
    //        it would also be important if deposit_reserve.config.collateral_enabled changes.
    if deposit_reserve.last_update.is_stale(clock.slot)? {
        return Err(LendingError::ReserveStale.into());
    }
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

    obligation
        .find_or_add_collateral_to_deposits(*deposit_reserve_info.key)?
        .deposit(collateral_amount)?;
    obligation.last_update.mark_stale();
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
    accounts: &[AccountInfo],
) -> ProgramResult {
    if collateral_amount == 0 {
        return Err(LendingError::InvalidAmount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let source_collateral_info = next_account_info(account_info_iter)?;
    let destination_collateral_info = next_account_info(account_info_iter)?;
    let withdraw_reserve_info = next_account_info(account_info_iter)?;
    let obligation_info = next_account_info(account_info_iter)?;
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
    if withdraw_reserve.last_update.is_stale(clock.slot)? {
        return Err(LendingError::ReserveStale.into());
    }

    let mut obligation = Obligation::unpack(&obligation_info.data.borrow())?;
    if obligation_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation.lending_market != lending_market_info.key {
        msg!("Invalid obligation lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if obligation.last_update.is_stale(clock.slot)? {
        return Err(LendingError::ObligationStale.into());
    }
    if obligation.last_update < withdraw_reserve.last_update {
        return Err(LendingError::ObligationStale.into());
    }

    let (collateral, collateral_index) = obligation.find_collateral_in_deposits(*withdraw_reserve_info.key)?;
    if collateral.deposited_amount == 0 {
        return Err(LendingError::ObligationCollateralEmpty.into());
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

    // @FIXME: LTV
    let loan_to_value_ratio = Rate::from_percent(50);
    // let loan_to_value_ratio = Rate::from_percent(lending_market.loan_to_value_ratio);
    let loan_to_value = obligation.loan_to_value()?;
    if loan_to_value >= loan_to_value_ratio {
        return Err(LendingError::ObligationLoanToValueLimit.into());
    }

    let max_withdraw_value = obligation.max_withdraw_value(loan_to_value_ratio)?;

    let withdraw_amount = if collateral_amount == u64::MAX {
        let withdraw_value = max_withdraw_value.min(collateral.market_value);
        withdraw_value
            .try_div(collateral.market_value)?
            .try_mul(collateral.deposited_amount)?
            .try_floor_u64()?
            .min(collateral.deposited_amount)
    } else {
        let withdraw_amount = collateral_amount.min(collateral.deposited_amount);
        let withdraw_pct = Decimal::from(withdraw_amount).try_div(collateral.deposited_amount)?;
        let withdraw_value = obligation.deposited_value()?.try_mul(withdraw_pct)?;
        if withdraw_value > max_withdraw_value {
            return Err(LendingError::WithdrawTooLarge.into());
        }
        withdraw_amount
    };
    if withdraw_amount == 0 {
        return Err(LendingError::WithdrawTooSmall.into());
    }

    let obligation_token_amount = collateral
        .collateral_to_obligation_token_amount(withdraw_amount, obligation_token_mint.supply)?;
    if obligation_token_amount == 0 {
        return Err(LendingError::WithdrawTooSmall.into());
    }

    obligation.withdraw(withdraw_amount, collateral_index)?;
    obligation.last_update.mark_stale();
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
fn process_borrow_obligation_liquidity(
    program_id: &Pubkey,
    liquidity_amount: u64,
    accounts: &[AccountInfo],
) -> ProgramResult {
    if liquidity_amount == 0 {
        return Err(LendingError::InvalidAmount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let source_liquidity_info = next_account_info(account_info_iter)?;
    let destination_liquidity_info = next_account_info(account_info_iter)?;
    let borrow_reserve_info = next_account_info(account_info_iter)?;
    let borrow_reserve_liquidity_fee_receiver_info = next_account_info(account_info_iter)?;
    let obligation_info = next_account_info(account_info_iter)?;
    let lending_market_info = next_account_info(account_info_iter)?;
    let lending_market_authority_info = next_account_info(account_info_iter)?;
    let obligation_owner_info = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_id = next_account_info(account_info_iter)?;

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
        msg!("Invalid reserve lending market account");
        return Err(LendingError::InvalidAccountInput.into());
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
    if borrow_reserve.last_update.is_stale(clock.slot)? {
        return Err(LendingError::ReserveStale.into());
    }

    let mut obligation = Obligation::unpack(&obligation_info.data.borrow())?;
    if obligation_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation.lending_market != lending_market_info.key {
        msg!("Invalid obligation lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if obligation.last_update.is_stale(clock.slot)? {
        return Err(LendingError::ObligationStale.into());
    }
    if obligation.last_update < borrow_reserve.last_update {
        return Err(LendingError::ObligationStale.into());
    }
    if &obligation.owner != obligation_owner_info.key {
        return Err(LendingError::InvalidObligationOwner.into());
    }
    if !obligation_owner_info.is_signer {
        return Err(LendingError::InvalidSigner.into());
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

    // @FIXME: LTV
    let loan_to_value_ratio = Rate::from_percent(50);
    // let loan_to_value_ratio = Rate::from_percent(lending_market.loan_to_value_ratio);
    let loan_to_value = obligation.loan_to_value()?;
    if loan_to_value >= loan_to_value_ratio {
        return Err(LendingError::ObligationLoanToValueLimit.into());
    }

    let max_borrow_value = obligation.max_borrow_value(loan_to_value_ratio)?;

    let BorrowLiquidityResult {
        borrow_amount,
        receive_amount,
        borrow_fee,
        host_fee,
    } = borrow_reserve.borrow_liquidity(liquidity_amount, max_borrow_value)?;

    if receive_amount == 0 {
        return Err(LendingError::BorrowTooSmall.into());
    }

    borrow_reserve.liquidity.borrow(borrow_amount)?;
    Reserve::pack(borrow_reserve, &mut borrow_reserve_info.data.borrow_mut())?;

    obligation
        .find_or_add_liquidity_to_borrows(*borrow_reserve_info.key)?
        .borrow(borrow_amount)?;
    obligation.last_update.mark_stale();
    Obligation::pack(obligation, &mut obligation_info.data.borrow_mut())?;

    let mut owner_fee = borrow_fee;
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
        amount: receive_amount,
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
    accounts: &[AccountInfo],
) -> ProgramResult {
    if liquidity_amount == 0 {
        return Err(LendingError::InvalidAmount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let source_liquidity_info = next_account_info(account_info_iter)?;
    let destination_liquidity_info = next_account_info(account_info_iter)?;
    let repay_reserve_info = next_account_info(account_info_iter)?;
    let repay_reserve_liquidity_supply_info = next_account_info(account_info_iter)?;
    let obligation_info = next_account_info(account_info_iter)?;
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
    if &repay_reserve.liquidity.supply_pubkey == source_liquidity_info.key {
        msg!("Invalid source liquidity account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &repay_reserve.liquidity.supply_pubkey != destination_liquidity_info.key {
        msg!("Invalid destination liquidity account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if repay_reserve.last_update.is_stale(clock.slot)? {
        return Err(LendingError::ReserveStale.into());
    }

    let mut obligation = Obligation::unpack(&obligation_info.data.borrow())?;
    if obligation_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation.lending_market != lending_market_info.key {
        msg!("Invalid obligation lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if obligation.last_update.is_stale(clock.slot)? {
        return Err(LendingError::ObligationStale.into());
    }
    if obligation.last_update < repay_reserve.last_update {
        return Err(LendingError::ObligationStale.into());
    }

    let (liquidity, liquidity_index) = obligation.find_liquidity_in_borrows(*repay_reserve_info.key)?;

    let authority_signer_seeds = &[
        lending_market_info.key.as_ref(),
        &[lending_market.bump_seed],
    ];
    let lending_market_authority_pubkey =
        Pubkey::create_program_address(authority_signer_seeds, program_id)?;
    if lending_market_authority_info.key != &lending_market_authority_pubkey {
        return Err(LendingError::InvalidMarketAuthority.into());
    }

    let RepayLiquidityResult {
        settle_amount,
        repay_amount,
    } = repay_reserve.repay_liquidity(liquidity_amount, liquidity.borrowed_amount_wads)?;

    if repay_amount == 0 {
        return Err(LendingError::RepayTooSmall.into());
    }

    repay_reserve.liquidity.repay(repay_amount, settle_amount)?;
    Reserve::pack(repay_reserve, &mut repay_reserve_info.data.borrow_mut())?;

    obligation.repay(settle_amount, liquidity_index)?;
    obligation.last_update.mark_stale();
    Obligation::pack(obligation, &mut obligation_info.data.borrow_mut())?;

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

#[inline(never)] // avoid stack frame limit
fn process_liquidate_obligation(
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
    let repay_reserve_info = next_account_info(account_info_iter)?;
    let repay_reserve_liquidity_supply_info = next_account_info(account_info_iter)?;
    let withdraw_reserve_info = next_account_info(account_info_iter)?;
    let withdraw_reserve_collateral_supply_info = next_account_info(account_info_iter)?;
    let obligation_info = next_account_info(account_info_iter)?;
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
        msg!("Invalid repay reserve lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &repay_reserve.liquidity.supply_pubkey != repay_reserve_liquidity_supply_info.key {
        msg!("Invalid repay reserve liquidity supply account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &repay_reserve.liquidity.supply_pubkey == source_liquidity_info.key {
        msg!("Invalid source liquidity account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if repay_reserve.last_update.is_stale(clock.slot)? {
        return Err(LendingError::ReserveStale.into());
    }

    let withdraw_reserve = Reserve::unpack(&withdraw_reserve_info.data.borrow())?;
    if withdraw_reserve_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &withdraw_reserve.lending_market != lending_market_info.key {
        msg!("Invalid withdraw reserve lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &withdraw_reserve.collateral.supply_pubkey != withdraw_reserve_collateral_supply_info.key {
        msg!("Invalid withdraw reserve collateral supply account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if &withdraw_reserve.collateral.supply_pubkey == destination_collateral_info.key {
        msg!("Invalid destination collateral account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if withdraw_reserve.last_update.is_stale(clock.slot)? {
        return Err(LendingError::ReserveStale.into());
    }

    // @TODO: what if a user borrows using the same reserve & mint for collateral?
    //        this is permitted in borrow_obligation_liquidity and could be used for leverage.
    //        maybe liquidation should be allowed, but reduce/eliminate the bonus.
    if repay_reserve_info.key == withdraw_reserve_info.key {
        return Err(LendingError::DuplicateReserve.into());
    }
    if repay_reserve.liquidity.mint_pubkey == withdraw_reserve.liquidity.mint_pubkey {
        return Err(LendingError::DuplicateReserveMint.into());
    }

    let mut obligation = Obligation::unpack(&obligation_info.data.borrow())?;
    if obligation_info.owner != program_id {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    if &obligation.lending_market != lending_market_info.key {
        msg!("Invalid obligation lending market account");
        return Err(LendingError::InvalidAccountInput.into());
    }
    if obligation.last_update.is_stale(clock.slot)? {
        return Err(LendingError::ObligationStale.into());
    }
    if obligation.last_update < repay_reserve.last_update {
        return Err(LendingError::ObligationStale.into());
    }
    if obligation.last_update < withdraw_reserve.last_update {
        return Err(LendingError::ObligationStale.into());
    }

    let (liquidity, liquidity_index) = obligation.find_liquidity_in_borrows(*repay_reserve_info.key)?;
    let (collateral, collateral_index) = obligation.find_collateral_in_deposits(*withdraw_reserve_info.key)?;

    let authority_signer_seeds = &[
        lending_market_info.key.as_ref(),
        &[lending_market.bump_seed],
    ];
    let lending_market_authority_pubkey =
        Pubkey::create_program_address(authority_signer_seeds, program_id)?;
    if lending_market_authority_info.key != &lending_market_authority_pubkey {
        return Err(LendingError::InvalidMarketAuthority.into());
    }

    // @FIXME: LTV
    let liquidation_threshold = Rate::from_percent(55);
    // let liquidation_threshold = Rate::from_percent(lending_market.liquidation_threshold);
    let loan_to_value = obligation.loan_to_value()?;
    if loan_to_value < liquidation_threshold {
        return Err(LendingError::ObligationHealthy.into());
    }

    let LiquidateObligationResult {
        settle_amount,
        repay_amount,
        withdraw_amount,
    } = withdraw_reserve.liquidate_obligation(
        liquidity_amount,
        &obligation,
        &liquidity,
        &collateral,
    )?;

    if repay_amount == 0 || withdraw_amount == 0 {
        return Err(LendingError::LiquidationTooSmall.into());
    }

    repay_reserve.liquidity.repay(repay_amount, settle_amount)?;
    Reserve::pack(repay_reserve, &mut repay_reserve_info.data.borrow_mut())?;

    obligation.repay(settle_amount, liquidity_index)?;
    obligation.withdraw(withdraw_amount, collateral_index)?;
    obligation.last_update.mark_stale();
    Obligation::pack(obligation, &mut obligation_info.data.borrow_mut())?;

    spl_token_transfer(TokenTransferParams {
        source: source_liquidity_info.clone(),
        destination: repay_reserve_liquidity_supply_info.clone(),
        amount: repay_amount,
        authority: user_transfer_authority_info.clone(),
        authority_signer_seeds: &[],
        token_program: token_program_id.clone(),
    })?;

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

fn assert_rent_exempt(rent: &Rent, account_info: &AccountInfo) -> ProgramResult {
    if !rent.is_exempt(account_info.lamports(), account_info.data_len()) {
        msg!(&rent.minimum_balance(account_info.data_len()).to_string());
        Err(LendingError::NotRentExempt.into())
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
