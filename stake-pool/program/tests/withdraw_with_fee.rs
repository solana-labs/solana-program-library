#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    bincode::deserialize,
    helpers::*,
    solana_program::{pubkey::Pubkey, stake},
    solana_program_test::*,
    solana_sdk::signature::{Keypair, Signer},
    spl_stake_pool::minimum_stake_lamports,
};

#[tokio::test]
async fn success_withdraw_all_fee_tokens() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_withdraw,
    ) = setup_for_withdraw(spl_token::id(), 0).await;

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();

    // move tokens to fee account
    transfer_spl_tokens(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts.token_program_id,
        &deposit_info.pool_account.pubkey(),
        &stake_pool_accounts.pool_mint.pubkey(),
        &stake_pool_accounts.pool_fee_account.pubkey(),
        &user_transfer_authority,
        tokens_to_withdraw / 2,
        stake_pool_accounts.pool_decimals,
    )
    .await;

    let fee_tokens = get_token_balance(
        &mut context.banks_client,
        &stake_pool_accounts.pool_fee_account.pubkey(),
    )
    .await;

    let user_transfer_authority = Keypair::new();
    delegate_tokens(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts.token_program_id,
        &stake_pool_accounts.pool_fee_account.pubkey(),
        &stake_pool_accounts.manager,
        &user_transfer_authority.pubkey(),
        fee_tokens,
    )
    .await;

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &validator_stake_account.stake_account,
            &new_authority,
            fee_tokens,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // Check balance is 0
    let fee_tokens = get_token_balance(
        &mut context.banks_client,
        &stake_pool_accounts.pool_fee_account.pubkey(),
    )
    .await;
    assert_eq!(fee_tokens, 0);
}

#[tokio::test]
async fn success_empty_out_stake_with_fee() {
    let (
        mut context,
        stake_pool_accounts,
        _,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_withdraw,
    ) = setup_for_withdraw(spl_token::id(), 0).await;

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();

    // add another validator and deposit into it
    let other_validator_stake_account = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts,
        None,
    )
    .await;

    let other_deposit_info = simple_deposit_stake(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts,
        &other_validator_stake_account,
        TEST_STAKE_AMOUNT,
    )
    .await
    .unwrap();

    // move tokens to new account
    transfer_spl_tokens(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts.token_program_id,
        &deposit_info.pool_account.pubkey(),
        &stake_pool_accounts.pool_mint.pubkey(),
        &other_deposit_info.pool_account.pubkey(),
        &user_transfer_authority,
        tokens_to_withdraw,
        stake_pool_accounts.pool_decimals,
    )
    .await;

    let user_tokens = get_token_balance(
        &mut context.banks_client,
        &other_deposit_info.pool_account.pubkey(),
    )
    .await;

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();

    let user_transfer_authority = Keypair::new();
    delegate_tokens(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts.token_program_id,
        &other_deposit_info.pool_account.pubkey(),
        &other_deposit_info.authority,
        &user_transfer_authority.pubkey(),
        user_tokens,
    )
    .await;

    // calculate exactly how much to withdraw, given the fee, to get the account
    // down to 0, using an inverse fee calculation
    let validator_stake_account = get_account(
        &mut context.banks_client,
        &other_validator_stake_account.stake_account,
    )
    .await;
    let stake_state =
        deserialize::<stake::state::StakeStateV2>(&validator_stake_account.data).unwrap();
    let meta = stake_state.meta().unwrap();
    let stake_minimum_delegation =
        stake_get_minimum_delegation(&mut context.banks_client, &context.payer, &last_blockhash)
            .await;
    let lamports_to_withdraw =
        validator_stake_account.lamports - minimum_stake_lamports(&meta, stake_minimum_delegation);
    let pool_tokens_to_withdraw =
        stake_pool_accounts.calculate_inverse_withdrawal_fee(lamports_to_withdraw);

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();
    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &other_deposit_info.pool_account.pubkey(),
            &other_validator_stake_account.stake_account,
            &new_authority,
            pool_tokens_to_withdraw,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // Check balance of validator stake account is MINIMUM + rent-exemption
    let validator_stake_account = get_account(
        &mut context.banks_client,
        &other_validator_stake_account.stake_account,
    )
    .await;
    let stake_state =
        deserialize::<stake::state::StakeStateV2>(&validator_stake_account.data).unwrap();
    let meta = stake_state.meta().unwrap();
    assert_eq!(
        validator_stake_account.lamports,
        minimum_stake_lamports(&meta, stake_minimum_delegation)
    );
}
