#![cfg(feature = "test-bpf")]

mod helpers;

use {
    helpers::*,
    solana_program_test::*,
    solana_sdk::{
        borsh::try_from_slice_unchecked,
        instruction::InstructionError,
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
    },
    spl_stake_pool::{
        error, id, instruction,
        state::{Fee, FeeType, StakePool},
    },
};

async fn setup(fee: Option<Fee>) -> (ProgramTestContext, StakePoolAccounts, Fee) {
    let mut context = program_test().start_with_context().await;
    let mut stake_pool_accounts = StakePoolAccounts::new();
    if let Some(fee) = fee {
        stake_pool_accounts.withdrawal_fee = fee;
    }
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            1,
        )
        .await
        .unwrap();
    let new_withdrawal_fee = Fee {
        numerator: 4,
        denominator: 1000,
    };

    (context, stake_pool_accounts, new_withdrawal_fee)
}

#[tokio::test]
async fn success() {
    let (mut context, stake_pool_accounts, new_withdrawal_fee) = setup(None).await;

    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(stake_pool.data.as_slice()).unwrap();
    let old_stake_withdrawal_fee = stake_pool.stake_withdrawal_fee;
    let old_sol_withdrawal_fee = stake_pool.sol_withdrawal_fee;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::StakeWithdrawal(new_withdrawal_fee),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::SolWithdrawal(new_withdrawal_fee),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(stake_pool.data.as_slice()).unwrap();

    assert_eq!(stake_pool.stake_withdrawal_fee, old_stake_withdrawal_fee);
    assert_eq!(
        stake_pool.next_stake_withdrawal_fee,
        Some(new_withdrawal_fee)
    );
    assert_eq!(stake_pool.sol_withdrawal_fee, old_sol_withdrawal_fee);
    assert_eq!(stake_pool.next_sol_withdrawal_fee, Some(new_withdrawal_fee));

    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;

    context
        .warp_to_slot(first_normal_slot + slots_per_epoch)
        .unwrap();
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &[],
            false,
        )
        .await;

    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(stake_pool.data.as_slice()).unwrap();
    assert_eq!(stake_pool.stake_withdrawal_fee, new_withdrawal_fee);
    assert_eq!(stake_pool.next_stake_withdrawal_fee, None);
    assert_eq!(stake_pool.sol_withdrawal_fee, new_withdrawal_fee);
    assert_eq!(stake_pool.next_sol_withdrawal_fee, None);
}

#[tokio::test]
async fn success_fee_cannot_increase_more_than_once() {
    let (mut context, stake_pool_accounts, new_withdrawal_fee) = setup(None).await;

    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(stake_pool.data.as_slice()).unwrap();
    let old_stake_withdrawal_fee = stake_pool.stake_withdrawal_fee;
    let old_sol_withdrawal_fee = stake_pool.sol_withdrawal_fee;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::StakeWithdrawal(new_withdrawal_fee),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::SolWithdrawal(new_withdrawal_fee),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(stake_pool.data.as_slice()).unwrap();

    assert_eq!(stake_pool.stake_withdrawal_fee, old_stake_withdrawal_fee);
    assert_eq!(
        stake_pool.next_stake_withdrawal_fee,
        Some(new_withdrawal_fee)
    );
    assert_eq!(stake_pool.sol_withdrawal_fee, old_sol_withdrawal_fee);
    assert_eq!(stake_pool.next_sol_withdrawal_fee, Some(new_withdrawal_fee));

    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;

    context
        .warp_to_slot(first_normal_slot + slots_per_epoch)
        .unwrap();
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &[],
            false,
        )
        .await;

    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(stake_pool.data.as_slice()).unwrap();
    assert_eq!(stake_pool.stake_withdrawal_fee, new_withdrawal_fee);
    assert_eq!(stake_pool.next_stake_withdrawal_fee, None);
    assert_eq!(stake_pool.sol_withdrawal_fee, new_withdrawal_fee);
    assert_eq!(stake_pool.next_sol_withdrawal_fee, None);

    // try setting to the old fee in the same epoch
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::StakeWithdrawal(old_stake_withdrawal_fee),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::SolWithdrawal(old_sol_withdrawal_fee),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(stake_pool.data.as_slice()).unwrap();
    assert_eq!(stake_pool.stake_withdrawal_fee, new_withdrawal_fee);
    assert_eq!(
        stake_pool.next_stake_withdrawal_fee,
        Some(old_stake_withdrawal_fee)
    );
    assert_eq!(stake_pool.sol_withdrawal_fee, new_withdrawal_fee);
    assert_eq!(
        stake_pool.next_sol_withdrawal_fee,
        Some(old_sol_withdrawal_fee)
    );

    let error = stake_pool_accounts
        .update_stake_pool_balance(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
        )
        .await;
    assert!(error.is_none());

    // Check that nothing has changed after updating the stake pool
    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(stake_pool.data.as_slice()).unwrap();
    assert_eq!(stake_pool.stake_withdrawal_fee, new_withdrawal_fee);
    assert_eq!(
        stake_pool.next_stake_withdrawal_fee,
        Some(old_stake_withdrawal_fee)
    );
    assert_eq!(stake_pool.sol_withdrawal_fee, new_withdrawal_fee);
    assert_eq!(
        stake_pool.next_sol_withdrawal_fee,
        Some(old_sol_withdrawal_fee)
    );
}

