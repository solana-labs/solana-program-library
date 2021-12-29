#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program_test::*;
use solana_sdk::{
    instruction::InstructionError,
    signature::{Keypair, Signer},
    transaction::{Transaction, TransactionError},
};
use spl_token_lending::{
    error::LendingError,
    instruction::{borrow_obligation_liquidity, refresh_obligation},
    math::Decimal,
    processor::process_instruction,
    state::{FeeCalculation, INITIAL_COLLATERAL_RATIO},
};
use std::u64;

#[tokio::test]
async fn test_borrow_usdc_fixed_amount() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_compute_max_units(60_000);

    const USDC_TOTAL_BORROW_FRACTIONAL: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const FEE_AMOUNT: u64 = 100;
    const HOST_FEE_AMOUNT: u64 = 20;

    const SOL_DEPOSIT_AMOUNT_LAMPORTS: u64 = 100 * LAMPORTS_TO_SOL * INITIAL_COLLATERAL_RATIO;
    const USDC_BORROW_AMOUNT_FRACTIONAL: u64 = USDC_TOTAL_BORROW_FRACTIONAL - FEE_AMOUNT;
    const SOL_RESERVE_COLLATERAL_LAMPORTS: u64 = 2 * SOL_DEPOSIT_AMOUNT_LAMPORTS;
    const USDC_RESERVE_LIQUIDITY_FRACTIONAL: u64 = 2 * USDC_TOTAL_BORROW_FRACTIONAL;

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
            ..AddObligationArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let initial_liquidity_supply =
        get_token_balance(&mut banks_client, usdc_test_reserve.liquidity_supply_pubkey).await;

    let mut transaction = Transaction::new_with_payer(
        &[
            refresh_obligation(
                spl_token_lending::id(),
                test_obligation.pubkey,
                vec![sol_test_reserve.pubkey],
            ),
            borrow_obligation_liquidity(
                spl_token_lending::id(),
                USDC_BORROW_AMOUNT_FRACTIONAL,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.pubkey,
                usdc_test_reserve.liquidity_fee_receiver_pubkey,
                test_obligation.pubkey,
                lending_market.pubkey,
                test_obligation.owner,
                Some(usdc_test_reserve.liquidity_host_pubkey),
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);
    assert!(banks_client.process_transaction(transaction).await.is_ok());

    let usdc_reserve = usdc_test_reserve.get_state(&mut banks_client).await;
    let obligation = test_obligation.get_state(&mut banks_client).await;

    let (total_fee, host_fee) = usdc_reserve
        .config
        .fees
        .calculate_borrow_fees(
            USDC_BORROW_AMOUNT_FRACTIONAL.into(),
            FeeCalculation::Exclusive,
        )
        .unwrap();
    assert_eq!(total_fee, FEE_AMOUNT);
    assert_eq!(host_fee, HOST_FEE_AMOUNT);

    let borrow_amount =
        get_token_balance(&mut banks_client, usdc_test_reserve.user_liquidity_pubkey).await;
    assert_eq!(borrow_amount, USDC_BORROW_AMOUNT_FRACTIONAL);

    let liquidity = &obligation.borrows[0];
    assert_eq!(
        liquidity.borrowed_amount_wads,
        Decimal::from(USDC_TOTAL_BORROW_FRACTIONAL)
    );
    assert_eq!(
        usdc_reserve.liquidity.borrowed_amount_wads,
        liquidity.borrowed_amount_wads
    );

    let liquidity_supply =
        get_token_balance(&mut banks_client, usdc_test_reserve.liquidity_supply_pubkey).await;
    assert_eq!(
        liquidity_supply,
        initial_liquidity_supply - USDC_TOTAL_BORROW_FRACTIONAL
    );

    let fee_balance = get_token_balance(
        &mut banks_client,
        usdc_test_reserve.liquidity_fee_receiver_pubkey,
    )
    .await;
    assert_eq!(fee_balance, FEE_AMOUNT - HOST_FEE_AMOUNT);

    let host_fee_balance =
        get_token_balance(&mut banks_client, usdc_test_reserve.liquidity_host_pubkey).await;
    assert_eq!(host_fee_balance, HOST_FEE_AMOUNT);
}

#[tokio::test]
async fn test_borrow_sol_max_amount() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_compute_max_units(60_000);

    const FEE_AMOUNT: u64 = 5000;
    const HOST_FEE_AMOUNT: u64 = 1000;

    const USDC_DEPOSIT_AMOUNT_FRACTIONAL: u64 =
        2_000 * FRACTIONAL_TO_USDC * INITIAL_COLLATERAL_RATIO;
    const SOL_BORROW_AMOUNT_LAMPORTS: u64 = 50 * LAMPORTS_TO_SOL;
    const USDC_RESERVE_COLLATERAL_FRACTIONAL: u64 = 2 * USDC_DEPOSIT_AMOUNT_FRACTIONAL;
    const SOL_RESERVE_LIQUIDITY_LAMPORTS: u64 = 2 * SOL_BORROW_AMOUNT_LAMPORTS;

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
            liquidity_amount: USDC_RESERVE_COLLATERAL_FRACTIONAL,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            mark_fresh: true,
            ..AddReserveArgs::default()
        },
    );

    let sol_oracle = add_sol_oracle(&mut test);
    let sol_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &sol_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            liquidity_amount: SOL_RESERVE_LIQUIDITY_LAMPORTS,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            liquidity_mint_decimals: 9,
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

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let initial_liquidity_supply =
        get_token_balance(&mut banks_client, sol_test_reserve.liquidity_supply_pubkey).await;

    let mut transaction = Transaction::new_with_payer(
        &[
            refresh_obligation(
                spl_token_lending::id(),
                test_obligation.pubkey,
                vec![usdc_test_reserve.pubkey],
            ),
            borrow_obligation_liquidity(
                spl_token_lending::id(),
                u64::MAX,
                sol_test_reserve.liquidity_supply_pubkey,
                sol_test_reserve.user_liquidity_pubkey,
                sol_test_reserve.pubkey,
                sol_test_reserve.liquidity_fee_receiver_pubkey,
                test_obligation.pubkey,
                lending_market.pubkey,
                test_obligation.owner,
                Some(sol_test_reserve.liquidity_host_pubkey),
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);
    assert!(banks_client.process_transaction(transaction).await.is_ok());

    let sol_reserve = sol_test_reserve.get_state(&mut banks_client).await;
    let obligation = test_obligation.get_state(&mut banks_client).await;

    let (total_fee, host_fee) = sol_reserve
        .config
        .fees
        .calculate_borrow_fees(SOL_BORROW_AMOUNT_LAMPORTS.into(), FeeCalculation::Inclusive)
        .unwrap();

    assert_eq!(total_fee, FEE_AMOUNT);
    assert_eq!(host_fee, HOST_FEE_AMOUNT);

    let borrow_amount =
        get_token_balance(&mut banks_client, sol_test_reserve.user_liquidity_pubkey).await;
    assert_eq!(borrow_amount, SOL_BORROW_AMOUNT_LAMPORTS - FEE_AMOUNT);

    let liquidity = &obligation.borrows[0];
    assert_eq!(
        liquidity.borrowed_amount_wads,
        Decimal::from(SOL_BORROW_AMOUNT_LAMPORTS)
    );

    let liquidity_supply =
        get_token_balance(&mut banks_client, sol_test_reserve.liquidity_supply_pubkey).await;
    assert_eq!(
        liquidity_supply,
        initial_liquidity_supply - SOL_BORROW_AMOUNT_LAMPORTS
    );

    let fee_balance = get_token_balance(
        &mut banks_client,
        sol_test_reserve.liquidity_fee_receiver_pubkey,
    )
    .await;
    assert_eq!(fee_balance, FEE_AMOUNT - HOST_FEE_AMOUNT);

    let host_fee_balance =
        get_token_balance(&mut banks_client, sol_test_reserve.liquidity_host_pubkey).await;
    assert_eq!(host_fee_balance, HOST_FEE_AMOUNT);
}

#[tokio::test]
async fn test_borrow_too_large() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    const SOL_DEPOSIT_AMOUNT_LAMPORTS: u64 = 100 * LAMPORTS_TO_SOL * INITIAL_COLLATERAL_RATIO;
    const USDC_BORROW_AMOUNT_FRACTIONAL: u64 = 1_000 * FRACTIONAL_TO_USDC + 1;
    const SOL_RESERVE_COLLATERAL_LAMPORTS: u64 = 2 * SOL_DEPOSIT_AMOUNT_LAMPORTS;
    const USDC_RESERVE_LIQUIDITY_FRACTIONAL: u64 = 2 * USDC_BORROW_AMOUNT_FRACTIONAL;

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
            ..AddObligationArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[
            refresh_obligation(
                spl_token_lending::id(),
                test_obligation.pubkey,
                vec![sol_test_reserve.pubkey],
            ),
            borrow_obligation_liquidity(
                spl_token_lending::id(),
                USDC_BORROW_AMOUNT_FRACTIONAL,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.pubkey,
                usdc_test_reserve.liquidity_fee_receiver_pubkey,
                test_obligation.pubkey,
                lending_market.pubkey,
                test_obligation.owner,
                Some(usdc_test_reserve.liquidity_host_pubkey),
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
            InstructionError::Custom(LendingError::BorrowTooLarge as u32)
        )
    );
}
