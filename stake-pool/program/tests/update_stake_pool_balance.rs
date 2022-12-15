#![allow(clippy::integer_arithmetic)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program::{
        borsh::try_from_slice_unchecked, instruction::InstructionError, pubkey::Pubkey,
    },
    solana_program_test::*,
    solana_sdk::{
        hash::Hash,
        signature::{Keypair, Signer},
        stake,
        transaction::TransactionError,
    },
    spl_stake_pool::{error::StakePoolError, state::StakePool, MINIMUM_RESERVE_LAMPORTS},
    std::num::NonZeroU32,
};

const NUM_VALIDATORS: u64 = 3;

async fn setup(
    num_validators: u64,
) -> (
    ProgramTestContext,
    Hash,
    StakePoolAccounts,
    Vec<ValidatorStakeAccount>,
) {
    let mut context = program_test().start_with_context().await;
    let stake_pool_accounts = StakePoolAccounts::default();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            MINIMUM_RESERVE_LAMPORTS,
        )
        .await
        .unwrap();

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>());
    let current_minimum_delegation = stake_pool_get_minimum_delegation(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;

    let error = stake_pool_accounts
        .deposit_sol(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_pool_accounts.pool_fee_account.pubkey(),
            (stake_rent + current_minimum_delegation) * num_validators,
            None,
        )
        .await;
    assert!(error.is_none());

    let mut last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();

    // Add several accounts
    let mut stake_accounts: Vec<ValidatorStakeAccount> = vec![];
    for i in 0..num_validators {
        let stake_account = ValidatorStakeAccount::new(
            &stake_pool_accounts.stake_pool.pubkey(),
            NonZeroU32::new(i as u32),
            u64::MAX,
        );
        create_vote(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &stake_account.validator,
            &stake_account.vote,
        )
        .await;

        let error = stake_pool_accounts
            .add_validator_to_pool(
                &mut context.banks_client,
                &context.payer,
                &last_blockhash,
                &stake_account.stake_account,
                &stake_account.vote.pubkey(),
                stake_account.validator_stake_seed,
            )
            .await;
        assert!(error.is_none());

        let mut deposit_account = DepositStakeAccount::new_with_vote(
            stake_account.vote.pubkey(),
            stake_account.stake_account,
            TEST_STAKE_AMOUNT,
        );
        deposit_account
            .create_and_delegate(&mut context.banks_client, &context.payer, &last_blockhash)
            .await;

        deposit_account
            .deposit_stake(
                &mut context.banks_client,
                &context.payer,
                &last_blockhash,
                &stake_pool_accounts,
            )
            .await;

        last_blockhash = context
            .banks_client
            .get_new_latest_blockhash(&last_blockhash)
            .await
            .unwrap();

        stake_accounts.push(stake_account);
    }

    (context, last_blockhash, stake_pool_accounts, stake_accounts)
}

