#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program_test::*;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use spl_token::instruction::approve;
use spl_token_lending::instruction::{flash_loan_end, flash_loan_start};
use spl_token_lending::processor::process_instruction;

#[tokio::test]
async fn test_flash_loan() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(118_000);

    let user_accounts_owner = Keypair::new();
    let user_transfer_authority = Keypair::new();
    let usdc_mint = add_usdc_mint(&mut test);
    let lending_market = add_lending_market(&mut test, usdc_mint.pubkey);

    let mut reserve_config = TEST_RESERVE_CONFIG;
    reserve_config.loan_to_value_ratio = 80;
    let flash_loan_amount = 1_000u64;
    let (flash_loan_fee, host_fee) = TEST_RESERVE_CONFIG
        .fees
        .calculate_flash_loan_fees(flash_loan_amount)
        .unwrap();

    let usdc_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            liquidity_amount: 1_000_000,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            user_liquidity_amount: flash_loan_fee,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let borrow_amount =
        get_token_balance(&mut banks_client, usdc_reserve.user_liquidity_account).await;
    assert_eq!(borrow_amount, flash_loan_fee);

    let mut transaction = Transaction::new_with_payer(
        &[
            flash_loan_start(
                spl_token_lending::id(),
                flash_loan_amount,
                2u8,
                usdc_reserve.user_liquidity_account,
                usdc_reserve.pubkey,
                usdc_reserve.liquidity_supply,
                lending_market.pubkey,
                spl_token::id(),
            ),
            approve(
                &spl_token::id(),
                &usdc_reserve.user_liquidity_account,
                &user_transfer_authority.pubkey(),
                &user_accounts_owner.pubkey(),
                &[],
                flash_loan_amount + flash_loan_fee,
            )
            .unwrap(),
            flash_loan_end(
                spl_token_lending::id(),
                usdc_reserve.pubkey,
                usdc_reserve.liquidity_supply,
                lending_market.pubkey,
                usdc_reserve.flash_loan_fees_receiver,
                usdc_reserve.user_liquidity_account,
                user_transfer_authority.pubkey(),
                Some(usdc_reserve.liquidity_host),
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(
        &[&payer, &user_accounts_owner, &user_transfer_authority],
        recent_blockhash,
    );
    assert!(banks_client.process_transaction(transaction).await.is_ok());

    let fee_balance =
        get_token_balance(&mut banks_client, usdc_reserve.flash_loan_fees_receiver).await;
    assert_eq!(fee_balance, flash_loan_fee - host_fee);

    let host_fee_balance = get_token_balance(&mut banks_client, usdc_reserve.liquidity_host).await;
    assert_eq!(host_fee_balance, host_fee);
}
