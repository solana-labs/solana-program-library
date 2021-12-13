#![cfg(feature = "test-bpf")]

mod helpers;

use {
    helpers::*,
    solana_program::{
        borsh::try_from_slice_unchecked, instruction::InstructionError, pubkey::Pubkey, stake,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
        transaction::TransactionError,
    },
    spl_stake_pool::{
        error::StakePoolError,
        id,
        instruction::{self, FundingType},
        state,
    },
};

async fn setup() -> (ProgramTestContext, StakePoolAccounts, Keypair, Pubkey, u64) {
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

    let user = Keypair::new();

    // make pool token account for user
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

    let error = stake_pool_accounts
        .deposit_sol(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &pool_token_account.pubkey(),
            TEST_STAKE_AMOUNT,
            None,
        )
        .await;
    assert!(error.is_none());

    let tokens_issued =
        get_token_balance(&mut context.banks_client, &pool_token_account.pubkey()).await;

    (
        context,
        stake_pool_accounts,
        user,
        pool_token_account.pubkey(),
        tokens_issued,
    )
}

#[tokio::test]
async fn success() {
    let (mut context, stake_pool_accounts, user, pool_token_account, pool_tokens) = setup().await;

    // Save stake pool state before withdrawing
    let pre_stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let pre_stake_pool =
        try_from_slice_unchecked::<state::StakePool>(pre_stake_pool.data.as_slice()).unwrap();

    // Save reserve state before withdrawing
    let pre_reserve_lamports = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await
    .lamports;

    let error = stake_pool_accounts
        .withdraw_sol(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &user,
            &pool_token_account,
            pool_tokens,
            None,
        )
        .await;
    assert!(error.is_none());

    // Stake pool should add its balance to the pool balance
    let post_stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let post_stake_pool =
        try_from_slice_unchecked::<state::StakePool>(post_stake_pool.data.as_slice()).unwrap();
    let amount_withdrawn_minus_fee =
        pool_tokens - stake_pool_accounts.calculate_withdrawal_fee(pool_tokens);
    assert_eq!(
        post_stake_pool.total_lamports,
        pre_stake_pool.total_lamports - amount_withdrawn_minus_fee
    );
    assert_eq!(
        post_stake_pool.pool_token_supply,
        pre_stake_pool.pool_token_supply - amount_withdrawn_minus_fee
    );

    // Check minted tokens
    let user_token_balance =
        get_token_balance(&mut context.banks_client, &pool_token_account).await;
    assert_eq!(user_token_balance, 0);

    // Check reserve
    let post_reserve_lamports = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await
    .lamports;
    assert_eq!(
        post_reserve_lamports,
        pre_reserve_lamports - amount_withdrawn_minus_fee
    );
}

#[tokio::test]
async fn fail_with_wrong_withdraw_authority() {
    let (mut context, mut stake_pool_accounts, user, pool_token_account, pool_tokens) =
        setup().await;

    stake_pool_accounts.withdraw_authority = Pubkey::new_unique();

    let error = stake_pool_accounts
        .withdraw_sol(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &user,
            &pool_token_account,
            pool_tokens,
            None,
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::InvalidProgramAddress as u32)
        )
    );
}

#[tokio::test]
async fn fail_overdraw_reserve() {
    let (mut context, stake_pool_accounts, user, pool_token_account, _) = setup().await;

    // add a validator and increase stake to drain the reserve
    let validator_stake = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>());
    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.vote.pubkey(),
            TEST_STAKE_AMOUNT - stake_rent,
            validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none());

    // try to withdraw one lamport, will overdraw
    let error = stake_pool_accounts
        .withdraw_sol(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &user,
            &pool_token_account,
            1,
            None,
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::SolWithdrawalTooLarge as u32)
        )
    );
}

#[tokio::test]
async fn success_with_sol_withdraw_authority() {
    let (mut context, stake_pool_accounts, user, pool_token_account, pool_tokens) = setup().await;
    let sol_withdraw_authority = Keypair::new();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_funding_authority(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            Some(&sol_withdraw_authority.pubkey()),
            FundingType::SolWithdraw,
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

    let error = stake_pool_accounts
        .withdraw_sol(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &user,
            &pool_token_account,
            pool_tokens,
            Some(&sol_withdraw_authority),
        )
        .await;
    assert!(error.is_none());
}

#[tokio::test]
async fn fail_without_sol_withdraw_authority_signature() {
    let (mut context, stake_pool_accounts, user, pool_token_account, pool_tokens) = setup().await;
    let sol_withdraw_authority = Keypair::new();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::set_funding_authority(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            Some(&sol_withdraw_authority.pubkey()),
            FundingType::SolWithdraw,
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

    let wrong_withdrawer = Keypair::new();
    let error = stake_pool_accounts
        .withdraw_sol(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &user,
            &pool_token_account,
            pool_tokens,
            Some(&wrong_withdrawer),
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::InvalidSolWithdrawAuthority as u32)
        )
    );
}
