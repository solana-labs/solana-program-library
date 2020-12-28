#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use spl_stake_pool::*;
use bincode::deserialize;

#[tokio::test]
async fn test_stake_pool_leave() {
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

    let tokens_to_burn = get_token_balance(&mut banks_client, &user_pool_account.pubkey()).await;
    delegate_tokens(&mut banks_client,
        &payer,
        &recent_blockhash,
        &user_pool_account.pubkey(), 
        &user, 
        &stake_pool_accounts.withdraw_authority, 
        tokens_to_burn).await;

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .leave_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.stake_account.pubkey(),
            &user_pool_account.pubkey(),
            &new_authority,
        )
        .await;
    assert!(error.is_none());

    // Check if all tokens were burned
    let tokens_left = get_token_balance(&mut banks_client, &user_pool_account.pubkey()).await;
    assert_eq!(tokens_left, 0);

    // Check if account was removed from the list of stake accounts
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
            validators: vec![]
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
                &new_authority
            );
            assert_eq!(
                &meta.authorized.withdrawer,
                &new_authority
            );
        }
        _ => panic!(),
    }
}