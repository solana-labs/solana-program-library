#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    bincode::deserialize,
    helpers::*,
    solana_program::{
        borsh1::try_from_slice_unchecked,
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
        MINIMUM_RESERVE_LAMPORTS,
    },
    std::num::NonZeroU32,
};

async fn setup() -> (ProgramTestContext, StakePoolAccounts, ValidatorStakeAccount) {
    let mut context = program_test().start_with_context().await;
    let stake_pool_accounts = StakePoolAccounts::default();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            10_000_000_000 + MINIMUM_RESERVE_LAMPORTS,
        )
        .await
        .unwrap();

    let validator_stake = ValidatorStakeAccount::new(
        &stake_pool_accounts.stake_pool.pubkey(),
        NonZeroU32::new(u32::MAX),
        u64::MAX,
    );
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
            validator_stake.validator_stake_seed,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);
    (context, stake_pool_accounts, validator_stake)
}

#[tokio::test]
async fn success() {
    let (mut context, stake_pool_accounts, validator_stake) = setup().await;

    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let error = stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            false,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

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
}

#[tokio::test]
async fn fail_with_wrong_stake_program_id() {
    let (context, stake_pool_accounts, validator_stake) = setup().await;

    let wrong_stake_program = Pubkey::new_unique();

    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.staker.pubkey(), true),
        AccountMeta::new_readonly(stake_pool_accounts.withdraw_authority, false),
        AccountMeta::new(stake_pool_accounts.validator_list.pubkey(), false),
        AccountMeta::new(validator_stake.stake_account, false),
        AccountMeta::new_readonly(validator_stake.transient_stake_account, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(wrong_stake_program, false),
    ];
    let instruction = Instruction {
        program_id: id(),
        accounts,
        data: borsh::to_vec(&instruction::StakePoolInstruction::RemoveValidatorFromPool).unwrap(),
    };

    let mut transaction =
        Transaction::new_with_payer(&[instruction], Some(&context.payer.pubkey()));
    transaction.sign(
        &[&context.payer, &stake_pool_accounts.staker],
        context.last_blockhash,
    );
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
    let (context, stake_pool_accounts, validator_stake) = setup().await;

    let wrong_validator_list = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::remove_validator_from_pool(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &wrong_validator_list.pubkey(),
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
        )],
        Some(&context.payer.pubkey()),
    );
    transaction.sign(
        &[&context.payer, &stake_pool_accounts.staker],
        context.last_blockhash,
    );
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
async fn success_at_large_value() {
    let (mut context, stake_pool_accounts, validator_stake) = setup().await;

    let current_minimum_delegation = stake_pool_get_minimum_delegation(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;

    let threshold_amount = current_minimum_delegation * 1_000;
    let _ = simple_deposit_stake(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
        &validator_stake,
        threshold_amount,
    )
    .await
    .unwrap();

    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);
}

#[tokio::test]
async fn fail_double_remove() {
    let (mut context, stake_pool_accounts, validator_stake) = setup().await;

    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let error = stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            false,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();

    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::BorshIoError("Unknown".to_string())
        )
    );
}

#[tokio::test]
async fn fail_wrong_staker() {
    let (context, stake_pool_accounts, validator_stake) = setup().await;

    let malicious = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::remove_validator_from_pool(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &malicious.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
        )],
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer, &malicious], context.last_blockhash);
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
    let (context, stake_pool_accounts, validator_stake) = setup().await;

    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.staker.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.withdraw_authority, false),
        AccountMeta::new(stake_pool_accounts.validator_list.pubkey(), false),
        AccountMeta::new(validator_stake.stake_account, false),
        AccountMeta::new_readonly(validator_stake.transient_stake_account, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];
    let instruction = Instruction {
        program_id: id(),
        accounts,
        data: borsh::to_vec(&instruction::StakePoolInstruction::RemoveValidatorFromPool).unwrap(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
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
async fn success_with_activating_transient_stake() {
    let (mut context, stake_pool_accounts, validator_stake) = setup().await;

    // increase the validator stake
    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.stake_account,
            &validator_stake.vote.pubkey(),
            2_000_000_000,
            validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // transient stake should be inactive now
    let stake = get_account(
        &mut context.banks_client,
        &validator_stake.transient_stake_account,
    )
    .await;
    let stake_state = deserialize::<stake::state::StakeStateV2>(&stake.data).unwrap();
    assert_ne!(
        stake_state.stake().unwrap().delegation.deactivation_epoch,
        u64::MAX
    );
}

#[tokio::test]
async fn success_with_deactivating_transient_stake() {
    let (mut context, stake_pool_accounts, validator_stake) = setup().await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());
    let current_minimum_delegation = stake_pool_get_minimum_delegation(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;
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
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            TEST_STAKE_AMOUNT + stake_rent,
            validator_stake.transient_stake_seed,
            DecreaseInstruction::Reserve,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

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
        &stake_pool_accounts.token_program_id,
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
            status: state::StakeStatus::DeactivatingAll.into(),
            vote_account_address: validator_stake.vote.pubkey(),
            last_update_epoch: 0.into(),
            active_stake_lamports: (stake_rent + current_minimum_delegation).into(),
            transient_stake_lamports: (TEST_STAKE_AMOUNT + stake_rent * 2).into(),
            transient_seed_suffix: validator_stake.transient_stake_seed.into(),
            unused: 0.into(),
            validator_seed_suffix: validator_stake
                .validator_stake_seed
                .map(|s| s.get())
                .unwrap_or(0)
                .into(),
        }],
    };
    assert_eq!(validator_list, expected_list);

    // Update will merge since activation and deactivation were in the same epoch
    let error = stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            false,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

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
        validators: vec![],
    };
    assert_eq!(validator_list, expected_list);
}

#[tokio::test]
async fn success_resets_preferred_validator() {
    let (mut context, stake_pool_accounts, validator_stake) = setup().await;

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
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let error = stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            false,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

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
}

#[tokio::test]
async fn success_with_hijacked_transient_account() {
    let (mut context, stake_pool_accounts, validator_stake) = setup().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());
    let current_minimum_delegation = stake_pool_get_minimum_delegation(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;
    let increase_amount = current_minimum_delegation + stake_rent;

    // increase stake on validator
    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.stake_account,
            &validator_stake.vote.pubkey(),
            increase_amount,
            validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // warp forward to merge
    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    let mut slot = first_normal_slot + slots_per_epoch + 1;
    context.warp_to_slot(slot).unwrap();
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            false,
        )
        .await;

    // decrease
    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            increase_amount,
            validator_stake.transient_stake_seed,
            DecreaseInstruction::Reserve,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

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
                current_minimum_delegation + stake_rent,
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
    assert!(error.is_none(), "{:?}", error);

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
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // warp forward to merge
    slot += slots_per_epoch;
    context.warp_to_slot(slot).unwrap();

    let error = stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            false,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

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
