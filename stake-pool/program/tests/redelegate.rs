#![allow(clippy::integer_arithmetic)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    bincode::deserialize,
    helpers::*,
    solana_program::{
        clock::Epoch, hash::Hash, instruction::InstructionError, pubkey::Pubkey, stake,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        stake::instruction::StakeError,
        transaction::{Transaction, TransactionError},
    },
    spl_stake_pool::{
        error::StakePoolError, find_ephemeral_stake_program_address,
        find_transient_stake_program_address, id, instruction, MINIMUM_RESERVE_LAMPORTS,
    },
};

async fn setup(
    do_warp: bool,
) -> (
    ProgramTestContext,
    Hash,
    StakePoolAccounts,
    ValidatorStakeAccount,
    ValidatorStakeAccount,
    u64,
    u64,
) {
    let mut context = program_test().start_with_context().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>());
    let current_minimum_delegation = stake_pool_get_minimum_delegation(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;

    let stake_pool_accounts = StakePoolAccounts::default();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            MINIMUM_RESERVE_LAMPORTS + current_minimum_delegation + stake_rent,
        )
        .await
        .unwrap();

    let source_validator_stake = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
        None,
    )
    .await;

    let destination_validator_stake = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
        None,
    )
    .await;

    let minimum_redelegate_lamports = current_minimum_delegation + stake_rent * 2;
    simple_deposit_stake(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
        &source_validator_stake,
        minimum_redelegate_lamports,
    )
    .await
    .unwrap();

    let mut slot = 0;
    if do_warp {
        slot = context.genesis_config().epoch_schedule.first_normal_slot;
        context.warp_to_slot(slot).unwrap();
        stake_pool_accounts
            .update_all(
                &mut context.banks_client,
                &context.payer,
                &context.last_blockhash,
                &[
                    source_validator_stake.vote.pubkey(),
                    destination_validator_stake.vote.pubkey(),
                ],
                false,
            )
            .await;
    }

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();

    (
        context,
        last_blockhash,
        stake_pool_accounts,
        source_validator_stake,
        destination_validator_stake,
        minimum_redelegate_lamports,
        slot,
    )
}

