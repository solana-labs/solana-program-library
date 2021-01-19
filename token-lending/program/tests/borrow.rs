#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program_test::*;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use spl_token_lending::{
    instruction::BorrowAmountType,
    math::Decimal,
    processor::process_instruction,
    state::{INITIAL_COLLATERAL_RATIO, SLOTS_PER_YEAR},
};

const LAMPORTS_TO_SOL: u64 = 1_000_000_000;
const FRACTIONAL_TO_USDC: u64 = 1_000_000;

#[tokio::test]
async fn test_borrow_quote_currency() {
    // Using SOL/USDC max 3 bids:
    //  $2.199,  300.0 SOL
    //  $2.192,  213.3 SOL
    //  $2.190, 1523.4 SOL
    //
    // Collateral amount = 750 * 0.8 (LTV) = 600 SOL
    // Borrow amount = 2.199 * 300 + 2.192 * 213.3 + 2.19 * 86.7 = 1,317.1266 USDC
    const SOL_COLLATERAL_AMOUNT_LAMPORTS: u64 = 750 * LAMPORTS_TO_SOL;
    const USDC_BORROW_AMOUNT_FRACTIONAL: u64 = 1_317_126_600;
    const INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL: u64 = 10_000 * FRACTIONAL_TO_USDC;
    const INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS: u64 = 2 * SOL_COLLATERAL_AMOUNT_LAMPORTS;

    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(185_000);

    let user_accounts_owner = Keypair::new();
    let sol_usdc_dex_market = TestDexMarket::setup(&mut test, TestDexMarketPair::SOL_USDC);
    let usdc_mint = add_usdc_mint(&mut test);
    let lending_market = add_lending_market(&mut test, usdc_mint.pubkey);

    let mut reserve_config = TEST_RESERVE_CONFIG;
    reserve_config.loan_to_value_ratio = 80;

    // Configure reserve to a fixed borrow rate of 1%
    reserve_config.min_borrow_rate = 1;
    reserve_config.optimal_borrow_rate = 1;
    reserve_config.optimal_utilization_rate = 100;

    let usdc_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            slots_elapsed: SLOTS_PER_YEAR,
            liquidity_amount: INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let sol_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            slots_elapsed: SLOTS_PER_YEAR,
            borrow_amount: 20, // slightly increase collateral value w/ interest accrual
            dex_market_pubkey: Some(sol_usdc_dex_market.pubkey),
            liquidity_amount: INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            liquidity_mint_decimals: 9,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let usdc_obligation = add_obligation(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddObligationArgs {
            borrow_reserve: &usdc_reserve,
            collateral_reserve: &sol_reserve,
            collateral_amount: 0,
            borrowed_liquidity_wads: Decimal::zero(),
        },
    );

    let (mut banks_client, payer, _recent_blockhash) = test.start().await;

    let borrow_amount =
        get_token_balance(&mut banks_client, usdc_reserve.user_liquidity_account).await;
    assert_eq!(borrow_amount, 0);

    let collateral_supply =
        get_token_balance(&mut banks_client, sol_reserve.collateral_supply).await;
    assert_eq!(collateral_supply, 0);

    let collateral_deposit_amount = INITIAL_COLLATERAL_RATIO * SOL_COLLATERAL_AMOUNT_LAMPORTS;
    lending_market
        .borrow(
            &mut banks_client,
            &payer,
            BorrowArgs {
                deposit_reserve: &sol_reserve,
                borrow_reserve: &usdc_reserve,
                dex_market: &sol_usdc_dex_market,
                borrow_amount_type: BorrowAmountType::CollateralDepositAmount,
                amount: collateral_deposit_amount,
                user_accounts_owner: &user_accounts_owner,
                obligation: &usdc_obligation,
            },
        )
        .await;

    let borrow_amount =
        get_token_balance(&mut banks_client, usdc_reserve.user_liquidity_account).await;
    assert_eq!(borrow_amount, USDC_BORROW_AMOUNT_FRACTIONAL);

    let borrow_fees = TEST_RESERVE_CONFIG
        .fees
        .calculate_borrow_fees(collateral_deposit_amount)
        .unwrap()
        .0;

    let collateral_supply =
        get_token_balance(&mut banks_client, sol_reserve.collateral_supply).await;
    assert_eq!(collateral_supply, collateral_deposit_amount - borrow_fees);

    lending_market
        .borrow(
            &mut banks_client,
            &payer,
            BorrowArgs {
                deposit_reserve: &sol_reserve,
                borrow_reserve: &usdc_reserve,
                dex_market: &sol_usdc_dex_market,
                borrow_amount_type: BorrowAmountType::LiquidityBorrowAmount,
                amount: borrow_amount,
                user_accounts_owner: &user_accounts_owner,
                obligation: &usdc_obligation,
            },
        )
        .await;

    let borrow_amount =
        get_token_balance(&mut banks_client, usdc_reserve.user_liquidity_account).await;
    assert_eq!(borrow_amount, 2 * USDC_BORROW_AMOUNT_FRACTIONAL);

    // The SOL reserve accumulates interest which slightly increases the value
    // of collateral, resulting in slightly less collateral required for new loans
    const COLLATERAL_EPSILON: u64 = 1;
    let user_collateral_balance =
        get_token_balance(&mut banks_client, sol_reserve.user_collateral_account).await;
    assert_eq!(user_collateral_balance, COLLATERAL_EPSILON);

    let collateral_deposited = 2 * collateral_deposit_amount - COLLATERAL_EPSILON;
    let (total_fee, host_fee) = TEST_RESERVE_CONFIG
        .fees
        .calculate_borrow_fees(collateral_deposited)
        .unwrap();

    assert!(total_fee > 0);
    assert!(host_fee > 0);

    let collateral_supply =
        get_token_balance(&mut banks_client, sol_reserve.collateral_supply).await;
    assert_eq!(collateral_supply, collateral_deposited - total_fee);

    let fee_balance =
        get_token_balance(&mut banks_client, sol_reserve.collateral_fees_receiver).await;
    assert_eq!(fee_balance, total_fee - host_fee);

    let host_fee_balance = get_token_balance(&mut banks_client, sol_reserve.collateral_host).await;
    assert_eq!(host_fee_balance, host_fee);
}

