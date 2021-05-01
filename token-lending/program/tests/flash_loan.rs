#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program_test::*;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use spl_token_lending::instruction::flash_loan;
use spl_token_lending::math::Decimal;
use spl_token_lending::processor::process_instruction;

#[tokio::test]
async fn test_flash_loan_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    let receiver_program_account = Keypair::new();
    let receiver_program_id = receiver_program_account.pubkey();
    test.add_program(
        "flash_loan_receiver",
        receiver_program_id.clone(),
        processor!(helpers::flash_loan_receiver::process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(118_000);

    let user_accounts_owner = Keypair::new();
    let usdc_mint = add_usdc_mint(&mut test);
    let lending_market = add_lending_market(&mut test, usdc_mint.pubkey);

    let mut reserve_config = TEST_RESERVE_CONFIG;
    reserve_config.loan_to_value_ratio = 80;
    let flash_loan_amount = 1_000_000u64;
    let (flash_loan_fee, host_fee) = TEST_RESERVE_CONFIG
        .fees
        .calculate_flash_loan_fees(Decimal::from(flash_loan_amount))
        .unwrap();

    let usdc_reserve = add_reserve(
        &mut test,
        &lending_market,
        &user_accounts_owner,
        AddReserveArgs {
            liquidity_amount: 1_000_000,
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

    let current_token_amount =
        get_token_balance(&mut banks_client, program_owned_token_account).await;
    // There should be enough token at the beginning to pay back the flash loan fee.
    assert_eq!(current_token_amount, flash_loan_fee);

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
            vec![receiver_authority_pubkey.clone()],
        )],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer], recent_blockhash);
    assert!(banks_client.process_transaction(transaction).await.is_ok());
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
