#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program::{borsh1::try_from_slice_unchecked, pubkey::Pubkey, stake},
    solana_program_test::*,
    solana_sdk::{
        native_token::LAMPORTS_PER_SOL,
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    spl_stake_pool::{
        find_stake_program_address, find_transient_stake_program_address, id,
        instruction::{self, PreferredValidatorType},
        state::{StakePool, StakeStatus, ValidatorList},
        MAX_VALIDATORS_TO_UPDATE,
    },
    test_case::test_case,
};

// Note: this is not the real max! The testing framework starts to blow out
// because the test require so many helper accounts.
// 20k is also a very safe number for the current upper bound of the network.
const MAX_POOL_SIZE_WITH_REQUESTED_COMPUTE_UNITS: u32 = 20_000;
const MAX_POOL_SIZE: u32 = 3_000;
const STAKE_AMOUNT: u64 = 200_000_000_000;

async fn setup(
    max_validators: u32,
    num_validators: u32,
    stake_amount: u64,
) -> (
    ProgramTestContext,
    StakePoolAccounts,
    Vec<Pubkey>,
    Pubkey,
    Keypair,
    Pubkey,
    Pubkey,
) {
    let mut program_test = program_test();
    let mut vote_account_pubkeys = vec![];
    let mut stake_pool_accounts = StakePoolAccounts {
        max_validators,
        ..Default::default()
    };
    if max_validators > MAX_POOL_SIZE {
        stake_pool_accounts.compute_unit_limit = Some(1_400_000);
    }

    let stake_pool_pubkey = stake_pool_accounts.stake_pool.pubkey();
    let (mut stake_pool, mut validator_list) = stake_pool_accounts.state();
    stake_pool.last_update_epoch = FIRST_NORMAL_EPOCH;

    for _ in 0..max_validators {
        vote_account_pubkeys.push(add_vote_account(&mut program_test));
    }

    for vote_account_address in vote_account_pubkeys.iter().take(num_validators as usize) {
        add_validator_stake_account(
            &mut program_test,
            &mut stake_pool,
            &mut validator_list,
            &stake_pool_pubkey,
            &stake_pool_accounts.withdraw_authority,
            vote_account_address,
            stake_amount,
            StakeStatus::Active,
        );
    }

    add_reserve_stake_account(
        &mut program_test,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.withdraw_authority,
        stake_amount,
    );
    add_stake_pool_account(
        &mut program_test,
        &stake_pool_accounts.stake_pool.pubkey(),
        &stake_pool,
    );
    add_validator_list_account(
        &mut program_test,
        &stake_pool_accounts.validator_list.pubkey(),
        &validator_list,
        max_validators,
    );

    add_mint_account(
        &mut program_test,
        &stake_pool_accounts.token_program_id,
        &stake_pool_accounts.pool_mint.pubkey(),
        &stake_pool_accounts.withdraw_authority,
        stake_pool.pool_token_supply,
    );
    add_token_account(
        &mut program_test,
        &stake_pool_accounts.token_program_id,
        &stake_pool_accounts.pool_fee_account.pubkey(),
        &stake_pool_accounts.pool_mint.pubkey(),
        &stake_pool_accounts.manager.pubkey(),
    );

    let mut context = program_test.start_with_context().await;
    let epoch_schedule = &context.genesis_config().epoch_schedule;
    let slot = epoch_schedule.first_normal_slot + epoch_schedule.slots_per_epoch + 1;
    context.warp_to_slot(slot).unwrap();

    let vote_pubkey = vote_account_pubkeys[max_validators as usize - 1];
    // make stake account
    let user = Keypair::new();
    let deposit_stake = Keypair::new();
    let lockup = stake::state::Lockup::default();

    let authorized = stake::state::Authorized {
        staker: user.pubkey(),
        withdrawer: user.pubkey(),
    };

    let _stake_lamports = create_independent_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &deposit_stake,
        &authorized,
        &lockup,
        stake_amount,
    )
    .await;

    delegate_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &deposit_stake.pubkey(),
        &user,
        &vote_pubkey,
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
        vote_account_pubkeys,
        vote_pubkey,
        user,
        deposit_stake.pubkey(),
        pool_token_account.pubkey(),
    )
}

