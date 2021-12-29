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
        stake, system_instruction, sysvar,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
    spl_stake_pool::{
        error::StakePoolError, find_transient_stake_program_address, id, instruction, state,
    },
};

async fn setup() -> (
    ProgramTestContext,
    StakePoolAccounts,
    ValidatorStakeAccount,
    Pubkey,
    Keypair,
) {
    let mut context = program_test().start_with_context().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            10_000_000_000,
        )
        .await
        .unwrap();

    let validator_stake =
        ValidatorStakeAccount::new(&stake_pool_accounts.stake_pool.pubkey(), u64::MAX);
    create_vote(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &validator_stake.validator,
        &validator_stake.vote,
    )
    .await;

    let error = stake_pool_accounts
        .add_validator_to_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.vote.pubkey(),
        )
        .await;
    assert!(error.is_none());

    let new_authority = Pubkey::new_unique();
    let destination_stake = Keypair::new();

    (
        context,
        stake_pool_accounts,
        validator_stake,
        new_authority,
        destination_stake,
    )
}

#[tokio::test]
async fn success() {
    let (mut context, stake_pool_accounts, validator_stake, new_authority, destination_stake) =
        setup().await;

    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &new_authority,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            &destination_stake,
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

    // Check if account was removed from the list of stake accounts
    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    assert_eq!(
        validator_list,
        state::ValidatorList {
            header: state::ValidatorListHeader {
                account_type: state::AccountType::ValidatorList,
                max_validators: stake_pool_accounts.max_validators,
            },
            validators: vec![]
        }
    );

    // Check stake account no longer exists
    let account = context
        .banks_client
        .get_account(validator_stake.stake_account)
        .await
        .unwrap();
    assert!(account.is_none());
    let stake = get_account(&mut context.banks_client, &destination_stake.pubkey()).await;
    let stake_state = deserialize::<stake::state::StakeState>(&stake.data).unwrap();
    match stake_state {
        stake::state::StakeState::Stake(meta, _) => {
            assert_eq!(&meta.authorized.staker, &new_authority);
            assert_eq!(&meta.authorized.withdrawer, &new_authority);
        }
        _ => panic!(),
    }
}

#[tokio::test]
async fn fail_with_wrong_stake_program_id() {
    let (mut context, stake_pool_accounts, validator_stake, new_authority, destination_stake) =
        setup().await;

    let wrong_stake_program = Pubkey::new_unique();

    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.staker.pubkey(), true),
        AccountMeta::new_readonly(stake_pool_accounts.withdraw_authority, false),
        AccountMeta::new_readonly(new_authority, false),
        AccountMeta::new(stake_pool_accounts.validator_list.pubkey(), false),
        AccountMeta::new(validator_stake.stake_account, false),
        AccountMeta::new_readonly(validator_stake.transient_stake_account, false),
        AccountMeta::new(destination_stake.pubkey(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(wrong_stake_program, false),
    ];
    let instruction = Instruction {
        program_id: id(),
        accounts,
        data: instruction::StakePoolInstruction::RemoveValidatorFromPool
            .try_to_vec()
            .unwrap(),
    };

    let mut transaction =
        Transaction::new_with_payer(&[instruction], Some(&context.payer.pubkey()));
    transaction.sign(
        &[&context.payer, &stake_pool_accounts.staker],
        context.last_blockhash,
    );
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
    let transaction_error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .into();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            error,
        )) => {
            assert_eq!(error, InstructionError::IncorrectProgramId);
        }
        _ => panic!("Wrong error occurs while try to remove validator stake address with wrong stake program ID"),
    }
}

#[tokio::test]
async fn fail_with_wrong_validator_list_account() {
    let (mut context, stake_pool_accounts, validator_stake, new_authority, destination_stake) =
        setup().await;

    let wrong_validator_list = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::remove_validator_from_pool(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &new_authority,
            &wrong_validator_list.pubkey(),
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            &destination_stake.pubkey(),
        )],
        Some(&context.payer.pubkey()),
    );
    transaction.sign(
        &[&context.payer, &stake_pool_accounts.staker],
        context.last_blockhash,
    );
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
    let transaction_error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .into();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = StakePoolError::InvalidValidatorStakeList as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to remove validator stake address with wrong validator stake list account"),
    }
}

