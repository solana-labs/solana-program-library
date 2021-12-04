#![cfg(feature = "test-bpf")]

mod helpers;

use {
    bincode::deserialize,
    helpers::*,
    solana_program::{
        clock::Epoch, hash::Hash, instruction::InstructionError, pubkey::Pubkey, stake,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
    },
    spl_stake_pool::{
        error::StakePoolError, find_transient_stake_program_address, id, instruction,
    },
};

async fn setup() -> (
    BanksClient,
    Keypair,
    Hash,
    StakePoolAccounts,
    ValidatorStakeAccount,
    DepositStakeAccount,
    u64,
) {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let validator_stake_account = simple_add_validator_to_pool(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let deposit_info = simple_deposit_stake(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
        &validator_stake_account,
        100_000_000,
    )
    .await
    .unwrap();

    let lamports = deposit_info.stake_lamports / 2;

    (
        banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        lamports,
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
        _deposit_info,
        decrease_lamports,
    ) = setup().await;

    // Save validator stake
    let pre_validator_stake_account =
        get_account(&mut banks_client, &validator_stake.stake_account).await;

    // Check no transient stake
    let transient_account = banks_client
        .get_account(validator_stake.transient_stake_account)
        .await
        .unwrap();
    assert!(transient_account.is_none());

    let error = stake_pool_accounts
        .decrease_validator_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            decrease_lamports,
            validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none());

    // Check validator stake account balance
    let validator_stake_account =
        get_account(&mut banks_client, &validator_stake.stake_account).await;
    let validator_stake_state =
        deserialize::<stake::state::StakeState>(&validator_stake_account.data).unwrap();
    assert_eq!(
        pre_validator_stake_account.lamports - decrease_lamports,
        validator_stake_account.lamports
    );
    assert_eq!(
        validator_stake_state
            .delegation()
            .unwrap()
            .deactivation_epoch,
        Epoch::MAX
    );

    // Check transient stake account state and balance
    let transient_stake_account =
        get_account(&mut banks_client, &validator_stake.transient_stake_account).await;
    let transient_stake_state =
        deserialize::<stake::state::StakeState>(&transient_stake_account.data).unwrap();
    assert_eq!(transient_stake_account.lamports, decrease_lamports);
    assert_ne!(
        transient_stake_state
            .delegation()
            .unwrap()
            .deactivation_epoch,
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
        _deposit_info,
        decrease_lamports,
    ) = setup().await;

    let wrong_authority = Pubkey::new_unique();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::decrease_validator_stake(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &wrong_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            decrease_lamports,
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
        _ => panic!("Wrong error occurs while decreasing with wrong withdraw authority"),
    }
}

#[tokio::test]
async fn fail_with_wrong_validator_list() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        mut stake_pool_accounts,
        validator_stake,
        _deposit_info,
        decrease_lamports,
    ) = setup().await;

    stake_pool_accounts.validator_list = Keypair::new();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::decrease_validator_stake(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            decrease_lamports,
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
        _ => panic!("Wrong error occurs while decreasing with wrong validator stake list account"),
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
        _deposit_info,
        decrease_lamports,
    ) = setup().await;

    let unknown_stake = create_unknown_validator_stake(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::decrease_validator_stake(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &unknown_stake.stake_account,
            &unknown_stake.transient_stake_account,
            decrease_lamports,
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

    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::InvalidStakeAccountAddress as u32)
        )
    );
}

#[tokio::test]
async fn fail_decrease_twice() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake,
        _deposit_info,
        decrease_lamports,
    ) = setup().await;

    let error = stake_pool_accounts
        .decrease_validator_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            decrease_lamports / 3,
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
        .decrease_validator_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &validator_stake.stake_account,
            &transient_stake_address,
            decrease_lamports / 2,
            transient_stake_seed,
        )
        .await
        .unwrap()
        .unwrap();
    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = StakePoolError::TransientAccountInUse as u32;
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
        _deposit_info,
        _decrease_lamports,
    ) = setup().await;

    let rent = banks_client.get_rent().await.unwrap();
    let lamports = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>());

    let error = stake_pool_accounts
        .decrease_validator_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            lamports,
            validator_stake.transient_stake_seed,
        )
        .await
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::AccountNotRentExempt) => {}
        _ => panic!("Wrong error occurs while try to decrease small stake"),
    }
}

#[tokio::test]
async fn fail_big_overdraw() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake,
        deposit_info,
        _decrease_lamports,
    ) = setup().await;

    let error = stake_pool_accounts
        .decrease_validator_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            deposit_info.stake_lamports * 1_000_000,
            validator_stake.transient_stake_seed,
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        error,
        TransactionError::InstructionError(0, InstructionError::InsufficientFunds)
    );
}

#[tokio::test]
async fn fail_overdraw() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake,
        deposit_info,
        _decrease_lamports,
    ) = setup().await;

    let rent = banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>());

    let error = stake_pool_accounts
        .decrease_validator_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            deposit_info.stake_lamports + stake_rent + 1,
            validator_stake.transient_stake_seed,
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        error,
        TransactionError::InstructionError(0, InstructionError::InsufficientFunds)
    );
}
