#![cfg(feature = "test-sbf")]

use {
    solana_program::instruction::InstructionError,
    solana_program_test::{processor, tokio, ProgramTest, ProgramTestContext},
    solana_sdk::{
        account::Account as SolanaAccount,
        feature::Feature,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_instruction, system_program,
        transaction::{Transaction, TransactionError},
    },
    spl_feature_gate::{
        error::FeatureGateError,
        feature_id::derive_feature_id,
        instruction::{
            activate_feature_with_authority, activate_feature_with_authority_with_rent_transfer,
            revoke_pending_activation_with_authority,
        },
    },
};

async fn setup_pending_feature_with_authority(
    context: &mut ProgramTestContext,
    feature_id: &Pubkey,
    nonce: u16,
) {
    let transaction = Transaction::new_signed_with_payer(
        &activate_feature_with_authority_with_rent_transfer(
            feature_id,
            &context.payer.pubkey(),
            nonce,
        ),
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_activate_feature_with_authority() {
    let mock_invalid_feature = Pubkey::new_unique();
    let mock_invalid_signer = Keypair::new();

    let mut program_test = ProgramTest::new(
        "spl_feature_gate",
        spl_feature_gate::id(),
        processor!(spl_feature_gate::processor::process),
    );

    // Need to fund this account for a test transfer later
    program_test.add_account(
        mock_invalid_signer.pubkey(),
        SolanaAccount {
            lamports: 500_000_000,
            owner: system_program::id(),
            ..SolanaAccount::default()
        },
    );
    // Add a mock account that's NOT a valid feature account for testing later
    program_test.add_account(
        mock_invalid_feature,
        SolanaAccount {
            lamports: 500_000_000,
            owner: spl_feature_gate::id(),
            ..SolanaAccount::default()
        },
    );

    let mut context = program_test.start_with_context().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports = rent.minimum_balance(Feature::size_of());

    let nonce = 0u16;
    let (feature_id, _) = derive_feature_id(&context.payer.pubkey(), nonce).unwrap();

    // Fail: incorrect feature ID
    let incorrect_id = Pubkey::new_unique();
    let transaction = Transaction::new_signed_with_payer(
        &[activate_feature_with_authority(
            &incorrect_id,
            &context.payer.pubkey(),
            nonce,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(FeatureGateError::IncorrectFeatureId as u32)
        )
    );

    // Fail: authority not signer
    let mut activate_ix =
        activate_feature_with_authority(&feature_id, &context.payer.pubkey(), nonce);
    activate_ix.accounts[1].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(&mock_invalid_signer.pubkey(), &feature_id, rent_lamports),
            activate_ix,
        ],
        Some(&mock_invalid_signer.pubkey()),
        &[&mock_invalid_signer],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(1, InstructionError::MissingRequiredSignature)
    );

    // Fail: feature not owned by system program
    let transaction = Transaction::new_signed_with_payer(
        &[activate_feature_with_authority(
            &mock_invalid_feature,
            &context.payer.pubkey(),
            nonce,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(FeatureGateError::InvalidFeatureAccount as u32),
        )
    );

    // Success: Submit a feature for activation
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(&context.payer.pubkey(), &feature_id, rent_lamports),
            activate_feature_with_authority(&feature_id, &context.payer.pubkey(), nonce),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Confirm feature account exists with proper configurations
    let feature_account = context
        .banks_client
        .get_account(feature_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(feature_account.owner, spl_feature_gate::id());

    // Cannot activate the same feature again
    let transaction = Transaction::new_signed_with_payer(
        &[activate_feature_with_authority(
            &feature_id,
            &context.payer.pubkey(),
            nonce,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(FeatureGateError::InvalidFeatureAccount as u32)
        )
    );
}

#[tokio::test]
async fn test_revoke_pending_activation_with_authority() {
    let destination = Pubkey::new_unique();
    let incorrect_id = Pubkey::new_unique();
    let mock_active_feature_keypair = Keypair::new();
    let mock_invalid_signer = Keypair::new();

    let mut program_test = ProgramTest::new(
        "spl_feature_gate",
        spl_feature_gate::id(),
        processor!(spl_feature_gate::processor::process),
    );

    // Need to fund this account for a test transfer later
    program_test.add_account(
        mock_invalid_signer.pubkey(),
        SolanaAccount {
            lamports: 500_000_000,
            owner: system_program::id(),
            ..SolanaAccount::default()
        },
    );
    // Add a mock empty feature for testing later
    program_test.add_account(
        incorrect_id,
        SolanaAccount {
            lamports: 500_000_000,
            owner: spl_feature_gate::id(),
            data: vec![],
            ..SolanaAccount::default()
        },
    );
    // Add a mock _active_ feature for testing later
    program_test.add_account(
        mock_active_feature_keypair.pubkey(),
        SolanaAccount {
            lamports: 500_000_000,
            owner: spl_feature_gate::id(),
            data: vec![
                1, // `Some()`
                45, 0, 0, 0, 0, 0, 0, 0, // Random slot `u64`
            ],
            ..SolanaAccount::default()
        },
    );

    let mut context = program_test.start_with_context().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports = rent.minimum_balance(Feature::size_of()); // For checking account balance later

    let nonce = 0u16;
    let (feature_id, _) = derive_feature_id(&context.payer.pubkey(), nonce).unwrap();

    setup_pending_feature_with_authority(&mut context, &feature_id, nonce).await;

    // Fail: incorrect feature ID
    let transaction = Transaction::new_signed_with_payer(
        &[revoke_pending_activation_with_authority(
            &incorrect_id,
            &context.payer.pubkey(),
            &destination,
            nonce,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(FeatureGateError::IncorrectFeatureId as u32)
        )
    );

    // Fail: authority not signer
    let mut revoke_ix = revoke_pending_activation_with_authority(
        &feature_id,
        &context.payer.pubkey(),
        &destination,
        nonce,
    );
    revoke_ix.accounts[1].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[revoke_ix],
        Some(&mock_invalid_signer.pubkey()),
        &[&mock_invalid_signer],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(0, InstructionError::MissingRequiredSignature)
    );

    // Success: Revoke a feature activation
    let transaction = Transaction::new_signed_with_payer(
        &[revoke_pending_activation_with_authority(
            &feature_id,
            &context.payer.pubkey(),
            &destination,
            nonce,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Confirm feature account was closed and destination account received lamports
    let feature_account = context.banks_client.get_account(feature_id).await.unwrap();
    assert!(feature_account.is_none());
    let destination_account = context
        .banks_client
        .get_account(destination)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(destination_account.lamports, rent_lamports);
}

// Fail: feature already active
#[tokio::test]
async fn test_revoke_pending_activation_with_authority_fail_already_active() {
    let destination = Pubkey::new_unique();
    let authority = Keypair::new();

    let nonce = 0u16;
    let (feature_id, _) = derive_feature_id(&authority.pubkey(), nonce).unwrap();

    let mut program_test = ProgramTest::new(
        "spl_feature_gate",
        spl_feature_gate::id(),
        processor!(spl_feature_gate::processor::process),
    );

    // Fund the authority
    program_test.add_account(
        authority.pubkey(),
        SolanaAccount {
            lamports: 500_000_000,
            owner: system_program::id(),
            data: vec![],
            ..SolanaAccount::default()
        },
    );
    // Make the feature account active
    program_test.add_account(
        feature_id,
        SolanaAccount {
            lamports: 500_000_000,
            owner: spl_feature_gate::id(),
            data: vec![
                1, // `Some()`
                45, 0, 0, 0, 0, 0, 0, 0, // Random slot `u64`
            ],
            ..SolanaAccount::default()
        },
    );

    let mut context = program_test.start_with_context().await;

    let transaction = Transaction::new_signed_with_payer(
        &[revoke_pending_activation_with_authority(
            &feature_id,
            &authority.pubkey(),
            &destination,
            nonce,
        )],
        Some(&authority.pubkey()),
        &[&authority],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(FeatureGateError::FeatureAlreadyActivated as u32)
        )
    );
}
