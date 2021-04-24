// Program test does not support calling a raw program entrypoint, only `process_instruction`
#![cfg(feature = "test-bpf")]

use solana_program_test::*;
use solana_sdk::{
    account::Account,
    instruction::InstructionError,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Signer,
    transaction::{Transaction, TransactionError},
};

#[tokio::test]
async fn assert_instruction_count() {
    const OFFSET: usize = 51;
    const NUM_TO_SHARE: usize = 500;
    let program_id = Pubkey::new_unique();
    let shared_key = Pubkey::new_unique();

    let mut program_test = ProgramTest::new(
        "spl_shared_memory", // Run the BPF version with `cargo test-bpf`
        program_id,
        None,
    );
    program_test.add_account(
        shared_key,
        Account {
            lamports: 5000000000000,
            data: vec![0_u8; NUM_TO_SHARE * 2],
            owner: program_id,
            ..Account::default()
        },
    );
    program_test.set_bpf_compute_max_units(480);
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // success
    let content = vec![42; NUM_TO_SHARE];
    let mut instruction_data = OFFSET.to_le_bytes().to_vec();
    instruction_data.extend_from_slice(&content);
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![AccountMeta::new(shared_key, false)],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn test_helloworld() {
    const OFFSET: usize = 51;
    const NUM_TO_SHARE: usize = 500;
    let program_id = Pubkey::new_unique();
    let shared_key = Pubkey::new_unique();

    let mut program_test = ProgramTest::new(
        "spl_shared_memory", // Run the BPF version with `cargo test-bpf`
        program_id,
        None,
    );
    program_test.add_account(
        shared_key,
        Account {
            lamports: 5000000000000,
            data: vec![0_u8; NUM_TO_SHARE * 2],
            owner: program_id,
            ..Account::default()
        },
    );
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // success
    let content = vec![42; NUM_TO_SHARE];
    let mut instruction_data = OFFSET.to_le_bytes().to_vec();
    instruction_data.extend_from_slice(&content);
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![AccountMeta::new(shared_key, false)],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // success zero offset
    let content = vec![42; NUM_TO_SHARE];
    let mut instruction_data = 0_usize.to_le_bytes().to_vec();
    instruction_data.extend_from_slice(&content);
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![AccountMeta::new(shared_key, false)],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // too few accounts
    let content = vec![42; NUM_TO_SHARE];
    let mut instruction_data = OFFSET.to_le_bytes().to_vec();
    instruction_data.extend_from_slice(&content);
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    let result = banks_client.process_transaction(transaction).await;
    assert_eq!(
        result.unwrap_err().unwrap(),
        TransactionError::InstructionError(0, InstructionError::NotEnoughAccountKeys)
    );

    // too many accounts
    let content = vec![42; NUM_TO_SHARE];
    let mut instruction_data = OFFSET.to_le_bytes().to_vec();
    instruction_data.extend_from_slice(&content);
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(shared_key, false),
                AccountMeta::new(shared_key, false),
            ],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    let result = banks_client.process_transaction(transaction).await;
    assert_eq!(
        result.unwrap_err().unwrap(),
        TransactionError::InstructionError(0, InstructionError::InvalidArgument)
    );

    // account data too small
    let content = vec![42; NUM_TO_SHARE * 10];
    let mut instruction_data = OFFSET.to_le_bytes().to_vec();
    instruction_data.extend_from_slice(&content);
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![AccountMeta::new(shared_key, false)],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    let result = banks_client.process_transaction(transaction).await;
    assert_eq!(
        result.unwrap_err().unwrap(),
        TransactionError::InstructionError(0, InstructionError::AccountDataTooSmall)
    );

    // offset too large
    let content = vec![42; NUM_TO_SHARE];
    let mut instruction_data = (OFFSET * 10).to_le_bytes().to_vec();
    instruction_data.extend_from_slice(&content);
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![AccountMeta::new(shared_key, false)],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    let result = banks_client.process_transaction(transaction).await;
    assert_eq!(
        result.unwrap_err().unwrap(),
        TransactionError::InstructionError(0, InstructionError::AccountDataTooSmall)
    );
}
