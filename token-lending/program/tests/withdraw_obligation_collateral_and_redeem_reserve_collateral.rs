#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solend_program::processor::process_instruction;

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "solend_program",
        solend_program::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_compute_max_units(70_000);

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
            config: test_reserve_config(),
            mark_fresh: true,
            ..AddReserveArgs::default()
        },
    );

    let test_obligation = add_obligation(
        &mut test,
        &lending_market,
        &user_accounts_owner,
        AddObligationArgs::default(),
    );

    let mut test_context = test.start_with_context().await;
    test_context.warp_to_slot(240).unwrap(); // clock.slot = 240

    let ProgramTestContext {
        mut banks_client,
        payer,
        last_blockhash: _recent_blockhash,
        ..
    } = test_context;

    test_obligation.validate_state(&mut banks_client).await;

    lending_market
        .deposit_obligation_and_collateral(
            &mut banks_client,
            &user_accounts_owner,
            &payer,
            &usdc_test_reserve,
            &test_obligation,
            100 * FRACTIONAL_TO_USDC,
        )
        .await;

    let usdc_reserve = usdc_test_reserve.get_state(&mut banks_client).await;
    assert!(usdc_reserve.last_update.stale);

    let user_liquidity_balance =
        get_token_balance(&mut banks_client, usdc_test_reserve.user_liquidity_pubkey).await;
    assert_eq!(user_liquidity_balance, 0);
    let liquidity_supply =
        get_token_balance(&mut banks_client, usdc_test_reserve.liquidity_supply_pubkey).await;
    assert_eq!(liquidity_supply, 10_100 * FRACTIONAL_TO_USDC);

    lending_market
        .refresh_reserve(&mut banks_client, &payer, &usdc_test_reserve)
        .await;

    lending_market
        .withdraw_and_redeem_collateral(
            &mut banks_client,
            &user_accounts_owner,
            &payer,
            &usdc_test_reserve,
            &test_obligation,
            50 * FRACTIONAL_TO_USDC,
        )
        .await;

    let usdc_reserve = usdc_test_reserve.get_state(&mut banks_client).await;
    assert!(usdc_reserve.last_update.stale);

    let user_liquidity_balance =
        get_token_balance(&mut banks_client, usdc_test_reserve.user_liquidity_pubkey).await;
    assert_eq!(user_liquidity_balance, 50 * FRACTIONAL_TO_USDC);
    let liquidity_supply =
        get_token_balance(&mut banks_client, usdc_test_reserve.liquidity_supply_pubkey).await;
    assert_eq!(liquidity_supply, 10_050 * FRACTIONAL_TO_USDC);
}
