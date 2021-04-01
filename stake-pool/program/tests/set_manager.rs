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
    spl_stake_pool::{error, id, instruction, state},
};

async fn setup() -> (BanksClient, Keypair, Hash, StakePoolAccounts, Keypair) {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    let new_manager = Keypair::new();

    (
        banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        new_manager,
    )
}

#[tokio::test]
async fn success_set_manager() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, new_manager) =
        setup().await;

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_manager(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.owner.pubkey(),
            &new_manager.pubkey(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &stake_pool_accounts.owner], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let stake_pool = get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool = state::StakePool::try_from_slice(&stake_pool.data.as_slice()).unwrap();

    assert_eq!(stake_pool.manager, new_manager.pubkey());
}

#[tokio::test]
async fn fail_wrong_owner() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, new_manager) =
        setup().await;

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_manager(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &new_manager.pubkey(),
            &new_manager.pubkey(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &new_manager], recent_blockhash);
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
            let program_error = error::StakePoolError::WrongOwner as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while malicious try to set owner"),
    }
}

#[tokio::test]
async fn fail_set_manager_without_signature() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, new_manager) =
        setup().await;

    let data = instruction::StakePoolInstruction::SetManager
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.owner.pubkey(), false),
        AccountMeta::new_readonly(new_manager.pubkey(), false),
    ];
    let instruction = Instruction {
        program_id: id(),
        accounts,
        data,
    };

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
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
            let program_error = error::StakePoolError::SignatureMissing as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to set new owner without signature"),
    }
}
