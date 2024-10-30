#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]
mod helpers;

use {
    helpers::*,
    solana_program::instruction::InstructionError,
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
    },
    spl_stake_pool::{
        error::StakePoolError::{SignatureMissing, WrongManager},
        instruction, MINIMUM_RESERVE_LAMPORTS,
    },
};

async fn setup() -> (ProgramTestContext, StakePoolAccounts) {
    let mut context = program_test_with_metadata_program()
        .start_with_context()
        .await;
    let stake_pool_accounts = StakePoolAccounts::default();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            MINIMUM_RESERVE_LAMPORTS,
        )
        .await
        .unwrap();

    let name = "test_name";
    let symbol = "SYM";
    let uri = "test_uri";

    let ix = instruction::create_token_metadata(
        &spl_stake_pool::id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        &stake_pool_accounts.manager.pubkey(),
        &stake_pool_accounts.pool_mint.pubkey(),
        &context.payer.pubkey(),
        name.to_string(),
        symbol.to_string(),
        uri.to_string(),
    );

    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    (context, stake_pool_accounts)
}

#[tokio::test]
async fn success_update_pool_token_metadata() {
    let (mut context, stake_pool_accounts) = setup().await;

    let updated_name = "updated_name";
    let updated_symbol = "USYM";
    let updated_uri = "updated_uri";

    let ix = instruction::update_token_metadata(
        &spl_stake_pool::id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        &stake_pool_accounts.manager.pubkey(),
        &stake_pool_accounts.pool_mint.pubkey(),
        updated_name.to_string(),
        updated_symbol.to_string(),
        updated_uri.to_string(),
    );

    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let metadata = get_metadata_account(
        &mut context.banks_client,
        &stake_pool_accounts.pool_mint.pubkey(),
    )
    .await;

    assert!(metadata.name.starts_with(updated_name));
    assert!(metadata.symbol.starts_with(updated_symbol));
    assert!(metadata.uri.starts_with(updated_uri));
}

#[tokio::test]
async fn fail_manager_did_not_sign() {
    let (context, stake_pool_accounts) = setup().await;

    let updated_name = "updated_name";
    let updated_symbol = "USYM";
    let updated_uri = "updated_uri";

    let mut ix = instruction::update_token_metadata(
        &spl_stake_pool::id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        &stake_pool_accounts.manager.pubkey(),
        &stake_pool_accounts.pool_mint.pubkey(),
        updated_name.to_string(),
        updated_symbol.to_string(),
        updated_uri.to_string(),
    );
    ix.accounts[1].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = SignatureMissing as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while manager signature missing"),
    }
}

#[tokio::test]
async fn fail_wrong_manager_signed() {
    let (context, stake_pool_accounts) = setup().await;

    let updated_name = "updated_name";
    let updated_symbol = "USYM";
    let updated_uri = "updated_uri";

    let random_keypair = Keypair::new();
    let ix = instruction::update_token_metadata(
        &spl_stake_pool::id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        &random_keypair.pubkey(),
        &stake_pool_accounts.pool_mint.pubkey(),
        updated_name.to_string(),
        updated_symbol.to_string(),
        updated_uri.to_string(),
    );

    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &random_keypair],
        context.last_blockhash,
    );

    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = WrongManager as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while signing with the wrong manager"),
    }
}
