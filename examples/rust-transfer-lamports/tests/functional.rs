use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use solana_program_test::{processor, BanksClientExt, ProgramTest};
use solana_sdk::{account::Account, signature::Signer, transaction::Transaction};
use spl_example_transfer_lamports::{id, processor::process_instruction};

#[tokio::test]
async fn test_lamport_transfer() {
    let source_pubkey = Pubkey::new_unique();
    let destination_pubkey = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "spl_example_transfer_lamports",
        id(),
        processor!(process_instruction),
    );
    program_test.add_account(
        source_pubkey,
        Account {
            lamports: 5,
            owner: id(), // Can only withdraw lamports from accounts owned by the program
            ..Account::default()
        },
    );
    program_test.add_account(
        destination_pubkey,
        Account {
            lamports: 5,
            ..Account::default()
        },
    );
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new(
            id(),
            &(),
            vec![
                AccountMeta::new(source_pubkey, false),
                AccountMeta::new(destination_pubkey, false),
            ],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}
