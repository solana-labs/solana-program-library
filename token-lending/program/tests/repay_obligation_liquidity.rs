#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_token::instruction::approve;
use spl_token_lending::instruction::refresh_obligation;
use spl_token_lending::{
    instruction::repay_obligation_liquidity, processor::process_instruction,
    state::INITIAL_COLLATERAL_RATIO,
};

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_compute_max_units(36_000);

    const SOL_DEPOSIT_AMOUNT_LAMPORTS: u64 = 100 * LAMPORTS_TO_SOL * INITIAL_COLLATERAL_RATIO;
    const USDC_BORROW_AMOUNT_FRACTIONAL: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const SOL_RESERVE_COLLATERAL_LAMPORTS: u64 = 2 * SOL_DEPOSIT_AMOUNT_LAMPORTS;
    const USDC_RESERVE_LIQUIDITY_FRACTIONAL: u64 = 2 * USDC_BORROW_AMOUNT_FRACTIONAL;

    let user_accounts_owner = Keypair::new();
    let user_transfer_authority = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut reserve_config = TEST_RESERVE_CONFIG;
    reserve_config.loan_to_value_ratio = 50;

    let sol_oracle = add_sol_oracle(&mut test);
    let sol_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &sol_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            collateral_amount: SOL_RESERVE_COLLATERAL_LAMPORTS,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            liquidity_mint_decimals: 9,
            config: reserve_config,
            mark_fresh: true,
            ..AddReserveArgs::default()
        },
    );

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            borrow_amount: USDC_BORROW_AMOUNT_FRACTIONAL,
            user_liquidity_amount: USDC_BORROW_AMOUNT_FRACTIONAL,
            liquidity_amount: USDC_RESERVE_LIQUIDITY_FRACTIONAL,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            mark_fresh: true,
            ..AddReserveArgs::default()
        },
    );

    let test_obligation = add_obligation(
        &mut test,
        &lending_market,
        &user_accounts_owner,
        AddObligationArgs {
            deposits: &[(&sol_test_reserve, SOL_DEPOSIT_AMOUNT_LAMPORTS)],
            borrows: &[(&usdc_test_reserve, USDC_BORROW_AMOUNT_FRACTIONAL)],
            ..AddObligationArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let initial_user_liquidity_balance =
        get_token_balance(&mut banks_client, usdc_test_reserve.user_liquidity_pubkey).await;
    let initial_liquidity_supply_balance =
        get_token_balance(&mut banks_client, usdc_test_reserve.liquidity_supply_pubkey).await;

    let mut transaction = Transaction::new_with_payer(
        &[
            approve(
                &spl_token::id(),
                &usdc_test_reserve.user_liquidity_pubkey,
                &user_transfer_authority.pubkey(),
                &user_accounts_owner.pubkey(),
                &[],
                USDC_BORROW_AMOUNT_FRACTIONAL,
            )
            .unwrap(),
            refresh_obligation(
                spl_token_lending::id(),
                test_obligation.pubkey,
                vec![sol_test_reserve.pubkey, usdc_test_reserve.pubkey],
            ),
            repay_obligation_liquidity(
                spl_token_lending::id(),
                USDC_BORROW_AMOUNT_FRACTIONAL,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.pubkey,
                test_obligation.pubkey,
                lending_market.pubkey,
                user_transfer_authority.pubkey(),
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(
        &[&payer, &user_accounts_owner, &user_transfer_authority],
        recent_blockhash,
    );
    assert!(banks_client.process_transaction(transaction).await.is_ok());

    let user_liquidity_balance =
        get_token_balance(&mut banks_client, usdc_test_reserve.user_liquidity_pubkey).await;
    assert_eq!(
        user_liquidity_balance,
        initial_user_liquidity_balance - USDC_BORROW_AMOUNT_FRACTIONAL
    );

    let liquidity_supply_balance =
        get_token_balance(&mut banks_client, usdc_test_reserve.liquidity_supply_pubkey).await;
    assert_eq!(
        liquidity_supply_balance,
        initial_liquidity_supply_balance + USDC_BORROW_AMOUNT_FRACTIONAL
    );

    let obligation = test_obligation.get_state(&mut banks_client).await;
    assert_eq!(obligation.borrows.len(), 0);
}
