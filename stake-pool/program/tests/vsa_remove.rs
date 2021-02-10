#![cfg(feature = "test-bpf")]

mod helpers;

use bincode::deserialize;
use helpers::*;
use solana_program::hash::Hash;
use solana_program::instruction::AccountMeta;
use solana_program::instruction::Instruction;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar;
use solana_program_test::BanksClient;
use solana_sdk::{
    instruction::InstructionError,
    signature::{Keypair, Signer},
    transaction::Transaction,
    transaction::TransactionError,
    transport::TransportError,
};
use spl_stake_pool::*;

async fn setup() -> (
    BanksClient,
    Keypair,
    Hash,
    StakePoolAccounts,
    ValidatorStakeAccount,
    Keypair,
    Keypair,
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
        user_pool_account,
        user,
    )
}

#[tokio::test]
async fn test_remove_validator_stake_account() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        user_stake,
        user_pool_account,
        user,
    ) = setup().await;

    let tokens_to_burn = get_token_balance(&mut banks_client, &user_pool_account.pubkey()).await;
    delegate_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_pool_account.pubkey(),
        &user,
        &stake_pool_accounts.withdraw_authority,
        tokens_to_burn,
    )
    .await;

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .remove_validator_stake_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.stake_account,
            &user_pool_account.pubkey(),
            &new_authority,
        )
        .await;
    assert!(error.is_none());

    // Check if all tokens were burned
    let tokens_left = get_token_balance(&mut banks_client, &user_pool_account.pubkey()).await;
    assert_eq!(tokens_left, 0);

    // Check if account was removed from the list of stake accounts
    let validator_stake_list = get_account(
        &mut banks_client,
        &stake_pool_accounts.validator_stake_list.pubkey(),
    )
    .await;
    let validator_stake_list =
        state::ValidatorStakeList::deserialize(validator_stake_list.data.as_slice()).unwrap();
    assert_eq!(
        validator_stake_list,
        state::ValidatorStakeList {
            version: state::ValidatorStakeList::VALIDATOR_STAKE_LIST_VERSION,
            validators: vec![]
        }
    );

    // Check of stake account authority has changed
    let stake = get_account(&mut banks_client, &user_stake.stake_account).await;
    let stake_state = deserialize::<stake::StakeState>(&stake.data).unwrap();
    match stake_state {
        stake::StakeState::Stake(meta, _) => {
            assert_eq!(&meta.authorized.staker, &new_authority);
            assert_eq!(&meta.authorized.withdrawer, &new_authority);
        }
        _ => panic!(),
    }
}

#[tokio::test]
async fn test_remove_validator_stake_account_with_wrong_stake_program_id() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        user_stake,
        user_pool_account,
        _,
    ) = setup().await;

    let wrong_stake_program = Keypair::new();

    let new_authority = Pubkey::new_unique();
    let mut transaction = Transaction::new_with_payer(
        &[instruction::remove_validator_stake_account(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.owner.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &new_authority,
            &stake_pool_accounts.validator_stake_list.pubkey(),
            &user_stake.stake_account,
            &user_pool_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &spl_token::id(),
            &wrong_stake_program.pubkey(),
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
            error,
        )) => {
            assert_eq!(error, InstructionError::IncorrectProgramId);
        }
        _ => panic!("Wrong error occurs while try to remove validator stake address with wrong stake program ID"),
    }
}

