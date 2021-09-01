#![cfg(feature = "test-bpf")]

mod helpers;

use {
    helpers::*,
    solana_program::{borsh::try_from_slice_unchecked, program_pack::Pack, pubkey::Pubkey},
    solana_program_test::*,
    solana_sdk::{signature::Signer, system_instruction, transaction::Transaction},
    spl_stake_pool::{
        find_transient_stake_program_address, id, instruction, stake_program,
        state::{StakePool, StakeStatus, ValidatorList},
        MAX_VALIDATORS_TO_UPDATE, MINIMUM_ACTIVE_STAKE,
    },
    spl_token::state::Mint,
};

async fn setup(
    num_validators: usize,
) -> (
    ProgramTestContext,
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
    let mut slot = first_normal_slot;
    context.warp_to_slot(slot).unwrap();

    let reserve_stake_amount = TEST_STAKE_AMOUNT * num_validators as u64;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            reserve_stake_amount + 1,
        )
        .await
        .unwrap();

    // Add several accounts with some stake
    let mut stake_accounts: Vec<ValidatorStakeAccount> = vec![];
    let mut deposit_accounts: Vec<DepositStakeAccount> = vec![];
    for _ in 0..num_validators {
        let stake_account =
            ValidatorStakeAccount::new(&stake_pool_accounts.stake_pool.pubkey(), u64::MAX);
        stake_account
            .create_and_delegate(
                &mut context.banks_client,
                &context.payer,
                &context.last_blockhash,
                &stake_pool_accounts.staker,
            )
            .await;

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
    slot += 2 * slots_per_epoch;
    context.warp_to_slot(slot).unwrap();

    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &[],
            false,
        )
        .await;

    for stake_account in &stake_accounts {
        let error = stake_pool_accounts
            .add_validator_to_pool(
                &mut context.banks_client,
                &context.payer,
                &context.last_blockhash,
                &stake_account.stake_account,
            )
            .await;
        assert!(error.is_none());
    }

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

    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            stake_accounts
                .iter()
                .map(|v| v.vote.pubkey())
                .collect::<Vec<Pubkey>>()
                .as_slice(),
            false,
        )
        .await;

    (
        context,
        stake_pool_accounts,
        stake_accounts,
        deposit_accounts,
        TEST_STAKE_AMOUNT,
        reserve_stake_amount,
        slot,
    )
}

#[tokio::test]
async fn success() {
    let num_validators = 5;
    let (
        mut context,
        stake_pool_accounts,
        stake_accounts,
        _,
        validator_lamports,
        reserve_lamports,
        mut slot,
    ) = setup(num_validators).await;

    // Check current balance in the list
    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake_program::StakeState>());
    // initially, have all of the deposits plus their rent, and the reserve stake
    let initial_lamports =
        (validator_lamports + stake_rent) * num_validators as u64 + reserve_lamports;
    assert_eq!(
        get_validator_list_sum(
            &mut context.banks_client,
            &stake_pool_accounts.reserve_stake.pubkey(),
            &stake_pool_accounts.validator_list.pubkey()
        )
        .await,
        initial_lamports,
    );

    // Simulate rewards
    for stake_account in &stake_accounts {
        context.increment_vote_account_credits(&stake_account.vote.pubkey(), 100);
    }

    // Warp one more epoch so the rewards are paid out
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    slot += slots_per_epoch;
    context.warp_to_slot(slot).unwrap();

    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            stake_accounts
                .iter()
                .map(|v| v.vote.pubkey())
                .collect::<Vec<Pubkey>>()
                .as_slice(),
            false,
        )
        .await;
    let new_lamports = get_validator_list_sum(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    assert!(new_lamports > initial_lamports);

    let stake_pool_info = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(&stake_pool_info.data).unwrap();
    assert_eq!(new_lamports, stake_pool.total_stake_lamports);
}