#[tokio::test]
async fn success() {
    let (mut context, last_blockhash, stake_pool_accounts, stake_accounts) =
        setup(NUM_VALIDATORS).await;

    let pre_fee = get_token_balance(
        &mut context.banks_client,
        &stake_pool_accounts.pool_fee_account.pubkey(),
    )
    .await;

    let pre_balance = get_validator_list_sum(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(stake_pool.data.as_slice()).unwrap();
    assert_eq!(pre_balance, stake_pool.total_lamports);

    let pre_token_supply = get_token_supply(
        &mut context.banks_client,
        &stake_pool_accounts.pool_mint.pubkey(),
    )
    .await;

    // Increment vote credits to earn rewards
    const VOTE_CREDITS: u64 = 1_000;
    for stake_account in &stake_accounts {
        context.increment_vote_account_credits(&stake_account.vote.pubkey(), VOTE_CREDITS);
    }

    // Update epoch
    let slot = context.genesis_config().epoch_schedule.first_normal_slot;
    context.warp_to_slot(slot).unwrap();

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();

    // Update list and pool
    let error = stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            stake_accounts
                .iter()
                .map(|v| v.vote.pubkey())
                .collect::<Vec<Pubkey>>()
                .as_slice(),
            false,
        )
        .await;
    assert!(error.is_none());

    // Check fee
    let post_balance = get_validator_list_sum(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(stake_pool.data.as_slice()).unwrap();
    assert_eq!(post_balance, stake_pool.total_lamports);

    let post_fee = get_token_balance(
        &mut context.banks_client,
        &stake_pool_accounts.pool_fee_account.pubkey(),
    )
    .await;
    let pool_token_supply = get_token_supply(
        &mut context.banks_client,
        &stake_pool_accounts.pool_mint.pubkey(),
    )
    .await;
    let actual_fee = post_fee - pre_fee;
    assert_eq!(pool_token_supply - pre_token_supply, actual_fee);

    let expected_fee_lamports = (post_balance - pre_balance) * stake_pool.epoch_fee.numerator
        / stake_pool.epoch_fee.denominator;
    let actual_fee_lamports = stake_pool.calc_pool_tokens_for_deposit(actual_fee).unwrap();
    assert_eq!(actual_fee_lamports, expected_fee_lamports);

    let expected_fee = expected_fee_lamports * pool_token_supply / post_balance;
    assert_eq!(expected_fee, actual_fee);

    assert_eq!(pool_token_supply, stake_pool.pool_token_supply);
    assert_eq!(pre_token_supply, stake_pool.last_epoch_pool_token_supply);
    assert_eq!(pre_balance, stake_pool.last_epoch_total_lamports);
}

#[tokio::test]
async fn success_absorbing_extra_lamports() {
    let (mut context, mut last_blockhash, stake_pool_accounts, stake_accounts) =
        setup(NUM_VALIDATORS).await;

    let pre_balance = get_validator_list_sum(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(stake_pool.data.as_slice()).unwrap();
    assert_eq!(pre_balance, stake_pool.total_lamports);

    let pre_token_supply = get_token_supply(
        &mut context.banks_client,
        &stake_pool_accounts.pool_mint.pubkey(),
    )
    .await;

    // Transfer extra funds, will be absorbed during update
    const EXTRA_STAKE_AMOUNT: u64 = 1_000_000;
    for stake_account in &stake_accounts {
        transfer(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &stake_account.stake_account,
            EXTRA_STAKE_AMOUNT,
        )
        .await;

        last_blockhash = context
            .banks_client
            .get_new_latest_blockhash(&last_blockhash)
            .await
            .unwrap();
    }

    let extra_lamports = EXTRA_STAKE_AMOUNT * stake_accounts.len() as u64;
    let expected_fee = stake_pool.calc_epoch_fee_amount(extra_lamports).unwrap();

    // Update epoch
    let slot = context.genesis_config().epoch_schedule.first_normal_slot;
    context.warp_to_slot(slot).unwrap();
    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();

    // Update list and pool
    let error = stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            stake_accounts
                .iter()
                .map(|v| v.vote.pubkey())
                .collect::<Vec<Pubkey>>()
                .as_slice(),
            false,
        )
        .await;
    assert!(error.is_none());

    // Check extra lamports are absorbed and fee'd as rewards
    let post_balance = get_validator_list_sum(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    assert_eq!(post_balance, pre_balance + extra_lamports);
    let pool_token_supply = get_token_supply(
        &mut context.banks_client,
        &stake_pool_accounts.pool_mint.pubkey(),
    )
    .await;
    assert_eq!(pool_token_supply, pre_token_supply + expected_fee);
}

#[tokio::test]
async fn fail_with_wrong_validator_list() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let mut stake_pool_accounts = StakePoolAccounts::default();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            MINIMUM_RESERVE_LAMPORTS,
        )
        .await
        .unwrap();

    let wrong_validator_list = Keypair::new();
    stake_pool_accounts.validator_list = wrong_validator_list;
    let error = stake_pool_accounts
        .update_stake_pool_balance(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        ) => {
            let program_error = StakePoolError::InvalidValidatorStakeList as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to update pool balance with wrong validator stake list account"),
    }
}

#[tokio::test]
async fn fail_with_wrong_pool_fee_account() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let mut stake_pool_accounts = StakePoolAccounts::default();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            MINIMUM_RESERVE_LAMPORTS,
        )
        .await
        .unwrap();

    let wrong_fee_account = Keypair::new();
    stake_pool_accounts.pool_fee_account = wrong_fee_account;
    let error = stake_pool_accounts
        .update_stake_pool_balance(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        ) => {
            let program_error = StakePoolError::InvalidFeeAccount as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to update pool balance with wrong validator stake list account"),
    }
}

#[tokio::test]
async fn fail_with_wrong_reserve() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let mut stake_pool_accounts = StakePoolAccounts::default();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            MINIMUM_RESERVE_LAMPORTS,
        )
        .await
        .unwrap();

    let wrong_reserve_stake = Keypair::new();
    stake_pool_accounts.reserve_stake = wrong_reserve_stake;
    let error = stake_pool_accounts
        .update_stake_pool_balance(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::InvalidProgramAddress as u32),
        )
    );
}

#[tokio::test]
async fn test_update_stake_pool_balance_with_uninitialized_validator_list() {} // TODO

#[tokio::test]
async fn test_update_stake_pool_balance_with_out_of_dated_validators_balances() {} // TODO
