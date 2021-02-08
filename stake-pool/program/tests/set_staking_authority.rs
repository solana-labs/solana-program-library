#![cfg(feature = "test-bpf")]

mod helpers;

use bincode::deserialize;
use helpers::*;
use solana_program::hash::Hash;
use solana_program::instruction::AccountMeta;
use solana_program::instruction::Instruction;
use solana_program::sysvar;
use solana_program_test::BanksClient;
use solana_sdk::{
    instruction::InstructionError, signature::Keypair, signature::Signer, transaction::Transaction,
    transaction::TransactionError, transport::TransportError,
};
use spl_stake_pool::*;

async fn setup() -> (
    BanksClient,
    Keypair,
    Hash,
    StakePoolAccounts,
    ValidatorStakeAccount,
) {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    let user = Keypair::new();

    let user_stake = ValidatorStakeAccount::new_with_target_authority(
        &stake_pool_accounts.deposit_authority,
        &stake_pool_accounts.stake_pool.pubkey(),
    );
    user_stake
        .create_and_delegate(&mut banks_client, &payer, &recent_blockhash)
        .await;

    // make pool token account
    let user_pool_account = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_pool_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &user.pubkey(),
    )
    .await
    .unwrap();

    let error = stake_pool_accounts
        .add_validator_stake_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.stake_account,
            &user_pool_account.pubkey(),
        )
        .await;
    assert!(error.is_none());

    (
        banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        user_stake,
    )
}

#[tokio::test]
async fn test_set_staking_authority() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, user_stake) =
        setup().await;

    let new_staking_pubkey = Keypair::new().pubkey();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_staking_authority(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.owner.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &user_stake.stake_account,
            &new_staking_pubkey,
            &stake::id(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &stake_pool_accounts.owner], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Check of stake account authority has changed
    let stake = get_account(&mut banks_client, &user_stake.stake_account).await;
    let stake_state = deserialize::<stake::StakeState>(&stake.data).unwrap();
    match stake_state {
        stake::StakeState::Stake(meta, _) => {
            assert_eq!(&meta.authorized.staker, &new_staking_pubkey);
            assert_eq!(
                &meta.authorized.withdrawer,
                &stake_pool_accounts.withdraw_authority
            );
        }
        _ => panic!(),
    }
}

#[tokio::test]
async fn test_set_staking_authority_with_wrong_stake_program_id() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, user_stake) =
        setup().await;

    let new_staking_pubkey = Keypair::new().pubkey();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_staking_authority(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.owner.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &user_stake.stake_account,
            &new_staking_pubkey,
            &Keypair::new().pubkey(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &stake_pool_accounts.owner], recent_blockhash);
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(_, error)) => {
            assert_eq!(error, InstructionError::IncorrectProgramId);
        }
        _ => panic!(
            "Wrong error occurs while try to set staking authority with wrong stake program ID"
        ),
    }
}

#[tokio::test]
async fn test_set_staking_authority_with_wrong_withdraw_authority() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, user_stake) =
        setup().await;

    let new_staking_pubkey = Keypair::new().pubkey();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_staking_authority(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.owner.pubkey(),
            &Keypair::new().pubkey(),
            &user_stake.stake_account,
            &new_staking_pubkey,
            &stake::id(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &stake_pool_accounts.owner], recent_blockhash);
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
            let program_error = error::StakePoolError::InvalidProgramAddress as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!(
            "Wrong error occurs while try to set staking authority with wrong withdraw authority"
        ),
    }
}

#[tokio::test]
async fn test_set_staking_authority_with_wrong_owner() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, user_stake) =
        setup().await;

    let new_staking_pubkey = Keypair::new().pubkey();
    let wrong_owner = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_staking_authority(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &wrong_owner.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &user_stake.stake_account,
            &new_staking_pubkey,
            &stake::id(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &wrong_owner], recent_blockhash);
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
        _ => panic!("Wrong error occurs while try to set staking authority with wrong owner"),
    }
}

#[tokio::test]
async fn test_set_staking_authority_without_signature() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, user_stake) =
        setup().await;

    let new_staking_pubkey = Keypair::new().pubkey();

    let args = instruction::StakePoolInstruction::SetStakingAuthority;
    let data = args.serialize().unwrap();
    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.owner.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.withdraw_authority, false),
        AccountMeta::new(user_stake.stake_account, false),
        AccountMeta::new_readonly(new_staking_pubkey, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(stake::id(), false),
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
        _ => panic!("Wrong error occurs while try to set staking authority without signature"),
    }
}
