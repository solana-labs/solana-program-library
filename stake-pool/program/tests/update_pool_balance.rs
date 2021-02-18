#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_sdk::{
    instruction::InstructionError, signature::Keypair, signature::Signer, transaction::Transaction,
    transaction::TransactionError, transport::TransportError,
};
use spl_stake_pool::*;

#[tokio::test]
async fn test_update_pool_balance() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    // TODO: Waiting for the ability to advance clock (or modify account data) to finish the tests
}

#[tokio::test]
async fn test_update_pool_balance_with_wrong_validator_stake_list() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    let wrong_stake_list_storage = Keypair::new();
    let mut transaction = Transaction::new_with_payer(
        &[instruction::update_pool_balance(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &wrong_stake_list_storage.pubkey(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer], recent_blockhash);
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::InvalidValidatorStakeList as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to update pool balance with wrong validator stake list account"),
    }
}

#[tokio::test]
async fn test_update_pool_balance_with_uninitialized_validator_stake_list() {} // TODO

#[tokio::test]
async fn test_update_pool_balance_with_out_of_dated_validators_balances() {} // TODO
