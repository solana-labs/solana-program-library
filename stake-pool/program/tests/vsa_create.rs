#![cfg(feature = "test-bpf")]

mod helpers;

use {
    bincode::deserialize,
    borsh::BorshSerialize,
    helpers::*,
    solana_program::{
        instruction::{AccountMeta, Instruction, InstructionError},
        pubkey::Pubkey,
        system_program, sysvar,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
        transaction::TransactionError,
        transport::TransportError,
    },
    spl_stake_pool::{error, find_stake_program_address, id, instruction, stake_program},
};

#[tokio::test]
async fn success_create_validator_stake_account() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let validator = Keypair::new();
    let vote = Keypair::new();
    create_vote(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &validator,
        &vote,
    )
    .await;

    let (stake_account, _) = find_stake_program_address(
        &id(),
        &vote.pubkey(),
        &stake_pool_accounts.stake_pool.pubkey(),
    );

    let mut transaction = Transaction::new_with_payer(
        &[instruction::create_validator_stake_account(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &payer.pubkey(),
            &stake_account,
            &vote.pubkey(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &stake_pool_accounts.staker], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Check authorities
    let stake = get_account(&mut banks_client, &stake_account).await;
    let stake_state = deserialize::<stake_program::StakeState>(&stake.data).unwrap();
    match stake_state {
        stake_program::StakeState::Stake(meta, stake) => {
            assert_eq!(
                &meta.authorized.staker,
                &stake_pool_accounts.staker.pubkey()
            );
            assert_eq!(
                &meta.authorized.withdrawer,
                &stake_pool_accounts.staker.pubkey()
            );
            assert_eq!(stake.delegation.voter_pubkey, vote.pubkey());
        }
        _ => panic!(),
    }
}

#[tokio::test]
async fn fail_create_validator_stake_account_on_non_vote_account() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let validator = Pubkey::new_unique();

    let (stake_account, _) =
        find_stake_program_address(&id(), &validator, &stake_pool_accounts.stake_pool.pubkey());

    let mut transaction = Transaction::new_with_payer(
        &[instruction::create_validator_stake_account(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &payer.pubkey(),
            &stake_account,
            &validator,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &stake_pool_accounts.staker], recent_blockhash);
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    assert_eq!(
        transaction_error,
        TransactionError::InstructionError(0, InstructionError::IncorrectProgramId,)
    );
}

#[tokio::test]
async fn fail_create_validator_stake_account_with_wrong_system_program() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let validator = Pubkey::new_unique();

    let (stake_account, _) =
        find_stake_program_address(&id(), &validator, &stake_pool_accounts.stake_pool.pubkey());
    let wrong_system_program = Pubkey::new_unique();
    let accounts = vec![
        AccountMeta::new_readonly(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.staker.pubkey(), true),
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(stake_account, false),
        AccountMeta::new_readonly(validator, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        AccountMeta::new_readonly(stake_program::config_id(), false),
        AccountMeta::new_readonly(wrong_system_program, false),
        AccountMeta::new_readonly(stake_program::id(), false),
    ];
    let instruction = Instruction {
        program_id: id(),
        accounts,
        data: instruction::StakePoolInstruction::CreateValidatorStakeAccount
            .try_to_vec()
            .unwrap(),
    };

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &stake_pool_accounts.staker], recent_blockhash);
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    assert_eq!(
        transaction_error,
        TransactionError::InstructionError(0, InstructionError::IncorrectProgramId,)
    );
}

#[tokio::test]
async fn fail_create_validator_stake_account_with_wrong_stake_program() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let validator = Pubkey::new_unique();

    let (stake_account, _) =
        find_stake_program_address(&id(), &validator, &stake_pool_accounts.stake_pool.pubkey());
    let wrong_stake_program = Pubkey::new_unique();
    let accounts = vec![
        AccountMeta::new_readonly(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.staker.pubkey(), true),
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(stake_account, false),
        AccountMeta::new_readonly(validator, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        AccountMeta::new_readonly(stake_program::config_id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(wrong_stake_program, false),
    ];
    let instruction = Instruction {
        program_id: id(),
        accounts,
        data: instruction::StakePoolInstruction::CreateValidatorStakeAccount
            .try_to_vec()
            .unwrap(),
    };

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &stake_pool_accounts.staker], recent_blockhash);
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    assert_eq!(
        transaction_error,
        TransactionError::InstructionError(0, InstructionError::IncorrectProgramId,)
    );
}

#[tokio::test]
async fn fail_create_validator_stake_account_with_incorrect_address() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let validator = Pubkey::new_unique();
    let stake_account = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::create_validator_stake_account(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &payer.pubkey(),
            &stake_account.pubkey(),
            &validator,
        )],
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
            let program_error = error::StakePoolError::InvalidStakeAccountAddress as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!(
            "Wrong error occurs while try to create validator stake account with incorrect address"
        ),
    }
}
