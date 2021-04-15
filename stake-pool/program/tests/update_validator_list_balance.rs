#![cfg(feature = "test-bpf")]

mod helpers;

use {
    crate::helpers::TEST_STAKE_AMOUNT,
    helpers::*,
    solana_program::{native_token, pubkey::Pubkey},
    solana_program_test::*,
    solana_sdk::signature::Signer,
    spl_stake_pool::stake_program,
};

#[tokio::test]
async fn success() {
    let mut context = program_test().start_with_context().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
        )
        .await
        .unwrap();

    // Add several accounts
    let mut stake_accounts: Vec<ValidatorStakeAccount> = vec![];
    const STAKE_ACCOUNTS: u64 = 3;
    for _ in 0..STAKE_ACCOUNTS {
        stake_accounts.push(
            simple_add_validator_to_pool(
                &mut context.banks_client,
                &context.payer,
                &context.last_blockhash,
                &stake_pool_accounts,
            )
            .await,
        );
    }

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake_program::StakeState>())
        + native_token::sol_to_lamports(1.0);

    // Check current balance in the list
    assert_eq!(
        get_validator_list_sum(
            &mut context.banks_client,
            &stake_pool_accounts.validator_list.pubkey()
        )
        .await,
        STAKE_ACCOUNTS * (stake_rent + TEST_STAKE_AMOUNT)
    );

    // Add extra funds, simulating rewards
    const EXTRA_STAKE_AMOUNT: u64 = 1_000_000;

    for stake_account in &stake_accounts {
        transfer(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_account.stake_account,
            EXTRA_STAKE_AMOUNT,
        )
        .await;
    }

    // Update epoch
    context.warp_to_slot(50_000).unwrap();

    stake_pool_accounts
        .update_validator_list_balance(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            stake_accounts
                .iter()
                .map(|v| v.stake_account)
                .collect::<Vec<Pubkey>>()
                .as_slice(),
        )
        .await;

    // Check balance updated
    assert_eq!(
        get_validator_list_sum(
            &mut context.banks_client,
            &stake_pool_accounts.validator_list.pubkey()
        )
        .await,
        STAKE_ACCOUNTS * (stake_rent + TEST_STAKE_AMOUNT + EXTRA_STAKE_AMOUNT)
    );
}

#[tokio::test]
async fn test_update_validator_list_balance_with_uninitialized_validator_list() {} // TODO

#[tokio::test]
async fn test_update_validator_list_balance_with_wrong_stake_state() {} // TODO
