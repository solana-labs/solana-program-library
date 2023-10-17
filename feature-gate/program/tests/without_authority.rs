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
        instruction::{
            activate_feature, activate_feature_with_rent_transfer, revoke_pending_activation,
        },
    },
};

async fn setup_pending_feature(context: &mut ProgramTestContext, feature_keypair: &Keypair) {
    let transaction = Transaction::new_signed_with_payer(
        &activate_feature_with_rent_transfer(&feature_keypair.pubkey(), &context.payer.pubkey()),
        Some(&context.payer.pubkey()),
        &[&context.payer, feature_keypair],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_activate_feature() {
    let feature_keypair = Keypair::new();
    let mock_invalid_feature = Keypair::new();
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
        mock_invalid_feature.pubkey(),
        SolanaAccount {
            lamports: 500_000_000,
            owner: spl_feature_gate::id(),
            ..SolanaAccount::default()
        },
    );

    let mut context = program_test.start_with_context().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports = rent.minimum_balance(Feature::size_of());

    // Fail: feature not signer
    let mut activate_ix = activate_feature(&feature_keypair.pubkey());
    activate_ix.accounts[0].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &mock_invalid_signer.pubkey(),
                &feature_keypair.pubkey(),
                rent_lamports,
            ),
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
        &[activate_feature(&mock_invalid_feature.pubkey())],
        Some(&context.payer.pubkey()),
        &[&context.payer, &mock_invalid_feature],
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
            system_instruction::transfer(
                &context.payer.pubkey(),
                &feature_keypair.pubkey(),
                rent_lamports,
            ),
            activate_feature(&feature_keypair.pubkey()),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &feature_keypair],
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
        .get_account(feature_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(feature_account.owner, spl_feature_gate::id());

    // Cannot activate the same feature again
    let transaction = Transaction::new_signed_with_payer(
        &[activate_feature(&feature_keypair.pubkey())],
        Some(&context.payer.pubkey()),
        &[&context.payer, &feature_keypair],
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
async fn test_revoke_pending_activation() {
    let feature_keypair = Keypair::new();
    let destination = Pubkey::new_unique();
    let mock_active_feature_keypair = Keypair::new();

    let mut program_test = ProgramTest::new(
        "spl_feature_gate",
        spl_feature_gate::id(),
        processor!(spl_feature_gate::processor::process),
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

    setup_pending_feature(&mut context, &feature_keypair).await;

    // Fail: feature not signer
    let mut revoke_ix = revoke_pending_activation(&feature_keypair.pubkey(), &destination);
    revoke_ix.accounts[0].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[revoke_ix],
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
        TransactionError::InstructionError(0, InstructionError::MissingRequiredSignature)
    );

    // Fail: feature is already active
    let transaction = Transaction::new_signed_with_payer(
        &[revoke_pending_activation(
            &mock_active_feature_keypair.pubkey(),
            &destination,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &mock_active_feature_keypair],
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

    // Success: Revoke a feature activation
    let transaction = Transaction::new_signed_with_payer(
        &[revoke_pending_activation(
            &feature_keypair.pubkey(),
            &destination,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &feature_keypair],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Confirm feature account was closed and destination account received lamports
    let feature_account = context
        .banks_client
        .get_account(feature_keypair.pubkey())
        .await
        .unwrap();
    assert!(feature_account.is_none());
    let destination_account = context
        .banks_client
        .get_account(destination)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(destination_account.lamports, rent_lamports);
}
