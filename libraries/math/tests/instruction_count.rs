// Mark this test as BPF-only due to current `ProgramTest` limitations when CPIing into the system program
#![cfg(feature = "test-bpf")]

use {
    solana_program_test::*,
    solana_sdk::{signature::Signer, transaction::Transaction},
    spl_math::{id, instruction, processor::process_instruction},
};

#[tokio::test]
async fn test_precise_sqrt_u64_max() {
    let mut pc = ProgramTest::new("spl_math", id(), processor!(process_instruction));

    // This is way too big!  It's possible to dial down the numbers to get to
    // something reasonable, but the better option is to do everything in u64
    pc.set_compute_max_units(350_000);

    let (mut banks_client, payer, recent_blockhash) = pc.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[instruction::precise_sqrt(u64::MAX)],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn test_precise_sqrt_u32_max() {
    let mut pc = ProgramTest::new("spl_math", id(), processor!(process_instruction));

    pc.set_compute_max_units(170_000);

    let (mut banks_client, payer, recent_blockhash) = pc.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[instruction::precise_sqrt(u32::MAX as u64)],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn test_sqrt_u64() {
    let mut pc = ProgramTest::new("spl_math", id(), processor!(process_instruction));

    // Dial down the BPF compute budget to detect if the operation gets bloated in the future
    pc.set_compute_max_units(2_500);

    let (mut banks_client, payer, recent_blockhash) = pc.start().await;

    let mut transaction =
        Transaction::new_with_payer(&[instruction::sqrt_u64(u64::MAX)], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn test_sqrt_u128() {
    let mut pc = ProgramTest::new("spl_math", id(), processor!(process_instruction));

    // Dial down the BPF compute budget to detect if the operation gets bloated in the future
    pc.set_compute_max_units(4_100);

    let (mut banks_client, payer, recent_blockhash) = pc.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[instruction::sqrt_u128(u64::MAX as u128)],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn test_sqrt_u128_max() {
    let mut pc = ProgramTest::new("spl_math", id(), processor!(process_instruction));

    pc.set_compute_max_units(7_000);

    let (mut banks_client, payer, recent_blockhash) = pc.start().await;

    let mut transaction =
        Transaction::new_with_payer(&[instruction::sqrt_u128(u128::MAX)], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn test_u64_multiply() {
    let mut pc = ProgramTest::new("spl_math", id(), processor!(process_instruction));

    pc.set_compute_max_units(1350);

    let (mut banks_client, payer, recent_blockhash) = pc.start().await;

    let mut transaction =
        Transaction::new_with_payer(&[instruction::u64_multiply(42, 84)], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn test_u64_divide() {
    let mut pc = ProgramTest::new("spl_math", id(), processor!(process_instruction));

    pc.set_compute_max_units(1650);

    let (mut banks_client, payer, recent_blockhash) = pc.start().await;

    let mut transaction =
        Transaction::new_with_payer(&[instruction::u64_divide(3, 1)], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn test_f32_multiply() {
    let mut pc = ProgramTest::new("spl_math", id(), processor!(process_instruction));

    pc.set_compute_max_units(1600);

    let (mut banks_client, payer, recent_blockhash) = pc.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[instruction::f32_multiply(1.5_f32, 2.0_f32)],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn test_f32_divide() {
    let mut pc = ProgramTest::new("spl_math", id(), processor!(process_instruction));

    pc.set_compute_max_units(1650);

    let (mut banks_client, payer, recent_blockhash) = pc.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[instruction::f32_divide(3_f32, 1.5_f32)],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn test_noop() {
    let mut pc = ProgramTest::new("spl_math", id(), processor!(process_instruction));

    pc.set_compute_max_units(1200);

    let (mut banks_client, payer, recent_blockhash) = pc.start().await;

    let mut transaction =
        Transaction::new_with_payer(&[instruction::noop()], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}
