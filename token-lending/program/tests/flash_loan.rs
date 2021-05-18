#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program::instruction::AccountMeta;
use solana_program_test::*;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use spl_token::solana_program::instruction::InstructionError;
use spl_token_lending::error::LendingError;
use spl_token_lending::instruction::flash_loan;
use spl_token_lending::math::Decimal;
use spl_token_lending::processor::process_instruction;

const INITIAL_RESERVE_LIQUIDITY: u64 = 1_000_000;

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(118_000);

    let receiver_program_account = Keypair::new();
    let receiver_program_id = receiver_program_account.pubkey();
    test.add_program(
        "flash_loan_receiver",
        receiver_program_id.clone(),
        processor!(helpers::flash_loan_receiver::process_instruction),
    );

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let reserve_config = TEST_RESERVE_CONFIG;
    let flash_loan_amount = 1_000_000u64;
    let (flash_loan_fee, host_fee) = TEST_RESERVE_CONFIG
        .fees
        .calculate_flash_loan_fees(Decimal::from(flash_loan_amount))
        .unwrap();

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            liquidity_amount: INITIAL_RESERVE_LIQUIDITY,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            user_liquidity_amount: flash_loan_fee,
            ..AddReserveArgs::default()
        },
    );

    let (receiver_authority_pubkey, _) =
        Pubkey::find_program_address(&[b"flashloan"], &receiver_program_id);
    let program_owned_token_account = add_account_for_program(
        &mut test,
        &receiver_authority_pubkey,
        flash_loan_fee,
        &usdc_mint.pubkey,
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let before_flash_loan_reserve_liquidity_token_balance =
        get_token_balance(&mut banks_client, usdc_reserve.liquidity_supply_pubkey).await;
    assert_eq!(
        before_flash_loan_reserve_liquidity_token_balance,
        INITIAL_RESERVE_LIQUIDITY
    );

    let before_flash_loan_reserve = usdc_reserve.get_state(&mut banks_client).await;
    assert_eq!(
        before_flash_loan_reserve.liquidity.available_amount,
        INITIAL_RESERVE_LIQUIDITY
    );

    let before_flash_loan_token_balance =
        get_token_balance(&mut banks_client, program_owned_token_account).await;
    // There should be enough token at the beginning to pay back the flash loan fee.
    assert_eq!(before_flash_loan_token_balance, flash_loan_fee);

    let mut transaction = Transaction::new_with_payer(
        &[flash_loan(
            spl_token_lending::id(),
            flash_loan_amount,
            usdc_reserve.liquidity_supply_pubkey,
            program_owned_token_account,
            usdc_reserve.pubkey,
            lending_market.pubkey,
            lending_market.authority,
            receiver_program_id.clone(),
            usdc_reserve.liquidity_fee_receiver_pubkey,
            usdc_reserve.liquidity_host_pubkey,
            vec![AccountMeta::new_readonly(
                receiver_authority_pubkey.clone(),
                false,
            )],
        )],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer], recent_blockhash);
    assert!(banks_client.process_transaction(transaction).await.is_ok());

    let after_flash_loan_reserve_liquidity_token_balance =
        get_token_balance(&mut banks_client, usdc_reserve.liquidity_supply_pubkey).await;
    assert_eq!(
        after_flash_loan_reserve_liquidity_token_balance,
        INITIAL_RESERVE_LIQUIDITY
    );

    let after_flash_loan_reserve = usdc_reserve.get_state(&mut banks_client).await;
    assert_eq!(
        after_flash_loan_reserve.liquidity.available_amount,
        INITIAL_RESERVE_LIQUIDITY
    );

    let after_flash_loan_token_balance =
        get_token_balance(&mut banks_client, program_owned_token_account).await;
    assert_eq!(after_flash_loan_token_balance, 0);
    let fee_balance = get_token_balance(
        &mut banks_client,
        usdc_reserve.liquidity_fee_receiver_pubkey,
    )
    .await;
    assert_eq!(fee_balance, flash_loan_fee - host_fee);

    let host_fee_balance =
        get_token_balance(&mut banks_client, usdc_reserve.liquidity_host_pubkey).await;
    assert_eq!(host_fee_balance, host_fee);
}

#[tokio::test]
async fn test_failure() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(118_000);

    let receiver_program_account = Keypair::new();
    let receiver_program_id = receiver_program_account.pubkey();
    test.add_program(
        "flash_loan_receiver",
        receiver_program_id.clone(),
        processor!(helpers::flash_loan_receiver::process_instruction),
    );

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut reserve_config = TEST_RESERVE_CONFIG;
    reserve_config.loan_to_value_ratio = 80;
    let flash_loan_amount = 1_000_000u64;
    let (flash_loan_fee, _host_fee) = TEST_RESERVE_CONFIG
        .fees
        .calculate_flash_loan_fees(Decimal::from(flash_loan_amount))
        .unwrap();

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            liquidity_amount: INITIAL_RESERVE_LIQUIDITY,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            user_liquidity_amount: flash_loan_fee,
            ..AddReserveArgs::default()
        },
    );
    let (receiver_authority_pubkey, _) =
        Pubkey::find_program_address(&[b"flashloan"], &receiver_program_id);
    let program_owned_token_account = add_account_for_program(
        &mut test,
        &receiver_authority_pubkey,
        // Provide Insufficient fund to exercise the flash loan returned fund check.
        flash_loan_fee - 1,
        &usdc_mint.pubkey,
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let before_flash_loan_token_balance =
        get_token_balance(&mut banks_client, program_owned_token_account).await;
    // There should be not enough token at the beginning.
    assert_eq!(before_flash_loan_token_balance, flash_loan_fee - 1);

    let mut transaction = Transaction::new_with_payer(
        &[flash_loan(
            spl_token_lending::id(),
            flash_loan_amount,
            usdc_reserve.liquidity_supply_pubkey,
            program_owned_token_account,
            usdc_reserve.pubkey,
            lending_market.pubkey,
            lending_market.authority,
            receiver_program_id.clone(),
            usdc_reserve.liquidity_fee_receiver_pubkey,
            usdc_reserve.liquidity_host_pubkey,
            vec![AccountMeta::new_readonly(
                receiver_authority_pubkey.clone(),
                false,
            )],
        )],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer], recent_blockhash);
    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::NotEnoughLiquidityAfterFlashLoan as u32)
        )
    );
}
