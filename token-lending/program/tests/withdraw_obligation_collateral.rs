#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program_test::*;
use solana_sdk::{
    instruction::InstructionError,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::{Transaction, TransactionError},
};
use spl_token_lending::{
    error::LendingError,
    instruction::{refresh_obligation, withdraw_obligation_collateral},
    processor::process_instruction,
    state::INITIAL_COLLATERAL_RATIO,
};
use std::u64;

#[tokio::test]
async fn test_withdraw_fixed_amount() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(33_000);

    const SOL_DEPOSIT_AMOUNT_LAMPORTS: u64 = 200 * LAMPORTS_TO_SOL * INITIAL_COLLATERAL_RATIO;
    const USDC_BORROW_AMOUNT_FRACTIONAL: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const SOL_RESERVE_COLLATERAL_LAMPORTS: u64 = 2 * SOL_DEPOSIT_AMOUNT_LAMPORTS;
    const WITHDRAW_AMOUNT: u64 = 100 * LAMPORTS_TO_SOL * INITIAL_COLLATERAL_RATIO;

    let user_accounts_owner = Keypair::new();
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
            liquidity_amount: USDC_BORROW_AMOUNT_FRACTIONAL,
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

    let test_collateral = &test_obligation.deposits[0];
    let test_liquidity = &test_obligation.borrows[0];

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    test_obligation.validate_state(&mut banks_client).await;
    test_collateral.validate_state(&mut banks_client).await;
    test_liquidity.validate_state(&mut banks_client).await;

    let initial_collateral_supply_balance =
        get_token_balance(&mut banks_client, sol_test_reserve.collateral_supply_pubkey).await;
    let initial_user_collateral_balance =
        get_token_balance(&mut banks_client, sol_test_reserve.user_collateral_pubkey).await;

    let mut transaction = Transaction::new_with_payer(
        &[
            refresh_obligation(
                spl_token_lending::id(),
                test_obligation.pubkey,
                vec![sol_test_reserve.pubkey, usdc_test_reserve.pubkey],
            ),
            withdraw_obligation_collateral(
                spl_token_lending::id(),
                WITHDRAW_AMOUNT,
                sol_test_reserve.collateral_supply_pubkey,
                sol_test_reserve.user_collateral_pubkey,
                sol_test_reserve.pubkey,
                test_obligation.pubkey,
                lending_market.pubkey,
                test_obligation.owner,
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);
    assert!(banks_client.process_transaction(transaction).await.is_ok());

    // check that collateral tokens were transferred
    let collateral_supply_balance =
        get_token_balance(&mut banks_client, sol_test_reserve.collateral_supply_pubkey).await;
    assert_eq!(
        collateral_supply_balance,
        initial_collateral_supply_balance - WITHDRAW_AMOUNT
    );
    let user_collateral_balance =
        get_token_balance(&mut banks_client, sol_test_reserve.user_collateral_pubkey).await;
    assert_eq!(
        user_collateral_balance,
        initial_user_collateral_balance + WITHDRAW_AMOUNT
    );

    let obligation = test_obligation.get_state(&mut banks_client).await;
    let collateral = &obligation.deposits[0];
    assert_eq!(
        collateral.deposited_amount,
        SOL_DEPOSIT_AMOUNT_LAMPORTS - WITHDRAW_AMOUNT
    );
}

#[tokio::test]
async fn test_withdraw_max_amount() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(28_000);

    const USDC_DEPOSIT_AMOUNT_FRACTIONAL: u64 =
        1_000 * FRACTIONAL_TO_USDC * INITIAL_COLLATERAL_RATIO;
    const USDC_RESERVE_COLLATERAL_FRACTIONAL: u64 = 2 * USDC_DEPOSIT_AMOUNT_FRACTIONAL;
    const WITHDRAW_AMOUNT: u64 = u64::MAX;

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut reserve_config = TEST_RESERVE_CONFIG;
    reserve_config.loan_to_value_ratio = 50;

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            collateral_amount: USDC_RESERVE_COLLATERAL_FRACTIONAL,
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
            deposits: &[(&usdc_test_reserve, USDC_DEPOSIT_AMOUNT_FRACTIONAL)],
            ..AddObligationArgs::default()
        },
    );

    let test_collateral = &test_obligation.deposits[0];

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    test_obligation.validate_state(&mut banks_client).await;
    test_collateral.validate_state(&mut banks_client).await;

    let initial_collateral_supply_balance = get_token_balance(
        &mut banks_client,
        usdc_test_reserve.collateral_supply_pubkey,
    )
    .await;
    let initial_user_collateral_balance =
        get_token_balance(&mut banks_client, usdc_test_reserve.user_collateral_pubkey).await;

    let mut transaction = Transaction::new_with_payer(
        &[
            refresh_obligation(
                spl_token_lending::id(),
                test_obligation.pubkey,
                vec![usdc_test_reserve.pubkey],
            ),
            withdraw_obligation_collateral(
                spl_token_lending::id(),
                WITHDRAW_AMOUNT,
                usdc_test_reserve.collateral_supply_pubkey,
                usdc_test_reserve.user_collateral_pubkey,
                usdc_test_reserve.pubkey,
                test_obligation.pubkey,
                lending_market.pubkey,
                test_obligation.owner,
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);
    assert!(banks_client.process_transaction(transaction).await.is_ok());

    // check that collateral tokens were transferred
    let collateral_supply_balance = get_token_balance(
        &mut banks_client,
        usdc_test_reserve.collateral_supply_pubkey,
    )
    .await;
    assert_eq!(
        collateral_supply_balance,
        initial_collateral_supply_balance - USDC_DEPOSIT_AMOUNT_FRACTIONAL
    );
    let user_collateral_balance =
        get_token_balance(&mut banks_client, usdc_test_reserve.user_collateral_pubkey).await;
    assert_eq!(
        user_collateral_balance,
        initial_user_collateral_balance + USDC_DEPOSIT_AMOUNT_FRACTIONAL
    );

    let obligation = test_obligation.get_state(&mut banks_client).await;
    assert_eq!(obligation.deposits.len(), 0);
}

#[tokio::test]
async fn test_withdraw_too_large() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    const SOL_DEPOSIT_AMOUNT_LAMPORTS: u64 = 200 * LAMPORTS_TO_SOL * INITIAL_COLLATERAL_RATIO;
    const USDC_BORROW_AMOUNT_FRACTIONAL: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const SOL_RESERVE_COLLATERAL_LAMPORTS: u64 = 2 * SOL_DEPOSIT_AMOUNT_LAMPORTS;
    const WITHDRAW_AMOUNT: u64 = (100 * LAMPORTS_TO_SOL * INITIAL_COLLATERAL_RATIO) + 1;

    let user_accounts_owner = Keypair::new();
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
            liquidity_amount: USDC_BORROW_AMOUNT_FRACTIONAL,
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

    let mut transaction = Transaction::new_with_payer(
        &[
            refresh_obligation(
                spl_token_lending::id(),
                test_obligation.pubkey,
                vec![sol_test_reserve.pubkey, usdc_test_reserve.pubkey],
            ),
            withdraw_obligation_collateral(
                spl_token_lending::id(),
                WITHDRAW_AMOUNT,
                sol_test_reserve.collateral_supply_pubkey,
                sol_test_reserve.user_collateral_pubkey,
                sol_test_reserve.pubkey,
                test_obligation.pubkey,
                lending_market.pubkey,
                test_obligation.owner,
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);

    // check that transaction fails
    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            1,
            InstructionError::Custom(LendingError::WithdrawTooLarge as u32)
        )
    );
}
