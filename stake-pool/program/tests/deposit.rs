#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;

use solana_program::hash::Hash;
use solana_program_test::BanksClient;
use solana_sdk::{
    instruction::InstructionError,
    signature::{Keypair, Signer},
    transaction::Transaction,
    transaction::TransactionError,
    transport::TransportError,
};
use spl_stake_pool::*;
use spl_token::error as token_error;

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

    let validator_stake_account: ValidatorStakeAccount = simple_add_validator_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    (
        banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake_account,
    )
}

#[tokio::test]
async fn test_stake_pool_deposit() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, validator_stake_account) =
        setup().await;

    let user = Keypair::new();
    // make stake account
    let user_stake = Keypair::new();
    let lockup = stake::Lockup::default();

    let stake_authority = Keypair::new();
    let authorized = stake::Authorized {
        staker: stake_authority.pubkey(),
        withdrawer: stake_authority.pubkey(),
    };

    let stake_lamports = create_independent_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake,
        &authorized,
        &lockup,
    )
    .await;

    create_vote(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &validator_stake_account.vote,
    )
    .await;
    delegate_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake.pubkey(),
        &stake_authority,
        &validator_stake_account.vote.pubkey(),
    )
    .await;

    // Change authority to the stake pool's deposit
    authorize_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake.pubkey(),
        &stake_authority,
        &stake_pool_accounts.deposit_authority,
        stake::StakeAuthorize::Withdrawer,
    )
    .await;
    authorize_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake.pubkey(),
        &stake_authority,
        &stake_pool_accounts.deposit_authority,
        stake::StakeAuthorize::Staker,
    )
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

    // Save stake pool state before depositing
    let stake_pool_before =
        get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool_before =
        state::StakePool::deserialize(&stake_pool_before.data.as_slice()).unwrap();

    // Save validator stake account record before depositing
    let validator_stake_list = get_account(
        &mut banks_client,
        &stake_pool_accounts.validator_stake_list.pubkey(),
    )
    .await;
    let validator_stake_list =
        state::ValidatorStakeList::deserialize(validator_stake_list.data.as_slice()).unwrap();
    let validator_stake_item_before = validator_stake_list
        .find(&validator_stake_account.vote.pubkey())
        .unwrap();

    stake_pool_accounts
        .deposit_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.pubkey(),
            &user_pool_account.pubkey(),
            &validator_stake_account.stake_account,
        )
        .await
        .unwrap();

    // Original stake account should be drained
    assert!(banks_client
        .get_account(user_stake.pubkey())
        .await
        .expect("get_account")
        .is_none());

    let tokens_issued = stake_lamports; // For now tokens are 1:1 to stake
    let fee = stake_pool_accounts.calculate_fee(tokens_issued);

    // Stake pool should add its balance to the pool balance
    let stake_pool = get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool = state::StakePool::deserialize(&stake_pool.data.as_slice()).unwrap();
    assert_eq!(
        stake_pool.stake_total,
        stake_pool_before.stake_total + stake_lamports
    );
    assert_eq!(
        stake_pool.pool_total,
        stake_pool_before.pool_total + tokens_issued
    );

    // Check minted tokens
    let user_token_balance =
        get_token_balance(&mut banks_client, &user_pool_account.pubkey()).await;
    assert_eq!(user_token_balance, tokens_issued - fee);
    let pool_fee_token_balance = get_token_balance(
        &mut banks_client,
        &stake_pool_accounts.pool_fee_account.pubkey(),
    )
    .await;
    assert_eq!(pool_fee_token_balance, fee);

    // Check balances in validator stake account list storage
    let validator_stake_list = get_account(
        &mut banks_client,
        &stake_pool_accounts.validator_stake_list.pubkey(),
    )
    .await;
    let validator_stake_list =
        state::ValidatorStakeList::deserialize(validator_stake_list.data.as_slice()).unwrap();
    let validator_stake_item = validator_stake_list
        .find(&validator_stake_account.vote.pubkey())
        .unwrap();
    assert_eq!(
        validator_stake_item.balance,
        validator_stake_item_before.balance + stake_lamports
    );

    // Check validator stake account actual SOL balance
    let validator_stake_account =
        get_account(&mut banks_client, &validator_stake_account.stake_account).await;
    assert_eq!(
        validator_stake_account.lamports,
        validator_stake_item.balance
    );
}

#[tokio::test]
async fn test_stake_pool_deposit_with_wrong_stake_program_id() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, validator_stake_account) =
        setup().await;

    let user = Keypair::new();
    // make stake account
    let user_stake = Keypair::new();

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

    let wrong_stake_program = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::deposit(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.validator_stake_list.pubkey(),
            &stake_pool_accounts.deposit_authority,
            &stake_pool_accounts.withdraw_authority,
            &user_stake.pubkey(),
            &validator_stake_account.stake_account,
            &user_pool_account.pubkey(),
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &spl_token::id(),
            &wrong_stake_program.pubkey(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(_, error)) => {
            assert_eq!(error, InstructionError::IncorrectProgramId);
        }
        _ => panic!("Wrong error occurs while try to make a deposit with wrong stake program ID"),
    }
}

