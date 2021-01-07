#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program_test::*;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use spl_token_lending::{
    instruction::BorrowAmountType, processor::process_instruction, state::INITIAL_COLLATERAL_RATE,
};

const LAMPORTS_TO_SOL: u64 = 1_000_000_000;
const FRACTIONAL_TO_USDC: u64 = 1_000_000;

// Market and collateral are setup to fill two orders in the dex market at an average
// price of 2210.5
const fn lamports_to_usdc_fractional(lamports: u64) -> u64 {
    lamports / LAMPORTS_TO_SOL * (2210 + 2211) / 2 * FRACTIONAL_TO_USDC / 1000
}

const INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS: u64 = 42_500 * LAMPORTS_TO_SOL;
const INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL: u64 =
    lamports_to_usdc_fractional(INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS);
const USER_SOL_COLLATERAL_LAMPORTS: u64 = 8_500 * LAMPORTS_TO_SOL;

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    let user_accounts_owner = Keypair::new();
    let sol_usdc_dex_market = TestDexMarket::setup(&mut test, TestDexMarketPair::SOL_USDC);
    let usdc_mint = add_usdc_mint(&mut test);
    let lending_market = add_lending_market(&mut test, usdc_mint.pubkey);

    let usdc_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            liquidity_amount: INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: TEST_RESERVE_CONFIG,
            ..AddReserveArgs::default()
        },
    );

    let sol_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            dex_market_pubkey: Some(sol_usdc_dex_market.pubkey),
            liquidity_amount: INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            liquidity_mint_decimals: 9,
            config: TEST_RESERVE_CONFIG,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, _recent_blockhash) = test.start().await;

    let borrow_amount = INITIAL_COLLATERAL_RATE * USER_SOL_COLLATERAL_LAMPORTS;
    let obligation = lending_market
        .borrow(
            &mut banks_client,
            &payer,
            BorrowArgs {
                deposit_reserve: &sol_reserve,
                borrow_reserve: &usdc_reserve,
                dex_market: &sol_usdc_dex_market,
                borrow_amount_type: BorrowAmountType::CollateralDepositAmount,
                amount: borrow_amount / 2,
                user_accounts_owner: &user_accounts_owner,
                obligation: None,
            },
        )
        .await;

    lending_market
        .borrow(
            &mut banks_client,
            &payer,
            BorrowArgs {
                deposit_reserve: &sol_reserve,
                borrow_reserve: &usdc_reserve,
                dex_market: &sol_usdc_dex_market,
                borrow_amount_type: BorrowAmountType::CollateralDepositAmount,
                amount: borrow_amount / 2,
                user_accounts_owner: &user_accounts_owner,
                obligation: Some(obligation),
            },
        )
        .await;

    // check that fee accounts have been properly credited
    let (total_fee, host_fee) = TEST_RESERVE_CONFIG
        .fees
        .calculate_borrow_fees(borrow_amount)
        .unwrap();

    assert!(total_fee > 0);
    assert!(host_fee > 0);

    let sol_collateral_supply =
        get_token_balance(&mut banks_client, sol_reserve.collateral_supply).await;
    assert_eq!(sol_collateral_supply, borrow_amount - total_fee);

    let sol_fee_balance =
        get_token_balance(&mut banks_client, sol_reserve.collateral_fees_receiver).await;
    assert_eq!(sol_fee_balance, total_fee - host_fee);

    let sol_host_balance = get_token_balance(&mut banks_client, sol_reserve.collateral_host).await;
    assert_eq!(sol_host_balance, host_fee);
}
