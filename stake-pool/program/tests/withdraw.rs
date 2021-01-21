#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program::pubkey::Pubkey;

use solana_sdk::signature::{Keypair, Signer};
use spl_stake_pool::*;

#[tokio::test]
async fn test_stake_pool_withdraw() {
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

    let deposit_info: DepositInfo = simple_deposit(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
        &validator_stake_account,
    )
    .await;

    let tokens_to_burn = deposit_info.pool_tokens / 4;
    let lamports_to_withdraw = tokens_to_burn; // For now math is 1:1

    // Delegate tokens for burning
    delegate_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &deposit_info.user_pool_account,
        &deposit_info.user,
        &stake_pool_accounts.withdraw_authority,
        tokens_to_burn,
    )
    .await;

    // Create stake account to withdraw to
    let user_stake_recipient = Keypair::new();
    let initial_stake_lamports = create_blank_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake_recipient,
    )
    .await;

    // Save stake pool state before withdrawal
    let stake_pool_before =
        get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool_before = state::State::deserialize(&stake_pool_before.data.as_slice())
        .unwrap()
        .stake_pool()
        .unwrap();

    // Save validator stake account record before withdrawal
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

    // Save user token balance
    let user_token_balance_before =
        get_token_balance(&mut banks_client, &deposit_info.user_pool_account).await;

    let new_authority = Pubkey::new_unique();
    stake_pool_accounts
        .withdraw_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake_recipient.pubkey(),
            &deposit_info.user_pool_account,
            &validator_stake_account.stake_account,
            &new_authority,
            lamports_to_withdraw,
        )
        .await;

    // Check pool stats
    let stake_pool = get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool = state::State::deserialize(&stake_pool.data.as_slice())
        .unwrap()
        .stake_pool()
        .unwrap();
    assert_eq!(
        stake_pool.stake_total,
        stake_pool_before.stake_total - lamports_to_withdraw
    );
    assert_eq!(
        stake_pool.pool_total,
        stake_pool_before.pool_total - tokens_to_burn
    );

    // Check validator stake list storage
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
        validator_stake_item_before.balance - lamports_to_withdraw
    );

    // Check tokens burned
    let user_token_balance =
        get_token_balance(&mut banks_client, &deposit_info.user_pool_account).await;
    assert_eq!(
        user_token_balance,
        user_token_balance_before - tokens_to_burn
    );

    // Check validator stake account balance
    let validator_stake_account =
        get_account(&mut banks_client, &validator_stake_account.stake_account).await;
    assert_eq!(
        validator_stake_account.lamports,
        validator_stake_item.balance
    );

    // Check user recipient stake account balance
    let user_stake_recipient_account =
        get_account(&mut banks_client, &user_stake_recipient.pubkey()).await;
    assert_eq!(
        user_stake_recipient_account.lamports,
        initial_stake_lamports + lamports_to_withdraw
    );
}
