#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program_test::*;
use solana_sdk::{
    instruction::InstructionError,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::{Transaction, TransactionError},
};
use spl_token_lending::{
    error::LendingError,
    instruction::{set_lending_market_owner, LendingInstruction},
    processor::process_instruction,
};

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(2_000);

    let lending_market = add_lending_market(&mut test);
    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let new_owner = Pubkey::new_unique();
    let mut transaction = Transaction::new_with_payer(
        &[set_lending_market_owner(
            spl_token_lending::id(),
            lending_market.pubkey,
            lending_market.owner.pubkey(),
            new_owner,
        )],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &lending_market.owner], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.unwrap())
        .unwrap();

    let lending_market_info = lending_market.get_state(&mut banks_client).await;
    assert_eq!(lending_market_info.owner, new_owner);
}

#[tokio::test]
async fn test_invalid_owner() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    let lending_market = add_lending_market(&mut test);
    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let invalid_owner = Keypair::new();
    let new_owner = Pubkey::new_unique();
    let mut transaction = Transaction::new_with_payer(
        &[set_lending_market_owner(
            spl_token_lending::id(),
            lending_market.pubkey,
            invalid_owner.pubkey(),
            new_owner,
        )],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &invalid_owner], recent_blockhash);

    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::InvalidMarketOwner as u32)
        )
    );
}

#[tokio::test]
async fn test_owner_not_signer() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    let lending_market = add_lending_market(&mut test);
    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let new_owner = Pubkey::new_unique();
    let mut transaction = Transaction::new_with_payer(
        &[Instruction {
            program_id: spl_token_lending::id(),
            accounts: vec![
                AccountMeta::new(lending_market.pubkey, false),
                AccountMeta::new_readonly(lending_market.owner.pubkey(), false),
            ],
            data: LendingInstruction::SetLendingMarketOwner { new_owner }.pack(),
        }],
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
            InstructionError::Custom(LendingError::InvalidSigner as u32)
        )
    );
}
