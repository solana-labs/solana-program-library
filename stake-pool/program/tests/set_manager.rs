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
    spl_stake_pool::{error, id, instruction, state},
};

async fn setup() -> (
    BanksClient,
    Keypair,
    Hash,
    StakePoolAccounts,
    Keypair,
    Keypair,
) {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let new_pool_fee = Keypair::new();
    let new_manager = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &new_pool_fee,
        &stake_pool_accounts.pool_mint.pubkey(),
        &new_manager.pubkey(),
    )
    .await
    .unwrap();

    (
        banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        new_pool_fee,
        new_manager,
    )
}

#[tokio::test]
async fn test_set_manager() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, new_pool_fee, new_manager) =
        setup().await;

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_manager(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            &new_manager.pubkey(),
            &new_pool_fee.pubkey(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(
        &[&payer, &stake_pool_accounts.manager, &new_manager],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let stake_pool = get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool =
        try_from_slice_unchecked::<state::StakePool>(stake_pool.data.as_slice()).unwrap();

    assert_eq!(stake_pool.manager, new_manager.pubkey());
}

#[tokio::test]
async fn test_set_manager_by_malicious() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, new_pool_fee, new_manager) =
        setup().await;

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_manager(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &new_manager.pubkey(),
            &new_manager.pubkey(),
            &new_pool_fee.pubkey(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &new_manager], recent_blockhash);
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .into();

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
async fn test_set_manager_without_existing_signature() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, new_pool_fee, new_manager) =
        setup().await;

    let data = instruction::StakePoolInstruction::SetManager
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.manager.pubkey(), false),
        AccountMeta::new_readonly(new_manager.pubkey(), true),
        AccountMeta::new_readonly(new_pool_fee.pubkey(), false),
    ];
    let instruction = Instruction {
        program_id: id(),
        accounts,
        data,
    };

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &new_manager], recent_blockhash);
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .into();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::SignatureMissing as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!(
            "Wrong error occurs while try to set new manager without existing manager signature"
        ),
    }
}

#[tokio::test]
async fn test_set_manager_without_new_signature() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, new_pool_fee, new_manager) =
        setup().await;

    let data = instruction::StakePoolInstruction::SetManager
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.manager.pubkey(), true),
        AccountMeta::new_readonly(new_manager.pubkey(), false),
        AccountMeta::new_readonly(new_pool_fee.pubkey(), false),
    ];
    let instruction = Instruction {
        program_id: id(),
        accounts,
        data,
    };

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &stake_pool_accounts.manager], recent_blockhash);
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .into();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::SignatureMissing as u32;
            assert_eq!(error_index, program_error);
        }
        _ => {
            panic!("Wrong error occurs while try to set new manager without new manager signature")
        }
    }
}

#[tokio::test]
async fn test_set_manager_with_wrong_mint_for_pool_fee_acc() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let new_mint = Keypair::new();
    let new_withdraw_auth = Keypair::new();
    let new_pool_fee = Keypair::new();
    let new_manager = Keypair::new();

    create_mint(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &new_mint,
        &new_withdraw_auth.pubkey(),
    )
    .await
    .unwrap();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &new_pool_fee,
        &new_mint.pubkey(),
        &new_manager.pubkey(),
    )
    .await
    .unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_manager(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            &new_manager.pubkey(),
            &new_pool_fee.pubkey(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(
        &[&payer, &stake_pool_accounts.manager, &new_manager],
        recent_blockhash,
    );
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .into();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::WrongAccountMint as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to set new manager with wrong mint"),
    }
}