#[tokio::test]
async fn fail_not_at_minimum() {
    let (mut context, stake_pool_accounts, validator_stake, new_authority, destination_stake) =
        setup().await;

    transfer(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &validator_stake.stake_account,
        1_000_001,
    )
    .await;

    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &new_authority,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            &destination_stake,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            1,
            InstructionError::Custom(StakePoolError::StakeLamportsNotEqualToMinimum as u32)
        ),
    );
}

#[tokio::test]
async fn fail_double_remove() {
    let (mut context, stake_pool_accounts, validator_stake, new_authority, destination_stake) =
        setup().await;

    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &new_authority,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            &destination_stake,
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

    let _latest_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let destination_stake = Keypair::new();
    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &new_authority,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            &destination_stake,
        )
        .await
        .unwrap()
        .unwrap();

    assert!(matches!(
        error,
        TransactionError::InstructionError(1, InstructionError::BorshIoError(_),)
    ));
}

#[tokio::test]
async fn fail_wrong_staker() {
    let (mut context, stake_pool_accounts, validator_stake, new_authority, destination_stake) =
        setup().await;

    let malicious = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::remove_validator_from_pool(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &malicious.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &new_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            &destination_stake.pubkey(),
        )],
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer, &malicious], context.last_blockhash);
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
    let transaction_error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .into();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = StakePoolError::WrongStaker as u32;
            assert_eq!(error_index, program_error);
        }
        _ => {
            panic!("Wrong error occurs while not an staker try to remove validator stake address")
        }
    }
}

#[tokio::test]
async fn fail_no_signature() {
    let (mut context, stake_pool_accounts, validator_stake, new_authority, destination_stake) =
        setup().await;

    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.staker.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.withdraw_authority, false),
        AccountMeta::new_readonly(new_authority, false),
        AccountMeta::new(stake_pool_accounts.validator_list.pubkey(), false),
        AccountMeta::new(validator_stake.stake_account, false),
        AccountMeta::new_readonly(validator_stake.transient_stake_account, false),
        AccountMeta::new(destination_stake.pubkey(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];
    let instruction = Instruction {
        program_id: id(),
        accounts,
        data: instruction::StakePoolInstruction::RemoveValidatorFromPool
            .try_to_vec()
            .unwrap(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
    let transaction_error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .into();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = StakePoolError::SignatureMissing as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while malicious try to remove validator stake account without signing transaction"),
    }
}

#[tokio::test]
async fn fail_with_activating_transient_stake() {
    let (mut context, stake_pool_accounts, validator_stake, new_authority, destination_stake) =
        setup().await;

    // increase the validator stake
    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.vote.pubkey(),
            2_000_000_000,
            validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none());

    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &new_authority,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            &destination_stake,
        )
        .await
        .unwrap()
        .unwrap();
    match error {
        TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        ) => {
            let program_error = StakePoolError::WrongStakeState as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while removing validator stake account while transient stake is activating"),
    }
}

#[tokio::test]
async fn success_with_deactivating_transient_stake() {
    let (mut context, stake_pool_accounts, validator_stake, new_authority, destination_stake) =
        setup().await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>());
    let deposit_info = simple_deposit_stake(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
        &validator_stake,
        TEST_STAKE_AMOUNT,
    )
    .await
    .unwrap();

    // increase the validator stake
    let error = stake_pool_accounts
        .decrease_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            TEST_STAKE_AMOUNT + stake_rent,
            validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none());

    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &new_authority,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            &destination_stake,
        )
        .await;
    assert!(error.is_none());

    // fail deposit
    let maybe_deposit = simple_deposit_stake(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
        &validator_stake,
        TEST_STAKE_AMOUNT,
    )
    .await;
    assert!(maybe_deposit.is_none());

    // fail withdraw
    let user_stake_recipient = Keypair::new();
    create_blank_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &user_stake_recipient,
    )
    .await;

    let user_transfer_authority = Keypair::new();
    let new_authority = Pubkey::new_unique();
    delegate_tokens(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &deposit_info.pool_account.pubkey(),
        &deposit_info.authority,
        &user_transfer_authority.pubkey(),
        1,
    )
    .await;
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake.stake_account,
            &new_authority,
            1,
        )
        .await;
    assert!(error.is_some());

    // check validator has changed
    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    let expected_list = state::ValidatorList {
        header: state::ValidatorListHeader {
            account_type: state::AccountType::ValidatorList,
            max_validators: stake_pool_accounts.max_validators,
        },
        validators: vec![state::ValidatorStakeInfo {
            status: state::StakeStatus::DeactivatingTransient,
            vote_account_address: validator_stake.vote.pubkey(),
            last_update_epoch: 0,
            active_stake_lamports: 0,
            transient_stake_lamports: TEST_STAKE_AMOUNT + stake_rent,
            transient_seed_suffix_start: validator_stake.transient_stake_seed,
            transient_seed_suffix_end: 0,
        }],
    };
    assert_eq!(validator_list, expected_list);

    // Update, should not change, no merges yet
    let error = stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &[validator_stake.vote.pubkey()],
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
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    assert_eq!(validator_list, expected_list);
}

