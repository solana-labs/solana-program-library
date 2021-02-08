#![cfg(feature = "test-bpf")]

mod helpers;

use crate::helpers::TEST_STAKE_AMOUNT;
use helpers::*;
use solana_program::pubkey::Pubkey;
use solana_program_test::BanksClient;
use solana_sdk::signature::Signer;
use spl_stake_pool::*;

async fn get_list_sum(banks_client: &mut BanksClient, validator_stake_list_key: &Pubkey) -> u64 {
    let validator_stake_list = banks_client
        .get_account(*validator_stake_list_key)
        .await
        .expect("get_account")
        .expect("validator stake list not none");
    let validator_stake_list =
        state::ValidatorStakeList::deserialize(validator_stake_list.data.as_slice()).unwrap();

    validator_stake_list
        .validators
        .iter()
        .map(|info| info.balance)
        .sum()
}

#[tokio::test]
async fn test_update_list_balance() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    // Add several accounts
    let mut stake_accounts: Vec<ValidatorStakeAccount> = vec![];
    const STAKE_ACCOUNTS: u64 = 3;
    for _ in 0..STAKE_ACCOUNTS {
        stake_accounts.push(
            simple_add_validator_stake_account(
                &mut banks_client,
                &payer,
                &recent_blockhash,
                &stake_pool_accounts,
            )
            .await,
        );
    }

    // Add stake extra funds
    const EXTRA_STAKE: u64 = 1_000_000;

    for stake_account in stake_accounts {
        transfer(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &stake_account.stake_account,
            EXTRA_STAKE,
        )
        .await;
    }

    let rent = banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::StakeState>()) + 1;

    // Check current balance in the list
    assert_eq!(
        get_list_sum(
            &mut banks_client,
            &stake_pool_accounts.validator_stake_list.pubkey()
        )
        .await,
        STAKE_ACCOUNTS * (stake_rent + TEST_STAKE_AMOUNT)
    );

    // TODO: Execute update list with updated clock
}

#[tokio::test]
async fn test_update_list_balance_with_uninitialized_validator_stake_list() {} // TODO

#[tokio::test]
async fn test_update_list_balance_with_wrong_stake_state() {} // TODO
