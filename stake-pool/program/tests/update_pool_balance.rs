#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;

#[tokio::test]
async fn test_update_pool_balance() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await;

    // TODO: Waiting for the ability to advance clock (or modify account data) to finish the tests
}