#[tokio::test]
async fn test_stake_pool_deposit_with_wrong_pool_fee_account() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        mut stake_pool_accounts,
        validator_stake_account,
    ) = setup().await;

    let user = Keypair::new();
    // make stake account
    let user_stake = Keypair::new();
    let lockup = stake::Lockup::default();
    let authorized = stake::Authorized {
        staker: stake_pool_accounts.deposit_authority,
        withdrawer: stake_pool_accounts.deposit_authority,
    };
    create_independent_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake,
        &authorized,
        &lockup,
    )
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

    let wrong_pool_fee_acc = Keypair::new();
    stake_pool_accounts.pool_fee_account = wrong_pool_fee_acc;

    let transaction_error = stake_pool_accounts
        .deposit_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.pubkey(),
            &user_pool_account.pubkey(),
            &validator_stake_account.stake_account,
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
        _ => panic!("Wrong error occurs while try to make a deposit with wrong pool fee account"),
    }
}

#[tokio::test]
async fn test_stake_pool_deposit_with_wrong_token_program_id() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, validator_stake_account) =
        setup().await;

    let user = Keypair::new();
    // make stake account
    let user_stake = Keypair::new();
    let lockup = stake::Lockup::default();
    let authorized = stake::Authorized {
        staker: stake_pool_accounts.deposit_authority,
        withdrawer: stake_pool_accounts.deposit_authority,
    };
    create_independent_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake,
        &authorized,
        &lockup,
    )
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

    let wrong_token_program = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::deposit(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.validator_stake_list.pubkey(),
            &stake_pool_accounts.deposit_authority,
            &stake_pool_accounts.withdraw_authority,
            &user_stake.pubkey(),
            &validator_stake_account.stake_account,
            &user_pool_account.pubkey(),
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &wrong_token_program.pubkey(),
            &stake::id(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(_, error)) => {
            assert_eq!(error, InstructionError::IncorrectProgramId);
        }
        _ => panic!("Wrong error occurs while try to make a deposit with wrong token program ID"),
    }
}

#[tokio::test]
async fn test_stake_pool_deposit_with_wrong_validator_stake_list_account() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        mut stake_pool_accounts,
        validator_stake_account,
    ) = setup().await;

    let user = Keypair::new();
    // make stake account
    let user_stake = Keypair::new();
    let lockup = stake::Lockup::default();
    let authorized = stake::Authorized {
        staker: stake_pool_accounts.deposit_authority,
        withdrawer: stake_pool_accounts.deposit_authority,
    };
    create_independent_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake,
        &authorized,
        &lockup,
    )
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

    let wrong_validator_stake_list = Keypair::new();
    stake_pool_accounts.validator_stake_list = wrong_validator_stake_list;

    let transaction_error = stake_pool_accounts
        .deposit_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.pubkey(),
            &user_pool_account.pubkey(),
            &validator_stake_account.stake_account,
        )
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
        _ => panic!("Wrong error occurs while try to make a deposit with wrong validator stake list account"),
    }
}

#[tokio::test]
async fn test_stake_pool_deposit_where_stake_acc_not_in_stake_state() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    let validator_stake_account = ValidatorStakeAccount::new_with_target_authority(
        &stake_pool_accounts.deposit_authority,
        &stake_pool_accounts.stake_pool.pubkey(),
    );

    let user_stake_authority = Keypair::new();
    create_validator_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &validator_stake_account.stake_pool,
        &validator_stake_account.stake_account,
        &validator_stake_account.vote.pubkey(),
        &user_stake_authority.pubkey(),
        &validator_stake_account.target_authority,
    )
    .await;

    let user_pool_account = Keypair::new();
    let user = Keypair::new();
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

    let user_stake_acc = Keypair::new();
    let lockup = stake::Lockup::default();
    let authorized = stake::Authorized {
        staker: stake_pool_accounts.deposit_authority,
        withdrawer: stake_pool_accounts.deposit_authority,
    };
    create_independent_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake_acc,
        &authorized,
        &lockup,
    )
    .await;
    let transaction_error = stake_pool_accounts
        .deposit_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake_acc.pubkey(),
            &user_pool_account.pubkey(),
            &validator_stake_account.stake_account,
        )
        .await
        .err()
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::WrongStakeState as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!(
            "Wrong error occurs while try to make a deposit when stake acc not in stake state"
        ),
    }
}