#[tokio::test]
async fn success_resets_preferred_validator() {
    let (mut context, stake_pool_accounts, validator_stake, new_authority, destination_stake) =
        setup().await;

    stake_pool_accounts
        .set_preferred_validator(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            instruction::PreferredValidatorType::Deposit,
            Some(validator_stake.vote.pubkey()),
        )
        .await;
    stake_pool_accounts
        .set_preferred_validator(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            instruction::PreferredValidatorType::Withdraw,
            Some(validator_stake.vote.pubkey()),
        )
        .await;

    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &new_authority,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            &destination_stake,
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

    // Check if account was removed from the list of stake accounts
    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    assert_eq!(
        validator_list,
        state::ValidatorList {
            header: state::ValidatorListHeader {
                account_type: state::AccountType::ValidatorList,
                max_validators: stake_pool_accounts.max_validators,
            },
            validators: vec![]
        }
    );

    // Check of stake account authority has changed
    let stake = get_account(&mut context.banks_client, &destination_stake.pubkey()).await;
    let stake_state = deserialize::<stake::state::StakeState>(&stake.data).unwrap();
    match stake_state {
        stake::state::StakeState::Stake(meta, _) => {
            assert_eq!(&meta.authorized.staker, &new_authority);
            assert_eq!(&meta.authorized.withdrawer, &new_authority);
        }
        _ => panic!(),
    }
}

#[tokio::test]
async fn success_with_hijacked_transient_account() {
    let (mut context, stake_pool_accounts, validator_stake, new_authority, destination_stake) =
        setup().await;

    // increase stake on validator
    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.vote.pubkey(),
            1_000_000_000,
            validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none());

    // warp forward to merge
    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    let mut slot = first_normal_slot + slots_per_epoch;
    context.warp_to_slot(slot).unwrap();
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &[validator_stake.vote.pubkey()],
            false,
        )
        .await;

    // decrease
    let error = stake_pool_accounts
        .decrease_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            1_000_000_000,
            validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none());

    // warp forward to merge
    slot += slots_per_epoch;
    context.warp_to_slot(slot).unwrap();

    // hijack
    let validator_list = stake_pool_accounts
        .get_validator_list(&mut context.banks_client)
        .await;
    let hijacker = Keypair::new();
    let transient_stake_address = find_transient_stake_program_address(
        &id(),
        &validator_stake.vote.pubkey(),
        &stake_pool_accounts.stake_pool.pubkey(),
        validator_stake.transient_stake_seed,
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
                &[validator_stake.vote.pubkey()],
                0,
                /* no_merge = */ false,
            ),
            system_instruction::transfer(
                &context.payer.pubkey(),
                &transient_stake_address,
                1_000_000_000,
            ),
            stake::instruction::initialize(
                &transient_stake_address,
                &stake::state::Authorized {
                    staker: hijacker.pubkey(),
                    withdrawer: hijacker.pubkey(),
                },
                &stake::state::Lockup::default(),
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

    // activate transient stake account
    delegate_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &transient_stake_address,
        &hijacker,
        &validator_stake.vote.pubkey(),
    )
    .await;

    // Remove works even though transient account is activating
    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &new_authority,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            &destination_stake,
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

    // Check if account was removed from the list of stake accounts
    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    assert_eq!(
        validator_list,
        state::ValidatorList {
            header: state::ValidatorListHeader {
                account_type: state::AccountType::ValidatorList,
                max_validators: stake_pool_accounts.max_validators,
            },
            validators: vec![]
        }
    );
}

#[tokio::test]
async fn fail_not_updated_stake_pool() {} // TODO

#[tokio::test]
async fn fail_with_uninitialized_validator_list_account() {} // TODO
