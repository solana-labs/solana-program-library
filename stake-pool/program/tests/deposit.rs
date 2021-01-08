#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;

use solana_sdk::signature::{Keypair, Signer};
use spl_stake_pool::*;

#[tokio::test]
async fn test_stake_pool_deposit() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await;

    let validator_stake_account: StakeAccount = simple_add_validator_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let user = Keypair::new();
    // make stake account
    let user_stake = Keypair::new();
    let lockup = stake::Lockup::default();
    let authorized = stake::Authorized {
        staker: stake_pool_accounts.deposit_authority,
        withdrawer: stake_pool_accounts.deposit_authority,
    };
    create_independent_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake,
        &authorized,
        &lockup,
    )
    .await;
    // make pool token account
    let user_pool_account = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_pool_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &user.pubkey(),
    )
    .await;
    stake_pool_accounts
        .deposit_stake(
            &user_stake.pubkey(),
            &user_pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &mut banks_client,
            &payer,
            &recent_blockhash,
        )
        .await;

    // Account should be drained
    assert!(banks_client
        .get_account(user_stake.pubkey())
        .await
        .expect("get_account")
        .is_none());
}
