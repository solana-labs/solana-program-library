#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::items_after_test_module)]
#![cfg(feature = "test-sbf")]
mod helpers;

use {
    helpers::*,
    solana_program::{instruction::InstructionError, pubkey::Pubkey},
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
    },
    spl_stake_pool::{
        error::StakePoolError::{AlreadyInUse, SignatureMissing, WrongManager},
        instruction, MINIMUM_RESERVE_LAMPORTS,
    },
    test_case::test_case,
};

async fn setup(token_program_id: Pubkey) -> (ProgramTestContext, StakePoolAccounts) {
    let mut context = program_test_with_metadata_program()
        .start_with_context()
        .await;
    let stake_pool_accounts = StakePoolAccounts::new_with_token_program(token_program_id);
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            MINIMUM_RESERVE_LAMPORTS,
        )
        .await
        .unwrap();

    (context, stake_pool_accounts)
}

#[test_case(spl_token::id(); "token")]
//#[test_case(spl_token_2022::id(); "token-2022")] enable once metaplex supports token-2022
#[tokio::test]
async fn success(token_program_id: Pubkey) {
    let (mut context, stake_pool_accounts) = setup(token_program_id).await;

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

    let metadata = get_metadata_account(
        &mut context.banks_client,
        &stake_pool_accounts.pool_mint.pubkey(),
    )
    .await;

    assert!(metadata.name.starts_with(name));
    assert!(metadata.symbol.starts_with(symbol));
    assert!(metadata.uri.starts_with(uri));
}

#[tokio::test]
async fn fail_manager_did_not_sign() {
    let (context, stake_pool_accounts) = setup(spl_token::id()).await;

    let name = "test_name";
    let symbol = "SYM";
    let uri = "test_uri";

    let mut ix = instruction::create_token_metadata(
        &spl_stake_pool::id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        &stake_pool_accounts.manager.pubkey(),
        &stake_pool_accounts.pool_mint.pubkey(),
        &context.payer.pubkey(),
        name.to_string(),
        symbol.to_string(),
        uri.to_string(),
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
    let (context, stake_pool_accounts) = setup(spl_token::id()).await;

    let name = "test_name";
    let symbol = "SYM";
    let uri = "test_uri";

    let random_keypair = Keypair::new();
    let ix = instruction::create_token_metadata(
        &spl_stake_pool::id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        &random_keypair.pubkey(),
        &stake_pool_accounts.pool_mint.pubkey(),
        &context.payer.pubkey(),
        name.to_string(),
        symbol.to_string(),
        uri.to_string(),
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

#[tokio::test]
async fn fail_wrong_mpl_metadata_program() {
    let (context, stake_pool_accounts) = setup(spl_token::id()).await;

    let name = "test_name";
    let symbol = "SYM";
    let uri = "test_uri";

    let random_keypair = Keypair::new();
    let mut ix = instruction::create_token_metadata(
        &spl_stake_pool::id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        &random_keypair.pubkey(),
        &stake_pool_accounts.pool_mint.pubkey(),
        &context.payer.pubkey(),
        name.to_string(),
        symbol.to_string(),
        uri.to_string(),
    );
    ix.accounts[7].pubkey = Pubkey::new_unique();

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
        TransactionError::InstructionError(_, error) => {
            assert_eq!(error, InstructionError::IncorrectProgramId);
        }
        _ => panic!(
            "Wrong error occurs while try to create metadata with wrong mpl token metadata program ID"
        ),
    }
}

#[tokio::test]
async fn fail_create_metadata_twice() {
    let (context, stake_pool_accounts) = setup(spl_token::id()).await;

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
        &[ix.clone()],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );

    let latest_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let transaction_2 = Transaction::new_signed_with_payer(
        &[ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        latest_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let error = context
        .banks_client
        .process_transaction(transaction_2)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = AlreadyInUse as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while trying to create pool token metadata twice"),
    }
}
