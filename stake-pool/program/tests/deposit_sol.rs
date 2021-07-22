#![cfg(feature = "test-bpf")]

mod helpers;

use {
    bincode::deserialize,
    borsh::BorshSerialize,
    helpers::*,
    solana_program::{
        borsh::try_from_slice_unchecked,
        instruction::{AccountMeta, Instruction, InstructionError},
        pubkey::Pubkey,
        sysvar,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
        transaction::TransactionError,
        transport::TransportError,
    },
    spl_stake_pool::{error, id, instruction, minimum_stake_lamports, stake_program, state},
    spl_token::error as token_error,
};

async fn setup() -> (
    ProgramTestContext,
    StakePoolAccounts,
    ValidatorStakeAccount,
    Keypair,
    Pubkey,
    Pubkey,
    u64,
) {
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

    let validator_stake_account = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    let mut slot = first_normal_slot;
    context.warp_to_slot(slot).unwrap();

    let user = Keypair::new();
    // make stake account
    let deposit_stake = Keypair::new();
    let lockup = stake_program::Lockup::default();

    let authorized = stake_program::Authorized {
        staker: user.pubkey(),
        withdrawer: user.pubkey(),
    };

    let stake_lamports = create_independent_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &deposit_stake,
        &authorized,
        &lockup,
        TEST_STAKE_AMOUNT,
    )
    .await;

    delegate_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &deposit_stake.pubkey(),
        &user,
        &validator_stake_account.vote.pubkey(),
    )
    .await;

    slot += 2 * slots_per_epoch;
    context.warp_to_slot(slot).unwrap();
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &[validator_stake_account.vote.pubkey()],
            false,
        )
        .await;

    // make pool token account
    let pool_token_account = Keypair::new();
    create_token_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &pool_token_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &user.pubkey(),
    )
    .await
    .unwrap();

    (
        context,
        stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake.pubkey(),
        pool_token_account.pubkey(),
        stake_lamports,
    )
}

#[tokio::test]
async fn success() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        pool_token_account,
        stake_lamports,
    ) = setup().await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake_program::StakeState>());

    // Save stake pool state before depositing
    let pre_stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let pre_stake_pool =
        try_from_slice_unchecked::<state::StakePool>(&pre_stake_pool.data.as_slice()).unwrap();

    // Save reserve state before depositing
    let pre_reserve_lamports = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await
    .lamports;

    let error = stake_pool_accounts
        .deposit_sol(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &pool_token_account,
            TEST_STAKE_AMOUNT
        )
        .await;
    assert!(error.is_none());

    let tokens_issued = TEST_STAKE_AMOUNT; // For now tokens are 1:1 to stake

    // Stake pool should add its balance to the pool balance
    let post_stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let post_stake_pool =
        try_from_slice_unchecked::<state::StakePool>(&post_stake_pool.data.as_slice()).unwrap();
    assert_eq!(
        post_stake_pool.total_stake_lamports,
        pre_stake_pool.total_stake_lamports + TEST_STAKE_AMOUNT
    );
    assert_eq!(
        post_stake_pool.pool_token_supply,
        pre_stake_pool.pool_token_supply + tokens_issued
    );

    // Check minted tokens
    let user_token_balance =
        get_token_balance(&mut context.banks_client, &pool_token_account).await;
    let tokens_issued_user =
        tokens_issued - stake_pool_accounts.calculate_deposit_fee(tokens_issued);
    assert_eq!(user_token_balance, tokens_issued_user);

    // Check reserve
    let post_reserve_lamports = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await
    .lamports;
    assert_eq!(post_reserve_lamports, pre_reserve_lamports + TEST_STAKE_AMOUNT);
}

#[tokio::test]
async fn fail_with_wrong_token_program_id() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        pool_token_account,
        _stake_lamports,
    ) = setup().await;

    let wrong_token_program = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &instruction::deposit_sol(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &stake_pool_accounts.reserve_stake.pubkey(),
            &context.payer.pubkey(),
            &pool_token_account,
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &wrong_token_program.pubkey(),
            10
        ),
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer], context.last_blockhash);
    let transaction_error = context
        .banks_client
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
async fn fail_with_wrong_withdraw_authority() {
    let (
        mut context,
        mut stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        pool_token_account,
        _stake_lamports,
    ) = setup().await;

    stake_pool_accounts.withdraw_authority = Pubkey::new_unique();

    let transaction_error = stake_pool_accounts
    .deposit_sol(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &pool_token_account,
        TEST_STAKE_AMOUNT
    )
        .await
        .unwrap()
        .unwrap();

    match transaction_error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = error::StakePoolError::InvalidProgramAddress as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to make a deposit with wrong withdraw authority"),
    }
}

#[tokio::test]
async fn fail_with_wrong_mint_for_receiver_acc() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        _pool_token_account,
        _stake_lamports,
    ) = setup().await;

    let outside_mint = Keypair::new();
    let outside_withdraw_auth = Keypair::new();
    let outside_manager = Keypair::new();
    let outside_pool_fee_acc = Keypair::new();

    create_mint(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &outside_mint,
        &outside_withdraw_auth.pubkey(),
    )
    .await
    .unwrap();

    create_token_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &outside_pool_fee_acc,
        &outside_mint.pubkey(),
        &outside_manager.pubkey(),
    )
    .await
    .unwrap();

    let transaction_error = stake_pool_accounts
    .deposit_sol(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &outside_pool_fee_acc.pubkey(),
        TEST_STAKE_AMOUNT
    )
        .await
        .unwrap()
        .unwrap();

    match transaction_error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = token_error::TokenError::MintMismatch as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to deposit with wrong mint fro receiver account"),
    }
}

#[tokio::test]
async fn success_with_deposit_authority() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let deposit_authority = Keypair::new();
    let stake_pool_accounts = StakePoolAccounts::new_with_deposit_authority(deposit_authority);
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let user = Keypair::new();
    let authorized = stake_program::Authorized {
        staker: user.pubkey(),
        withdrawer: user.pubkey(),
    };

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
        .deposit_sol(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_pool_account.pubkey(),
            TEST_STAKE_AMOUNT
        )
        .await;
    assert!(error.is_none());
}

#[tokio::test]
async fn fail_without_deposit_authority_signature() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let deposit_authority = Keypair::new();
    let mut stake_pool_accounts = StakePoolAccounts::new_with_deposit_authority(deposit_authority);
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

    let user = Keypair::new();

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

    let wrong_depositor = Keypair::new();
    stake_pool_accounts.deposit_authority = wrong_depositor.pubkey();
    stake_pool_accounts.deposit_authority_keypair = Some(wrong_depositor);

    let error = stake_pool_accounts
        .deposit_sol(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_pool_account.pubkey(),
            TEST_STAKE_AMOUNT
        )
        .await
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            assert_eq!(
                error_index,
                error::StakePoolError::InvalidProgramAddress as u32
            );
        }
        _ => panic!("Wrong error occurs while try to make a deposit with wrong stake program ID"),
    }
}
#[tokio::test]
async fn fail_with_invalid_referrer() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        pool_token_account,
        _stake_lamports,
    ) = setup().await;

    let invalid_token_account = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &instruction::deposit_sol(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &stake_pool_accounts.reserve_stake.pubkey(),
            &context.payer.pubkey(),
            &pool_token_account,
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &invalid_token_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &spl_token::id(),
            10
        ),
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer], context.last_blockhash);
    let transaction_error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match transaction_error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            assert_eq!(error_index, error::StakePoolError::InvalidFeeAccount as u32);
        }
        _ => panic!(
            "Wrong error occurs while try to make a deposit with an invalid referrer account"
        ),
    }
}