#[tokio::test]
async fn success() {
    let (
        mut context,
        last_blockhash,
        stake_pool_accounts,
        source_validator_stake,
        destination_validator_stake,
        redelegate_lamports,
        mut slot,
    ) = setup(true).await;

    // Save validator stake
    let pre_validator_stake_account = get_account(
        &mut context.banks_client,
        &source_validator_stake.stake_account,
    )
    .await;

    // Save validator stake
    let pre_destination_validator_stake_account = get_account(
        &mut context.banks_client,
        &destination_validator_stake.stake_account,
    )
    .await;

    // Check no transient stake
    let transient_account = context
        .banks_client
        .get_account(source_validator_stake.transient_stake_account)
        .await
        .unwrap();
    assert!(transient_account.is_none());

    let ephemeral_stake_seed = 100;
    let ephemeral_stake = find_ephemeral_stake_program_address(
        &id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        ephemeral_stake_seed,
    )
    .0;
    let error = stake_pool_accounts
        .redelegate(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &source_validator_stake.stake_account,
            &source_validator_stake.transient_stake_account,
            &ephemeral_stake,
            &destination_validator_stake.transient_stake_account,
            &destination_validator_stake.stake_account,
            &destination_validator_stake.vote.pubkey(),
            redelegate_lamports,
            source_validator_stake.transient_stake_seed,
            ephemeral_stake_seed,
            destination_validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none());

    // Check validator stake account balance
    let validator_stake_account = get_account(
        &mut context.banks_client,
        &source_validator_stake.stake_account,
    )
    .await;
    let validator_stake_state =
        deserialize::<stake::state::StakeState>(&validator_stake_account.data).unwrap();
    assert_eq!(
        pre_validator_stake_account.lamports - redelegate_lamports,
        validator_stake_account.lamports
    );
    assert_eq!(
        validator_stake_state
            .delegation()
            .unwrap()
            .deactivation_epoch,
        Epoch::MAX
    );

    // Check source transient stake account state and balance
    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>());

    let source_transient_stake_account = get_account(
        &mut context.banks_client,
        &source_validator_stake.transient_stake_account,
    )
    .await;
    let transient_stake_state =
        deserialize::<stake::state::StakeState>(&source_transient_stake_account.data).unwrap();
    assert_eq!(source_transient_stake_account.lamports, stake_rent);
    let transient_delegation = transient_stake_state.delegation().unwrap();
    assert_ne!(transient_delegation.deactivation_epoch, Epoch::MAX);
    assert_eq!(transient_delegation.stake, redelegate_lamports - stake_rent);

    // Check ephemeral account doesn't exist
    let maybe_account = context
        .banks_client
        .get_account(ephemeral_stake)
        .await
        .unwrap();
    assert!(maybe_account.is_none());

    // Check destination transient stake account
    let destination_transient_stake_account = get_account(
        &mut context.banks_client,
        &destination_validator_stake.transient_stake_account,
    )
    .await;
    let transient_stake_state =
        deserialize::<stake::state::StakeState>(&destination_transient_stake_account.data).unwrap();
    assert_eq!(
        destination_transient_stake_account.lamports,
        redelegate_lamports - stake_rent
    );
    let transient_delegation = transient_stake_state.delegation().unwrap();
    assert_eq!(transient_delegation.deactivation_epoch, Epoch::MAX);
    assert_ne!(transient_delegation.activation_epoch, Epoch::MAX);
    assert_eq!(
        transient_delegation.stake,
        redelegate_lamports - stake_rent * 2
    );

    // Check validator list
    let validator_list = stake_pool_accounts
        .get_validator_list(&mut context.banks_client)
        .await;
    let source_item = validator_list
        .find(&source_validator_stake.vote.pubkey())
        .unwrap();
    assert_eq!(
        source_item.active_stake_lamports,
        validator_stake_account.lamports
    );
    assert_eq!(
        source_item.transient_stake_lamports,
        source_transient_stake_account.lamports
    );
    assert_eq!(
        source_item.transient_seed_suffix,
        source_validator_stake.transient_stake_seed
    );

    let destination_item = validator_list
        .find(&destination_validator_stake.vote.pubkey())
        .unwrap();
    assert_eq!(
        destination_item.transient_stake_lamports,
        destination_transient_stake_account.lamports
    );
    assert_eq!(
        destination_item.transient_seed_suffix,
        destination_validator_stake.transient_stake_seed
    );

    // Warp forward and merge all
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    slot += slots_per_epoch;
    context.warp_to_slot(slot).unwrap();
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &[
                source_validator_stake.vote.pubkey(),
                destination_validator_stake.vote.pubkey(),
            ],
            false,
        )
        .await;

    // Check transient accounts are gone
    let maybe_account = context
        .banks_client
        .get_account(destination_validator_stake.transient_stake_account)
        .await
        .unwrap();
    assert!(maybe_account.is_none());
    let maybe_account = context
        .banks_client
        .get_account(source_validator_stake.transient_stake_account)
        .await
        .unwrap();
    assert!(maybe_account.is_none());

    // Check validator list
    let validator_list = stake_pool_accounts
        .get_validator_list(&mut context.banks_client)
        .await;
    let source_item = validator_list
        .find(&source_validator_stake.vote.pubkey())
        .unwrap();
    assert_eq!(
        source_item.active_stake_lamports,
        validator_stake_account.lamports
    );
    assert_eq!(source_item.transient_stake_lamports, 0);

    let destination_item = validator_list
        .find(&destination_validator_stake.vote.pubkey())
        .unwrap();
    assert_eq!(destination_item.transient_stake_lamports, 0);
    assert_eq!(
        destination_item.active_stake_lamports,
        pre_destination_validator_stake_account.lamports + redelegate_lamports - stake_rent * 2
    );
    let post_destination_validator_stake_account = get_account(
        &mut context.banks_client,
        &destination_validator_stake.stake_account,
    )
    .await;
    assert_eq!(
        post_destination_validator_stake_account.lamports,
        pre_destination_validator_stake_account.lamports + redelegate_lamports - stake_rent * 2
    );
}

