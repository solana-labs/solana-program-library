#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program_test::*,
    solana_sdk::{
        account::AccountSharedData,
        signature::Signer,
        stake::{
            stake_flags::StakeFlags,
            state::{Delegation, Stake, StakeStateV2},
        },
        transaction::Transaction,
    },
    spl_single_pool::{error::SinglePoolError, id, instruction},
    test_case::test_case,
};

#[tokio::test]
async fn success() {
    let mut context = program_test(false).start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    accounts
        .initialize_for_deposit(&mut context, TEST_STAKE_AMOUNT, None)
        .await;
    advance_epoch(&mut context).await;

    // deactivate the pool stake account
    let (meta, stake, _) =
        get_stake_account(&mut context.banks_client, &accounts.stake_account).await;
    let delegation = Delegation {
        activation_epoch: 0,
        deactivation_epoch: 0,
        ..stake.unwrap().delegation
    };
    let mut account_data = vec![0; std::mem::size_of::<StakeStateV2>()];
    bincode::serialize_into(
        &mut account_data[..],
        &StakeStateV2::Stake(
            meta,
            Stake {
                delegation,
                ..stake.unwrap()
            },
            StakeFlags::empty(),
        ),
    )
    .unwrap();

    let mut stake_account = get_account(&mut context.banks_client, &accounts.stake_account).await;
    stake_account.data = account_data;
    context.set_account(
        &accounts.stake_account,
        &AccountSharedData::from(stake_account),
    );

    // make sure deposit fails
    let instructions = instruction::deposit(
        &id(),
        &accounts.pool,
        &accounts.alice_stake.pubkey(),
        &accounts.alice_token,
        &accounts.alice.pubkey(),
        &accounts.alice.pubkey(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.alice],
        context.last_blockhash,
    );

    let e = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
    check_error(e, SinglePoolError::WrongStakeStake);

    // reactivate
    let instruction = instruction::reactivate_pool_stake(&id(), &accounts.vote_account.pubkey());
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    advance_epoch(&mut context).await;

    // deposit works again
    let instructions = instruction::deposit(
        &id(),
        &accounts.pool,
        &accounts.alice_stake.pubkey(),
        &accounts.alice_token,
        &accounts.alice.pubkey(),
        &accounts.alice.pubkey(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.alice],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    assert!(context
        .banks_client
        .get_account(accounts.alice_stake.pubkey())
        .await
        .expect("get_account")
        .is_none());
}

#[test_case(true; "activated")]
#[test_case(false; "activating")]
#[tokio::test]
async fn fail_not_deactivated(activate: bool) {
    let mut context = program_test(false).start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    accounts.initialize(&mut context).await;

    if activate {
        advance_epoch(&mut context).await;
    }

    let instruction = instruction::reactivate_pool_stake(&id(), &accounts.vote_account.pubkey());
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    let e = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
    check_error(e, SinglePoolError::WrongStakeStake);
}
