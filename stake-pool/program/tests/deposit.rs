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
    let stake_lamports = create_independent_stake_account(
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

    // Save stake pool state before depositing
    let stake_pool_before =
        get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool_before = state::State::deserialize(&stake_pool_before.data.as_slice())
        .unwrap()
        .stake_pool()
        .unwrap();

    // Save validator stake account record before depositing
    let validator_stake_list = get_account(
        &mut banks_client,
        &stake_pool_accounts.validator_stake_list.pubkey(),
    )
    .await;
    let validator_stake_list =
        state::ValidatorStakeList::deserialize(validator_stake_list.data.as_slice()).unwrap();
    let validator_stake_item_before = validator_stake_list
        .find(&validator_stake_account.vote.pubkey())
        .unwrap();

    stake_pool_accounts
        .deposit_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.pubkey(),
            &user_pool_account.pubkey(),
            &validator_stake_account.stake_account,
        )
        .await;

    // Original stake account should be drained
    assert!(banks_client
        .get_account(user_stake.pubkey())
        .await
        .expect("get_account")
        .is_none());

    let tokens_issued = stake_lamports; // For now tokens are 1:1 to stake
    let fee = stake_pool_accounts.calculate_fee(tokens_issued);

    // Stake pool should add its balance to the pool balance
    let stake_pool = get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool = state::State::deserialize(&stake_pool.data.as_slice())
        .unwrap()
        .stake_pool()
        .unwrap();
    assert_eq!(
        stake_pool.stake_total,
        stake_pool_before.stake_total + stake_lamports
    );
    assert_eq!(
        stake_pool.pool_total,
        stake_pool_before.pool_total + tokens_issued
    );

    // Check minted tokens
    let user_token_balance =
        get_token_balance(&mut banks_client, &user_pool_account.pubkey()).await;
    assert_eq!(user_token_balance, tokens_issued - fee);
    let pool_fee_token_balance = get_token_balance(
        &mut banks_client,
        &stake_pool_accounts.pool_fee_account.pubkey(),
    )
    .await;
    assert_eq!(pool_fee_token_balance, fee);

    // Check balances in validator stake account list storage
    let validator_stake_list = get_account(
        &mut banks_client,
        &stake_pool_accounts.validator_stake_list.pubkey(),
    )
    .await;
    let validator_stake_list =
        state::ValidatorStakeList::deserialize(validator_stake_list.data.as_slice()).unwrap();
    let validator_stake_item = validator_stake_list
        .find(&validator_stake_account.vote.pubkey())
        .unwrap();
    assert_eq!(
        validator_stake_item.balance,
        validator_stake_item_before.balance + stake_lamports
    );

    // Check validator stake account actual SOL balance
    let validator_stake_account =
        get_account(&mut banks_client, &validator_stake_account.stake_account).await;
    assert_eq!(
        validator_stake_account.lamports,
        validator_stake_item.balance
    );
}