#[tokio::test]
async fn success_with_increasing_stake() {
    let (
        mut context,
        last_blockhash,
        stake_pool_accounts,
        source_validator_stake,
        destination_validator_stake,
        redelegate_lamports,
        mut slot,
    ) = setup(true).await;

    // Save validator stake
    let pre_validator_stake_account = get_account(
        &mut context.banks_client,
        &destination_validator_stake.stake_account,
    )
    .await;

    let current_minimum_delegation = stake_pool_get_minimum_delegation(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
    )
    .await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>());

    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &destination_validator_stake.transient_stake_account,
            &destination_validator_stake.stake_account,
            &destination_validator_stake.vote.pubkey(),
            current_minimum_delegation,
            destination_validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none());

    let validator_list = stake_pool_accounts
        .get_validator_list(&mut context.banks_client)
        .await;
    let destination_item = validator_list
        .find(&destination_validator_stake.vote.pubkey())
        .unwrap();
    assert_eq!(
        destination_item.transient_stake_lamports,
        current_minimum_delegation + stake_rent
    );
    let pre_transient_stake_account = get_account(
        &mut context.banks_client,
        &destination_validator_stake.transient_stake_account,
    )
    .await;
    assert_eq!(
        pre_transient_stake_account.lamports,
        current_minimum_delegation + stake_rent
    );

    let ephemeral_stake_seed = 10;
    let ephemeral_stake = find_ephemeral_stake_program_address(
        &id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        ephemeral_stake_seed,
    )
    .0;

    // fail with incorrect transient stake derivation
    let wrong_transient_stake_seed = destination_validator_stake
        .transient_stake_seed
        .wrapping_add(1);
    let (wrong_transient_stake_account, _) = find_transient_stake_program_address(
        &id(),
        &destination_validator_stake.vote.pubkey(),
        &stake_pool_accounts.stake_pool.pubkey(),
        wrong_transient_stake_seed,
    );
    let error = stake_pool_accounts
        .redelegate(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &source_validator_stake.stake_account,
            &source_validator_stake.transient_stake_account,
            &ephemeral_stake,
            &wrong_transient_stake_account,
            &destination_validator_stake.stake_account,
            &destination_validator_stake.vote.pubkey(),
            redelegate_lamports,
            source_validator_stake.transient_stake_seed,
            ephemeral_stake_seed,
            wrong_transient_stake_seed,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::InvalidStakeAccountAddress as u32)
        )
    );

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();

    let error = stake_pool_accounts
        .redelegate(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &source_validator_stake.stake_account,
            &source_validator_stake.transient_stake_account,
            &ephemeral_stake,
            &destination_validator_stake.transient_stake_account,
            &destination_validator_stake.stake_account,
            &destination_validator_stake.vote.pubkey(),
            redelegate_lamports,
            source_validator_stake.transient_stake_seed,
            ephemeral_stake_seed,
            destination_validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none());

    // Check destination transient stake account
    let destination_transient_stake_account = get_account(
        &mut context.banks_client,
        &destination_validator_stake.transient_stake_account,
    )
    .await;
    let transient_stake_state =
        deserialize::<stake::state::StakeState>(&destination_transient_stake_account.data).unwrap();
    // stake rent cancels out
    assert_eq!(
        destination_transient_stake_account.lamports,
        redelegate_lamports + current_minimum_delegation
    );

    let transient_delegation = transient_stake_state.delegation().unwrap();
    assert_eq!(transient_delegation.deactivation_epoch, Epoch::MAX);
    assert_ne!(transient_delegation.activation_epoch, Epoch::MAX);
    assert_eq!(
        transient_delegation.stake,
        redelegate_lamports + current_minimum_delegation - stake_rent
    );

    // Check validator list
    let validator_list = stake_pool_accounts
        .get_validator_list(&mut context.banks_client)
        .await;
    let destination_item = validator_list
        .find(&destination_validator_stake.vote.pubkey())
        .unwrap();
    assert_eq!(
        destination_item.transient_stake_lamports,
        destination_transient_stake_account.lamports
    );
    assert_eq!(
        destination_item.transient_seed_suffix,
        destination_validator_stake.transient_stake_seed
    );

    // Warp forward and merge all
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    slot += slots_per_epoch;
    context.warp_to_slot(slot).unwrap();
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &[
                source_validator_stake.vote.pubkey(),
                destination_validator_stake.vote.pubkey(),
            ],
            false,
        )
        .await;

    // Check transient account is gone
    let maybe_account = context
        .banks_client
        .get_account(destination_validator_stake.transient_stake_account)
        .await
        .unwrap();
    assert!(maybe_account.is_none());

    // Check validator list
    let validator_list = stake_pool_accounts
        .get_validator_list(&mut context.banks_client)
        .await;
    let destination_item = validator_list
        .find(&destination_validator_stake.vote.pubkey())
        .unwrap();
    assert_eq!(destination_item.transient_stake_lamports, 0);
    // redelegate is smart enough to activate *everything*, so there's only one rent-exemption
    // worth of inactive stake!
    assert_eq!(
        destination_item.active_stake_lamports,
        pre_validator_stake_account.lamports + redelegate_lamports + current_minimum_delegation
            - stake_rent
    );
    let post_validator_stake_account = get_account(
        &mut context.banks_client,
        &destination_validator_stake.stake_account,
    )
    .await;
    assert_eq!(
        post_validator_stake_account.lamports,
        pre_validator_stake_account.lamports + redelegate_lamports + current_minimum_delegation
            - stake_rent
    );
}

