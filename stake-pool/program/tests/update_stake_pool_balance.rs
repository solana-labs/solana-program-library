#![cfg(feature = "test-bpf")]

mod helpers;

use {
    helpers::*,
    solana_program_test::*,
    solana_sdk::{
        instruction::InstructionError, signature::Keypair, transaction::TransactionError,
    },
    spl_stake_pool::*,
};

#[tokio::test]
async fn success() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    let error = stake_pool_accounts
        .update_stake_pool_balance(&mut banks_client, &payer, &recent_blockhash)
        .await;
    assert!(error.is_none());
    // TODO: Waiting for the ability to advance clock (or modify account data) to finish the tests
}

#[tokio::test]
async fn fail_with_wrong_validator_list() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let mut stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    let wrong_validator_list = Keypair::new();
    stake_pool_accounts.validator_list = wrong_validator_list;
    let error = stake_pool_accounts
        .update_stake_pool_balance(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        ) => {
            let program_error = error::StakePoolError::InvalidValidatorStakeList as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to update pool balance with wrong validator stake list account"),
    }
}

#[tokio::test]
async fn fail_with_wrong_pool_fee_account() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let mut stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    let wrong_fee_account = Keypair::new();
    stake_pool_accounts.pool_fee_account = wrong_fee_account;
    let error = stake_pool_accounts
        .update_stake_pool_balance(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        ) => {
            let program_error = error::StakePoolError::InvalidFeeAccount as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to update pool balance with wrong validator stake list account"),
    }
}

#[tokio::test]
async fn test_update_stake_pool_balance_with_uninitialized_validator_list() {} // TODO

#[tokio::test]
async fn test_update_stake_pool_balance_with_out_of_dated_validators_balances() {} // TODO