#[tokio::test]
async fn success_increase_fee_from_0() {
    let (mut context, stake_pool_accounts, _) = setup(Some(Fee {
        numerator: 0,
        denominator: 1,
    }))
    .await;
    let new_withdrawal_fee = Fee {
        numerator: 15,
        denominator: 10000,
    };

    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(stake_pool.data.as_slice()).unwrap();
    let old_stake_withdrawal_fee = stake_pool.stake_withdrawal_fee;
    let old_sol_withdrawal_fee = stake_pool.sol_withdrawal_fee;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::StakeWithdrawal(new_withdrawal_fee),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::SolWithdrawal(new_withdrawal_fee),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(stake_pool.data.as_slice()).unwrap();

    assert_eq!(stake_pool.stake_withdrawal_fee, old_stake_withdrawal_fee);
    assert_eq!(
        stake_pool.next_stake_withdrawal_fee,
        Some(new_withdrawal_fee)
    );
    assert_eq!(stake_pool.sol_withdrawal_fee, old_sol_withdrawal_fee);
    assert_eq!(stake_pool.next_sol_withdrawal_fee, Some(new_withdrawal_fee));

    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;

    context
        .warp_to_slot(first_normal_slot + slots_per_epoch)
        .unwrap();
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &[],
            false,
        )
        .await;

    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(stake_pool.data.as_slice()).unwrap();
    assert_eq!(stake_pool.stake_withdrawal_fee, new_withdrawal_fee);
    assert_eq!(stake_pool.next_stake_withdrawal_fee, None);
    assert_eq!(stake_pool.sol_withdrawal_fee, new_withdrawal_fee);
    assert_eq!(stake_pool.next_sol_withdrawal_fee, None);
}

#[tokio::test]
async fn fail_wrong_manager() {
    let (mut context, stake_pool_accounts, new_stake_withdrawal_fee) = setup(None).await;

    let wrong_manager = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &wrong_manager.pubkey(),
            FeeType::StakeWithdrawal(new_stake_withdrawal_fee),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wrong_manager],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = error::StakePoolError::WrongManager as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while signing with the wrong manager"),
    }
}

#[tokio::test]
async fn fail_high_withdrawal_fee() {
    let (mut context, stake_pool_accounts, _new_stake_withdrawal_fee) = setup(None).await;

    let new_stake_withdrawal_fee = Fee {
        numerator: 11,
        denominator: 10,
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::StakeWithdrawal(new_stake_withdrawal_fee),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = error::StakePoolError::FeeTooHigh as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs when setting fee too high"),
    }
}

#[tokio::test]
async fn fail_high_stake_fee_increase() {
    let (mut context, stake_pool_accounts, _new_stake_withdrawal_fee) = setup(None).await;
    let new_withdrawal_fee = Fee {
        numerator: 46,
        denominator: 10_000,
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::StakeWithdrawal(new_withdrawal_fee),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = error::StakePoolError::FeeIncreaseTooHigh as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs when increasing fee by too large a factor"),
    }
}

#[tokio::test]
async fn fail_high_sol_fee_increase() {
    let (mut context, stake_pool_accounts, _new_stake_withdrawal_fee) = setup(None).await;
    let new_withdrawal_fee = Fee {
        numerator: 46,
        denominator: 10_000,
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::SolWithdrawal(new_withdrawal_fee),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = error::StakePoolError::FeeIncreaseTooHigh as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs when increasing fee by too large a factor"),
    }
}

#[tokio::test]
async fn fail_high_stake_fee_increase_from_0() {
    let (mut context, stake_pool_accounts, _new_stake_withdrawal_fee) = setup(Some(Fee {
        numerator: 0,
        denominator: 1,
    }))
    .await;
    let new_withdrawal_fee = Fee {
        numerator: 16,
        denominator: 10_000,
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::StakeWithdrawal(new_withdrawal_fee),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = error::StakePoolError::FeeIncreaseTooHigh as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs when increasing fee by too large a factor"),
    }
}

#[tokio::test]
async fn fail_high_sol_fee_increase_from_0() {
    let (mut context, stake_pool_accounts, _new_stake_withdrawal_fee) = setup(Some(Fee {
        numerator: 0,
        denominator: 1,
    }))
    .await;
    let new_withdrawal_fee = Fee {
        numerator: 16,
        denominator: 10_000,
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::SolWithdrawal(new_withdrawal_fee),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = error::StakePoolError::FeeIncreaseTooHigh as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs when increasing fee by too large a factor"),
    }
}

#[tokio::test]
async fn fail_not_updated() {
    let mut context = program_test().start_with_context().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            1,
        )
        .await
        .unwrap();
    let new_stake_withdrawal_fee = Fee {
        numerator: 11,
        denominator: 100,
    };

    // move forward so an update is required
    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    context
        .warp_to_slot(first_normal_slot + slots_per_epoch)
        .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::StakeWithdrawal(new_stake_withdrawal_fee),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = error::StakePoolError::StakeListAndPoolOutOfDate as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs when stake pool out of date"),
    }
}
