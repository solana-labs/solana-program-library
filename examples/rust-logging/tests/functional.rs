use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use solana_program_test::{processor, BanksClientExt, ProgramTest};
use solana_sdk::{signature::Signer, transaction::Transaction};
use spl_example_logging::{id, processor::process_instruction};

#[tokio::test]
async fn test_logging() {
    let (mut banks_client, payer, recent_blockhash) =
        ProgramTest::new("spl_example_logging", id(), processor!(process_instruction))
            .start()
            .await;
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new(
            id(),
            &[10_u8, 11, 12, 13, 14],
            vec![AccountMeta::new(Pubkey::new_unique(), false)],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}