#[tokio::test]
async fn fail_with_decreasing_stake() {
    let (
        mut context,
        last_blockhash,
        stake_pool_accounts,
        source_validator_stake,
        destination_validator_stake,
        redelegate_lamports,
        mut slot,
    ) = setup(false).await;

    let current_minimum_delegation = stake_pool_get_minimum_delegation(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
    )
    .await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>());
    let minimum_decrease_lamports = current_minimum_delegation + stake_rent;

    simple_deposit_stake(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts,
        &destination_validator_stake,
        redelegate_lamports,
    )
    .await
    .unwrap();

    slot += context.genesis_config().epoch_schedule.first_normal_slot;
    context.warp_to_slot(slot).unwrap();
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &[
                source_validator_stake.vote.pubkey(),
                destination_validator_stake.vote.pubkey(),
            ],
            false,
        )
        .await;

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();

    let error = stake_pool_accounts
        .decrease_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &destination_validator_stake.stake_account,
            &destination_validator_stake.transient_stake_account,
            minimum_decrease_lamports,
            destination_validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none());

    let ephemeral_stake_seed = 20;
    let ephemeral_stake = find_ephemeral_stake_program_address(
        &id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        ephemeral_stake_seed,
    )
    .0;

    let error = stake_pool_accounts
        .redelegate(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &source_validator_stake.stake_account,
            &source_validator_stake.transient_stake_account,
            &ephemeral_stake,
            &destination_validator_stake.transient_stake_account,
            &destination_validator_stake.stake_account,
            &destination_validator_stake.vote.pubkey(),
            redelegate_lamports,
            source_validator_stake.transient_stake_seed,
            ephemeral_stake_seed,
            destination_validator_stake.transient_stake_seed,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::WrongStakeState as u32)
        )
    );
}