#[tokio::test]
async fn merge_into_reserve() {
    let (mut context, stake_pool_accounts, stake_accounts, _, lamports, _, mut slot) =
        setup(MAX_VALIDATORS_TO_UPDATE).await;

    let pre_lamports = get_validator_list_sum(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;

    let reserve_stake = context
        .banks_client
        .get_account(stake_pool_accounts.reserve_stake.pubkey())
        .await
        .unwrap()
        .unwrap();
    let pre_reserve_lamports = reserve_stake.lamports;

    println!("Decrease from all validators");
    for stake_account in &stake_accounts {
        let error = stake_pool_accounts
            .decrease_validator_stake(
                &mut context.banks_client,
                &context.payer,
                &context.last_blockhash,
                &stake_account.stake_account,
                &stake_account.transient_stake_account,
                lamports,
                stake_account.transient_stake_seed,
            )
            .await;
        assert!(error.is_none());
    }

    println!("Update, should not change, no merges yet");
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            stake_accounts
                .iter()
                .map(|v| v.vote.pubkey())
                .collect::<Vec<Pubkey>>()
                .as_slice(),
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
    assert_eq!(expected_lamports, stake_pool.total_stake_lamports);

    println!("Warp one more epoch so the stakes deactivate");
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    slot += slots_per_epoch;
    context.warp_to_slot(slot).unwrap();

    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            stake_accounts
                .iter()
                .map(|v| v.vote.pubkey())
                .collect::<Vec<Pubkey>>()
                .as_slice(),
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

    let reserve_stake = context
        .banks_client
        .get_account(stake_pool_accounts.reserve_stake.pubkey())
        .await
        .unwrap()
        .unwrap();
    let post_reserve_lamports = reserve_stake.lamports;
    assert!(post_reserve_lamports > pre_reserve_lamports);

    let stake_pool_info = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(&stake_pool_info.data).unwrap();
    assert_eq!(expected_lamports, stake_pool.total_stake_lamports);
}

#[tokio::test]
async fn merge_into_validator_stake() {
    let (mut context, stake_pool_accounts, stake_accounts, _, lamports, reserve_lamports, mut slot) =
        setup(MAX_VALIDATORS_TO_UPDATE).await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let pre_lamports = get_validator_list_sum(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;

    // Increase stake to all validators
    for stake_account in &stake_accounts {
        let error = stake_pool_accounts
            .increase_validator_stake(
                &mut context.banks_client,
                &context.payer,
                &context.last_blockhash,
                &stake_account.transient_stake_account,
                &stake_account.vote.pubkey(),
                reserve_lamports / stake_accounts.len() as u64,
                stake_account.transient_stake_seed,
            )
            .await;
        assert!(error.is_none());
    }

    // Warp just a little bit to get a new blockhash and update again
    context.warp_to_slot(slot + 10).unwrap();

    // Update, should not change, no merges yet
    let error = stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            stake_accounts
                .iter()
                .map(|v| v.vote.pubkey())
                .collect::<Vec<Pubkey>>()
                .as_slice(),
            false,
        )
        .await;
    assert!(error.is_none());

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
    assert_eq!(expected_lamports, stake_pool.total_stake_lamports);

    // Warp one more epoch so the stakes activate, ready to merge
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    slot += slots_per_epoch;
    context.warp_to_slot(slot).unwrap();

    let error = stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            stake_accounts
                .iter()
                .map(|v| v.vote.pubkey())
                .collect::<Vec<Pubkey>>()
                .as_slice(),
            false,
        )
        .await;
    assert!(error.is_none());
    let current_lamports = get_validator_list_sum(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let stake_pool_info = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(&stake_pool_info.data).unwrap();
    assert_eq!(current_lamports, stake_pool.total_stake_lamports);

    // Check that transient accounts are gone
    for stake_account in &stake_accounts {
        assert!(context
            .banks_client
            .get_account(stake_account.transient_stake_account)
            .await
            .unwrap()
            .is_none());
    }

    // Check validator stake accounts have the expected balance now:
    // validator stake account minimum + deposited lamports + rents + increased lamports
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake_program::StakeState>());
    let expected_lamports = MINIMUM_ACTIVE_STAKE
        + lamports
        + reserve_lamports / stake_accounts.len() as u64
        + stake_rent;
    for stake_account in &stake_accounts {
        let validator_stake =
            get_account(&mut context.banks_client, &stake_account.stake_account).await;
        assert_eq!(validator_stake.lamports, expected_lamports);
    }

    // Check reserve stake accounts for expected balance:
    // own rent, other account rents, and 1 extra lamport
    let reserve_stake = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await;
    assert_eq!(
        reserve_stake.lamports,
        1 + stake_rent * (1 + stake_accounts.len() as u64)
    );
}

