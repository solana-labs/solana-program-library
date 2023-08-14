#![allow(clippy::integer_arithmetic)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program_test::*,
    solana_sdk::{program_pack::Pack, signature::Signer, stake, transaction::Transaction},
    spl_single_validator_pool::{error::SinglePoolError, id, instruction},
    spl_token::state::Mint,
};

#[tokio::test]
async fn success() {
    let mut context = program_test().start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    accounts.initialize(&mut context).await;

    // mint exists
    let mint_account = get_account(&mut context.banks_client, &accounts.mint).await;
    Mint::unpack_from_slice(&mint_account.data).unwrap();

    // stake account exists
    let stake_account = get_account(&mut context.banks_client, &accounts.stake_account).await;
    assert_eq!(stake_account.owner, stake::program::id());
}

#[tokio::test]
async fn fail_double_init() {
    let mut context = program_test().start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    let minimum_delegation = accounts.initialize(&mut context).await;
    refresh_blockhash(&mut context).await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let instructions = instruction::initialize(
        &id(),
        &accounts.vote_account.pubkey(),
        &context.payer.pubkey(),
        &rent,
        minimum_delegation,
    );
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    let e = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
    check_error(e, SinglePoolError::PoolAlreadyInitialized);
}

// TODO test that init can succeed without mpl program
