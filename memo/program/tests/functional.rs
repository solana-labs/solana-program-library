#![cfg(feature = "test-bpf")]

use solana_program::{
    instruction::{AccountMeta, Instruction, InstructionError},
    pubkey::Pubkey,
};
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::{Transaction, TransactionError},
};
use spl_memo::*;

fn program_test() -> ProgramTest {
    ProgramTest::new("spl_memo", id(), processor!(processor::process_instruction))
}

#[tokio::test]
async fn test_memo_signing() {
    let memo = "🐆".as_bytes();
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    let keypairs = vec![Keypair::new(), Keypair::new(), Keypair::new()];
    let pubkeys: Vec<Pubkey> = keypairs.iter().map(|keypair| keypair.pubkey()).collect();

    // Test complete signing
    let signer_key_refs: Vec<&Pubkey> = pubkeys.iter().collect();
    let mut transaction =
        Transaction::new_with_payer(&[build_memo(memo, &signer_key_refs)], Some(&payer.pubkey()));
    let mut signers = vec![&payer];
    for keypair in keypairs.iter() {
        signers.push(keypair);
    }
    transaction.sign(&signers, recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Test unsigned memo
    let mut transaction =
        Transaction::new_with_payer(&[build_memo(memo, &[])], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Demonstrate success on signature provided, regardless of specific memo AccountMeta
    let mut transaction = Transaction::new_with_payer(
        &[Instruction {
            program_id: id(),
            accounts: vec![
                AccountMeta::new_readonly(keypairs[0].pubkey(), true),
                AccountMeta::new_readonly(keypairs[1].pubkey(), true),
                AccountMeta::new_readonly(payer.pubkey(), false),
            ],
            data: memo.to_vec(),
        }],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &keypairs[0], &keypairs[1]], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Test missing signer(s)
    let mut transaction = Transaction::new_with_payer(
        &[Instruction {
            program_id: id(),
            accounts: vec![
                AccountMeta::new_readonly(keypairs[0].pubkey(), true),
                AccountMeta::new_readonly(keypairs[1].pubkey(), false),
                AccountMeta::new_readonly(keypairs[2].pubkey(), true),
            ],
            data: memo.to_vec(),
        }],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &keypairs[0], &keypairs[2]], recent_blockhash);
    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::MissingRequiredSignature)
    );

    let mut transaction = Transaction::new_with_payer(
        &[Instruction {
            program_id: id(),
            accounts: vec![
                AccountMeta::new_readonly(keypairs[0].pubkey(), false),
                AccountMeta::new_readonly(keypairs[1].pubkey(), false),
                AccountMeta::new_readonly(keypairs[2].pubkey(), false),
            ],
            data: memo.to_vec(),
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
        TransactionError::InstructionError(0, InstructionError::MissingRequiredSignature)
    );

    // Test invalid utf-8; demonstrate log
    let invalid_utf8 = [0xF0, 0x9F, 0x90, 0x86, 0xF0, 0x9F, 0xFF, 0x86];
    let mut transaction =
        Transaction::new_with_payer(&[build_memo(&invalid_utf8, &[])], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::InvalidInstructionData)
    );
}

#[tokio::test]
#[ignore]
async fn test_memo_compute_limits() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    // Test memo length
    let mut memo = vec![];
    for _ in 0..1000 {
        let mut vec = vec![0x53, 0x4F, 0x4C];
        memo.append(&mut vec);
    }

    let mut transaction =
        Transaction::new_with_payer(&[build_memo(&memo[..450], &[])], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let mut transaction =
        Transaction::new_with_payer(&[build_memo(&memo[..600], &[])], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    let err = banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    let failed_to_complete =
        TransactionError::InstructionError(0, InstructionError::ProgramFailedToComplete);
    let computational_budget_exceeded =
        TransactionError::InstructionError(0, InstructionError::ComputationalBudgetExceeded);
    assert!(err == failed_to_complete || err == computational_budget_exceeded);

    let mut memo = vec![];
    for _ in 0..100 {
        let mut vec = vec![0xE2, 0x97, 0x8E];
        memo.append(&mut vec);
    }

    let mut transaction =
        Transaction::new_with_payer(&[build_memo(&memo[..60], &[])], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let mut transaction =
        Transaction::new_with_payer(&[build_memo(&memo[..63], &[])], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    let err = banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert!(err == failed_to_complete || err == computational_budget_exceeded);

    // Test num signers with 32-byte memo
    let memo = Pubkey::new_unique().to_bytes();
    let mut keypairs = vec![];
    for _ in 0..20 {
        keypairs.push(Keypair::new());
    }
    let pubkeys: Vec<Pubkey> = keypairs.iter().map(|keypair| keypair.pubkey()).collect();
    let signer_key_refs: Vec<&Pubkey> = pubkeys.iter().collect();

    let mut signers = vec![&payer];
    for keypair in keypairs[..12].iter() {
        signers.push(keypair);
    }
    let mut transaction = Transaction::new_with_payer(
        &[build_memo(&memo, &signer_key_refs[..12])],
        Some(&payer.pubkey()),
    );
    transaction.sign(&signers, recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let mut signers = vec![&payer];
    for keypair in keypairs[..15].iter() {
        signers.push(keypair);
    }
    let mut transaction = Transaction::new_with_payer(
        &[build_memo(&memo, &signer_key_refs[..15])],
        Some(&payer.pubkey()),
    );
    transaction.sign(&signers, recent_blockhash);
    let err = banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert!(err == failed_to_complete || err == computational_budget_exceeded);
}
