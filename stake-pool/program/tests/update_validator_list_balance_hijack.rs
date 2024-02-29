#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

use spl_stake_pool::instruction;

mod helpers;

use {
    helpers::*,
    solana_program::{borsh1::try_from_slice_unchecked, pubkey::Pubkey, stake},
    solana_program_test::*,
    solana_sdk::{
        hash::Hash,
        instruction::InstructionError,
        signature::Signer,
        stake::state::{Authorized, Lockup, StakeStateV2},
        system_instruction,
        transaction::{Transaction, TransactionError},
    },
    spl_stake_pool::{
        error::StakePoolError, find_stake_program_address, find_transient_stake_program_address,
        find_withdraw_authority_program_address, id, state::StakePool, MINIMUM_RESERVE_LAMPORTS,
    },
    std::num::NonZeroU32,
};

async fn setup(
    num_validators: usize,
) -> (
    ProgramTestContext,
    Hash,
    StakePoolAccounts,
    Vec<ValidatorStakeAccount>,
    Vec<DepositStakeAccount>,
    u64,
    u64,
    u64,
) {
    let mut context = program_test().start_with_context().await;
    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    let mut slot = first_normal_slot + 1;
    context.warp_to_slot(slot).unwrap();

    let reserve_stake_amount = TEST_STAKE_AMOUNT * 2 * num_validators as u64;
    let stake_pool_accounts = StakePoolAccounts::default();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            reserve_stake_amount + MINIMUM_RESERVE_LAMPORTS,
        )
        .await
        .unwrap();

    // Add several accounts with some stake
    let mut stake_accounts: Vec<ValidatorStakeAccount> = vec![];
    let mut deposit_accounts: Vec<DepositStakeAccount> = vec![];
    for i in 0..num_validators {
        let stake_account = ValidatorStakeAccount::new(
            &stake_pool_accounts.stake_pool.pubkey(),
            NonZeroU32::new(i as u32),
            u64::MAX,
        );
        create_vote(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_account.validator,
            &stake_account.vote,
        )
        .await;

        let error = stake_pool_accounts
            .add_validator_to_pool(
                &mut context.banks_client,
                &context.payer,
                &context.last_blockhash,
                &stake_account.stake_account,
                &stake_account.vote.pubkey(),
                stake_account.validator_stake_seed,
            )
            .await;
        assert!(error.is_none(), "{:?}", error);

        let deposit_account = DepositStakeAccount::new_with_vote(
            stake_account.vote.pubkey(),
            stake_account.stake_account,
            TEST_STAKE_AMOUNT,
        );
        deposit_account
            .create_and_delegate(
                &mut context.banks_client,
                &context.payer,
                &context.last_blockhash,
            )
            .await;

        stake_accounts.push(stake_account);
        deposit_accounts.push(deposit_account);
    }

    // Warp forward so the stakes properly activate, and deposit
    slot += slots_per_epoch;
    context.warp_to_slot(slot).unwrap();

    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            false,
        )
        .await;

    for deposit_account in &mut deposit_accounts {
        deposit_account
            .deposit_stake(
                &mut context.banks_client,
                &context.payer,
                &context.last_blockhash,
                &stake_pool_accounts,
            )
            .await;
    }

    slot += slots_per_epoch;
    context.warp_to_slot(slot).unwrap();

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();

    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            false,
        )
        .await;

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();

    (
        context,
        last_blockhash,
        stake_pool_accounts,
        stake_accounts,
        deposit_accounts,
        TEST_STAKE_AMOUNT,
        reserve_stake_amount,
        slot,
    )
}

#[tokio::test]
async fn success_ignoring_hijacked_transient_stake_with_authorized() {
    let hijacker = Pubkey::new_unique();
    check_ignored_hijacked_transient_stake(Some(&Authorized::auto(&hijacker)), None).await;
}

#[tokio::test]
async fn success_ignoring_hijacked_transient_stake_with_lockup() {
    let hijacker = Pubkey::new_unique();
    check_ignored_hijacked_transient_stake(
        None,
        Some(&Lockup {
            custodian: hijacker,
            ..Lockup::default()
        }),
    )
    .await;
}