#[tokio::test]
async fn merge_transient_stake_after_remove() {
    let (mut context, stake_pool_accounts, stake_accounts, _, lamports, reserve_lamports, mut slot) =
        setup(1).await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake_program::StakeState>());
    let deactivated_lamports = lamports;
    let new_authority = Pubkey::new_unique();
    // Decrease and remove all validators
    for stake_account in &stake_accounts {
        let error = stake_pool_accounts
            .decrease_validator_stake(
                &mut context.banks_client,
                &context.payer,
                &context.last_blockhash,
                &stake_account.stake_account,
                &stake_account.transient_stake_account,
                deactivated_lamports,
                stake_account.transient_stake_seed,
            )
            .await;
        assert!(error.is_none());
        let error = stake_pool_accounts
            .remove_validator_from_pool(
                &mut context.banks_client,
                &context.payer,
                &context.last_blockhash,
                &new_authority,
                &stake_account.stake_account,
                &stake_account.transient_stake_account,
            )
            .await;
        assert!(error.is_none());
    }

    // Warp forward to merge time
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    slot += slots_per_epoch;
    context.warp_to_slot(slot).unwrap();

    // Update without merge, status should be DeactivatingTransient
    let error = stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            stake_accounts
                .iter()
                .map(|v| v.vote.pubkey())
                .collect::<Vec<Pubkey>>()
                .as_slice(),
            true,
        )
        .await;
    assert!(error.is_none());

    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<ValidatorList>(validator_list.data.as_slice()).unwrap();
    assert_eq!(validator_list.validators.len(), 1);
    assert_eq!(
        validator_list.validators[0].status,
        StakeStatus::DeactivatingTransient
    );
    assert_eq!(validator_list.validators[0].active_stake_lamports, 0);
    assert_eq!(
        validator_list.validators[0].transient_stake_lamports,
        deactivated_lamports
    );

    // Update with merge, status should be ReadyForRemoval and no lamports
    let error = stake_pool_accounts
        .update_validator_list_balance(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            stake_accounts
                .iter()
                .map(|v| v.vote.pubkey())
                .collect::<Vec<Pubkey>>()
                .as_slice(),
            false,
        )
        .await;
    assert!(error.is_none());

    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<ValidatorList>(validator_list.data.as_slice()).unwrap();
    assert_eq!(validator_list.validators.len(), 1);
    assert_eq!(
        validator_list.validators[0].status,
        StakeStatus::ReadyForRemoval
    );
    assert_eq!(validator_list.validators[0].stake_lamports(), 0);

    let reserve_stake = context
        .banks_client
        .get_account(stake_pool_accounts.reserve_stake.pubkey())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        reserve_stake.lamports,
        reserve_lamports + deactivated_lamports + 2 * stake_rent + 1
    );

    // Update stake pool balance and cleanup, should be gone
    let error = stake_pool_accounts
        .update_stake_pool_balance(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
        )
        .await;
    assert!(error.is_none());

    let error = stake_pool_accounts
        .cleanup_removed_validator_entries(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
        )
        .await;
    assert!(error.is_none());

    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<ValidatorList>(validator_list.data.as_slice()).unwrap();
    assert_eq!(validator_list.validators.len(), 0);
}

