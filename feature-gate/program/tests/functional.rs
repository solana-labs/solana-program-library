#![cfg(feature = "test-sbf")]

use {
    solana_program::instruction::InstructionError,
    solana_program_test::{processor, tokio, ProgramTest},
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
        instruction::{activate, revoke},
    },
};

#[tokio::test]
async fn test_functional() {
    let mock_feature_keypair = Keypair::new();
    let feature_keypair = Keypair::new();
    let authority_keypair = Keypair::new();
    let destination = Pubkey::new_unique();

    let mut program_test = ProgramTest::new(
        "spl_feature_gate",
        spl_feature_gate::id(),
        processor!(spl_feature_gate::processor::process),
    );

    // Create the authority account with some lamports for transaction fees
    program_test.add_account(
        authority_keypair.pubkey(),
        SolanaAccount {
            lamports: 500_000_000,
            ..SolanaAccount::default()
        },
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
    let mut activate_ix = activate(
        &spl_feature_gate::id(),
        &feature_keypair.pubkey(),
        &authority_keypair.pubkey(),
    );
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
        &[&context.payer, &authority_keypair],
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

    // Activate: Fail authority not signer
    let mut activate_ix = activate(
        &spl_feature_gate::id(),
        &feature_keypair.pubkey(),
        &authority_keypair.pubkey(),
    );
    activate_ix.accounts[1].is_signer = false;
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
        TransactionError::InstructionError(1, InstructionError::MissingRequiredSignature)
    );

    // Activate: Fail feature not owned by system program
    let transaction = Transaction::new_signed_with_payer(
        &[activate(
            &spl_feature_gate::id(),
            &mock_feature_keypair.pubkey(),
            &authority_keypair.pubkey(),
        )],
        Some(&authority_keypair.pubkey()),
        &[&mock_feature_keypair, &authority_keypair],
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
            activate(
                &spl_feature_gate::id(),
                &feature_keypair.pubkey(),
                &authority_keypair.pubkey(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &feature_keypair, &authority_keypair],
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

    // Revoke: Fail authority not signer
    let mut revoke_ix = revoke(
        &spl_feature_gate::id(),
        &feature_keypair.pubkey(),
        &destination,
        &authority_keypair.pubkey(),
    );
    revoke_ix.accounts[2].is_signer = false;
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

    // Revoke a feature activation
    let transaction = Transaction::new_signed_with_payer(
        &[revoke(
            &spl_feature_gate::id(),
            &feature_keypair.pubkey(),
            &destination,
            &authority_keypair.pubkey(),
        )],
        Some(&authority_keypair.pubkey()),
        &[&authority_keypair],
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
