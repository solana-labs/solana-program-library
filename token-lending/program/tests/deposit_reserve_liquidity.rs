#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program_test::*;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use spl_token_lending::processor::process_instruction;

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(50_000);

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: 100 * FRACTIONAL_TO_USDC,
            liquidity_amount: 10_000 * FRACTIONAL_TO_USDC,
            liquidity_mint_decimals: usdc_mint.decimals,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            borrow_amount: 5_000 * FRACTIONAL_TO_USDC,
            config: test_reserve_config(),
            mark_fresh: true,
            ..AddReserveArgs::default()
        },
    );

    let mut test_context = test.start_with_context().await;
    test_context.warp_to_slot(300).unwrap(); // clock.slot = 300

    let ProgramTestContext {
        mut banks_client,
        payer,
        ..
    } = test_context;

    let initial_ctoken_amount =
        get_token_balance(&mut banks_client, usdc_test_reserve.user_collateral_pubkey).await;
    let pre_usdc_reserve = usdc_test_reserve.get_state(&mut banks_client).await;
    let old_borrow_rate = pre_usdc_reserve.liquidity.cumulative_borrow_rate_wads;

    lending_market
        .deposit(
            &mut banks_client,
            &user_accounts_owner,
            &payer,
            &usdc_test_reserve,
            100 * FRACTIONAL_TO_USDC,
        )
        .await;

    let usdc_reserve = usdc_test_reserve.get_state(&mut banks_client).await;
    assert_eq!(usdc_reserve.last_update.stale, true);

    let user_remaining_liquidity_amount =
        get_token_balance(&mut banks_client, usdc_test_reserve.user_liquidity_pubkey).await;
    assert_eq!(user_remaining_liquidity_amount, 0);

    let final_ctoken_amount =
        get_token_balance(&mut banks_client, usdc_test_reserve.user_collateral_pubkey).await;
    assert!(final_ctoken_amount - initial_ctoken_amount < 100 * FRACTIONAL_TO_USDC);

    assert!(usdc_reserve.liquidity.cumulative_borrow_rate_wads > old_borrow_rate);
}