async fn check_ignored_hijacked_transient_stake(
    hijack_authorized: Option<&Authorized>,
    hijack_lockup: Option<&Lockup>,
) {
    let num_validators = 1;
    let (
        mut context,
        last_blockhash,
        stake_pool_accounts,
        stake_accounts,
        _,
        lamports,
        _,
        mut slot,
    ) = setup(num_validators).await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<StakeStateV2>());

    let pre_lamports = get_validator_list_sum(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let (withdraw_authority, _) =
        find_withdraw_authority_program_address(&id(), &stake_pool_accounts.stake_pool.pubkey());

    println!("Decrease from all validators");
    let stake_account = &stake_accounts[0];
    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &stake_account.stake_account,
            &stake_account.transient_stake_account,
            lamports,
            stake_account.transient_stake_seed,
            DecreaseInstruction::Reserve,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    println!("Warp one epoch so the stakes deactivate and merge");
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    slot += slots_per_epoch;
    context.warp_to_slot(slot).unwrap();

    println!("During update, hijack the transient stake account");
    let validator_list = stake_pool_accounts
        .get_validator_list(&mut context.banks_client)
        .await;
    let transient_stake_address = find_transient_stake_program_address(
        &id(),
        &stake_account.vote.pubkey(),
        &stake_pool_accounts.stake_pool.pubkey(),
        stake_account.transient_stake_seed,
    )
    .0;
    let transaction = Transaction::new_signed_with_payer(
        &[
            instruction::update_validator_list_balance_chunk(
                &id(),
                &stake_pool_accounts.stake_pool.pubkey(),
                &stake_pool_accounts.withdraw_authority,
                &stake_pool_accounts.validator_list.pubkey(),
                &stake_pool_accounts.reserve_stake.pubkey(),
                &validator_list,
                1,
                0,
                /* no_merge = */ false,
            )
            .unwrap(),
            system_instruction::transfer(
                &context.payer.pubkey(),
                &transient_stake_address,
                stake_rent + MINIMUM_RESERVE_LAMPORTS,
            ),
            stake::instruction::initialize(
                &transient_stake_address,
                hijack_authorized.unwrap_or(&Authorized::auto(&withdraw_authority)),
                hijack_lockup.unwrap_or(&Lockup::default()),
            ),
            instruction::update_stake_pool_balance(
                &id(),
                &stake_pool_accounts.stake_pool.pubkey(),
                &stake_pool_accounts.withdraw_authority,
                &stake_pool_accounts.validator_list.pubkey(),
                &stake_pool_accounts.reserve_stake.pubkey(),
                &stake_pool_accounts.pool_fee_account.pubkey(),
                &stake_pool_accounts.pool_mint.pubkey(),
                &spl_token::id(),
            ),
            instruction::cleanup_removed_validator_entries(
                &id(),
                &stake_pool_accounts.stake_pool.pubkey(),
                &stake_pool_accounts.validator_list.pubkey(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err();
    assert!(error.is_none(), "{:?}", error);

    println!("Update again normally, should be no change in the lamports");
    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            false,
        )
        .await;

    let expected_lamports = get_validator_list_sum(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    assert_eq!(pre_lamports, expected_lamports);

    let stake_pool_info = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(&stake_pool_info.data).unwrap();
    assert_eq!(pre_lamports, stake_pool.total_lamports);
}

#[tokio::test]
async fn success_ignoring_hijacked_validator_stake_with_authorized() {
    let hijacker = Pubkey::new_unique();
    check_ignored_hijacked_transient_stake(Some(&Authorized::auto(&hijacker)), None).await;
}

#[tokio::test]
async fn success_ignoring_hijacked_validator_stake_with_lockup() {
    let hijacker = Pubkey::new_unique();
    check_ignored_hijacked_validator_stake(
        None,
        Some(&Lockup {
            custodian: hijacker,
            ..Lockup::default()
        }),
    )
    .await;
}

async fn check_ignored_hijacked_validator_stake(
    hijack_authorized: Option<&Authorized>,
    hijack_lockup: Option<&Lockup>,
) {
    let num_validators = 1;
    let (
        mut context,
        last_blockhash,
        stake_pool_accounts,
        stake_accounts,
        _,
        lamports,
        _,
        mut slot,
    ) = setup(num_validators).await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<StakeStateV2>());

    let pre_lamports = get_validator_list_sum(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let (withdraw_authority, _) =
        find_withdraw_authority_program_address(&id(), &stake_pool_accounts.stake_pool.pubkey());

    let stake_account = &stake_accounts[0];
    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &stake_account.stake_account,
            &stake_account.transient_stake_account,
            lamports,
            stake_account.transient_stake_seed,
            DecreaseInstruction::Reserve,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &stake_account.stake_account,
            &stake_account.transient_stake_account,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    println!("Warp one epoch so the stakes deactivate and merge");
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    slot += slots_per_epoch;
    context.warp_to_slot(slot).unwrap();

    println!("During update, hijack the validator stake account");
    let validator_list = stake_pool_accounts
        .get_validator_list(&mut context.banks_client)
        .await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            instruction::update_validator_list_balance_chunk(
                &id(),
                &stake_pool_accounts.stake_pool.pubkey(),
                &stake_pool_accounts.withdraw_authority,
                &stake_pool_accounts.validator_list.pubkey(),
                &stake_pool_accounts.reserve_stake.pubkey(),
                &validator_list,
                1,
                0,
                /* no_merge = */ false,
            )
            .unwrap(),
            system_instruction::transfer(
                &context.payer.pubkey(),
                &stake_account.stake_account,
                stake_rent + MINIMUM_RESERVE_LAMPORTS,
            ),
            stake::instruction::initialize(
                &stake_account.stake_account,
                hijack_authorized.unwrap_or(&Authorized::auto(&withdraw_authority)),
                hijack_lockup.unwrap_or(&Lockup::default()),
            ),
            instruction::update_stake_pool_balance(
                &id(),
                &stake_pool_accounts.stake_pool.pubkey(),
                &stake_pool_accounts.withdraw_authority,
                &stake_pool_accounts.validator_list.pubkey(),
                &stake_pool_accounts.reserve_stake.pubkey(),
                &stake_pool_accounts.pool_fee_account.pubkey(),
                &stake_pool_accounts.pool_mint.pubkey(),
                &spl_token::id(),
            ),
            instruction::cleanup_removed_validator_entries(
                &id(),
                &stake_pool_accounts.stake_pool.pubkey(),
                &stake_pool_accounts.validator_list.pubkey(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err();
    assert!(error.is_none(), "{:?}", error);

    println!("Update again normally, should be no change in the lamports");
    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            false,
        )
        .await;

    let expected_lamports = get_validator_list_sum(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    assert_eq!(pre_lamports, expected_lamports);

    let stake_pool_info = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(&stake_pool_info.data).unwrap();
    assert_eq!(pre_lamports, stake_pool.total_lamports);

    println!("Fail adding validator back in with first seed");
    let error = stake_pool_accounts
        .add_validator_to_pool(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &stake_account.stake_account,
            &stake_account.vote.pubkey(),
            stake_account.validator_stake_seed,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::AlreadyInUse as u32),
        )
    );

    println!("Succeed adding validator back in with new seed");
    let seed = NonZeroU32::new(1);
    let validator = stake_account.vote.pubkey();
    let (stake_account, _) = find_stake_program_address(
        &id(),
        &validator,
        &stake_pool_accounts.stake_pool.pubkey(),
        seed,
    );
    let error = stake_pool_accounts
        .add_validator_to_pool(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &stake_account,
            &validator,
            seed,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let stake_pool_info = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(&stake_pool_info.data).unwrap();
    assert_eq!(pre_lamports, stake_pool.total_lamports);

    let expected_lamports = get_validator_list_sum(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    assert_eq!(pre_lamports, expected_lamports);
}
