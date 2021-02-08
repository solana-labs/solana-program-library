#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program::hash::Hash;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    system_instruction, sysvar,
};
use solana_program_test::BanksClient;
use solana_sdk::{
    instruction::InstructionError, signature::Keypair, signature::Signer, transaction::Transaction,
    transaction::TransactionError, transport::TransportError,
};
use spl_stake_pool::*;

async fn create_mint_and_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake_pool_accounts: &StakePoolAccounts,
) {
    create_mint(
        banks_client,
        payer,
        recent_blockhash,
        &stake_pool_accounts.pool_mint,
        &stake_pool_accounts.withdraw_authority,
    )
    .await
    .unwrap();

    create_token_account(
        banks_client,
        payer,
        recent_blockhash,
        &stake_pool_accounts.pool_fee_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &stake_pool_accounts.owner.pubkey(),
    )
    .await
    .unwrap();
}

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
    assert_eq!(stake_pool.data.len(), state::StakePool::LEN);
    assert_eq!(stake_pool.owner, id());

    // Validator stake list storage initialized
    let validator_stake_list = get_account(
        &mut banks_client,
        &stake_pool_accounts.validator_stake_list.pubkey(),
    )
    .await;
    let validator_stake_list =
        state::ValidatorStakeList::deserialize(validator_stake_list.data.as_slice()).unwrap();
    assert_eq!(validator_stake_list.is_initialized(), true);
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

    create_mint_and_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    // create wrong mint
    create_mint(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &wrong_mint,
        &stake_pool_accounts.withdraw_authority,
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
        &stake_pool_accounts.owner,
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

    let wrong_token_program = Keypair::new();

    create_mint(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.pool_mint,
        &stake_pool_accounts.withdraw_authority,
    )
    .await
    .unwrap();

    let rent = banks_client.get_rent().await.unwrap();

    let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);
    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::create_account(
            &payer.pubkey(),
            &stake_pool_accounts.pool_fee_account.pubkey(),
            account_rent,
            spl_token::state::Account::LEN as u64,
            &wrong_token_program.pubkey(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(
        &[&payer, &stake_pool_accounts.pool_fee_account],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let rent_stake_pool = rent.minimum_balance(state::StakePool::LEN);
    let rent_validator_stake_list = rent.minimum_balance(state::ValidatorStakeList::LEN);
    let init_args = instruction::InitArgs {
        fee: stake_pool_accounts.fee,
    };

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.stake_pool.pubkey(),
                rent_stake_pool,
                state::StakePool::LEN as u64,
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
            &stake_pool_accounts.owner,
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

#[tokio::test]
async fn test_initialize_stake_pool_with_wrong_fee_accounts_owner() {
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
    let rent = banks_client.get_rent().await.unwrap();
    let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::create_account(
            &payer.pubkey(),
            &stake_pool_accounts.pool_fee_account.pubkey(),
            account_rent,
            spl_token::state::Account::LEN as u64,
            &Keypair::new().pubkey(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(
        &[&payer, &stake_pool_accounts.pool_fee_account],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let transaction_error = create_stake_pool(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.stake_pool,
        &stake_pool_accounts.validator_stake_list,
        &stake_pool_accounts.pool_mint.pubkey(),
        &stake_pool_accounts.pool_fee_account.pubkey(),
        &stake_pool_accounts.owner,
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
            let program_error = error::StakePoolError::InvalidFeeAccount as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!(
            "Wrong error occurs while try to initialize stake pool with wrong fee account's owner"
        ),
    }
}

#[tokio::test]
async fn test_initialize_stake_pool_with_wrong_withdraw_authority() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let mut stake_pool_accounts = StakePoolAccounts::new();

    stake_pool_accounts.withdraw_authority = Keypair::new().pubkey();

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
            let program_error = error::StakePoolError::WrongMintingAuthority as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!(
            "Wrong error occurs while try to initialize stake pool with wrong withdraw authority"
        ),
    }
}

#[tokio::test]
async fn test_initialize_stake_pool_with_not_rent_exempt_pool() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();

    create_mint_and_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let rent = banks_client.get_rent().await.unwrap();
    let rent_validator_stake_list = rent.minimum_balance(state::ValidatorStakeList::LEN);
    let init_args = instruction::InitArgs {
        fee: stake_pool_accounts.fee,
    };

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.stake_pool.pubkey(),
                1,
                state::StakePool::LEN as u64,
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
                &spl_token::id(),
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
            &stake_pool_accounts.owner,
        ],
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
            let program_error = error::StakePoolError::AccountNotRentExempt as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!(
            "Wrong error occurs while try to initialize stake pool with not rent exempt stake pool account"
        ),
    }
}

#[tokio::test]
async fn test_initialize_stake_pool_with_not_rent_exempt_validator_stake_list() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();

    create_mint_and_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let rent = banks_client.get_rent().await.unwrap();
    let rent_stake_pool = rent.minimum_balance(state::StakePool::LEN);
    let init_args = instruction::InitArgs {
        fee: stake_pool_accounts.fee,
    };

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.stake_pool.pubkey(),
                rent_stake_pool,
                state::StakePool::LEN as u64,
                &id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.validator_stake_list.pubkey(),
                1,
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
                &spl_token::id(),
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
            &stake_pool_accounts.owner,
        ],
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
            let program_error = error::StakePoolError::AccountNotRentExempt as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!(
            "Wrong error occurs while try to initialize stake pool with not rent exempt validator stake list account"
        ),
    }
}

#[tokio::test]
async fn test_initialize_stake_pool_without_owner_signature() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();

    create_mint_and_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let rent = banks_client.get_rent().await.unwrap();
    let rent_stake_pool = rent.minimum_balance(state::StakePool::LEN);
    let init_args = instruction::InitArgs {
        fee: stake_pool_accounts.fee,
    };

    let init_data = instruction::StakePoolInstruction::Initialize(init_args);
    let data = init_data.serialize().unwrap();
    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), true),
        AccountMeta::new_readonly(stake_pool_accounts.owner.pubkey(), false),
        AccountMeta::new(stake_pool_accounts.validator_stake_list.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.pool_mint.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.pool_fee_account.pubkey(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let stake_pool_init_instruction = Instruction {
        program_id: id(),
        accounts,
        data,
    };

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.stake_pool.pubkey(),
                rent_stake_pool,
                state::StakePool::LEN as u64,
                &id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.validator_stake_list.pubkey(),
                state::ValidatorStakeList::LEN as u64,
                state::ValidatorStakeList::LEN as u64,
                &id(),
            ),
            stake_pool_init_instruction,
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
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::SignatureMissing as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!(
            "Wrong error occurs while try to initialize stake pool without owner's signature"
        ),
    }
}