#[tokio::test]
async fn fail_with_wrong_withdraw_authority() {
    let (
        mut context,
        last_blockhash,
        stake_pool_accounts,
        source_validator_stake,
        destination_validator_stake,
        redelegate_lamports,
        _,
    ) = setup(true).await;

    let ephemeral_stake_seed = 2;
    let ephemeral_stake = find_ephemeral_stake_program_address(
        &id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        ephemeral_stake_seed,
    )
    .0;

    let wrong_withdraw_authority = Pubkey::new_unique();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::redelegate(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &wrong_withdraw_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &source_validator_stake.stake_account,
            &source_validator_stake.transient_stake_account,
            &ephemeral_stake,
            &destination_validator_stake.transient_stake_account,
            &destination_validator_stake.stake_account,
            &destination_validator_stake.vote.pubkey(),
            redelegate_lamports,
            source_validator_stake.transient_stake_seed,
            ephemeral_stake_seed,
            destination_validator_stake.transient_stake_seed,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.staker],
        last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
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
async fn fail_with_wrong_validator_list() {
    let (
        mut context,
        last_blockhash,
        stake_pool_accounts,
        source_validator_stake,
        destination_validator_stake,
        redelegate_lamports,
        _,
    ) = setup(true).await;

    let ephemeral_stake_seed = 2;
    let ephemeral_stake = find_ephemeral_stake_program_address(
        &id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        ephemeral_stake_seed,
    )
    .0;

    let wrong_validator_list = Pubkey::new_unique();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::redelegate(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &wrong_validator_list,
            &source_validator_stake.stake_account,
            &source_validator_stake.transient_stake_account,
            &ephemeral_stake,
            &destination_validator_stake.transient_stake_account,
            &destination_validator_stake.stake_account,
            &destination_validator_stake.vote.pubkey(),
            redelegate_lamports,
            source_validator_stake.transient_stake_seed,
            ephemeral_stake_seed,
            destination_validator_stake.transient_stake_seed,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.staker],
        last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::InvalidValidatorStakeList as u32)
        )
    );
}

#[tokio::test]
async fn fail_with_wrong_staker() {
    let (
        mut context,
        last_blockhash,
        stake_pool_accounts,
        source_validator_stake,
        destination_validator_stake,
        redelegate_lamports,
        _,
    ) = setup(true).await;

    let ephemeral_stake_seed = 2;
    let ephemeral_stake = find_ephemeral_stake_program_address(
        &id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        ephemeral_stake_seed,
    )
    .0;

    let wrong_staker = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::redelegate(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &wrong_staker.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &source_validator_stake.stake_account,
            &source_validator_stake.transient_stake_account,
            &ephemeral_stake,
            &destination_validator_stake.transient_stake_account,
            &destination_validator_stake.stake_account,
            &destination_validator_stake.vote.pubkey(),
            redelegate_lamports,
            source_validator_stake.transient_stake_seed,
            ephemeral_stake_seed,
            destination_validator_stake.transient_stake_seed,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wrong_staker],
        last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::WrongStaker as u32)
        )
    );
}

#[tokio::test]
async fn fail_with_unknown_validator() {
    let (
        mut context,
        last_blockhash,
        stake_pool_accounts,
        source_validator_stake,
        destination_validator_stake,
        redelegate_lamports,
        _,
    ) = setup(true).await;

    let unknown_validator_stake = create_unknown_validator_stake(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts.stake_pool.pubkey(),
        redelegate_lamports,
    )
    .await;

    let ephemeral_stake_seed = 42;
    let ephemeral_stake = find_ephemeral_stake_program_address(
        &id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        ephemeral_stake_seed,
    )
    .0;
    let error = stake_pool_accounts
        .redelegate(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &source_validator_stake.stake_account,
            &source_validator_stake.transient_stake_account,
            &ephemeral_stake,
            &unknown_validator_stake.transient_stake_account,
            &unknown_validator_stake.stake_account,
            &unknown_validator_stake.vote.pubkey(),
            redelegate_lamports,
            source_validator_stake.transient_stake_seed,
            ephemeral_stake_seed,
            unknown_validator_stake.transient_stake_seed,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::ValidatorNotFound as u32)
        )
    );

    let error = stake_pool_accounts
        .redelegate(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &unknown_validator_stake.stake_account,
            &unknown_validator_stake.transient_stake_account,
            &ephemeral_stake,
            &destination_validator_stake.transient_stake_account,
            &destination_validator_stake.stake_account,
            &destination_validator_stake.vote.pubkey(),
            redelegate_lamports,
            unknown_validator_stake.transient_stake_seed,
            ephemeral_stake_seed,
            destination_validator_stake.transient_stake_seed,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::ValidatorNotFound as u32)
        )
    );
}