#[tokio::test]
async fn test_remove_validator_stake_account_with_wrong_token_program_id() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        user_stake,
        user_pool_account,
        _,
    ) = setup().await;

    let wrong_token_program = Keypair::new();

    let new_authority = Pubkey::new_unique();
    let mut transaction = Transaction::new_with_payer(
        &[instruction::remove_validator_stake_account(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.owner.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &new_authority,
            &stake_pool_accounts.validator_stake_list.pubkey(),
            &user_stake.stake_account,
            &user_pool_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &wrong_token_program.pubkey(),
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
        TransportError::TransactionError(TransactionError::InstructionError(_, error)) => {
            assert_eq!(error, InstructionError::IncorrectProgramId);
        }
        _ => panic!("Wrong error occurs while try to remove validator stake address with wrong token program ID"),
    }
}

#[tokio::test]
async fn test_remove_validator_stake_account_with_wrong_pool_mint_account() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        user_stake,
        user_pool_account,
        _,
    ) = setup().await;

    let wrong_pool_mint = Keypair::new();

    let new_authority = Pubkey::new_unique();
    let mut transaction = Transaction::new_with_payer(
        &[instruction::remove_validator_stake_account(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.owner.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &new_authority,
            &stake_pool_accounts.validator_stake_list.pubkey(),
            &user_stake.stake_account,
            &user_pool_account.pubkey(),
            &wrong_pool_mint.pubkey(),
            &spl_token::id(),
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
            let program_error = error::StakePoolError::WrongPoolMint as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to remove validator stake address with wrong pool mint account"),
    }
}

#[tokio::test]
async fn test_remove_validator_stake_account_with_wrong_validator_stake_list_account() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        user_stake,
        user_pool_account,
        _,
    ) = setup().await;

    let wrong_validator_stake_list = Keypair::new();

    let new_authority = Pubkey::new_unique();
    let mut transaction = Transaction::new_with_payer(
        &[instruction::remove_validator_stake_account(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.owner.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &new_authority,
            &wrong_validator_stake_list.pubkey(),
            &user_stake.stake_account,
            &user_pool_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &spl_token::id(),
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
            let program_error = error::StakePoolError::InvalidValidatorStakeList as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to remove validator stake address with wrong validator stake list account"),
    }
}

#[tokio::test]
async fn test_remove_already_removed_validator_stake_account() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        user_stake,
        user_pool_account,
        user,
    ) = setup().await;

    let tokens_to_burn = get_token_balance(&mut banks_client, &user_pool_account.pubkey()).await;
    delegate_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_pool_account.pubkey(),
        &user,
        &stake_pool_accounts.withdraw_authority,
        tokens_to_burn,
    )
    .await;

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .remove_validator_stake_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.stake_account,
            &user_pool_account.pubkey(),
            &new_authority,
        )
        .await;
    assert!(error.is_none());

    let latest_blockhash = banks_client.get_recent_blockhash().await.unwrap();

    let transaction_error = stake_pool_accounts
        .remove_validator_stake_account(
            &mut banks_client,
            &payer,
            &latest_blockhash,
            &user_stake.stake_account,
            &user_pool_account.pubkey(),
            &new_authority,
        )
        .await
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::ValidatorNotFound as u32;
            assert_eq!(error_index, program_error);
        }
        _ => {
            panic!("Wrong error occurs while try to remove already removed validator stake address")
        }
    }
}

#[tokio::test]
async fn test_not_owner_try_to_remove_validator_stake_account() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        user_stake,
        user_pool_account,
        _,
    ) = setup().await;

    let malicious = Keypair::new();

    let new_authority = Pubkey::new_unique();
    let mut transaction = Transaction::new_with_payer(
        &[instruction::remove_validator_stake_account(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &malicious.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &new_authority,
            &stake_pool_accounts.validator_stake_list.pubkey(),
            &user_stake.stake_account,
            &user_pool_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &spl_token::id(),
            &stake::id(),
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
            let program_error = error::StakePoolError::WrongOwner as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while not an owner try to remove validator stake address"),
    }
}

#[tokio::test]
async fn test_not_owner_try_to_remove_validator_stake_account_without_signature() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        user_stake,
        user_pool_account,
        _,
    ) = setup().await;

    let new_authority = Pubkey::new_unique();

    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.owner.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.withdraw_authority, false),
        AccountMeta::new_readonly(new_authority, false),
        AccountMeta::new(stake_pool_accounts.validator_stake_list.pubkey(), false),
        AccountMeta::new(user_stake.stake_account, false),
        AccountMeta::new(user_pool_account.pubkey(), false),
        AccountMeta::new(stake_pool_accounts.pool_mint.pubkey(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(stake::id(), false),
    ];
    let instruction = Instruction {
        program_id: id(),
        accounts,
        data: instruction::StakePoolInstruction::RemoveValidatorStakeAccount
            .serialize()
            .unwrap(),
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
        _ => panic!("Wrong error occurs while malicious try to remove validator stake account without signing transaction"),
    }
}

#[tokio::test]
async fn test_remove_validator_stake_account_when_stake_acc_not_in_stake_state() {
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
    let user_stake_authority = Keypair::new();
    create_validator_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake.stake_pool,
        &user_stake.stake_account,
        &user_stake.vote.pubkey(),
        &user_stake_authority.pubkey(),
        &user_stake.target_authority,
    )
    .await;

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

    let new_authority = Pubkey::new_unique();

    let transaction_error = stake_pool_accounts
        .remove_validator_stake_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.stake_account,
            &user_pool_account.pubkey(),
            &new_authority,
        )
        .await
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::WrongStakeState as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to add validator stake account when it isn't in stake state"),
    }
}

#[tokio::test]
async fn test_remove_validator_stake_account_from_unupdated_stake_pool() {} // TODO

#[tokio::test]
async fn test_remove_validator_stake_account_with_uninitialized_validator_stake_list_account() {} // TODO
