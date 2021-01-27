#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program::system_instruction;
use solana_sdk::{
    instruction::InstructionError, signature::Keypair, signature::Signer, transaction::Transaction,
    transaction::TransactionError, transport::TransportError,
};
use spl_stake_pool::*;

#[tokio::test]
async fn test_stake_pool_initialize() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    // Stake pool now exists
    let stake_pool = get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    assert_eq!(stake_pool.data.len(), state::State::LEN);
    assert_eq!(stake_pool.owner, id());

    // Validator stake list storage initialized
    let validator_stake_list = get_account(
        &mut banks_client,
        &stake_pool_accounts.validator_stake_list.pubkey(),
    )
    .await;
    let validator_stake_list =
        state::ValidatorStakeList::deserialize(validator_stake_list.data.as_slice()).unwrap();
    assert_eq!(validator_stake_list.is_initialized, true);
}

#[tokio::test]
async fn test_initialize_already_initialized_stake_pool() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    let latest_blockhash = banks_client.get_recent_blockhash().await.unwrap();

    let mut second_stake_pool_accounts = StakePoolAccounts::new();
    second_stake_pool_accounts.stake_pool = stake_pool_accounts.stake_pool;

    let transaction_error = second_stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &latest_blockhash)
        .await
        .err()
        .unwrap();
    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::AlreadyInUse as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to initialize already initialized stake pool"),
    }
}

#[tokio::test]
async fn test_initialize_stake_pool_with_already_initialized_stake_list_storage() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    let latest_blockhash = banks_client.get_recent_blockhash().await.unwrap();

    let mut second_stake_pool_accounts = StakePoolAccounts::new();
    second_stake_pool_accounts.validator_stake_list = stake_pool_accounts.validator_stake_list;

    let transaction_error = second_stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &latest_blockhash)
        .await
        .err()
        .unwrap();
    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::AlreadyInUse as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to initialize stake pool with already initialized stake list storage"),
    }
}

#[tokio::test]
async fn test_initialize_stake_pool_with_high_fee() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let mut stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts.fee = instruction::Fee {
        numerator: 100001,
        denominator: 100000,
    };

    let transaction_error = stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await
        .err()
        .unwrap();
    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::FeeTooHigh as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to initialize stake pool with high fee"),
    }
}

#[tokio::test]
async fn test_initialize_stake_pool_with_wrong_mint_authority() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    let wrong_mint = Keypair::new();

    create_mint(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.pool_mint,
        &stake_pool_accounts.withdraw_authority,
    )
    .await
    .unwrap();

    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.pool_fee_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &stake_pool_accounts.owner.pubkey(),
    )
    .await
    .unwrap();

    let transaction_error = create_stake_pool(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.stake_pool,
        &stake_pool_accounts.validator_stake_list,
        &wrong_mint.pubkey(),
        &stake_pool_accounts.pool_fee_account.pubkey(),
        &stake_pool_accounts.owner.pubkey(),
        &stake_pool_accounts.fee,
    )
    .await
    .err()
    .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::WrongAccountMint as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to initialize stake pool with wrong mint authority of pool fee account"),
    }
}

#[tokio::test]
async fn test_initialize_stake_pool_with_wrong_token_program_id() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();

    create_mint(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.pool_mint,
        &stake_pool_accounts.withdraw_authority,
    )
    .await
    .unwrap();

    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.pool_fee_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &stake_pool_accounts.owner.pubkey(),
    )
    .await
    .unwrap();

    let rent = banks_client.get_rent().await.unwrap();
    let rent_stake_pool = rent.minimum_balance(state::State::LEN);
    let rent_validator_stake_list = rent.minimum_balance(state::ValidatorStakeList::LEN);
    let init_args = instruction::InitArgs {
        fee: stake_pool_accounts.fee,
    };
    let wrong_token_program = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.stake_pool.pubkey(),
                rent_stake_pool,
                state::State::LEN as u64,
                &id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.validator_stake_list.pubkey(),
                rent_validator_stake_list,
                state::ValidatorStakeList::LEN as u64,
                &id(),
            ),
            instruction::initialize(
                &id(),
                &stake_pool_accounts.stake_pool.pubkey(),
                &stake_pool_accounts.owner.pubkey(),
                &stake_pool_accounts.validator_stake_list.pubkey(),
                &stake_pool_accounts.pool_mint.pubkey(),
                &stake_pool_accounts.pool_fee_account.pubkey(),
                &wrong_token_program.pubkey(),
                init_args,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(
        &[
            &payer,
            &stake_pool_accounts.stake_pool,
            &stake_pool_accounts.validator_stake_list,
        ],
        recent_blockhash,
    );
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
            "Wrong error occurs while try to initialize stake pool with wrong token program ID"
        ),
    }
}
