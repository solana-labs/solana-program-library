#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;

use solana_sdk::signature::{Keypair, Signer};
use spl_stake_pool::*;
use bincode::deserialize;

#[tokio::test]
async fn test_stake_pool_join() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await;

    let user = Keypair::new();

    let user_stake =
        StakeAccount::new_with_target_authority(&stake_pool_accounts.deposit_authority);
    user_stake.create_and_delegate(&mut banks_client, &payer, &recent_blockhash).await;

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
    let error = stake_pool_accounts
        .join_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.stake_account.pubkey(),
            &user_pool_account.pubkey(),
        )
        .await;
    assert!(error.is_none());

    let stake_account_balance = banks_client
        .get_account(user_stake.stake_account.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let deposit_tokens = stake_account_balance; // For now 1:1 math
    // Check token account balance
    let token_balance = get_token_balance(&mut banks_client, &user_pool_account.pubkey()).await;
    assert_eq!(token_balance, deposit_tokens);

    // Check if validator account was added to the list
    let validator_stake_list = banks_client
        .get_account(stake_pool_accounts.validator_stake_list.pubkey())
        .await
        .expect("get_account")
        .expect("validator stake list not none");
    let validator_stake_list =
        state::ValidatorStakeList::deserialize(validator_stake_list.data.as_slice()).unwrap();
    assert_eq!(
        validator_stake_list,
        state::ValidatorStakeList {
            is_initialized: true,
            validators: vec![state::ValidatorStakeInfo {
                validator_account: user_stake.vote.pubkey(),
                last_update_epoch: 0,
                balance: stake_account_balance,
            }]
        }
    );

    // Check of stake account authority has changed
    let stake = banks_client
        .get_account(user_stake.stake_account.pubkey())
        .await
        .expect("get_account")
        .expect("stake not none");
    let stake_state = deserialize::<stake::StakeState>(&stake.data).unwrap();
    match stake_state {
        stake::StakeState::Stake(meta, _) => {
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
