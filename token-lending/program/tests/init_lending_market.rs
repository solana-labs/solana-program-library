#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program_test::*;
use solana_sdk::{
    instruction::InstructionError,
    pubkey::Pubkey,
    signature::Signer,
    transaction::{Transaction, TransactionError},
};
use spl_token_lending::{
    error::LendingError, instruction::init_lending_market, processor::process_instruction,
};

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(15_000);

    let usdc_mint = add_usdc_mint(&mut test);
    let (mut banks_client, payer, _recent_blockhash) = test.start().await;

    let test_lending_market =
        TestLendingMarket::init(&mut banks_client, usdc_mint.pubkey, &payer).await;

    test_lending_market.validate_state(&mut banks_client).await;

    let lending_market = test_lending_market.get_state(&mut banks_client).await;
    assert_eq!(lending_market.quote_token_mint, usdc_mint.pubkey);
}

#[tokio::test]
async fn test_already_initialized() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    let usdc_mint = add_usdc_mint(&mut test);
    let existing_market = add_lending_market(&mut test, usdc_mint.pubkey);
    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[init_lending_market(
            spl_token_lending::id(),
            existing_market.pubkey,
            existing_market.owner.pubkey(),
            usdc_mint.pubkey,
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
            InstructionError::Custom(LendingError::AlreadyInitialized as u32)
        )
    );
}
