#![cfg(feature = "test-bpf")]

mod helpers;

use {
    borsh::{BorshDeserialize, BorshSerialize},
    helpers::*,
    solana_program::{
        hash::Hash,
        instruction::{AccountMeta, Instruction},
    },
    solana_program_test::*,
    solana_sdk::{
        instruction::InstructionError, signature::Keypair, signature::Signer,
        transaction::Transaction, transaction::TransactionError, transport::TransportError,
    },
    spl_stake_pool::{error, id, instruction, state::{Fee, StakePool}},
};

async fn setup() -> (BanksClient, Keypair, Hash, StakePoolAccounts) {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    (
        banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
    )
}

#[tokio::test]
async fn success() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts) = setup().await;

    let new_fee = Fee { numerator: 10, denominator: 100 };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            new_fee.clone(),
        )],
        Some(&payer.pubkey()),
        &[&payer, &stake_pool_accounts.manager],
        recent_blockhash
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let stake_pool = get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool = StakePool::try_from_slice(&stake_pool.data.as_slice()).unwrap();

    assert_eq!(stake_pool.fee, new_fee);
}

#[tokio::test]
async fn fail_wrong_manager() {
}

#[tokio::test]
async fn fail_bad_fee() {
}

#[tokio::test]
async fn fail_not_updated() {
}