#[tokio::test]
async fn fail_redelegate_twice() {
    let (
        mut context,
        last_blockhash,
        stake_pool_accounts,
        source_validator_stake,
        destination_validator_stake,
        redelegate_lamports,
        mut slot,
    ) = setup(false).await;

    simple_deposit_stake(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts,
        &source_validator_stake,
        redelegate_lamports,
    )
    .await
    .unwrap();

    slot += context.genesis_config().epoch_schedule.first_normal_slot;
    context.warp_to_slot(slot).unwrap();
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &[
                source_validator_stake.vote.pubkey(),
                destination_validator_stake.vote.pubkey(),
            ],
            false,
        )
        .await;

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();

    let ephemeral_stake_seed = 100;
    let ephemeral_stake = find_ephemeral_stake_program_address(
        &id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        ephemeral_stake_seed,
    )
    .0;
    let error = stake_pool_accounts
        .redelegate(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &source_validator_stake.stake_account,
            &source_validator_stake.transient_stake_account,
            &ephemeral_stake,
            &destination_validator_stake.transient_stake_account,
            &destination_validator_stake.stake_account,
            &destination_validator_stake.vote.pubkey(),
            redelegate_lamports,
            source_validator_stake.transient_stake_seed,
            ephemeral_stake_seed,
            destination_validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none());

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();

    let error = stake_pool_accounts
        .redelegate(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &source_validator_stake.stake_account,
            &source_validator_stake.transient_stake_account,
            &ephemeral_stake,
            &destination_validator_stake.transient_stake_account,
            &destination_validator_stake.stake_account,
            &destination_validator_stake.vote.pubkey(),
            redelegate_lamports,
            source_validator_stake.transient_stake_seed,
            ephemeral_stake_seed,
            destination_validator_stake.transient_stake_seed,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::TransientAccountInUse as u32)
        )
    );
}

#[tokio::test]
async fn fail_with_small_lamport_amount() {
    let (
        mut context,
        last_blockhash,
        stake_pool_accounts,
        source_validator_stake,
        destination_validator_stake,
        redelegate_lamports,
        _,
    ) = setup(true).await;

    let ephemeral_stake_seed = 7_000;
    let ephemeral_stake = find_ephemeral_stake_program_address(
        &id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        ephemeral_stake_seed,
    )
    .0;

    let error = stake_pool_accounts
        .redelegate(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &source_validator_stake.stake_account,
            &source_validator_stake.transient_stake_account,
            &ephemeral_stake,
            &destination_validator_stake.transient_stake_account,
            &destination_validator_stake.stake_account,
            &destination_validator_stake.vote.pubkey(),
            redelegate_lamports - 1,
            source_validator_stake.transient_stake_seed,
            ephemeral_stake_seed,
            destination_validator_stake.transient_stake_seed,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakeError::InsufficientDelegation as u32)
        )
    );
}

#[tokio::test]
async fn fail_drain_source_account() {
    let (
        mut context,
        last_blockhash,
        stake_pool_accounts,
        source_validator_stake,
        destination_validator_stake,
        _,
        _,
    ) = setup(true).await;

    let validator_stake_account = get_account(
        &mut context.banks_client,
        &source_validator_stake.stake_account,
    )
    .await;

    let ephemeral_stake_seed = 2;
    let ephemeral_stake = find_ephemeral_stake_program_address(
        &id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        ephemeral_stake_seed,
    )
    .0;

    let error = stake_pool_accounts
        .redelegate(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &source_validator_stake.stake_account,
            &source_validator_stake.transient_stake_account,
            &ephemeral_stake,
            &destination_validator_stake.transient_stake_account,
            &destination_validator_stake.stake_account,
            &destination_validator_stake.vote.pubkey(),
            validator_stake_account.lamports,
            source_validator_stake.transient_stake_seed,
            ephemeral_stake_seed,
            destination_validator_stake.transient_stake_seed,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(0, InstructionError::InsufficientFunds)
    );
}