#[tokio::test]
async fn success_with_burned_tokens() {
    let num_validators = 5;
    let (mut context, stake_pool_accounts, stake_accounts, deposit_accounts, _, _, mut slot) =
        setup(num_validators).await;

    let mint_info = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.pool_mint.pubkey(),
    )
    .await;
    let mint = Mint::unpack(&mint_info.data).unwrap();

    let stake_pool_info = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(&stake_pool_info.data).unwrap();
    assert_eq!(mint.supply, stake_pool.pool_token_supply);

    burn_tokens(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts.pool_mint.pubkey(),
        &deposit_accounts[0].pool_account.pubkey(),
        &deposit_accounts[0].authority,
        deposit_accounts[0].pool_tokens,
    )
    .await
    .unwrap();

    let mint_info = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.pool_mint.pubkey(),
    )
    .await;
    let mint = Mint::unpack(&mint_info.data).unwrap();
    assert_ne!(mint.supply, stake_pool.pool_token_supply);

    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    slot += slots_per_epoch;
    context.warp_to_slot(slot).unwrap();

    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            stake_accounts
                .iter()
                .map(|v| v.vote.pubkey())
                .collect::<Vec<Pubkey>>()
                .as_slice(),
            false,
        )
        .await;

    let stake_pool_info = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(&stake_pool_info.data).unwrap();

    assert_eq!(mint.supply, stake_pool.pool_token_supply);
}

#[tokio::test]
async fn success_ignoring_hijacked_transient_stake() {
    let num_validators = 1;
    let (mut context, stake_pool_accounts, stake_accounts, _, lamports, _, mut slot) =
        setup(num_validators).await;

    let pre_lamports = get_validator_list_sum(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;

    println!("Decrease from all validators");
    let stake_account = &stake_accounts[0];
    let error = stake_pool_accounts
        .decrease_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_account.stake_account,
            &stake_account.transient_stake_account,
            lamports,
            stake_account.transient_stake_seed,
        )
        .await;
    assert!(error.is_none());

    println!("Warp one epoch so the stakes deactivate and merge");
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    slot += slots_per_epoch;
    context.warp_to_slot(slot).unwrap();

    println!("During update, hijack the transient stake account");
    let validator_list = stake_pool_accounts
        .get_validator_list(&mut context.banks_client)
        .await;
    let hijacker = Pubkey::new_unique();
    let transient_stake_address = find_transient_stake_program_address(
        &id(),
        &stake_account.vote.pubkey(),
        &stake_pool_accounts.stake_pool.pubkey(),
        stake_account.transient_stake_seed,
    )
    .0;
    let transaction = Transaction::new_signed_with_payer(
        &[
            instruction::update_validator_list_balance(
                &id(),
                &stake_pool_accounts.stake_pool.pubkey(),
                &stake_pool_accounts.withdraw_authority,
                &stake_pool_accounts.validator_list.pubkey(),
                &stake_pool_accounts.reserve_stake.pubkey(),
                &validator_list,
                &[stake_account.vote.pubkey()],
                0,
                /* no_merge = */ false,
            ),
            system_instruction::transfer(
                &context.payer.pubkey(),
                &transient_stake_address,
                1_000_000_000,
            ),
            stake_program::initialize(
                &transient_stake_address,
                &stake_program::Authorized {
                    staker: hijacker,
                    withdrawer: hijacker,
                },
                &stake_program::Lockup::default(),
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
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err();
    assert!(error.is_none());

    println!("Update again normally, should be no change in the lamports");
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            stake_accounts
                .iter()
                .map(|v| v.vote.pubkey())
                .collect::<Vec<Pubkey>>()
                .as_slice(),
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
    assert_eq!(pre_lamports, stake_pool.total_stake_lamports);
}

#[tokio::test]
async fn fail_with_uninitialized_validator_list() {} // TODO

#[tokio::test]
async fn fail_with_wrong_stake_state() {} // TODO
