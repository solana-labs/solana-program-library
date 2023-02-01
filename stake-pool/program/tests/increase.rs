#![allow(clippy::integer_arithmetic)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    bincode::deserialize,
    helpers::*,
    solana_program::{clock::Epoch, instruction::InstructionError, pubkey::Pubkey, stake},
    solana_program_test::*,
    solana_sdk::{
        signature::Signer,
        stake::instruction::StakeError,
        transaction::{Transaction, TransactionError},
    },
    spl_stake_pool::{
        error::StakePoolError, find_ephemeral_stake_program_address,
        find_transient_stake_program_address, id, instruction, MINIMUM_RESERVE_LAMPORTS,
    },
    test_case::test_case,
};

async fn setup() -> (
    ProgramTestContext,
    StakePoolAccounts,
    ValidatorStakeAccount,
    u64,
) {
    let mut context = program_test().start_with_context().await;
    let stake_pool_accounts = StakePoolAccounts::default();
    let reserve_lamports = 100_000_000_000 + MINIMUM_RESERVE_LAMPORTS;
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            reserve_lamports,
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

    let current_minimum_delegation = stake_pool_get_minimum_delegation(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>());

    let _deposit_info = simple_deposit_stake(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
        &validator_stake_account,
        current_minimum_delegation * 2 + stake_rent,
    )
    .await
    .unwrap();

    (
        context,
        stake_pool_accounts,
        validator_stake_account,
        reserve_lamports,
    )
}

#[test_case(true; "additional")]
#[test_case(false; "non-additional")]
#[tokio::test]
async fn success(use_additional_instruction: bool) {
    let (mut context, stake_pool_accounts, validator_stake, reserve_lamports) = setup().await;

    // Save reserve stake
    let pre_reserve_stake_account = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await;

    // Check no transient stake
    let transient_account = context
        .banks_client
        .get_account(validator_stake.transient_stake_account)
        .await
        .unwrap();
    assert!(transient_account.is_none());

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>());
    let increase_amount = reserve_lamports - stake_rent - MINIMUM_RESERVE_LAMPORTS;
    let error = stake_pool_accounts
        .increase_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.stake_account,
            &validator_stake.vote.pubkey(),
            increase_amount,
            validator_stake.transient_stake_seed,
            use_additional_instruction,
        )
        .await;
    assert!(error.is_none());

    // Check reserve stake account balance
    let reserve_stake_account = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await;
    let reserve_stake_state =
        deserialize::<stake::state::StakeState>(&reserve_stake_account.data).unwrap();
    assert_eq!(
        pre_reserve_stake_account.lamports - increase_amount - stake_rent,
        reserve_stake_account.lamports
    );
    assert!(reserve_stake_state.delegation().is_none());

    // Check transient stake account state and balance
    let transient_stake_account = get_account(
        &mut context.banks_client,
        &validator_stake.transient_stake_account,
    )
    .await;
    let transient_stake_state =
        deserialize::<stake::state::StakeState>(&transient_stake_account.data).unwrap();
    assert_eq!(
        transient_stake_account.lamports,
        increase_amount + stake_rent
    );
    assert_ne!(
        transient_stake_state.delegation().unwrap().activation_epoch,
        Epoch::MAX
    );
}

#[tokio::test]
async fn fail_with_wrong_withdraw_authority() {
    let (mut context, stake_pool_accounts, validator_stake, reserve_lamports) = setup().await;

    let wrong_authority = Pubkey::new_unique();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::increase_validator_stake(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &wrong_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &stake_pool_accounts.reserve_stake.pubkey(),
            &validator_stake.transient_stake_account,
            &validator_stake.stake_account,
            &validator_stake.vote.pubkey(),
            reserve_lamports / 2,
            validator_stake.transient_stake_seed,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.staker],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = StakePoolError::InvalidProgramAddress as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error"),
    }
}

