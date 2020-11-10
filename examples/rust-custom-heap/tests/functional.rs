use solana_program::{instruction::Instruction, pubkey::Pubkey};
use solana_program_test::{processor, BanksClientExt, ProgramTest};
use solana_sdk::{signature::Signer, transaction::Transaction};
use spl_example_custom_heap::{id, processor::process_instruction};

#[tokio::test]
async fn test_custom_heap() {
    let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
        "spl_example_custom_heap",
        id(),
        processor!(process_instruction),
    )
    .start()
    .await;
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new(id(), &[10_u8, 11, 12, 13, 14], vec![])],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}
