// #![cfg(feature = "test-sbf")]

use {
    solana_program::instruction::InstructionError,
    solana_program_test::{processor, tokio, ProgramTest, ProgramTestContext},
    solana_sdk::{
        account::Account as SolanaAccount,
        feature::Feature,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_instruction,
        transaction::{Transaction, TransactionError},
    },
    spl_feature_gate::{
        error::FeatureGateError,
        instruction::{activate, activate_with_rent_transfer, revoke},
    },
};

async fn setup_feature(context: &mut ProgramTestContext, feature_keypair: &Keypair) {
    let transaction = Transaction::new_signed_with_payer(
        &activate_with_rent_transfer(
            &spl_feature_gate::id(),
            &feature_keypair.pubkey(),
            &context.payer.pubkey(),
        ),
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
async fn test_activate() {
    let mock_feature_keypair = Keypair::new();
    let feature_keypair = Keypair::new();

    let mut program_test = ProgramTest::new(
        "spl_feature_gate",
        spl_feature_gate::id(),
        processor!(spl_feature_gate::processor::process),
    );

    // Add a mock feature for testing later
    program_test.add_account(
        mock_feature_keypair.pubkey(),
        SolanaAccount {
            lamports: 500_000_000,
            owner: spl_feature_gate::id(),
            ..SolanaAccount::default()
        },
    );

    let mut context = program_test.start_with_context().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports = rent.minimum_balance(Feature::size_of());

    // Activate: Fail feature not signer
    let mut activate_ix = activate(&spl_feature_gate::id(), &feature_keypair.pubkey());
    activate_ix.accounts[0].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &context.payer.pubkey(),
                &feature_keypair.pubkey(),
                rent_lamports,
            ),
            activate_ix,
        ],
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
        TransactionError::InstructionError(1, InstructionError::MissingRequiredSignature)
    );

    // Activate: Fail feature not owned by system program
    let transaction = Transaction::new_signed_with_payer(
        &[activate(
            &spl_feature_gate::id(),
            &mock_feature_keypair.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &mock_feature_keypair],
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
            InstructionError::Custom(FeatureGateError::FeatureNotSystemAccount as u32),
        )
    );

    // Submit a feature for activation
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &context.payer.pubkey(),
                &feature_keypair.pubkey(),
                rent_lamports,
            ),
            activate(&spl_feature_gate::id(), &feature_keypair.pubkey()),
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
}

#[tokio::test]
async fn test_revoke() {
    let feature_keypair = Keypair::new();
    let destination = Pubkey::new_unique();
    let mock_active_feature_keypair = Keypair::new();

    let mut program_test = ProgramTest::new(
        "spl_feature_gate",
        spl_feature_gate::id(),
        processor!(spl_feature_gate::processor::process),
    );

    // Add a mock feature that might be active for testing later
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

    setup_feature(&mut context, &feature_keypair).await;

    // Revoke: Fail feature not signer
    let mut revoke_ix = revoke(
        &spl_feature_gate::id(),
        &feature_keypair.pubkey(),
        &destination,
    );
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

    // Revoke: Fail feature not inactive
    let transaction = Transaction::new_signed_with_payer(
        &[revoke(
            &spl_feature_gate::id(),
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
            InstructionError::Custom(FeatureGateError::FeatureNotInactive as u32)
        )
    );

    // Revoke a feature activation
    let transaction = Transaction::new_signed_with_payer(
        &[revoke(
            &spl_feature_gate::id(),
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
