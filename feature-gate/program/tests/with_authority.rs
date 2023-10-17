#![cfg(feature = "test-sbf")]

use {
    solana_program::instruction::InstructionError,
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{
        account::Account as SolanaAccount,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_program,
        transaction::{Transaction, TransactionError},
    },
    spl_feature_gate::{
        error::FeatureGateError, feature_id::derive_feature_id, instruction::activate_feature,
    },
};

#[tokio::test]
async fn test_activate_feature_with_authority() {
    let authority = Keypair::new();
    let mock_invalid_signer = Keypair::new();
    let mock_invalid_feature = Pubkey::new_unique();

    let mut program_test = ProgramTest::new(
        "spl_feature_gate",
        spl_feature_gate::id(),
        processor!(spl_feature_gate::processor::process),
    );

    // Create the authority
    program_test.add_account(
        authority.pubkey(),
        SolanaAccount {
            lamports: 500_000_000,
            owner: system_program::id(),
            ..SolanaAccount::default()
        },
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

    let nonce = 0u16;
    let (feature_id, _) = derive_feature_id(&authority.pubkey(), nonce).unwrap();

    // Fail: incorrect feature ID
    let incorrect_id = Pubkey::new_unique();
    let transaction = Transaction::new_signed_with_payer(
        &[activate_feature(
            &incorrect_id,
            &context.payer.pubkey(),
            Some((&authority.pubkey(), nonce)),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authority],
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
    let mut activate_ix = activate_feature(
        &feature_id,
        &context.payer.pubkey(),
        Some((&authority.pubkey(), nonce)),
    );
    activate_ix.accounts[3].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[activate_ix],
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

    // Fail: feature not owned by system program
    let transaction = Transaction::new_signed_with_payer(
        &[activate_feature(
            &mock_invalid_feature,
            &context.payer.pubkey(),
            Some((&authority.pubkey(), nonce)),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authority],
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
        &[activate_feature(
            &feature_id,
            &context.payer.pubkey(),
            Some((&authority.pubkey(), nonce)),
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

    // Confirm feature account exists with proper configurations
    let feature_account = context
        .banks_client
        .get_account(feature_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(feature_account.owner, spl_feature_gate::id());

    // Cannot activate the same feature again
    let new_latest_blockhash = context.get_new_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[activate_feature(
            &feature_id,
            &context.payer.pubkey(),
            Some((&authority.pubkey(), nonce)),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authority],
        new_latest_blockhash,
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