#[test_case(MAX_POOL_SIZE_WITH_REQUESTED_COMPUTE_UNITS; "compute-budget")]
#[test_case(MAX_POOL_SIZE; "no-compute-budget")]
#[tokio::test]
async fn update(max_validators: u32) {
    let (mut context, stake_pool_accounts, _, _, _, _, _) =
        setup(max_validators, max_validators, STAKE_AMOUNT).await;

    let error = stake_pool_accounts
        .update_validator_list_balance(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            MAX_VALIDATORS_TO_UPDATE,
            false, /* no_merge */
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let error = stake_pool_accounts
        .update_stake_pool_balance(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let error = stake_pool_accounts
        .cleanup_removed_validator_entries(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);
}

//#[test_case(MAX_POOL_SIZE_WITH_REQUESTED_COMPUTE_UNITS; "compute-budget")]
#[test_case(MAX_POOL_SIZE; "no-compute-budget")]
#[tokio::test]
async fn remove_validator_from_pool(max_validators: u32) {
    let (mut context, stake_pool_accounts, vote_account_pubkeys, _, _, _, _) =
        setup(max_validators, max_validators, LAMPORTS_PER_SOL).await;

    let first_vote = vote_account_pubkeys[0];
    let (stake_address, _) = find_stake_program_address(
        &id(),
        &first_vote,
        &stake_pool_accounts.stake_pool.pubkey(),
        None,
    );
    let transient_stake_seed = u64::MAX;
    let (transient_stake_address, _) = find_transient_stake_program_address(
        &id(),
        &first_vote,
        &stake_pool_accounts.stake_pool.pubkey(),
        transient_stake_seed,
    );

    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_address,
            &transient_stake_address,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let middle_index = max_validators as usize / 2;
    let middle_vote = vote_account_pubkeys[middle_index];
    let (stake_address, _) = find_stake_program_address(
        &id(),
        &middle_vote,
        &stake_pool_accounts.stake_pool.pubkey(),
        None,
    );
    let (transient_stake_address, _) = find_transient_stake_program_address(
        &id(),
        &middle_vote,
        &stake_pool_accounts.stake_pool.pubkey(),
        transient_stake_seed,
    );

    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_address,
            &transient_stake_address,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let last_index = max_validators as usize - 1;
    let last_vote = vote_account_pubkeys[last_index];
    let (stake_address, _) = find_stake_program_address(
        &id(),
        &last_vote,
        &stake_pool_accounts.stake_pool.pubkey(),
        None,
    );
    let (transient_stake_address, _) = find_transient_stake_program_address(
        &id(),
        &last_vote,
        &stake_pool_accounts.stake_pool.pubkey(),
        transient_stake_seed,
    );

    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_address,
            &transient_stake_address,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<ValidatorList>(validator_list.data.as_slice()).unwrap();
    let first_element = &validator_list.validators[0];
    assert_eq!(
        first_element.status,
        StakeStatus::DeactivatingValidator.into()
    );
    assert_eq!(
        u64::from(first_element.active_stake_lamports),
        LAMPORTS_PER_SOL + STAKE_ACCOUNT_RENT_EXEMPTION
    );
    assert_eq!(u64::from(first_element.transient_stake_lamports), 0);

    let middle_element = &validator_list.validators[middle_index];
    assert_eq!(
        middle_element.status,
        StakeStatus::DeactivatingValidator.into()
    );
    assert_eq!(
        u64::from(middle_element.active_stake_lamports),
        LAMPORTS_PER_SOL + STAKE_ACCOUNT_RENT_EXEMPTION
    );
    assert_eq!(u64::from(middle_element.transient_stake_lamports), 0);

    let last_element = &validator_list.validators[last_index];
    assert_eq!(
        last_element.status,
        StakeStatus::DeactivatingValidator.into()
    );
    assert_eq!(
        u64::from(last_element.active_stake_lamports),
        LAMPORTS_PER_SOL + STAKE_ACCOUNT_RENT_EXEMPTION
    );
    assert_eq!(u64::from(last_element.transient_stake_lamports), 0);

    let error = stake_pool_accounts
        .update_validator_list_balance(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            1,
            false, /* no_merge */
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let mut instructions = vec![instruction::update_validator_list_balance_chunk(
        &id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        &stake_pool_accounts.withdraw_authority,
        &stake_pool_accounts.validator_list.pubkey(),
        &stake_pool_accounts.reserve_stake.pubkey(),
        &validator_list,
        1,
        middle_index,
        /* no_merge = */ false,
    )
    .unwrap()];
    stake_pool_accounts.maybe_add_compute_budget_instruction(&mut instructions);
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err();
    assert!(error.is_none(), "{:?}", error);

    let mut instructions = vec![instruction::update_validator_list_balance_chunk(
        &id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        &stake_pool_accounts.withdraw_authority,
        &stake_pool_accounts.validator_list.pubkey(),
        &stake_pool_accounts.reserve_stake.pubkey(),
        &validator_list,
        1,
        last_index,
        /* no_merge = */ false,
    )
    .unwrap()];
    stake_pool_accounts.maybe_add_compute_budget_instruction(&mut instructions);
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err();
    assert!(error.is_none(), "{:?}", error);

    let error = stake_pool_accounts
        .cleanup_removed_validator_entries(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<ValidatorList>(validator_list.data.as_slice()).unwrap();
    assert_eq!(validator_list.validators.len() as u32, max_validators - 3);
    // assert they're gone
    assert!(!validator_list
        .validators
        .iter()
        .any(|x| x.vote_account_address == first_vote));
    assert!(!validator_list
        .validators
        .iter()
        .any(|x| x.vote_account_address == middle_vote));
    assert!(!validator_list
        .validators
        .iter()
        .any(|x| x.vote_account_address == last_vote));

    // but that we didn't remove too many
    assert!(validator_list
        .validators
        .iter()
        .any(|x| x.vote_account_address == vote_account_pubkeys[1]));
    assert!(validator_list
        .validators
        .iter()
        .any(|x| x.vote_account_address == vote_account_pubkeys[middle_index - 1]));
    assert!(validator_list
        .validators
        .iter()
        .any(|x| x.vote_account_address == vote_account_pubkeys[middle_index + 1]));
    assert!(validator_list
        .validators
        .iter()
        .any(|x| x.vote_account_address == vote_account_pubkeys[last_index - 1]));
}

//#[test_case(MAX_POOL_SIZE_WITH_REQUESTED_COMPUTE_UNITS; "compute-budget")]
#[test_case(MAX_POOL_SIZE; "no-compute-budget")]
#[tokio::test]
async fn add_validator_to_pool(max_validators: u32) {
    let (mut context, stake_pool_accounts, _, test_vote_address, _, _, _) =
        setup(max_validators, max_validators - 1, STAKE_AMOUNT).await;

    let last_index = max_validators as usize - 1;
    let stake_pool_pubkey = stake_pool_accounts.stake_pool.pubkey();
    let (stake_address, _) =
        find_stake_program_address(&id(), &test_vote_address, &stake_pool_pubkey, None);

    let error = stake_pool_accounts
        .add_validator_to_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_address,
            &test_vote_address,
            None,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<ValidatorList>(validator_list.data.as_slice()).unwrap();
    assert_eq!(validator_list.validators.len(), last_index + 1);
    let last_element = validator_list.validators[last_index];
    assert_eq!(last_element.status, StakeStatus::Active.into());
    assert_eq!(
        u64::from(last_element.active_stake_lamports),
        LAMPORTS_PER_SOL + STAKE_ACCOUNT_RENT_EXEMPTION
    );
    assert_eq!(u64::from(last_element.transient_stake_lamports), 0);
    assert_eq!(last_element.vote_account_address, test_vote_address);

    let transient_stake_seed = u64::MAX;
    let (transient_stake_address, _) = find_transient_stake_program_address(
        &id(),
        &test_vote_address,
        &stake_pool_pubkey,
        transient_stake_seed,
    );
    let increase_amount = LAMPORTS_PER_SOL;
    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &transient_stake_address,
            &stake_address,
            &test_vote_address,
            increase_amount,
            transient_stake_seed,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<ValidatorList>(validator_list.data.as_slice()).unwrap();
    let last_element = validator_list.validators[last_index];
    assert_eq!(last_element.status, StakeStatus::Active.into());
    assert_eq!(
        u64::from(last_element.active_stake_lamports),
        LAMPORTS_PER_SOL + STAKE_ACCOUNT_RENT_EXEMPTION
    );
    assert_eq!(
        u64::from(last_element.transient_stake_lamports),
        increase_amount + STAKE_ACCOUNT_RENT_EXEMPTION
    );
    assert_eq!(last_element.vote_account_address, test_vote_address);
}

//#[test_case(MAX_POOL_SIZE_WITH_REQUESTED_COMPUTE_UNITS; "compute-budget")]
#[test_case(MAX_POOL_SIZE; "no-compute-budget")]
#[tokio::test]
async fn set_preferred(max_validators: u32) {
    let (mut context, stake_pool_accounts, _, vote_account_address, _, _, _) =
        setup(max_validators, max_validators, STAKE_AMOUNT).await;

    let error = stake_pool_accounts
        .set_preferred_validator(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            PreferredValidatorType::Deposit,
            Some(vote_account_address),
        )
        .await;
    assert!(error.is_none(), "{:?}", error);
    let error = stake_pool_accounts
        .set_preferred_validator(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            PreferredValidatorType::Withdraw,
            Some(vote_account_address),
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(stake_pool.data.as_slice()).unwrap();

    assert_eq!(
        stake_pool.preferred_deposit_validator_vote_address,
        Some(vote_account_address)
    );
    assert_eq!(
        stake_pool.preferred_withdraw_validator_vote_address,
        Some(vote_account_address)
    );
}

#[test_case(MAX_POOL_SIZE_WITH_REQUESTED_COMPUTE_UNITS; "compute-budget")]
#[test_case(MAX_POOL_SIZE; "no-compute-budget")]
#[tokio::test]
async fn deposit_stake(max_validators: u32) {
    let (mut context, stake_pool_accounts, _, vote_pubkey, user, stake_pubkey, pool_account_pubkey) =
        setup(max_validators, max_validators, STAKE_AMOUNT).await;

    let (stake_address, _) = find_stake_program_address(
        &id(),
        &vote_pubkey,
        &stake_pool_accounts.stake_pool.pubkey(),
        None,
    );

    let error = stake_pool_accounts
        .deposit_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_pubkey,
            &pool_account_pubkey,
            &stake_address,
            &user,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);
}

#[test_case(MAX_POOL_SIZE_WITH_REQUESTED_COMPUTE_UNITS; "compute-budget")]
#[test_case(MAX_POOL_SIZE; "no-compute-budget")]
#[tokio::test]
async fn withdraw(max_validators: u32) {
    let (mut context, stake_pool_accounts, _, vote_pubkey, user, stake_pubkey, pool_account_pubkey) =
        setup(max_validators, max_validators, STAKE_AMOUNT).await;

    let (stake_address, _) = find_stake_program_address(
        &id(),
        &vote_pubkey,
        &stake_pool_accounts.stake_pool.pubkey(),
        None,
    );

    let error = stake_pool_accounts
        .deposit_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_pubkey,
            &pool_account_pubkey,
            &stake_address,
            &user,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // Create stake account to withdraw to
    let user_stake_recipient = Keypair::new();
    create_blank_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &user_stake_recipient,
    )
    .await;

    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &user_stake_recipient.pubkey(),
            &user,
            &pool_account_pubkey,
            &stake_address,
            &user.pubkey(),
            TEST_STAKE_AMOUNT,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);
}

//#[test_case(MAX_POOL_SIZE_WITH_REQUESTED_COMPUTE_UNITS; "compute-budget")]
#[test_case(MAX_POOL_SIZE; "no-compute-budget")]
#[tokio::test]
async fn cleanup_all(max_validators: u32) {
    let mut program_test = program_test();
    let mut vote_account_pubkeys = vec![];
    let mut stake_pool_accounts = StakePoolAccounts {
        max_validators,
        ..Default::default()
    };
    if max_validators > MAX_POOL_SIZE {
        stake_pool_accounts.compute_unit_limit = Some(1_400_000);
    }

    let stake_pool_pubkey = stake_pool_accounts.stake_pool.pubkey();
    let (mut stake_pool, mut validator_list) = stake_pool_accounts.state();

    for _ in 0..max_validators {
        vote_account_pubkeys.push(add_vote_account(&mut program_test));
    }

    for vote_account_address in vote_account_pubkeys.iter() {
        add_validator_stake_account(
            &mut program_test,
            &mut stake_pool,
            &mut validator_list,
            &stake_pool_pubkey,
            &stake_pool_accounts.withdraw_authority,
            vote_account_address,
            STAKE_AMOUNT,
            StakeStatus::ReadyForRemoval,
        );
    }

    add_stake_pool_account(
        &mut program_test,
        &stake_pool_accounts.stake_pool.pubkey(),
        &stake_pool,
    );
    add_validator_list_account(
        &mut program_test,
        &stake_pool_accounts.validator_list.pubkey(),
        &validator_list,
        max_validators,
    );
    let mut context = program_test.start_with_context().await;

    let error = stake_pool_accounts
        .cleanup_removed_validator_entries(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);
}
