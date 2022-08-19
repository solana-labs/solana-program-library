#![cfg(feature = "test-bpf")]

mod helpers;

use std::str::FromStr;

use helpers::*;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solend_program::{
    instruction::{redeem_fees, refresh_reserve},
    math::{Decimal, Rate, TryAdd, TryDiv, TryMul, TrySub},
    processor::process_instruction,
    state::SLOTS_PER_YEAR,
};

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "solend_program",
        solend_program::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_compute_max_units(228_000);

    const SOL_RESERVE_LIQUIDITY_LAMPORTS: u64 = 100000000 * LAMPORTS_TO_SOL;
    const USDC_RESERVE_LIQUIDITY_FRACTIONAL: u64 = 100000 * FRACTIONAL_TO_USDC;
    const BORROW_AMOUNT: u64 = 100000;
    const SLOTS_ELAPSED: u64 = 69420;

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut usdc_reserve_config = test_reserve_config();
    usdc_reserve_config.loan_to_value_ratio = 80;

    // Configure reserve to a fixed borrow rate of 200%
    const BORROW_RATE: u8 = 250;
    usdc_reserve_config.min_borrow_rate = BORROW_RATE;
    usdc_reserve_config.optimal_borrow_rate = BORROW_RATE;
    usdc_reserve_config.optimal_utilization_rate = 100;

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_oracle(
        &mut test,
        Pubkey::from_str(SRM_PYTH_PRODUCT).unwrap(),
        Pubkey::from_str(SRM_PYTH_PRICE).unwrap(),
        Pubkey::from_str(SRM_SWITCHBOARD_FEED).unwrap(),
        // Set USDC price to $1
        Decimal::from(1u64),
        SLOTS_ELAPSED,
    );
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            borrow_amount: BORROW_AMOUNT,
            liquidity_amount: USDC_RESERVE_LIQUIDITY_FRACTIONAL,
            liquidity_mint_decimals: usdc_mint.decimals,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            config: usdc_reserve_config,
            slots_elapsed: 1, // elapsed from 1; clock.slot = 2
            ..AddReserveArgs::default()
        },
    );

    let mut sol_reserve_config = test_reserve_config();
    sol_reserve_config.loan_to_value_ratio = 80;

    // Configure reserve to a fixed borrow rate of 1%
    sol_reserve_config.min_borrow_rate = BORROW_RATE;
    sol_reserve_config.optimal_borrow_rate = BORROW_RATE;
    sol_reserve_config.optimal_utilization_rate = 100;
    let sol_oracle = add_oracle(
        &mut test,
        Pubkey::from_str(SOL_PYTH_PRODUCT).unwrap(),
        Pubkey::from_str(SOL_PYTH_PRICE).unwrap(),
        Pubkey::from_str(SOL_SWITCHBOARD_FEED).unwrap(),
        // Set SOL price to $20
        Decimal::from(20u64),
        SLOTS_ELAPSED,
    );
    let sol_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &sol_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            borrow_amount: BORROW_AMOUNT,
            liquidity_amount: SOL_RESERVE_LIQUIDITY_LAMPORTS,
            liquidity_mint_decimals: 9,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            config: sol_reserve_config,
            slots_elapsed: 1, // elapsed from 1; clock.slot = 2
            ..AddReserveArgs::default()
        },
    );

    let mut test_context = test.start_with_context().await;
    test_context.warp_to_slot(2 + SLOTS_ELAPSED).unwrap(); // clock.slot = 100

    let ProgramTestContext {
        mut banks_client,
        payer,
        last_blockhash: recent_blockhash,
        ..
    } = test_context;

    let mut transaction = Transaction::new_with_payer(
        &[
            refresh_reserve(
                solend_program::id(),
                usdc_test_reserve.pubkey,
                usdc_oracle.pyth_price_pubkey,
                usdc_oracle.switchboard_feed_pubkey,
            ),
            refresh_reserve(
                solend_program::id(),
                sol_test_reserve.pubkey,
                sol_oracle.pyth_price_pubkey,
                sol_oracle.switchboard_feed_pubkey,
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer], recent_blockhash);
    assert!(banks_client.process_transaction(transaction).await.is_ok());

    let sol_reserve_before = sol_test_reserve.get_state(&mut banks_client).await;
    let usdc_reserve_before = usdc_test_reserve.get_state(&mut banks_client).await;
    let sol_balance_before =
        get_token_balance(&mut banks_client, sol_reserve_before.config.fee_receiver).await;
    let usdc_balance_before =
        get_token_balance(&mut banks_client, usdc_reserve_before.config.fee_receiver).await;

    let mut transaction2 = Transaction::new_with_payer(
        &[
            redeem_fees(
                solend_program::id(),
                usdc_test_reserve.pubkey,
                usdc_test_reserve.config.fee_receiver,
                usdc_test_reserve.liquidity_supply_pubkey,
                lending_market.pubkey,
            ),
            redeem_fees(
                solend_program::id(),
                sol_test_reserve.pubkey,
                sol_test_reserve.config.fee_receiver,
                sol_test_reserve.liquidity_supply_pubkey,
                lending_market.pubkey,
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction2.sign(&[&payer], recent_blockhash);
    assert!(banks_client.process_transaction(transaction2).await.is_ok());

    let sol_reserve = sol_test_reserve.get_state(&mut banks_client).await;
    let usdc_reserve = usdc_test_reserve.get_state(&mut banks_client).await;
    let sol_balance_after =
        get_token_balance(&mut banks_client, sol_reserve.config.fee_receiver).await;
    let usdc_balance_after =
        get_token_balance(&mut banks_client, usdc_reserve.config.fee_receiver).await;

    let slot_rate = Rate::from_percent(BORROW_RATE)
        .try_div(SLOTS_PER_YEAR)
        .unwrap();
    let compound_rate = Rate::one()
        .try_add(slot_rate)
        .unwrap()
        .try_pow(SLOTS_ELAPSED)
        .unwrap();
    let compound_borrow = Decimal::from(BORROW_AMOUNT).try_mul(compound_rate).unwrap();

    let net_new_debt = compound_borrow
        .try_sub(Decimal::from(BORROW_AMOUNT))
        .unwrap();
    let protocol_take_rate = Rate::from_percent(sol_test_reserve.config.protocol_take_rate);
    let delta_accumulated_protocol_fees = net_new_debt.try_mul(protocol_take_rate).unwrap();

    assert_eq!(
        usdc_reserve_before.liquidity.total_supply(),
        usdc_reserve.liquidity.total_supply(),
    );
    assert_eq!(
        sol_reserve_before.liquidity.total_supply(),
        sol_reserve.liquidity.total_supply(),
    );
    assert_eq!(
        Rate::from(usdc_reserve_before.collateral_exchange_rate().unwrap()),
        Rate::from(usdc_reserve.collateral_exchange_rate().unwrap()),
    );
    assert_eq!(
        Rate::from(sol_reserve_before.collateral_exchange_rate().unwrap()),
        Rate::from(sol_reserve.collateral_exchange_rate().unwrap()),
    );

    // utilization increases because redeeming adds to borrows and takes from availible
    assert_eq!(
        usdc_reserve_before.liquidity.utilization_rate().unwrap(),
        usdc_reserve.liquidity.utilization_rate().unwrap(),
    );
    assert_eq!(
        sol_reserve_before.liquidity.utilization_rate().unwrap(),
        sol_reserve.liquidity.utilization_rate().unwrap(),
    );
    assert_eq!(
        sol_reserve.liquidity.cumulative_borrow_rate_wads,
        compound_rate.into()
    );
    assert_eq!(
        sol_reserve.liquidity.cumulative_borrow_rate_wads,
        usdc_reserve.liquidity.cumulative_borrow_rate_wads
    );
    assert_eq!(sol_reserve.liquidity.borrowed_amount_wads, compound_borrow);
    assert_eq!(usdc_reserve.liquidity.borrowed_amount_wads, compound_borrow);
    assert_eq!(
        Decimal::from(delta_accumulated_protocol_fees.try_floor_u64().unwrap()),
        usdc_reserve_before
            .liquidity
            .accumulated_protocol_fees_wads
            .try_sub(usdc_reserve.liquidity.accumulated_protocol_fees_wads)
            .unwrap()
    );
    assert_eq!(
        Decimal::from(delta_accumulated_protocol_fees.try_floor_u64().unwrap()),
        sol_reserve_before
            .liquidity
            .accumulated_protocol_fees_wads
            .try_sub(sol_reserve.liquidity.accumulated_protocol_fees_wads)
            .unwrap()
    );
    assert_eq!(
        usdc_reserve_before.liquidity.accumulated_protocol_fees_wads,
        delta_accumulated_protocol_fees
    );
    assert_eq!(
        sol_reserve_before.liquidity.accumulated_protocol_fees_wads,
        delta_accumulated_protocol_fees
    );
    assert_eq!(
        usdc_reserve_before
            .liquidity
            .accumulated_protocol_fees_wads
            .try_floor_u64()
            .unwrap(),
        usdc_balance_after - usdc_balance_before
    );
    assert_eq!(
        usdc_reserve.liquidity.accumulated_protocol_fees_wads,
        usdc_reserve_before
            .liquidity
            .accumulated_protocol_fees_wads
            .try_sub(Decimal::from(usdc_balance_after - usdc_balance_before))
            .unwrap()
    );
    assert_eq!(
        sol_reserve_before
            .liquidity
            .accumulated_protocol_fees_wads
            .try_floor_u64()
            .unwrap(),
        sol_balance_after - sol_balance_before
    );
    assert_eq!(
        sol_reserve.liquidity.accumulated_protocol_fees_wads,
        sol_reserve_before
            .liquidity
            .accumulated_protocol_fees_wads
            .try_sub(Decimal::from(sol_balance_after - sol_balance_before))
            .unwrap()
    );
    assert_eq!(
        sol_reserve.liquidity.borrowed_amount_wads,
        usdc_reserve.liquidity.borrowed_amount_wads
    );
    assert_eq!(
        sol_reserve.liquidity.market_price,
        sol_test_reserve.market_price
    );
    assert_eq!(
        usdc_reserve.liquidity.market_price,
        usdc_test_reserve.market_price
    );
}