#[tokio::test]
async fn fail_with_wrong_validator_list() {
    let (mut context, stake_pool_accounts, validator_stake, reserve_lamports) = setup().await;

    let wrong_validator_list = Pubkey::new_unique();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::increase_validator_stake(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &wrong_validator_list,
            &stake_pool_accounts.reserve_stake.pubkey(),
            &validator_stake.transient_stake_account,
            &validator_stake.stake_account,
            &validator_stake.vote.pubkey(),
            reserve_lamports / 2,
            validator_stake.transient_stake_seed,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.staker],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = StakePoolError::InvalidValidatorStakeList as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error"),
    }
}

#[tokio::test]
async fn fail_with_unknown_validator() {
    let (mut context, stake_pool_accounts, _validator_stake, reserve_lamports) = setup().await;

    let unknown_stake = create_unknown_validator_stake(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts.stake_pool.pubkey(),
        0,
    )
    .await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::increase_validator_stake(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &stake_pool_accounts.reserve_stake.pubkey(),
            &unknown_stake.transient_stake_account,
            &unknown_stake.stake_account,
            &unknown_stake.vote.pubkey(),
            reserve_lamports / 2,
            unknown_stake.transient_stake_seed,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.staker],
        context.last_blockhash,
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
            InstructionError::Custom(StakePoolError::ValidatorNotFound as u32)
        )
    );
}

#[test_case(true; "additional")]
#[test_case(false; "non-additional")]
#[tokio::test]
async fn fail_twice_diff_seed(use_additional_instruction: bool) {
    let (mut context, stake_pool_accounts, validator_stake, reserve_lamports) = setup().await;

    let first_increase = reserve_lamports / 3;
    let second_increase = reserve_lamports / 4;
    let error = stake_pool_accounts
        .increase_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.stake_account,
            &validator_stake.vote.pubkey(),
            first_increase,
            validator_stake.transient_stake_seed,
            use_additional_instruction,
        )
        .await;
    assert!(error.is_none());

    let transient_stake_seed = validator_stake.transient_stake_seed * 100;
    let transient_stake_address = find_transient_stake_program_address(
        &id(),
        &validator_stake.vote.pubkey(),
        &stake_pool_accounts.stake_pool.pubkey(),
        transient_stake_seed,
    )
    .0;
    let error = stake_pool_accounts
        .increase_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &transient_stake_address,
            &validator_stake.stake_account,
            &validator_stake.vote.pubkey(),
            second_increase,
            transient_stake_seed,
            use_additional_instruction,
        )
        .await
        .unwrap()
        .unwrap();

    if use_additional_instruction {
        assert_eq!(
            error,
            TransactionError::InstructionError(0, InstructionError::InvalidSeeds)
        );
    } else {
        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(StakePoolError::TransientAccountInUse as u32)
            )
        );
    }
}

