// Mark this test as BPF-only due to current `ProgramTest` limitations when CPIing into the system program
#![cfg(feature = "test-bpf")]

use {
    solana_program::{
        borsh_utils::get_packed_len,
        instruction::{AccountMeta, Instruction, InstructionError},
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
    },
    solana_program_test::{processor, ProgramTest, ProgramTestContext},
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
        transport,
    },
    spl_crud::{
        error::CrudError,
        id, instruction,
        processor::process_instruction,
        state::{AccountData, Data},
    },
};

fn program_test() -> ProgramTest {
    ProgramTest::new("spl_crud", id(), processor!(process_instruction))
}

async fn create_account_data(
    context: &mut ProgramTestContext,
    owner: &Pubkey,
    account: &Keypair,
    data: Data,
) -> transport::Result<()> {
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &account.pubkey(),
                1.max(Rent::default().minimum_balance(get_packed_len::<AccountData>())),
                get_packed_len::<AccountData>() as u64,
                &id(),
            ),
            instruction::create(&account.pubkey(), owner, data),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, account],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(transaction).await
}

#[tokio::test]
async fn create_success() {
    let mut context = program_test().start_with_context().await;

    let owner = Pubkey::new_unique();
    let account = Keypair::new();
    let data = Data {
        bytes: [111u8; Data::DATA_SIZE],
    };
    create_account_data(&mut context, &owner, &account, data.clone())
        .await
        .unwrap();
    let account_data = context
        .banks_client
        .get_account_data::<AccountData>(account.pubkey())
        .await
        .unwrap();
    assert_eq!(account_data.data, data);
    assert_eq!(account_data.owner, owner);
    assert_eq!(account_data.version, AccountData::CURRENT_VERSION);
}

#[tokio::test]
async fn create_twice_fail() {
    let mut context = program_test().start_with_context().await;

    let owner = Pubkey::new_unique();
    let account = Keypair::new();
    let data = Data {
        bytes: [111u8; Data::DATA_SIZE],
    };
    create_account_data(&mut context, &owner, &account, data.clone())
        .await
        .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::create(&account.pubkey(), &owner, data)],
        Some(&context.payer.pubkey()),
        &[&context.payer, &account],
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
async fn update_success() {
    let mut context = program_test().start_with_context().await;

    let owner = Keypair::new();
    let account = Keypair::new();
    let data = Data {
        bytes: [222u8; Data::DATA_SIZE],
    };
    create_account_data(&mut context, &owner.pubkey(), &account, data)
        .await
        .unwrap();

    let new_data = Data {
        bytes: [200u8; Data::DATA_SIZE],
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::update(
            &account.pubkey(),
            &owner.pubkey(),
            new_data.clone(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &owner],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account_data = context
        .banks_client
        .get_account_data::<AccountData>(account.pubkey())
        .await
        .unwrap();
    assert_eq!(account_data.data, new_data);
    assert_eq!(account_data.owner, owner.pubkey());
    assert_eq!(account_data.version, AccountData::CURRENT_VERSION);
}

#[tokio::test]
async fn update_fail_wrong_owner() {
    let mut context = program_test().start_with_context().await;

    let owner = Keypair::new();
    let account = Keypair::new();
    let data = Data {
        bytes: [222u8; Data::DATA_SIZE],
    };
    create_account_data(&mut context, &owner.pubkey(), &account, data)
        .await
        .unwrap();

    let new_data = Data {
        bytes: [200u8; Data::DATA_SIZE],
    };
    let wrong_owner = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::update(
            &account.pubkey(),
            &wrong_owner.pubkey(),
            new_data.clone(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wrong_owner],
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
            InstructionError::Custom(CrudError::IncorrectOwner as u32)
        )
    );
}

#[tokio::test]
async fn update_fail_unsigned() {
    let mut context = program_test().start_with_context().await;

    let owner = Keypair::new();
    let account = Keypair::new();
    let data = Data {
        bytes: [222u8; Data::DATA_SIZE],
    };
    create_account_data(&mut context, &owner.pubkey(), &account, data)
        .await
        .unwrap();

    let data = Data {
        bytes: [200u8; Data::DATA_SIZE],
    };
    let transaction = Transaction::new_signed_with_payer(
        &[Instruction::new_from_borsh(
            id(),
            &instruction::CrudInstruction::Update { data },
            vec![
                AccountMeta::new(account.pubkey(), false),
                AccountMeta::new_readonly(owner.pubkey(), false),
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
async fn delete_success() {
    let mut context = program_test().start_with_context().await;

    let owner = Keypair::new();
    let account = Keypair::new();
    let data = Data {
        bytes: [222u8; Data::DATA_SIZE],
    };
    create_account_data(&mut context, &owner.pubkey(), &account, data)
        .await
        .unwrap();
    let recipient = Pubkey::new_unique();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::delete(
            &account.pubkey(),
            &owner.pubkey(),
            &recipient,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &owner],
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
        1.max(Rent::default().minimum_balance(get_packed_len::<AccountData>()))
    );
}

#[tokio::test]
async fn delete_fail_wrong_owner() {
    let mut context = program_test().start_with_context().await;

    let owner = Keypair::new();
    let account = Keypair::new();
    let data = Data {
        bytes: [222u8; Data::DATA_SIZE],
    };
    create_account_data(&mut context, &owner.pubkey(), &account, data)
        .await
        .unwrap();

    let wrong_owner = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &[Instruction::new_from_borsh(
            id(),
            &instruction::CrudInstruction::Delete,
            vec![
                AccountMeta::new(account.pubkey(), false),
                AccountMeta::new_readonly(wrong_owner.pubkey(), true),
                AccountMeta::new(Pubkey::new_unique(), false),
            ],
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wrong_owner],
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
            InstructionError::Custom(CrudError::IncorrectOwner as u32)
        )
    );
}

#[tokio::test]
async fn delete_fail_unsigned() {
    let mut context = program_test().start_with_context().await;

    let owner = Keypair::new();
    let account = Keypair::new();
    let data = Data {
        bytes: [222u8; Data::DATA_SIZE],
    };
    create_account_data(&mut context, &owner.pubkey(), &account, data)
        .await
        .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction::new_from_borsh(
            id(),
            &instruction::CrudInstruction::Delete,
            vec![
                AccountMeta::new(account.pubkey(), false),
                AccountMeta::new_readonly(owner.pubkey(), false),
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
