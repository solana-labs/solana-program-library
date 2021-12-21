#![cfg(feature = "test-bpf")]

use {
    borsh::BorshSerialize,
    solana_program::{
        borsh::get_packed_len,
        instruction::{AccountMeta, Instruction, InstructionError},
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
    },
    spl_record::{
        error::RecordError,
        id, instruction,
        processor::process_instruction,
        state::{Data, RecordData},
    },
};

fn program_test() -> ProgramTest {
    ProgramTest::new("spl_record", id(), processor!(process_instruction))
}

async fn initialize_storage_account(
    context: &mut ProgramTestContext,
    authority: &Keypair,
    account: &Keypair,
    data: Data,
) {
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &account.pubkey(),
                1.max(Rent::default().minimum_balance(get_packed_len::<RecordData>())),
                get_packed_len::<RecordData>() as u64,
                &id(),
            ),
            instruction::initialize(&account.pubkey(), &authority.pubkey()),
            instruction::write(
                &account.pubkey(),
                &authority.pubkey(),
                0,
                data.try_to_vec().unwrap(),
            ),
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
    let data = Data {
        bytes: [111u8; Data::DATA_SIZE],
    };
    initialize_storage_account(&mut context, &authority, &account, data.clone()).await;
    let account_data = context
        .banks_client
        .get_account_data_with_borsh::<RecordData>(account.pubkey())
        .await
        .unwrap();
    assert_eq!(account_data.data, data);
    assert_eq!(account_data.authority, authority.pubkey());
    assert_eq!(account_data.version, RecordData::CURRENT_VERSION);
}

#[tokio::test]
async fn initialize_with_seed_success() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let seed = "storage";
    let account = Pubkey::create_with_seed(&authority.pubkey(), seed, &id()).unwrap();
    let data = Data {
        bytes: [111u8; Data::DATA_SIZE],
    };
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account_with_seed(
                &context.payer.pubkey(),
                &account,
                &authority.pubkey(),
                seed,
                1.max(Rent::default().minimum_balance(get_packed_len::<RecordData>())),
                get_packed_len::<RecordData>() as u64,
                &id(),
            ),
            instruction::initialize(&account, &authority.pubkey()),
            instruction::write(&account, &authority.pubkey(), 0, data.try_to_vec().unwrap()),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
    let account_data = context
        .banks_client
        .get_account_data_with_borsh::<RecordData>(account)
        .await
        .unwrap();
    assert_eq!(account_data.data, data);
    assert_eq!(account_data.authority, authority.pubkey());
    assert_eq!(account_data.version, RecordData::CURRENT_VERSION);
}

#[tokio::test]
async fn initialize_twice_fail() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = Data {
        bytes: [111u8; Data::DATA_SIZE],
    };
    initialize_storage_account(&mut context, &authority, &account, data).await;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::initialize(
            &account.pubkey(),
            &authority.pubkey(),
        )],
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
        TransactionError::InstructionError(0, InstructionError::AccountAlreadyInitialized)
    );
}

#[tokio::test]
async fn write_success() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = Data {
        bytes: [222u8; Data::DATA_SIZE],
    };
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let new_data = Data {
        bytes: [200u8; Data::DATA_SIZE],
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::write(
            &account.pubkey(),
            &authority.pubkey(),
            0,
            new_data.try_to_vec().unwrap(),
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

    let account_data = context
        .banks_client
        .get_account_data_with_borsh::<RecordData>(account.pubkey())
        .await
        .unwrap();
    assert_eq!(account_data.data, new_data);
    assert_eq!(account_data.authority, authority.pubkey());
    assert_eq!(account_data.version, RecordData::CURRENT_VERSION);
}

#[tokio::test]
async fn write_fail_wrong_authority() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = Data {
        bytes: [222u8; Data::DATA_SIZE],
    };
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let new_data = Data {
        bytes: [200u8; Data::DATA_SIZE],
    };
    let wrong_authority = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::write(
            &account.pubkey(),
            &wrong_authority.pubkey(),
            0,
            new_data.try_to_vec().unwrap(),
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
            InstructionError::Custom(RecordError::IncorrectAuthority as u32)
        )
    );
}

