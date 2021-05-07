#![cfg(feature = "test-bpf")]

use solana_program::pubkey::Pubkey;
use solana_program_template::*;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

pub fn program_test() -> ProgramTest {
    ProgramTest::new(
        "solana-program-template",
        id(),
        processor!(processor::Processor::process_instruction),
    )
}

#[tokio::test]
async fn test_call_example_instruction() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    let new_acc = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::init(&id(), &new_acc.pubkey()).unwrap()],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}
