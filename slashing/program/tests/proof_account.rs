#![cfg(feature = "test-sbf")]

use {
    solana_program::{
        instruction::{AccountMeta, Instruction, InstructionError},
        pubkey::Pubkey,
        rent::Rent,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
    },
    spl_pod::bytemuck::pod_from_bytes,
    spl_slashing::{
        error::SlashingError,
        id, instruction,
        processor::process_instruction,
        state::{ProofData, ProofType},
    },
};

fn program_test() -> ProgramTest {
    ProgramTest::new("spl_slashing", id(), processor!(process_instruction))
}

async fn initialize_proof_account(
    context: &mut ProgramTestContext,
    authority: &Keypair,
    account: &Keypair,
    proof_type: ProofType,
    data: &[u8],
) {
    let transaction = Transaction::new_signed_with_payer(
        &[
            instruction::initialize_proof_account(
                &account.pubkey(),
                proof_type,
                &context.payer.pubkey(),
                &authority.pubkey(),
            ),
            instruction::write(&account.pubkey(), &authority.pubkey(), 0, data),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, account, authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

#[tokio::test]
async fn initialize_success() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[111u8; 8];
    let end_index = ProofData::WRITABLE_START_INDEX + data.len();
    initialize_proof_account(
        &mut context,
        &authority,
        &account,
        ProofType::DuplicateBlockProof,
        data,
    )
    .await;

    let account = context
        .banks_client
        .get_account(account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let account_data =
        pod_from_bytes::<ProofData>(&account.data[..ProofData::WRITABLE_START_INDEX]).unwrap();
    assert_eq!(
        ProofType::from(account_data.proof_type),
        ProofType::DuplicateBlockProof
    );
    assert_eq!(account_data.authority, authority.pubkey());
    assert_eq!(account_data.version, ProofData::CURRENT_VERSION);
    assert_eq!(
        &account.data[ProofData::WRITABLE_START_INDEX..end_index],
        data
    );
}

#[tokio::test]
async fn initialize_twice_fail() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[111u8; 8];
    initialize_proof_account(
        &mut context,
        &authority,
        &account,
        ProofType::DuplicateBlockProof,
        data,
    )
    .await;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::initialize_proof_account(
            &account.pubkey(),
            ProofType::DuplicateBlockProof,
            &context.payer.pubkey(),
            &authority.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &account],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
}

#[tokio::test]
async fn write_success() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    initialize_proof_account(
        &mut context,
        &authority,
        &account,
        ProofType::DuplicateBlockProof,
        data,
    )
    .await;

    let new_data = &[200u8; 16];
    let end_index = new_data.len() + ProofData::WRITABLE_START_INDEX;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::write(
            &account.pubkey(),
            &authority.pubkey(),
            0,
            new_data,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account = context
        .banks_client
        .get_account(account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let account_data =
        pod_from_bytes::<ProofData>(&account.data[..ProofData::WRITABLE_START_INDEX]).unwrap();
    assert_eq!(account_data.authority, authority.pubkey());
    assert_eq!(account_data.version, ProofData::CURRENT_VERSION);
    assert_eq!(
        &account.data[ProofData::WRITABLE_START_INDEX..end_index],
        new_data
    );
}

#[tokio::test]
async fn write_fail_wrong_authority() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    initialize_proof_account(
        &mut context,
        &authority,
        &account,
        ProofType::DuplicateBlockProof,
        data,
    )
    .await;

    let new_data = &[200u8; 8];
    let wrong_authority = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::write(
            &account.pubkey(),
            &wrong_authority.pubkey(),
            0,
            new_data,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wrong_authority],
        context.last_blockhash,
    );
    assert_eq!(
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(SlashingError::IncorrectAuthority as u32)
        )
    );
}

#[tokio::test]
async fn write_fail_unsigned() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    initialize_proof_account(
        &mut context,
        &authority,
        &account,
        ProofType::DuplicateBlockProof,
        data,
    )
    .await;

    let data = &[200u8; 8];

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id: id(),
            accounts: vec![
                AccountMeta::new(account.pubkey(), false),
                AccountMeta::new_readonly(authority.pubkey(), false),
            ],
            data: instruction::SlashingInstruction::Write { offset: 0, data }.pack(),
        }],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    assert_eq!(
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::MissingRequiredSignature)
    );
}

#[tokio::test]
async fn close_account_success() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    let account_length = ProofType::DuplicateBlockProof
        .proof_account_length()
        .unwrap();
    initialize_proof_account(
        &mut context,
        &authority,
        &account,
        ProofType::DuplicateBlockProof,
        data,
    )
    .await;
    let recipient = Pubkey::new_unique();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::close_account(
            &account.pubkey(),
            &authority.pubkey(),
            &recipient,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account = context
        .banks_client
        .get_account(recipient)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        account.lamports,
        1.max(Rent::default().minimum_balance(account_length))
    );
}

#[tokio::test]
async fn close_account_fail_wrong_authority() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    initialize_proof_account(
        &mut context,
        &authority,
        &account,
        ProofType::DuplicateBlockProof,
        data,
    )
    .await;

    let wrong_authority = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id: id(),
            accounts: vec![
                AccountMeta::new(account.pubkey(), false),
                AccountMeta::new_readonly(wrong_authority.pubkey(), true),
                AccountMeta::new(Pubkey::new_unique(), false),
            ],
            data: instruction::SlashingInstruction::CloseAccount.pack(),
        }],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wrong_authority],
        context.last_blockhash,
    );
    assert_eq!(
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(SlashingError::IncorrectAuthority as u32)
        )
    );
}

#[tokio::test]
async fn close_account_fail_unsigned() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8, 8];
    initialize_proof_account(
        &mut context,
        &authority,
        &account,
        ProofType::DuplicateBlockProof,
        data,
    )
    .await;

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id: id(),
            accounts: vec![
                AccountMeta::new(account.pubkey(), false),
                AccountMeta::new_readonly(authority.pubkey(), false),
                AccountMeta::new(Pubkey::new_unique(), false),
            ],
            data: instruction::SlashingInstruction::CloseAccount.pack(),
        }],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    assert_eq!(
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::MissingRequiredSignature)
    );
}