#[tokio::test]
async fn write_fail_unsigned() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = Data {
        bytes: [222u8; Data::DATA_SIZE],
    };
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let data = Data {
        bytes: [200u8; Data::DATA_SIZE],
    }
    .try_to_vec()
    .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[Instruction::new_with_borsh(
            id(),
            &instruction::RecordInstruction::Write { offset: 0, data },
            vec![
                AccountMeta::new(account.pubkey(), false),
                AccountMeta::new_readonly(authority.pubkey(), false),
            ],
        )],
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
    let data = Data {
        bytes: [222u8; Data::DATA_SIZE],
    };
    initialize_storage_account(&mut context, &authority, &account, data).await;
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
        1.max(Rent::default().minimum_balance(get_packed_len::<RecordData>()))
    );
}

#[tokio::test]
async fn close_account_fail_wrong_authority() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = Data {
        bytes: [222u8; Data::DATA_SIZE],
    };
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let wrong_authority = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &[Instruction::new_with_borsh(
            id(),
            &instruction::RecordInstruction::CloseAccount,
            vec![
                AccountMeta::new(account.pubkey(), false),
                AccountMeta::new_readonly(wrong_authority.pubkey(), true),
                AccountMeta::new(Pubkey::new_unique(), false),
            ],
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
            InstructionError::Custom(RecordError::IncorrectAuthority as u32)
        )
    );
}

#[tokio::test]
async fn close_account_fail_unsigned() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = Data {
        bytes: [222u8; Data::DATA_SIZE],
    };
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction::new_with_borsh(
            id(),
            &instruction::RecordInstruction::CloseAccount,
            vec![
                AccountMeta::new(account.pubkey(), false),
                AccountMeta::new_readonly(authority.pubkey(), false),
                AccountMeta::new(Pubkey::new_unique(), false),
            ],
        )],
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
async fn set_authority_success() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = Data {
        bytes: [222u8; Data::DATA_SIZE],
    };
    initialize_storage_account(&mut context, &authority, &account, data).await;
    let new_authority = Keypair::new();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_authority(
            &account.pubkey(),
            &authority.pubkey(),
            &new_authority.pubkey(),
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

    let account_data = context
        .banks_client
        .get_account_data_with_borsh::<RecordData>(account.pubkey())
        .await
        .unwrap();
    assert_eq!(account_data.authority, new_authority.pubkey());

    let new_data = Data {
        bytes: [200u8; Data::DATA_SIZE],
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::write(
            &account.pubkey(),
            &new_authority.pubkey(),
            0,
            new_data.try_to_vec().unwrap(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &new_authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account_data = context
        .banks_client
        .get_account_data_with_borsh::<RecordData>(account.pubkey())
        .await
        .unwrap();
    assert_eq!(account_data.data, new_data);
    assert_eq!(account_data.authority, new_authority.pubkey());
    assert_eq!(account_data.version, RecordData::CURRENT_VERSION);
}

#[tokio::test]
async fn set_authority_fail_wrong_authority() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = Data {
        bytes: [222u8; Data::DATA_SIZE],
    };
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let wrong_authority = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &[Instruction::new_with_borsh(
            id(),
            &instruction::RecordInstruction::SetAuthority,
            vec![
                AccountMeta::new(account.pubkey(), false),
                AccountMeta::new_readonly(wrong_authority.pubkey(), true),
                AccountMeta::new(Pubkey::new_unique(), false),
            ],
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
            InstructionError::Custom(RecordError::IncorrectAuthority as u32)
        )
    );
}

#[tokio::test]
async fn set_authority_fail_unsigned() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = Data {
        bytes: [222u8; Data::DATA_SIZE],
    };
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction::new_with_borsh(
            id(),
            &instruction::RecordInstruction::SetAuthority,
            vec![
                AccountMeta::new(account.pubkey(), false),
                AccountMeta::new_readonly(authority.pubkey(), false),
                AccountMeta::new(Pubkey::new_unique(), false),
            ],
        )],
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
