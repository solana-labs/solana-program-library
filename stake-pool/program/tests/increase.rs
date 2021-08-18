#![cfg(feature = "test-bpf")]

mod helpers;

use {
    bincode::deserialize,
    helpers::*,
    solana_program::{clock::Epoch, hash::Hash, instruction::InstructionError, pubkey::Pubkey},
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
    },
    spl_stake_pool::{
        error::StakePoolError, find_transient_stake_program_address, id, instruction, stake_program,
    },
};

async fn setup() -> (
    BanksClient,
    Keypair,
    Hash,
    StakePoolAccounts,
    ValidatorStakeAccount,
    u64,
) {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    let reserve_lamports = 100_000_000_000;
    stake_pool_accounts
        .initialize_stake_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            reserve_lamports,
        )
        .await
        .unwrap();

    let validator_stake_account = simple_add_validator_to_pool(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let _deposit_info = simple_deposit_stake(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
        &validator_stake_account,
        5_000_000,
    )
    .await
    .unwrap();

    (
        banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake_account,
        reserve_lamports,
    )
}

#[tokio::test]
async fn success() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake,
        reserve_lamports,
    ) = setup().await;

    // Save reserve stake
    let pre_reserve_stake_account = get_account(
        &mut banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await;

    // Check no transient stake
    let transient_account = banks_client
        .get_account(validator_stake.transient_stake_account)
        .await
        .unwrap();
    assert!(transient_account.is_none());

    let rent = banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake_program::StakeState>());
    let increase_amount = reserve_lamports - stake_rent - 1;
    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.vote.pubkey(),
            increase_amount,
            validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none());

    // Check reserve stake account balance
    let reserve_stake_account = get_account(
        &mut banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await;
    let reserve_stake_state =
        deserialize::<stake_program::StakeState>(&reserve_stake_account.data).unwrap();
    assert_eq!(
        pre_reserve_stake_account.lamports - increase_amount - stake_rent,
        reserve_stake_account.lamports
    );
    assert!(reserve_stake_state.delegation().is_none());

    // Check transient stake account state and balance
    let transient_stake_account =
        get_account(&mut banks_client, &validator_stake.transient_stake_account).await;
    let transient_stake_state =
        deserialize::<stake_program::StakeState>(&transient_stake_account.data).unwrap();
    assert_eq!(
        transient_stake_account.lamports,
        increase_amount + stake_rent
    );
    assert_ne!(
        transient_stake_state.delegation().unwrap().activation_epoch,
        Epoch::MAX
    );
}

#[tokio::test]
async fn fail_with_wrong_withdraw_authority() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake,
        reserve_lamports,
    ) = setup().await;

    let wrong_authority = Pubkey::new_unique();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::increase_validator_stake(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &wrong_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &stake_pool_accounts.reserve_stake.pubkey(),
            &validator_stake.transient_stake_account,
            &validator_stake.vote.pubkey(),
            reserve_lamports / 2,
            validator_stake.transient_stake_seed,
        )],
        Some(&payer.pubkey()),
        &[&payer, &stake_pool_accounts.staker],
        recent_blockhash,
    );
    let error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = StakePoolError::InvalidProgramAddress as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error"),
    }
}

#[tokio::test]
async fn fail_with_wrong_validator_list() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake,
        reserve_lamports,
    ) = setup().await;

    let wrong_validator_list = Pubkey::new_unique();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::increase_validator_stake(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &wrong_validator_list,
            &stake_pool_accounts.reserve_stake.pubkey(),
            &validator_stake.transient_stake_account,
            &validator_stake.vote.pubkey(),
            reserve_lamports / 2,
            validator_stake.transient_stake_seed,
        )],
        Some(&payer.pubkey()),
        &[&payer, &stake_pool_accounts.staker],
        recent_blockhash,
    );
    let error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = StakePoolError::InvalidValidatorStakeList as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error"),
    }
}

#[tokio::test]
async fn fail_with_unknown_validator() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        _validator_stake,
        reserve_lamports,
    ) = setup().await;

    let unknown_stake = ValidatorStakeAccount::new(&stake_pool_accounts.stake_pool.pubkey(), 1241);
    unknown_stake
        .create_and_delegate(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &stake_pool_accounts.staker,
        )
        .await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::increase_validator_stake(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &stake_pool_accounts.reserve_stake.pubkey(),
            &unknown_stake.transient_stake_account,
            &unknown_stake.vote.pubkey(),
            reserve_lamports / 2,
            unknown_stake.transient_stake_seed,
        )],
        Some(&payer.pubkey()),
        &[&payer, &stake_pool_accounts.staker],
        recent_blockhash,
    );
    let error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = StakePoolError::ValidatorNotFound as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error"),
    }
}

#[tokio::test]
async fn fail_increase_twice() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake,
        reserve_lamports,
    ) = setup().await;

    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.vote.pubkey(),
            reserve_lamports / 3,
            validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none());

    let transient_stake_seed = validator_stake.transient_stake_seed * 100;
    let transient_stake_address = find_transient_stake_program_address(
        &id(),
        &validator_stake.vote.pubkey(),
        &stake_pool_accounts.stake_pool.pubkey(),
        transient_stake_seed,
    )
    .0;
    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &validator_stake.stake_account,
            &transient_stake_address,
            reserve_lamports / 4,
            transient_stake_seed,
        )
        .await
        .unwrap()
        .unwrap();
    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = StakePoolError::ValidatorNotFound as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error"),
    }
}

#[tokio::test]
async fn fail_with_small_lamport_amount() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake,
        _reserve_lamports,
    ) = setup().await;

    let rent = banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake_program::StakeState>());

    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.vote.pubkey(),
            stake_rent,
            validator_stake.transient_stake_seed,
        )
        .await
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::AccountNotRentExempt) => {}
        _ => panic!("Wrong error"),
    }
}

#[tokio::test]
async fn fail_overdraw_reserve() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake,
        reserve_lamports,
    ) = setup().await;

    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.vote.pubkey(),
            reserve_lamports,
            validator_stake.transient_stake_seed,
        )
        .await
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::InsufficientFunds) => {}
        _ => panic!("Wrong error occurs while overdrawing reserve stake"),
    }
}
