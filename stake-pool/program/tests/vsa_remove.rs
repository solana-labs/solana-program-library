#![cfg(feature = "test-bpf")]

mod helpers;

use {
    bincode::deserialize,
    borsh::BorshSerialize,
    helpers::*,
    solana_program::{
        hash::Hash,
        instruction::{AccountMeta, Instruction, InstructionError},
        pubkey::Pubkey,
        sysvar,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
    spl_stake_pool::{
        borsh::try_from_slice_unchecked, error::StakePoolError, id, instruction, stake_program,
        state,
    },
};

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
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 10_000_000_000)
        .await
        .unwrap();

    let user_stake = ValidatorStakeAccount::new_with_target_authority(
        &stake_pool_accounts.deposit_authority,
        &stake_pool_accounts.stake_pool.pubkey(),
    );
    user_stake
        .create_and_delegate(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &stake_pool_accounts.staker,
        )
        .await;

    let error = stake_pool_accounts
        .add_validator_to_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.stake_account,
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
async fn success() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, user_stake) =
        setup().await;

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &new_authority,
            &user_stake.stake_account,
            &user_stake.transient_stake_account,
        )
        .await;
    assert!(error.is_none());

    // Check if account was removed from the list of stake accounts
    let validator_list = get_account(
        &mut banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    assert_eq!(
        validator_list,
        state::ValidatorList {
            account_type: state::AccountType::ValidatorList,
            max_validators: stake_pool_accounts.max_validators,
            validators: vec![]
        }
    );

    // Check of stake account authority has changed
    let stake = get_account(&mut banks_client, &user_stake.stake_account).await;
    let stake_state = deserialize::<stake_program::StakeState>(&stake.data).unwrap();
    match stake_state {
        stake_program::StakeState::Stake(meta, _) => {
            assert_eq!(&meta.authorized.staker, &new_authority);
            assert_eq!(&meta.authorized.withdrawer, &new_authority);
        }
        _ => panic!(),
    }
}

#[tokio::test]
async fn fail_with_wrong_stake_program_id() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, user_stake) =
        setup().await;

    let wrong_stake_program = Pubkey::new_unique();

    let new_authority = Pubkey::new_unique();
    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.staker.pubkey(), true),
        AccountMeta::new_readonly(stake_pool_accounts.withdraw_authority, false),
        AccountMeta::new_readonly(new_authority, false),
        AccountMeta::new(stake_pool_accounts.validator_list.pubkey(), false),
        AccountMeta::new(user_stake.stake_account, false),
        AccountMeta::new_readonly(user_stake.transient_stake_account, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(wrong_stake_program, false),
    ];
    let instruction = Instruction {
        program_id: id(),
        accounts,
        data: instruction::StakePoolInstruction::RemoveValidatorFromPool
            .try_to_vec()
            .unwrap(),
    };

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &stake_pool_accounts.staker], recent_blockhash);
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            error,
        )) => {
            assert_eq!(error, InstructionError::IncorrectProgramId);
        }
        _ => panic!("Wrong error occurs while try to remove validator stake address with wrong stake program ID"),
    }
}

#[tokio::test]
async fn fail_with_wrong_validator_list_account() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, user_stake) =
        setup().await;

    let wrong_validator_list = Keypair::new();

    let new_authority = Pubkey::new_unique();
    let mut transaction = Transaction::new_with_payer(
        &[instruction::remove_validator_from_pool(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &new_authority,
            &wrong_validator_list.pubkey(),
            &user_stake.stake_account,
            &user_stake.transient_stake_account,
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &stake_pool_accounts.staker], recent_blockhash);
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
            let program_error = StakePoolError::InvalidValidatorStakeList as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to remove validator stake address with wrong validator stake list account"),
    }
}

#[tokio::test]
async fn fail_not_at_minimum() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, user_stake) =
        setup().await;

    transfer(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake.stake_account,
        1_000_001,
    )
    .await;

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &new_authority,
            &user_stake.stake_account,
            &user_stake.transient_stake_account,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::StakeLamportsNotEqualToMinimum as u32)
        ),
    );
}

#[tokio::test]
async fn fail_double_remove() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, user_stake) =
        setup().await;

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &new_authority,
            &user_stake.stake_account,
            &user_stake.transient_stake_account,
        )
        .await;
    assert!(error.is_none());

    let latest_blockhash = banks_client.get_recent_blockhash().await.unwrap();

    let transaction_error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut banks_client,
            &payer,
            &latest_blockhash,
            &new_authority,
            &user_stake.stake_account,
            &user_stake.transient_stake_account,
        )
        .await
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = StakePoolError::ValidatorNotFound as u32;
            assert_eq!(error_index, program_error);
        }
        _ => {
            panic!("Wrong error occurs while try to remove already removed validator stake address")
        }
    }
}

#[tokio::test]
async fn fail_wrong_staker() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, user_stake) =
        setup().await;

    let malicious = Keypair::new();

    let new_authority = Pubkey::new_unique();
    let mut transaction = Transaction::new_with_payer(
        &[instruction::remove_validator_from_pool(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &malicious.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &new_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &user_stake.stake_account,
            &user_stake.transient_stake_account,
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &malicious], recent_blockhash);
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
            let program_error = StakePoolError::WrongStaker as u32;
            assert_eq!(error_index, program_error);
        }
        _ => {
            panic!("Wrong error occurs while not an staker try to remove validator stake address")
        }
    }
}

#[tokio::test]
async fn fail_no_signature() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, user_stake) =
        setup().await;

    let new_authority = Pubkey::new_unique();

    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.staker.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.withdraw_authority, false),
        AccountMeta::new_readonly(new_authority, false),
        AccountMeta::new(stake_pool_accounts.validator_list.pubkey(), false),
        AccountMeta::new(user_stake.stake_account, false),
        AccountMeta::new_readonly(user_stake.transient_stake_account, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(stake_program::id(), false),
    ];
    let instruction = Instruction {
        program_id: id(),
        accounts,
        data: instruction::StakePoolInstruction::RemoveValidatorFromPool
            .try_to_vec()
            .unwrap(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
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
            let program_error = StakePoolError::SignatureMissing as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while malicious try to remove validator stake account without signing transaction"),
    }
}

#[tokio::test]
async fn fail_with_activating_transient_stake() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, user_stake) =
        setup().await;

    // increase the validator stake
    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.transient_stake_account,
            &user_stake.vote.pubkey(),
            2_000_000_000,
        )
        .await;
    assert!(error.is_none());

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &new_authority,
            &user_stake.stake_account,
            &user_stake.transient_stake_account,
        )
        .await
        .unwrap()
        .unwrap();
    match error {
        TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        ) => {
            let program_error = StakePoolError::WrongStakeState as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while removing validator stake account while transient stake is activating"),
    }
}

#[tokio::test]
async fn fail_not_updated_stake_pool() {} // TODO

#[tokio::test]
async fn fail_with_uninitialized_validator_list_account() {} // TODO
