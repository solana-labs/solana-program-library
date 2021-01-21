#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;

use bincode::deserialize;
use solana_sdk::signature::{Keypair, Signer};
use spl_stake_pool::*;

#[tokio::test]
async fn test_add_validator_stake_account() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await;

    let user = Keypair::new();

    let user_stake = StakeAccount::new_with_target_authority(
        &stake_pool_accounts.deposit_authority,
        &stake_pool_accounts.stake_pool.pubkey(),
    );
    user_stake
        .create_and_delegate(&mut banks_client, &payer, &recent_blockhash)
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
    let error = stake_pool_accounts
        .add_validator_stake_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.stake_account,
            &user_pool_account.pubkey(),
        )
        .await;
    assert!(error.is_none());

    let stake_account_balance = banks_client
        .get_account(user_stake.stake_account)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let deposit_tokens = stake_account_balance; // For now 1:1 math
                                                // Check token account balance
    let token_balance = get_token_balance(&mut banks_client, &user_pool_account.pubkey()).await;
    assert_eq!(token_balance, deposit_tokens);
    let pool_fee_token_balance = get_token_balance(
        &mut banks_client,
        &stake_pool_accounts.pool_fee_account.pubkey(),
    )
    .await;
    assert_eq!(pool_fee_token_balance, 0); // No fee when adding validator stake accounts

    // Check if validator account was added to the list
    let validator_stake_list = get_account(
        &mut banks_client,
        &stake_pool_accounts.validator_stake_list.pubkey(),
    )
    .await;
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
    let stake = get_account(&mut banks_client, &user_stake.stake_account).await;
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
