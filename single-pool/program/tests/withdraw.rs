#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program_test::*,
    solana_sdk::{signature::Signer, transaction::Transaction},
    spl_single_pool::{error::SinglePoolError, id, instruction},
    test_case::test_case,
};

#[test_case(true, 0, false, false, false; "activated::minimum_disabled")]
#[test_case(true, 0, false, false, true; "activated::minimum_disabled::small")]
#[test_case(true, 0, false, true, false; "activated::minimum_enabled")]
#[test_case(false, 0, false, false, false; "activating::minimum_disabled")]
#[test_case(false, 0, false, false, true; "activating::minimum_disabled::small")]
#[test_case(false, 0, false, true, false; "activating::minimum_enabled")]
#[test_case(true, 100_000, false, false, false; "activated::extra")]
#[test_case(false, 100_000, false, false, false; "activating::extra")]
#[test_case(true, 0, true, false, false; "activated::second")]
#[test_case(false, 0, true, false, false; "activating::second")]
#[tokio::test]
async fn success(
    activate: bool,
    extra_lamports: u64,
    prior_deposit: bool,
    enable_minimum_delegation: bool,
    small_deposit: bool,
) {
    let mut context = program_test(enable_minimum_delegation)
        .start_with_context()
        .await;
    let accounts = SinglePoolAccounts::default();

    let amount_deposited = if small_deposit { 1 } else { TEST_STAKE_AMOUNT };

    let minimum_delegation = accounts
        .initialize_for_withdraw(
            &mut context,
            amount_deposited,
            if prior_deposit {
                Some(TEST_STAKE_AMOUNT * 10)
            } else {
                None
            },
            activate,
        )
        .await;

    let (_, _, pool_lamports_before) =
        get_stake_account(&mut context.banks_client, &accounts.stake_account).await;

    let wallet_lamports_before = get_account(&mut context.banks_client, &accounts.alice.pubkey())
        .await
        .lamports;

    if extra_lamports > 0 {
        transfer(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &accounts.stake_account,
            extra_lamports,
        )
        .await;
    }

    let instructions = instruction::withdraw(
        &id(),
        &accounts.pool,
        &accounts.alice_stake.pubkey(),
        &accounts.alice.pubkey(),
        &accounts.alice_token,
        &accounts.alice.pubkey(),
        get_token_balance(&mut context.banks_client, &accounts.alice_token).await,
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

    let wallet_lamports_after = get_account(&mut context.banks_client, &accounts.alice.pubkey())
        .await
        .lamports;

    let (_, alice_stake_after, _) =
        get_stake_account(&mut context.banks_client, &accounts.alice_stake.pubkey()).await;
    let alice_stake_after = alice_stake_after.unwrap().delegation.stake;

    let (_, pool_stake_after, pool_lamports_after) =
        get_stake_account(&mut context.banks_client, &accounts.stake_account).await;
    let pool_stake_after = pool_stake_after.unwrap().delegation.stake;

    // when active, the depositor gets their rent back, but when activating, its
    // just added to stake
    let expected_deposit = if activate {
        amount_deposited
    } else {
        amount_deposited + get_stake_account_rent(&mut context.banks_client).await
    };

    let prior_deposits = if prior_deposit {
        if activate {
            TEST_STAKE_AMOUNT * 10
        } else {
            TEST_STAKE_AMOUNT * 10 + get_stake_account_rent(&mut context.banks_client).await
        }
    } else {
        0
    };

    // alice received her stake back
    assert_eq!(alice_stake_after, expected_deposit);

    // alice nothing to withdraw
    // (we create the blank account before getting wallet_lamports_before)
    assert_eq!(wallet_lamports_after, wallet_lamports_before);

    // pool retains minstake
    assert_eq!(pool_stake_after, prior_deposits + minimum_delegation);

    // pool lamports otherwise unchanged. unexpected transfers affect nothing
    assert_eq!(
        pool_lamports_after,
        pool_lamports_before - expected_deposit + extra_lamports
    );

    // alice has no tokens
    assert_eq!(
        get_token_balance(&mut context.banks_client, &accounts.alice_token).await,
        0,
    );

    // tokens were burned
    assert_eq!(
        get_token_supply(&mut context.banks_client, &accounts.mint).await,
        prior_deposits,
    );
}

#[tokio::test]
async fn success_with_rewards() {
    let alice_deposit = TEST_STAKE_AMOUNT;
    let bob_deposit = TEST_STAKE_AMOUNT * 3;

    let mut context = program_test(false).start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    let minimum_delegation = accounts
        .initialize_for_withdraw(&mut context, alice_deposit, Some(bob_deposit), true)
        .await;

    context.increment_vote_account_credits(&accounts.vote_account.pubkey(), 1);
    advance_epoch(&mut context).await;

    let alice_tokens = get_token_balance(&mut context.banks_client, &accounts.alice_token).await;
    let bob_tokens = get_token_balance(&mut context.banks_client, &accounts.bob_token).await;

    // tokens correspond to deposit after rewards
    assert_eq!(alice_tokens, alice_deposit);
    assert_eq!(bob_tokens, bob_deposit);

    let (_, pool_stake, _) =
        get_stake_account(&mut context.banks_client, &accounts.stake_account).await;
    let pool_stake = pool_stake.unwrap().delegation.stake;
    let total_rewards = pool_stake - alice_deposit - bob_deposit - minimum_delegation;

    let instructions = instruction::withdraw(
        &id(),
        &accounts.pool,
        &accounts.alice_stake.pubkey(),
        &accounts.alice.pubkey(),
        &accounts.alice_token,
        &accounts.alice.pubkey(),
        alice_tokens,
    );
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&accounts.alice.pubkey()),
        &[&accounts.alice],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let alice_tokens = get_token_balance(&mut context.banks_client, &accounts.alice_token).await;
    let bob_tokens = get_token_balance(&mut context.banks_client, &accounts.bob_token).await;

    let (_, alice_stake, _) =
        get_stake_account(&mut context.banks_client, &accounts.alice_stake.pubkey()).await;
    let alice_rewards = alice_stake.unwrap().delegation.stake - alice_deposit;

    let (_, bob_stake, _) =
        get_stake_account(&mut context.banks_client, &accounts.stake_account).await;
    let bob_rewards = bob_stake.unwrap().delegation.stake - minimum_delegation - bob_deposit;

    // alice tokens are fully burned, bob remains unchanged
    assert_eq!(alice_tokens, 0);
    assert_eq!(bob_tokens, bob_deposit);

    // reward amounts are proportional to deposits
    assert_eq!(
        (alice_rewards as f64 / total_rewards as f64 * 100.0).round(),
        25.0
    );
    assert_eq!(
        (bob_rewards as f64 / total_rewards as f64 * 100.0).round(),
        75.0
    );
}

#[test_case(true; "activated")]
#[test_case(false; "activating")]
#[tokio::test]
async fn fail_automorphic(activate: bool) {
    let mut context = program_test(false).start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    accounts
        .initialize_for_withdraw(&mut context, TEST_STAKE_AMOUNT, None, activate)
        .await;

    let instructions = instruction::withdraw(
        &id(),
        &accounts.pool,
        &accounts.stake_account,
        &accounts.stake_authority,
        &accounts.alice_token,
        &accounts.alice.pubkey(),
        TEST_STAKE_AMOUNT,
    );
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&accounts.alice.pubkey()),
        &[&accounts.alice],
        context.last_blockhash,
    );

    let e = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
    check_error(e, SinglePoolError::InvalidPoolStakeAccountUsage);
}