#[tokio::test]
async fn test_stake_pool_deposit_to_unknown_validator() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    let validator_stake_account = ValidatorStakeAccount::new_with_target_authority(
        &stake_pool_accounts.deposit_authority,
        &stake_pool_accounts.stake_pool.pubkey(),
    );
    validator_stake_account
        .create_and_delegate(&mut banks_client, &payer, &recent_blockhash)
        .await;

    let user_pool_account = Keypair::new();
    let user = Keypair::new();
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

    // make stake account
    let user_stake = Keypair::new();
    let lockup = stake::Lockup::default();
    let authorized = stake::Authorized {
        staker: stake_pool_accounts.deposit_authority,
        withdrawer: stake_pool_accounts.deposit_authority,
    };
    create_independent_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake,
        &authorized,
        &lockup,
    )
    .await;

    let transaction_error = stake_pool_accounts
        .deposit_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.pubkey(),
            &user_pool_account.pubkey(),
            &validator_stake_account.stake_account,
        )
        .await
        .err()
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
            panic!("Wrong error occurs while try to make a deposit with unknown validator account")
        }
    }
}

#[tokio::test]
async fn test_stake_pool_deposit_with_wrong_deposit_authority() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        mut stake_pool_accounts,
        validator_stake_account,
    ) = setup().await;

    let user = Keypair::new();
    // make stake account
    let user_stake = Keypair::new();
    let lockup = stake::Lockup::default();
    let authorized = stake::Authorized {
        staker: stake_pool_accounts.deposit_authority,
        withdrawer: stake_pool_accounts.deposit_authority,
    };
    create_independent_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake,
        &authorized,
        &lockup,
    )
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

    stake_pool_accounts.deposit_authority = Keypair::new().pubkey();

    let transaction_error = stake_pool_accounts
        .deposit_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.pubkey(),
            &user_pool_account.pubkey(),
            &validator_stake_account.stake_account,
        )
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
        _ => panic!("Wrong error occurs while try to make a deposit with wrong deposit authority"),
    }
}

#[tokio::test]
async fn test_stake_pool_deposit_with_wrong_withdraw_authority() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        mut stake_pool_accounts,
        validator_stake_account,
    ) = setup().await;

    let user = Keypair::new();
    // make stake account
    let user_stake = Keypair::new();
    let lockup = stake::Lockup::default();
    let authorized = stake::Authorized {
        staker: stake_pool_accounts.deposit_authority,
        withdrawer: stake_pool_accounts.deposit_authority,
    };
    create_independent_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake,
        &authorized,
        &lockup,
    )
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

    stake_pool_accounts.withdraw_authority = Keypair::new().pubkey();

    let transaction_error = stake_pool_accounts
        .deposit_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.pubkey(),
            &user_pool_account.pubkey(),
            &validator_stake_account.stake_account,
        )
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
        _ => panic!("Wrong error occurs while try to make a deposit with wrong withdraw authority"),
    }
}

#[tokio::test]
async fn test_stake_pool_deposit_with_wrong_set_deposit_authority() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, validator_stake_account) =
        setup().await;

    let user = Keypair::new();
    // make stake account
    let user_stake = Keypair::new();
    let lockup = stake::Lockup::default();
    let authorized = stake::Authorized {
        staker: Keypair::new().pubkey(),
        withdrawer: stake_pool_accounts.deposit_authority,
    };
    create_independent_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake,
        &authorized,
        &lockup,
    )
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

    let transaction_error = stake_pool_accounts
        .deposit_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.pubkey(),
            &user_pool_account.pubkey(),
            &validator_stake_account.stake_account,
        )
        .await
        .err()
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(_, error)) => {
            assert_eq!(error, InstructionError::MissingRequiredSignature);
        }
        _ => {
            panic!("Wrong error occurs while try to make deposit with wrong set deposit authority")
        }
    }
}

#[tokio::test]
async fn test_stake_pool_deposit_with_wrong_mint_for_receiver_acc() {
    let (mut banks_client, payer, recent_blockhash, stake_pool_accounts, validator_stake_account) =
        setup().await;

    // make stake account
    let user_stake = Keypair::new();
    let lockup = stake::Lockup::default();
    let authorized = stake::Authorized {
        staker: stake_pool_accounts.deposit_authority,
        withdrawer: stake_pool_accounts.deposit_authority,
    };
    create_independent_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake,
        &authorized,
        &lockup,
    )
    .await;

    let outside_mint = Keypair::new();
    let outside_withdraw_auth = Keypair::new();
    let outside_owner = Keypair::new();
    let outside_pool_fee_acc = Keypair::new();

    create_mint(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &outside_mint,
        &outside_withdraw_auth.pubkey(),
    )
    .await
    .unwrap();

    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &outside_pool_fee_acc,
        &outside_mint.pubkey(),
        &outside_owner.pubkey(),
    )
    .await
    .unwrap();

    let transaction_error = stake_pool_accounts
        .deposit_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.pubkey(),
            &outside_pool_fee_acc.pubkey(),
            &validator_stake_account.stake_account,
        )
        .await
        .err()
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = token_error::TokenError::MintMismatch as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to deposit with wrong mint fro receiver account"),
    }
}

#[tokio::test]
async fn test_deposit_with_uninitialized_validator_stake_list() {} // TODO

#[tokio::test]
async fn test_deposit_with_out_of_dated_pool_balances() {} // TODO
