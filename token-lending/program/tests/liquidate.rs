#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program_test::*;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use spl_token_lending::{
    math::Decimal, processor::process_instruction, state::INITIAL_COLLATERAL_RATIO,
};

const LAMPORTS_TO_SOL: u64 = 1_000_000_000;
const FRACTIONAL_TO_USDC: u64 = 1_000_000;

const INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS: u64 = 100 * LAMPORTS_TO_SOL;
const INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL: u64 = 100 * FRACTIONAL_TO_USDC;

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(200_000);

    // set loan values to about 90% of collateral value so that it gets liquidated
    // assumes SOL is ~$14
    const USDC_LOAN: u64 = 12 * FRACTIONAL_TO_USDC;
    const USDC_LOAN_SOL_COLLATERAL: u64 = INITIAL_COLLATERAL_RATIO * LAMPORTS_TO_SOL;

    const SOL_LOAN: u64 = LAMPORTS_TO_SOL;
    const SOL_LOAN_USDC_COLLATERAL: u64 = 12 * INITIAL_COLLATERAL_RATIO * FRACTIONAL_TO_USDC;

    let user_accounts_owner = Keypair::new();
    let sol_usdc_dex_market = TestDexMarket::setup(&mut test, TestDexMarketPair::SOL_USDC);
    let usdc_mint = add_usdc_mint(&mut test);
    let lending_market = add_lending_market(&mut test, usdc_mint.pubkey);

    // Loans are unhealthy if borrow is more than 80% of collateral
    let mut reserve_config = TEST_RESERVE_CONFIG;
    reserve_config.liquidation_threshold = 80;

    let usdc_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            config: reserve_config,
            initial_borrow_rate: 1,
            liquidity_amount: INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            borrow_amount: USDC_LOAN * 101 / 100,
            user_liquidity_amount: USDC_LOAN,
            collateral_amount: SOL_LOAN_USDC_COLLATERAL,
            ..AddReserveArgs::default()
        },
    );

    let sol_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            config: reserve_config,
            initial_borrow_rate: 1,
            liquidity_amount: INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS,
            liquidity_mint_decimals: 9,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            dex_market_pubkey: Some(sol_usdc_dex_market.pubkey),
            collateral_amount: USDC_LOAN_SOL_COLLATERAL,
            borrow_amount: SOL_LOAN * 101 / 100,
            user_liquidity_amount: SOL_LOAN,
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
            collateral_amount: USDC_LOAN_SOL_COLLATERAL,
            borrowed_liquidity_wads: Decimal::from(USDC_LOAN),
        },
    );

    let sol_obligation = add_obligation(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddObligationArgs {
            borrow_reserve: &sol_reserve,
            collateral_reserve: &usdc_reserve,
            collateral_amount: SOL_LOAN_USDC_COLLATERAL,
            borrowed_liquidity_wads: Decimal::from(SOL_LOAN),
        },
    );

    let (mut banks_client, payer, _recent_blockhash) = test.start().await;

    lending_market
        .liquidate(
            &mut banks_client,
            &payer,
            LiquidateArgs {
                repay_reserve: &usdc_reserve,
                withdraw_reserve: &sol_reserve,
                dex_market: &sol_usdc_dex_market,
                amount: USDC_LOAN,
                user_accounts_owner: &user_accounts_owner,
                obligation: &usdc_obligation,
            },
        )
        .await;

    lending_market
        .liquidate(
            &mut banks_client,
            &payer,
            LiquidateArgs {
                repay_reserve: &sol_reserve,
                withdraw_reserve: &usdc_reserve,
                dex_market: &sol_usdc_dex_market,
                amount: SOL_LOAN,
                user_accounts_owner: &user_accounts_owner,
                obligation: &sol_obligation,
            },
        )
        .await;

    let usdc_liquidity_supply =
        get_token_balance(&mut banks_client, usdc_reserve.liquidity_supply).await;
    let usdc_loan_state = usdc_obligation.get_state(&mut banks_client).await;
    let usdc_liquidated = usdc_liquidity_supply - INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL;
    assert!(usdc_liquidated > USDC_LOAN / 2);
    assert_eq!(
        usdc_liquidated,
        usdc_loan_state
            .borrowed_liquidity_wads
            .try_floor_u64()
            .unwrap()
    );

    let sol_liquidity_supply =
        get_token_balance(&mut banks_client, sol_reserve.liquidity_supply).await;
    let sol_loan_state = sol_obligation.get_state(&mut banks_client).await;
    let sol_liquidated = sol_liquidity_supply - INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS;
    assert!(sol_liquidated > SOL_LOAN / 2);
    assert_eq!(
        sol_liquidated,
        sol_loan_state
            .borrowed_liquidity_wads
            .try_floor_u64()
            .unwrap()
    );
}
