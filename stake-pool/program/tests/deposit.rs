#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;

use bincode::deserialize;
use solana_sdk::signature::{Keypair, Signer};
use spl_stake_pool::*;

#[tokio::test]
async fn test_stake_pool_deposit() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
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
            &mut banks_client,
            &payer,
            &recent_blockhash,
        )
        .await;

    let stake = banks_client
        .get_account(user_stake.pubkey())
        .await
        .expect("get_account")
        .expect("stake not none");
    assert_eq!(stake.data.len(), std::mem::size_of::<stake::StakeState>());
    assert_eq!(stake.owner, stake::id());

    let stake_state = deserialize::<stake::StakeState>(&stake.data).unwrap();
    match stake_state {
        stake::StakeState::Initialized(meta) => {
            assert_eq!(
                &meta.authorized.staker,
                &stake_pool_accounts.withdraw_authority
            );
            assert_eq!(
                &meta.authorized.withdrawer,
                &stake_pool_accounts.withdraw_authority
            );
        }
        _ => panic!(),
    }
}
