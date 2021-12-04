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
        state::StakePool,
        state::{Fee, FeeType},
    },
};

async fn setup(fee: Option<Fee>) -> (ProgramTestContext, StakePoolAccounts, Fee) {
    let mut context = program_test().start_with_context().await;
    let mut stake_pool_accounts = StakePoolAccounts::new();
    if let Some(fee) = fee {
        stake_pool_accounts.deposit_fee = fee;
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
    let new_deposit_fee = Fee {
        numerator: 823,
        denominator: 1000,
    };

    (context, stake_pool_accounts, new_deposit_fee)
}

#[tokio::test]
async fn success_stake() {
    let (mut context, stake_pool_accounts, new_deposit_fee) = setup(None).await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::StakeDeposit(new_deposit_fee),
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
    assert_eq!(stake_pool.stake_deposit_fee, new_deposit_fee);
}

#[tokio::test]
async fn success_stake_increase_fee_from_0() {
    let (mut context, stake_pool_accounts, _) = setup(Some(Fee {
        numerator: 0,
        denominator: 0,
    }))
    .await;
    let new_deposit_fee = Fee {
        numerator: 324,
        denominator: 1234,
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::StakeDeposit(new_deposit_fee),
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
    assert_eq!(stake_pool.stake_deposit_fee, new_deposit_fee);
}

#[tokio::test]
async fn fail_stake_wrong_manager() {
    let (mut context, stake_pool_accounts, new_deposit_fee) = setup(None).await;

    let wrong_manager = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &wrong_manager.pubkey(),
            FeeType::StakeDeposit(new_deposit_fee),
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
async fn fail_stake_high_deposit_fee() {
    let (mut context, stake_pool_accounts, _new_deposit_fee) = setup(None).await;

    let new_deposit_fee = Fee {
        numerator: 100001,
        denominator: 100000,
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::StakeDeposit(new_deposit_fee),
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
async fn success_sol() {
    let (mut context, stake_pool_accounts, new_deposit_fee) = setup(None).await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::SolDeposit(new_deposit_fee),
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
    assert_eq!(stake_pool.sol_deposit_fee, new_deposit_fee);
}

#[tokio::test]
async fn fail_sol_wrong_manager() {
    let (mut context, stake_pool_accounts, new_deposit_fee) = setup(None).await;

    let wrong_manager = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &wrong_manager.pubkey(),
            FeeType::SolDeposit(new_deposit_fee),
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
async fn fail_sol_high_deposit_fee() {
    let (mut context, stake_pool_accounts, _new_deposit_fee) = setup(None).await;

    let new_deposit_fee = Fee {
        numerator: 100001,
        denominator: 100000,
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_fee(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            FeeType::SolDeposit(new_deposit_fee),
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
