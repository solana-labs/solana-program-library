#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program_test::*,
    solana_sdk::{program_pack::Pack, signature::Signer, stake, transaction::Transaction},
    spl_single_pool::{error::SinglePoolError, id, instruction},
    spl_token::state::Mint,
    test_case::test_case,
};

#[test_case(true; "minimum_enabled")]
#[test_case(false; "minimum_disabled")]
#[tokio::test]
async fn success(enable_minimum_delegation: bool) {
    let mut context = program_test(enable_minimum_delegation)
        .start_with_context()
        .await;
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
    let mut context = program_test(false).start_with_context().await;
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

#[test_case(true; "minimum_enabled")]
#[test_case(false; "minimum_disabled")]
#[tokio::test]
async fn fail_below_pool_minimum(enable_minimum_delegation: bool) {
    let mut context = program_test(enable_minimum_delegation)
        .start_with_context()
        .await;
    let accounts = SinglePoolAccounts::default();
    let slot = context.genesis_config().epoch_schedule.first_normal_slot + 1;
    context.warp_to_slot(slot).unwrap();

    create_vote(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &accounts.validator,
        &accounts.voter.pubkey(),
        &accounts.withdrawer.pubkey(),
        &accounts.vote_account,
    )
    .await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let minimum_delegation = get_pool_minimum_delegation(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;

    let instructions = instruction::initialize(
        &id(),
        &accounts.vote_account.pubkey(),
        &context.payer.pubkey(),
        &rent,
        minimum_delegation - 1,
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
    check_error(e, SinglePoolError::WrongRentAmount);
}

// TODO test that init can succeed without mpl program
