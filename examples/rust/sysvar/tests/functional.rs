use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        sysvar::{self},
    },
    solana_program_test::*,
    solana_sdk::{signature::Signer, transaction::Transaction},
    spl_example_sysvar::processor::process_instruction,
    std::str::FromStr,
};

#[tokio::test]
async fn test_sysvar() {
    let program_id = Pubkey::from_str("Sysvar1111111111111111111111111111111111111").unwrap();
    let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
        "spl_example_sysvar",
        program_id,
        processor!(process_instruction),
    )
    .start()
    .await;

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &(),
            vec![
                AccountMeta::new(sysvar::clock::id(), false),
                AccountMeta::new(sysvar::rent::id(), false),
            ],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

async fn test_transaction() {
    let program_id = Pubkey::new_unique();

    let mut program_test = ProgramTest::new(
        "bpf_program_template",
        program_id,
        processor!(process_instruction),
    );

    // Replace the SlotHashes sysvar will a fully populated version that was grabbed off Mainnet
    // Beta by running:
    //      solana account SysvarS1otHashes111111111111111111111111111 -o slot_hashes.bin
    program_test.add_account_with_file_data(
        sysvar::slot_hashes::id(),
        sol_to_lamports(1.),
        Pubkey::default(),
        "slot_hashes.bin",
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[Instruction {
            program_id,
            accounts: vec![AccountMeta::new(sysvar::slot_hashes::id(), false)],
            data: vec![1, 2, 3],
        }],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);

    assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));
}
