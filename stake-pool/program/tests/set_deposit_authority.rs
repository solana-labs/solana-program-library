#![cfg(feature = "test-bpf")]

mod helpers;

use {
    borsh::BorshSerialize,
    helpers::*,
    solana_program::{
        borsh::try_from_slice_unchecked,
        hash::Hash,
        instruction::{AccountMeta, Instruction},
    },
    solana_program_test::*,
    solana_sdk::{
        instruction::InstructionError, signature::Keypair, signature::Signer,
        transaction::Transaction, transaction::TransactionError, transport::TransportError,
    },
    spl_stake_pool::{
        error, find_deposit_authority_program_address, id,
        instruction::{self, DepositType},
        state,
    },
};

async fn setup() -> (BanksClient, Keypair, Hash, StakePoolAccounts, Keypair) {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let new_deposit_authority = Keypair::new();

    (
        banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        new_deposit_authority,
    )
}

#[tokio::test]
async fn success_set_stake_deposit_authority() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        new_stake_deposit_authority,
    ) = setup().await;

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_deposit_authority(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            Some(&new_stake_deposit_authority.pubkey()),
            DepositType::Stake,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &stake_pool_accounts.manager], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let stake_pool = get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool =
        try_from_slice_unchecked::<state::StakePool>(&stake_pool.data.as_slice()).unwrap();

    assert_eq!(
        stake_pool.stake_deposit_authority,
        new_stake_deposit_authority.pubkey()
    );
}

#[tokio::test]
async fn success_set_stake_deposit_authority_to_none() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        new_stake_deposit_authority,
    ) = setup().await;

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_deposit_authority(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            Some(&new_stake_deposit_authority.pubkey()),
            DepositType::Stake,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &stake_pool_accounts.manager], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let stake_pool = get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool =
        try_from_slice_unchecked::<state::StakePool>(&stake_pool.data.as_slice()).unwrap();

    assert_eq!(
        stake_pool.stake_deposit_authority,
        new_stake_deposit_authority.pubkey()
    );

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_deposit_authority(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            None,
            DepositType::Stake,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &stake_pool_accounts.manager], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let stake_pool = get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool =
        try_from_slice_unchecked::<state::StakePool>(&stake_pool.data.as_slice()).unwrap();

    assert_eq!(
        stake_pool.stake_deposit_authority,
        find_deposit_authority_program_address(&id(), &stake_pool_accounts.stake_pool.pubkey()).0
    );
}

#[tokio::test]
async fn fail_stake_wrong_manager() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        new_stake_deposit_authority,
    ) = setup().await;

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_deposit_authority(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &new_stake_deposit_authority.pubkey(),
            Some(&new_stake_deposit_authority.pubkey()),
            DepositType::Stake,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &new_stake_deposit_authority], recent_blockhash);
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
            let program_error = error::StakePoolError::WrongManager as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while malicious try to set manager"),
    }
}

#[tokio::test]
async fn fail_set_stake_deposit_authority_without_signature() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        new_stake_deposit_authority,
    ) = setup().await;

    let data =
        instruction::StakePoolInstruction::SetDepositAuthority(instruction::DepositType::Stake)
            .try_to_vec()
            .unwrap();
    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.manager.pubkey(), false),
        AccountMeta::new_readonly(new_stake_deposit_authority.pubkey(), false),
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
        _ => panic!("Wrong error occurs while try to set new manager without signature"),
    }
}

#[tokio::test]
async fn success_set_sol_deposit_authority() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, new_sol_deposit_authority) =
        setup().await;

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_deposit_authority(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            Some(&new_sol_deposit_authority.pubkey()),
            DepositType::Sol,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &stake_pool_accounts.manager], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let stake_pool = get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool =
        try_from_slice_unchecked::<state::StakePool>(&stake_pool.data.as_slice()).unwrap();

    assert_eq!(
        stake_pool.sol_deposit_authority,
        Some(new_sol_deposit_authority.pubkey())
    );
}

#[tokio::test]
async fn success_set_sol_deposit_authority_to_none() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, new_sol_deposit_authority) =
        setup().await;

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_deposit_authority(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            Some(&new_sol_deposit_authority.pubkey()),
            DepositType::Sol,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &stake_pool_accounts.manager], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let stake_pool = get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool =
        try_from_slice_unchecked::<state::StakePool>(&stake_pool.data.as_slice()).unwrap();

    assert_eq!(
        stake_pool.sol_deposit_authority,
        Some(new_sol_deposit_authority.pubkey())
    );

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_deposit_authority(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            None,
            DepositType::Sol,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &stake_pool_accounts.manager], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let stake_pool = get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool =
        try_from_slice_unchecked::<state::StakePool>(&stake_pool.data.as_slice()).unwrap();

    assert_eq!(stake_pool.sol_deposit_authority, None);
}

#[tokio::test]
async fn fail_sol_wrong_manager() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, new_sol_deposit_authority) =
        setup().await;

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_deposit_authority(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &new_sol_deposit_authority.pubkey(),
            Some(&new_sol_deposit_authority.pubkey()),
            DepositType::Sol,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &new_sol_deposit_authority], recent_blockhash);
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
            let program_error = error::StakePoolError::WrongManager as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while malicious try to set manager"),
    }
}

#[tokio::test]
async fn fail_set_sol_deposit_authority_without_signature() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, new_sol_deposit_authority) =
        setup().await;

    let data =
        instruction::StakePoolInstruction::SetDepositAuthority(instruction::DepositType::Sol)
            .try_to_vec()
            .unwrap();
    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.manager.pubkey(), false),
        AccountMeta::new_readonly(new_sol_deposit_authority.pubkey(), false),
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
        _ => panic!("Wrong error occurs while try to set new manager without signature"),
    }
}
