#![cfg(feature = "test-bpf")]

use {
    solana_program::{instruction::Instruction, pubkey::Pubkey},
    solana_program_test::*,
    solana_sdk::{signature::Signer, transaction::Transaction},
    spl_example_packed_len::processor::process_instruction,
    std::str::FromStr,
};

#[tokio::test]
async fn test_packed_len() {
    let program_id = Pubkey::from_str(&"PackedLen1111111111111111111111111111111111").unwrap();
    let program_test = ProgramTest::new(
        "spl_example_packed_len",
        program_id,
        processor!(process_instruction),
    );
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &(),
            vec![],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}
