#![cfg(feature = "test-sbf")]

use {
    solana_instruction::{error::InstructionError, AccountMeta, Instruction},
    solana_program_test::*,
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    solana_sdk::{
        signature::{Keypair, Signer},
        system_instruction,
        transaction::{Transaction, TransactionError},
    },
    spl_record::{
        error::RecordError, id, instruction, processor::process_instruction, state::RecordData,
    },
};

fn program_test() -> ProgramTest {
    ProgramTest::new("spl_record", id(), processor!(process_instruction))
}

async fn initialize_storage_account(
    context: &mut ProgramTestContext,
    authority: &Keypair,
    account: &Keypair,
    data: &[u8],
) {
    let account_length = std::mem::size_of::<RecordData>()
        .checked_add(data.len())
        .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &account.pubkey(),
                1.max(Rent::default().minimum_balance(account_length)),
                account_length as u64,
                &id(),
            ),
            instruction::initialize(&account.pubkey(), &authority.pubkey()),
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
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let account = context
        .banks_client
        .get_account(account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let account_data =
        bytemuck::try_from_bytes::<RecordData>(&account.data[..RecordData::WRITABLE_START_INDEX])
            .unwrap();
    assert_eq!(account_data.authority, authority.pubkey());
    assert_eq!(account_data.version, RecordData::CURRENT_VERSION);
    assert_eq!(&account.data[RecordData::WRITABLE_START_INDEX..], data);
}

#[tokio::test]
async fn initialize_with_seed_success() {
    let context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let seed = "storage";
    let account = Pubkey::create_with_seed(&authority.pubkey(), seed, &id()).unwrap();
    let data = &[111u8; 8];
    let account_length = std::mem::size_of::<RecordData>()
        .checked_add(data.len())
        .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account_with_seed(
                &context.payer.pubkey(),
                &account,
                &authority.pubkey(),
                seed,
                1.max(Rent::default().minimum_balance(account_length)),
                account_length as u64,
                &id(),
            ),
            instruction::initialize(&account, &authority.pubkey()),
            instruction::write(&account, &authority.pubkey(), 0, data),
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
    let account = context
        .banks_client
        .get_account(account)
        .await
        .unwrap()
        .unwrap();
    let account_data =
        bytemuck::try_from_bytes::<RecordData>(&account.data[..RecordData::WRITABLE_START_INDEX])
            .unwrap();
    assert_eq!(account_data.authority, authority.pubkey());
    assert_eq!(account_data.version, RecordData::CURRENT_VERSION);
    assert_eq!(&account.data[RecordData::WRITABLE_START_INDEX..], data);
}

#[tokio::test]
async fn initialize_twice_fail() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[111u8; 8];
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
    let data = &[222u8; 8];
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let new_data = &[200u8; 8];
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
        bytemuck::try_from_bytes::<RecordData>(&account.data[..RecordData::WRITABLE_START_INDEX])
            .unwrap();
    assert_eq!(account_data.authority, authority.pubkey());
    assert_eq!(account_data.version, RecordData::CURRENT_VERSION);
    assert_eq!(&account.data[RecordData::WRITABLE_START_INDEX..], new_data);
}

#[tokio::test]
async fn write_fail_wrong_authority() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    initialize_storage_account(&mut context, &authority, &account, data).await;

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
            InstructionError::Custom(RecordError::IncorrectAuthority as u32)
        )
    );
}

#[tokio::test]
async fn write_fail_unsigned() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let data = &[200u8; 8];

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id: id(),
            accounts: vec![
                AccountMeta::new(account.pubkey(), false),
                AccountMeta::new_readonly(authority.pubkey(), false),
            ],
            data: instruction::RecordInstruction::Write { offset: 0, data }.pack(),
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
    let account_length = std::mem::size_of::<RecordData>()
        .checked_add(data.len())
        .unwrap();
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
        1.max(Rent::default().minimum_balance(account_length))
    );
}

#[tokio::test]
async fn close_account_fail_wrong_authority() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let wrong_authority = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id: id(),
            accounts: vec![
                AccountMeta::new(account.pubkey(), false),
                AccountMeta::new_readonly(wrong_authority.pubkey(), true),
                AccountMeta::new(Pubkey::new_unique(), false),
            ],
            data: instruction::RecordInstruction::CloseAccount.pack(),
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
            InstructionError::Custom(RecordError::IncorrectAuthority as u32)
        )
    );
}

