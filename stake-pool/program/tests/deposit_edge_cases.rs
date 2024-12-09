#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program::{
        borsh1::try_from_slice_unchecked, instruction::InstructionError, pubkey::Pubkey, stake,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
    },
    spl_stake_pool::{error::StakePoolError, id, instruction, state, MINIMUM_RESERVE_LAMPORTS},
};

async fn setup(
    token_program_id: Pubkey,
) -> (
    ProgramTestContext,
    StakePoolAccounts,
    ValidatorStakeAccount,
    Keypair,
    Pubkey,
    Pubkey,
    u64,
) {
    let mut context = program_test().start_with_context().await;

    let stake_pool_accounts = StakePoolAccounts::new_with_token_program(token_program_id);
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            MINIMUM_RESERVE_LAMPORTS,
        )
        .await
        .unwrap();

    let validator_stake_account = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
        None,
    )
    .await;

    let user = Keypair::new();
    // make stake account
    let deposit_stake = Keypair::new();
    let lockup = stake::state::Lockup::default();

    let authorized = stake::state::Authorized {
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

    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    context.warp_to_slot(first_normal_slot + 1).unwrap();
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            false,
        )
        .await;

    // make pool token account
    let pool_token_account = Keypair::new();
    create_token_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts.token_program_id,
        &pool_token_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &user,
        &[],
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
async fn success_with_preferred_deposit() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake,
        user,
        deposit_stake,
        pool_token_account,
        _stake_lamports,
    ) = setup(spl_token::id()).await;

    stake_pool_accounts
        .set_preferred_validator(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            instruction::PreferredValidatorType::Deposit,
            Some(validator_stake.vote.pubkey()),
        )
        .await;

    let error = stake_pool_accounts
        .deposit_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &deposit_stake,
            &pool_token_account,
            &validator_stake.stake_account,
            &user,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);
}

#[tokio::test]
async fn fail_with_wrong_preferred_deposit() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake,
        user,
        deposit_stake,
        pool_token_account,
        _stake_lamports,
    ) = setup(spl_token::id()).await;

    let preferred_validator = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
        None,
    )
    .await;

    stake_pool_accounts
        .set_preferred_validator(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            instruction::PreferredValidatorType::Deposit,
            Some(preferred_validator.vote.pubkey()),
        )
        .await;

    let error = stake_pool_accounts
        .deposit_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &deposit_stake,
            &pool_token_account,
            &validator_stake.stake_account,
            &user,
        )
        .await
        .unwrap()
        .unwrap();
    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            assert_eq!(
                error_index,
                StakePoolError::IncorrectDepositVoteAddress as u32
            );
        }
        _ => panic!("Wrong error occurs while try to make a deposit with wrong stake program ID"),
    }
}

#[tokio::test]
async fn success_with_referral_fee() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        pool_token_account,
        stake_lamports,
    ) = setup(spl_token::id()).await;

    let referrer = Keypair::new();
    let referrer_token_account = Keypair::new();
    create_token_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts.token_program_id,
        &referrer_token_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &referrer,
        &[],
    )
    .await
    .unwrap();

    let referrer_balance_pre =
        get_token_balance(&mut context.banks_client, &referrer_token_account.pubkey()).await;

    let mut transaction = Transaction::new_with_payer(
        &instruction::deposit_stake(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.validator_list.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &deposit_stake,
            &user.pubkey(),
            &validator_stake_account.stake_account,
            &stake_pool_accounts.reserve_stake.pubkey(),
            &pool_token_account,
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &referrer_token_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &spl_token::id(),
        ),
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer, &user], context.last_blockhash);
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let referrer_balance_post =
        get_token_balance(&mut context.banks_client, &referrer_token_account.pubkey()).await;
    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool =
        try_from_slice_unchecked::<state::StakePool>(stake_pool.data.as_slice()).unwrap();
    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());
    let fee_tokens = stake_pool
        .calc_pool_tokens_sol_deposit_fee(stake_rent)
        .unwrap()
        + stake_pool
            .calc_pool_tokens_stake_deposit_fee(stake_lamports - stake_rent)
            .unwrap();
    let referral_fee = stake_pool_accounts.calculate_referral_fee(fee_tokens);
    assert!(referral_fee > 0);
    assert_eq!(referrer_balance_pre + referral_fee, referrer_balance_post);
}

#[tokio::test]
async fn fail_with_invalid_referrer() {
    let (
        context,
        stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        pool_token_account,
        _stake_lamports,
    ) = setup(spl_token::id()).await;

    let invalid_token_account = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &instruction::deposit_stake(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.validator_list.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &deposit_stake,
            &user.pubkey(),
            &validator_stake_account.stake_account,
            &stake_pool_accounts.reserve_stake.pubkey(),
            &pool_token_account,
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &invalid_token_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &spl_token::id(),
        ),
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer, &user], context.last_blockhash);
    let transaction_error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match transaction_error {
        TransactionError::InstructionError(_, InstructionError::InvalidAccountData) => (),
        _ => panic!(
            "Wrong error occurs while try to make a deposit with an invalid referrer account"
        ),
    }
}