#[tokio::test]
async fn test_borrow_base_currency() {
    // Using SOL/USDC min 3 asks:
    //  $2.212, 1825.6 SOL
    //  $2.211,  300.0 SOL
    //  $2.210,  212.5 SOL
    //
    // Borrow amount = 600 SOL
    // Collateral amount = 2.21 * 212.5 + 2.211 * 300 + 2.212 * 87.5 = 1,326.475 USDC
    const SOL_BORROW_AMOUNT_LAMPORTS: u64 = 600 * LAMPORTS_TO_SOL;
    const USDC_COLLATERAL_LAMPORTS: u64 = 1_326_475_000;
    const INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS: u64 = 5000 * LAMPORTS_TO_SOL;
    const INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL: u64 = 2 * USDC_COLLATERAL_LAMPORTS;

    // The USDC collateral reserve accumulates interest which slightly increases
    // the value of collateral, resulting in additional borrow power
    const INTEREST_EPSILON: u64 = 2;

    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(188_000);

    let user_accounts_owner = Keypair::new();
    let sol_usdc_dex_market = TestDexMarket::setup(&mut test, TestDexMarketPair::SOL_USDC);
    let usdc_mint = add_usdc_mint(&mut test);
    let lending_market = add_lending_market(&mut test, usdc_mint.pubkey);

    let mut reserve_config = TEST_RESERVE_CONFIG;
    reserve_config.loan_to_value_ratio = 100;

    // Configure reserve to a fixed borrow rate of 1%
    reserve_config.min_borrow_rate = 1;
    reserve_config.optimal_borrow_rate = 1;
    reserve_config.optimal_utilization_rate = 100;

    let usdc_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            slots_elapsed: SLOTS_PER_YEAR,
            borrow_amount: 1, // slightly increase collateral value w/ interest accrual
            liquidity_amount: INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let sol_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            slots_elapsed: SLOTS_PER_YEAR,
            dex_market_pubkey: Some(sol_usdc_dex_market.pubkey),
            liquidity_amount: INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            liquidity_mint_decimals: 9,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let sol_obligation = add_obligation(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddObligationArgs {
            borrow_reserve: &sol_reserve,
            collateral_reserve: &usdc_reserve,
            collateral_amount: 0,
            borrowed_liquidity_wads: Decimal::zero(),
        },
    );

    let (mut banks_client, payer, _recent_blockhash) = test.start().await;

    let borrow_amount =
        get_token_balance(&mut banks_client, sol_reserve.user_liquidity_account).await;
    assert_eq!(borrow_amount, 0);

    let collateral_supply =
        get_token_balance(&mut banks_client, usdc_reserve.collateral_supply).await;
    assert_eq!(collateral_supply, 0);

    let collateral_deposit_amount = INITIAL_COLLATERAL_RATIO * USDC_COLLATERAL_LAMPORTS;
    lending_market
        .borrow(
            &mut banks_client,
            &payer,
            BorrowArgs {
                deposit_reserve: &usdc_reserve,
                borrow_reserve: &sol_reserve,
                dex_market: &sol_usdc_dex_market,
                borrow_amount_type: BorrowAmountType::CollateralDepositAmount,
                amount: collateral_deposit_amount,
                user_accounts_owner: &user_accounts_owner,
                obligation: &sol_obligation,
            },
        )
        .await;

    let borrow_amount =
        get_token_balance(&mut banks_client, sol_reserve.user_liquidity_account).await;
    assert_eq!(borrow_amount, SOL_BORROW_AMOUNT_LAMPORTS + INTEREST_EPSILON);

    let borrow_fees = TEST_RESERVE_CONFIG
        .fees
        .calculate_borrow_fees(collateral_deposit_amount)
        .unwrap()
        .0;

    let collateral_supply =
        get_token_balance(&mut banks_client, usdc_reserve.collateral_supply).await;
    assert_eq!(collateral_supply, collateral_deposit_amount - borrow_fees);

    lending_market
        .borrow(
            &mut banks_client,
            &payer,
            BorrowArgs {
                deposit_reserve: &usdc_reserve,
                borrow_reserve: &sol_reserve,
                dex_market: &sol_usdc_dex_market,
                borrow_amount_type: BorrowAmountType::LiquidityBorrowAmount,
                amount: borrow_amount - INTEREST_EPSILON,
                user_accounts_owner: &user_accounts_owner,
                obligation: &sol_obligation,
            },
        )
        .await;

    let borrow_amount =
        get_token_balance(&mut banks_client, sol_reserve.user_liquidity_account).await;
    assert_eq!(
        borrow_amount,
        2 * SOL_BORROW_AMOUNT_LAMPORTS + INTEREST_EPSILON
    );

    let (mut total_fee, mut host_fee) = TEST_RESERVE_CONFIG
        .fees
        .calculate_borrow_fees(collateral_deposit_amount)
        .unwrap();

    // avoid rounding error by assessing fees individually
    total_fee *= 2;
    host_fee *= 2;

    assert!(total_fee > 0);
    assert!(host_fee > 0);

    let collateral_supply =
        get_token_balance(&mut banks_client, usdc_reserve.collateral_supply).await;
    assert_eq!(collateral_supply, 2 * collateral_deposit_amount - total_fee);

    let fee_balance =
        get_token_balance(&mut banks_client, usdc_reserve.collateral_fees_receiver).await;
    assert_eq!(fee_balance, total_fee - host_fee);

    let host_fee_balance = get_token_balance(&mut banks_client, usdc_reserve.collateral_host).await;
    assert_eq!(host_fee_balance, host_fee);
}