#[tokio::test]
async fn close_account_fail_unsigned() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8, 8];
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id: id(),
            accounts: vec![
                AccountMeta::new(account.pubkey(), false),
                AccountMeta::new_readonly(authority.pubkey(), false),
                AccountMeta::new(Pubkey::new_unique(), false),
            ],
            data: instruction::RecordInstruction::CloseAccount.pack(),
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
async fn set_authority_success() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
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

    let account_handle = context
        .banks_client
        .get_account(account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let account_data = bytemuck::try_from_bytes::<RecordData>(
        &account_handle.data[..RecordData::WRITABLE_START_INDEX],
    )
    .unwrap();
    assert_eq!(account_data.authority, new_authority.pubkey());

    let new_data = &[200u8; 8];
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::write(
            &account.pubkey(),
            &new_authority.pubkey(),
            0,
            new_data,
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

    let account_handle = context
        .banks_client
        .get_account(account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let account_data = bytemuck::try_from_bytes::<RecordData>(
        &account_handle.data[..RecordData::WRITABLE_START_INDEX],
    )
    .unwrap();
    assert_eq!(account_data.authority, new_authority.pubkey());
    assert_eq!(account_data.version, RecordData::CURRENT_VERSION);
    assert_eq!(
        &account_handle.data[RecordData::WRITABLE_START_INDEX..],
        new_data,
    );
}

#[tokio::test]
async fn set_authority_fail_wrong_authority() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let wrong_authority = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id: id(),
            accounts: vec![
                AccountMeta::new(account.pubkey(), false),
                AccountMeta::new_readonly(wrong_authority.pubkey(), true),
                AccountMeta::new(Pubkey::new_unique(), false),
            ],
            data: instruction::RecordInstruction::SetAuthority.pack(),
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
            InstructionError::Custom(RecordError::IncorrectAuthority as u32)
        )
    );
}

#[tokio::test]
async fn set_authority_fail_unsigned() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id: id(),
            accounts: vec![
                AccountMeta::new(account.pubkey(), false),
                AccountMeta::new_readonly(authority.pubkey(), false),
                AccountMeta::new(Pubkey::new_unique(), false),
            ],
            data: instruction::RecordInstruction::SetAuthority.pack(),
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
async fn reallocate_success() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let new_data_length = 16u64;
    let expected_account_data_length = RecordData::WRITABLE_START_INDEX
        .checked_add(new_data_length as usize)
        .unwrap();

    let delta_account_data_length = new_data_length.saturating_sub(data.len() as u64);
    let additional_lamports_needed =
        Rent::default().minimum_balance(delta_account_data_length as usize);

    let transaction = Transaction::new_signed_with_payer(
        &[
            instruction::reallocate(&account.pubkey(), &authority.pubkey(), new_data_length),
            system_instruction::transfer(
                &context.payer.pubkey(),
                &account.pubkey(),
                additional_lamports_needed,
            ),
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

    let account_handle = context
        .banks_client
        .get_account(account.pubkey())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(account_handle.data.len(), expected_account_data_length);

    // reallocate to a smaller length
    let old_data_length = 8u64;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::reallocate(
            &account.pubkey(),
            &authority.pubkey(),
            old_data_length,
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

    assert_eq!(account.data.len(), expected_account_data_length);
}

#[tokio::test]
async fn reallocate_fail_wrong_authority() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let new_data_length = 16u64;
    let delta_account_data_length = new_data_length.saturating_sub(data.len() as u64);
    let additional_lamports_needed =
        Rent::default().minimum_balance(delta_account_data_length as usize);

    let wrong_authority = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &[
            Instruction {
                program_id: id(),
                accounts: vec![
                    AccountMeta::new(account.pubkey(), false),
                    AccountMeta::new(wrong_authority.pubkey(), true),
                ],
                data: instruction::RecordInstruction::Reallocate {
                    data_length: new_data_length,
                }
                .pack(),
            },
            system_instruction::transfer(
                &context.payer.pubkey(),
                &account.pubkey(),
                additional_lamports_needed,
            ),
        ],
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
async fn reallocate_fail_unsigned() {
    let mut context = program_test().start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let new_data_length = 16u64;
    let delta_account_data_length = new_data_length.saturating_sub(data.len() as u64);
    let additional_lamports_needed =
        Rent::default().minimum_balance(delta_account_data_length as usize);

    let transaction = Transaction::new_signed_with_payer(
        &[
            Instruction {
                program_id: id(),
                accounts: vec![
                    AccountMeta::new(account.pubkey(), false),
                    AccountMeta::new(authority.pubkey(), false),
                ],
                data: instruction::RecordInstruction::Reallocate {
                    data_length: new_data_length,
                }
                .pack(),
            },
            system_instruction::transfer(
                &context.payer.pubkey(),
                &account.pubkey(),
                additional_lamports_needed,
            ),
        ],
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