#[test_case(true, true, true; "success-all-additional")]
#[test_case(true, false, true; "success-with-additional")]
#[test_case(false, true, false; "fail-without-additional")]
#[test_case(false, false, false; "fail-no-additional")]
#[tokio::test]
async fn twice(success: bool, use_additional_first_time: bool, use_additional_second_time: bool) {
    let (mut context, stake_pool_accounts, validator_stake, reserve_lamports) = setup().await;

    let pre_reserve_stake_account = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await;

    let first_increase = reserve_lamports / 3;
    let second_increase = reserve_lamports / 4;
    let total_increase = first_increase + second_increase;
    let error = stake_pool_accounts
        .increase_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.stake_account,
            &validator_stake.vote.pubkey(),
            first_increase,
            validator_stake.transient_stake_seed,
            use_additional_first_time,
        )
        .await;
    assert!(error.is_none());

    let error = stake_pool_accounts
        .increase_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.stake_account,
            &validator_stake.vote.pubkey(),
            second_increase,
            validator_stake.transient_stake_seed,
            use_additional_second_time,
        )
        .await;

    if success {
        assert!(error.is_none());
        let rent = context.banks_client.get_rent().await.unwrap();
        let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>());
        // no ephemeral account
        let ephemeral_stake = find_ephemeral_stake_program_address(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            0,
        )
        .0;
        let ephemeral_account = context
            .banks_client
            .get_account(ephemeral_stake)
            .await
            .unwrap();
        assert!(ephemeral_account.is_none());
        // Check reserve stake account balance
        let reserve_stake_account = get_account(
            &mut context.banks_client,
            &stake_pool_accounts.reserve_stake.pubkey(),
        )
        .await;
        let reserve_stake_state =
            deserialize::<stake::state::StakeState>(&reserve_stake_account.data).unwrap();
        assert_eq!(
            pre_reserve_stake_account.lamports - total_increase - stake_rent * 2,
            reserve_stake_account.lamports
        );
        assert!(reserve_stake_state.delegation().is_none());

        // Check transient stake account state and balance
        let transient_stake_account = get_account(
            &mut context.banks_client,
            &validator_stake.transient_stake_account,
        )
        .await;
        let transient_stake_state =
            deserialize::<stake::state::StakeState>(&transient_stake_account.data).unwrap();
        assert_eq!(
            transient_stake_account.lamports,
            total_increase + stake_rent * 2
        );
        assert_ne!(
            transient_stake_state.delegation().unwrap().activation_epoch,
            Epoch::MAX
        );

        // marked correctly in the list
        let validator_list = stake_pool_accounts
            .get_validator_list(&mut context.banks_client)
            .await;
        let entry = validator_list.find(&validator_stake.vote.pubkey()).unwrap();
        assert_eq!(
            entry.transient_stake_lamports,
            total_increase + stake_rent * 2
        );
    } else {
        let error = error.unwrap().unwrap();
        match error {
            TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
                let program_error = StakePoolError::TransientAccountInUse as u32;
                assert_eq!(error_index, program_error);
            }
            _ => panic!("Wrong error"),
        }
    }
}

#[test_case(true; "additional")]
#[test_case(false; "non-additional")]
#[tokio::test]
async fn fail_with_small_lamport_amount(use_additional_instruction: bool) {
    let (mut context, stake_pool_accounts, validator_stake, _reserve_lamports) = setup().await;

    let current_minimum_delegation = stake_pool_get_minimum_delegation(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;

    let error = stake_pool_accounts
        .increase_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.stake_account,
            &validator_stake.vote.pubkey(),
            current_minimum_delegation - 1,
            validator_stake.transient_stake_seed,
            use_additional_instruction,
        )
        .await
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = StakeError::InsufficientDelegation as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error"),
    }
}

#[test_case(true; "additional")]
#[test_case(false; "non-additional")]
#[tokio::test]
async fn fail_overdraw_reserve(use_additional_instruction: bool) {
    let (mut context, stake_pool_accounts, validator_stake, reserve_lamports) = setup().await;

    let error = stake_pool_accounts
        .increase_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.stake_account,
            &validator_stake.vote.pubkey(),
            reserve_lamports,
            validator_stake.transient_stake_seed,
            use_additional_instruction,
        )
        .await
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::InsufficientFunds) => {}
        _ => panic!("Wrong error occurs while overdrawing reserve stake"),
    }
}

#[tokio::test]
async fn fail_additional_with_decreasing() {
    let (mut context, stake_pool_accounts, validator_stake, reserve_lamports) = setup().await;

    let current_minimum_delegation = stake_pool_get_minimum_delegation(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>());

    // warp forward to activation
    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    context.warp_to_slot(first_normal_slot).unwrap();
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
            &[validator_stake.vote.pubkey()],
            false,
        )
        .await;

    let error = stake_pool_accounts
        .decrease_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            current_minimum_delegation + stake_rent,
            validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none());

    let error = stake_pool_accounts
        .increase_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.stake_account,
            &validator_stake.vote.pubkey(),
            reserve_lamports / 2,
            validator_stake.transient_stake_seed,
            true,
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
async fn fail_with_force_destaked_validator() {}
