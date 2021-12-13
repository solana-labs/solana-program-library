#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program::instruction::AccountMeta;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::{Transaction, TransactionError},
};
use spl_token::solana_program::instruction::InstructionError;
use spl_token_lending::{
    error::LendingError, instruction::flash_loan, processor::process_instruction,
};

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(50_000);

    const FLASH_LOAN_AMOUNT: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const FEE_AMOUNT: u64 = 3_000_000;
    const HOST_FEE_AMOUNT: u64 = 600_000;

    let receiver_program_account = Keypair::new();
    let receiver_program_id = receiver_program_account.pubkey();
    test.prefer_bpf(false);
    test.add_program(
        "flash_loan_receiver",
        receiver_program_id.clone(),
        processor!(helpers::flash_loan_receiver::process_instruction),
    );

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut reserve_config = TEST_RESERVE_CONFIG;
    reserve_config.fees.flash_loan_fee_wad = 3_000_000_000_000_000;

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            liquidity_amount: FLASH_LOAN_AMOUNT,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let (receiver_authority_pubkey, _) =
        Pubkey::find_program_address(&[b"flashloan"], &receiver_program_id);
    let program_owned_token_account = add_account_for_program(
        &mut test,
        &receiver_authority_pubkey,
        FEE_AMOUNT,
        &usdc_mint.pubkey,
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let initial_liquidity_supply =
        get_token_balance(&mut banks_client, usdc_test_reserve.liquidity_supply_pubkey).await;
    assert_eq!(initial_liquidity_supply, FLASH_LOAN_AMOUNT);

    let usdc_reserve = usdc_test_reserve.get_state(&mut banks_client).await;
    let initial_available_amount = usdc_reserve.liquidity.available_amount;
    assert_eq!(initial_available_amount, FLASH_LOAN_AMOUNT);

    let initial_token_balance =
        get_token_balance(&mut banks_client, program_owned_token_account).await;
    assert_eq!(initial_token_balance, FEE_AMOUNT);

    let mut transaction = Transaction::new_with_payer(
        &[flash_loan(
            spl_token_lending::id(),
            FLASH_LOAN_AMOUNT,
            usdc_test_reserve.liquidity_supply_pubkey,
            program_owned_token_account,
            usdc_test_reserve.pubkey,
            usdc_test_reserve.liquidity_fee_receiver_pubkey,
            usdc_test_reserve.liquidity_host_pubkey,
            lending_market.pubkey,
            receiver_program_id.clone(),
            vec![AccountMeta::new_readonly(
                receiver_authority_pubkey.clone(),
                false,
            )],
        )],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer], recent_blockhash);
    assert!(banks_client.process_transaction(transaction).await.is_ok());

    let usdc_reserve = usdc_test_reserve.get_state(&mut banks_client).await;
    assert_eq!(
        usdc_reserve.liquidity.available_amount,
        initial_available_amount
    );

    let (total_fee, host_fee) = usdc_reserve
        .config
        .fees
        .calculate_flash_loan_fees(FLASH_LOAN_AMOUNT.into())
        .unwrap();
    assert_eq!(total_fee, FEE_AMOUNT);
    assert_eq!(host_fee, HOST_FEE_AMOUNT);

    let liquidity_supply =
        get_token_balance(&mut banks_client, usdc_test_reserve.liquidity_supply_pubkey).await;
    assert_eq!(liquidity_supply, initial_liquidity_supply);

    let token_balance = get_token_balance(&mut banks_client, program_owned_token_account).await;
    assert_eq!(token_balance, initial_token_balance - FEE_AMOUNT);

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
async fn test_failure() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    const FLASH_LOAN_AMOUNT: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const FEE_AMOUNT: u64 = 3_000_000;

    let flash_loan_receiver_program_keypair = Keypair::new();
    let flash_loan_receiver_program_id = flash_loan_receiver_program_keypair.pubkey();
    test.prefer_bpf(false);
    test.add_program(
        "flash_loan_receiver",
        flash_loan_receiver_program_id.clone(),
        processor!(helpers::flash_loan_receiver::process_instruction),
    );

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut reserve_config = TEST_RESERVE_CONFIG;
    reserve_config.fees.flash_loan_fee_wad = 3_000_000_000_000_000;

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            liquidity_amount: FLASH_LOAN_AMOUNT,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let (receiver_authority_pubkey, _) =
        Pubkey::find_program_address(&[b"flashloan"], &flash_loan_receiver_program_id);
    let program_owned_token_account = add_account_for_program(
        &mut test,
        &receiver_authority_pubkey,
        FEE_AMOUNT - 1,
        &usdc_mint.pubkey,
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let initial_token_balance =
        get_token_balance(&mut banks_client, program_owned_token_account).await;
    assert_eq!(initial_token_balance, FEE_AMOUNT - 1);

    let mut transaction = Transaction::new_with_payer(
        &[flash_loan(
            spl_token_lending::id(),
            FLASH_LOAN_AMOUNT,
            usdc_test_reserve.liquidity_supply_pubkey,
            program_owned_token_account,
            usdc_test_reserve.pubkey,
            usdc_test_reserve.liquidity_fee_receiver_pubkey,
            usdc_test_reserve.liquidity_host_pubkey,
            lending_market.pubkey,
            flash_loan_receiver_program_id.clone(),
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
