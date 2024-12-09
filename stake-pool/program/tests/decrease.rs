#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::items_after_test_module)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    assert_matches::assert_matches,
    bincode::deserialize,
    helpers::*,
    solana_program::{clock::Epoch, instruction::InstructionError, pubkey::Pubkey, stake},
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
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
    DepositStakeAccount,
    u64,
    u64,
) {
    let mut context = program_test().start_with_context().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());
    let current_minimum_delegation = stake_pool_get_minimum_delegation(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;

    let stake_pool_accounts = StakePoolAccounts::default();
    let reserve_lamports = MINIMUM_RESERVE_LAMPORTS + stake_rent + current_minimum_delegation;
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

    let decrease_lamports = (current_minimum_delegation + stake_rent) * 3;
    let deposit_info = simple_deposit_stake(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
        &validator_stake_account,
        decrease_lamports,
    )
    .await
    .unwrap();

    (
        context,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        decrease_lamports,
        reserve_lamports + stake_rent,
    )
}

#[test_case(DecreaseInstruction::Additional; "additional")]
#[test_case(DecreaseInstruction::Reserve; "reserve")]
#[test_case(DecreaseInstruction::Deprecated; "deprecated")]
#[tokio::test]
async fn success(instruction_type: DecreaseInstruction) {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake,
        _deposit_info,
        decrease_lamports,
        reserve_lamports,
    ) = setup().await;

    // Save validator stake
    let pre_validator_stake_account =
        get_account(&mut context.banks_client, &validator_stake.stake_account).await;

    // Check no transient stake
    let transient_account = context
        .banks_client
        .get_account(validator_stake.transient_stake_account)
        .await
        .unwrap();
    assert!(transient_account.is_none());

    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            decrease_lamports,
            validator_stake.transient_stake_seed,
            instruction_type,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // Check validator stake account balance
    let validator_stake_account =
        get_account(&mut context.banks_client, &validator_stake.stake_account).await;
    let validator_stake_state =
        deserialize::<stake::state::StakeStateV2>(&validator_stake_account.data).unwrap();
    assert_eq!(
        pre_validator_stake_account.lamports - decrease_lamports,
        validator_stake_account.lamports
    );
    assert_eq!(
        validator_stake_state
            .delegation()
            .unwrap()
            .deactivation_epoch,
        Epoch::MAX
    );

    // Check transient stake account state and balance
    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());

    let transient_stake_account = get_account(
        &mut context.banks_client,
        &validator_stake.transient_stake_account,
    )
    .await;
    let transient_stake_state =
        deserialize::<stake::state::StakeStateV2>(&transient_stake_account.data).unwrap();
    let transient_lamports = decrease_lamports + stake_rent;
    assert_eq!(transient_stake_account.lamports, transient_lamports);
    let reserve_lamports = if instruction_type == DecreaseInstruction::Deprecated {
        reserve_lamports
    } else {
        reserve_lamports - stake_rent
    };
    let reserve_stake_account = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await;
    assert_eq!(reserve_stake_account.lamports, reserve_lamports);
    assert_ne!(
        transient_stake_state
            .delegation()
            .unwrap()
            .deactivation_epoch,
        Epoch::MAX
    );
}

#[tokio::test]
async fn fail_with_wrong_withdraw_authority() {
    let (context, stake_pool_accounts, validator_stake, _deposit_info, decrease_lamports, _) =
        setup().await;

    let wrong_authority = Pubkey::new_unique();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::decrease_validator_stake_with_reserve(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &wrong_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &stake_pool_accounts.reserve_stake.pubkey(),
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            decrease_lamports,
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
    let (context, mut stake_pool_accounts, validator_stake, _deposit_info, decrease_lamports, _) =
        setup().await;

    stake_pool_accounts.validator_list = Keypair::new();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::decrease_validator_stake_with_reserve(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &stake_pool_accounts.reserve_stake.pubkey(),
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            decrease_lamports,
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

    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::InvalidValidatorStakeList as u32)
        )
    );
}

#[tokio::test]
async fn fail_with_unknown_validator() {
    let (mut context, stake_pool_accounts, _validator_stake, _deposit_info, decrease_lamports, _) =
        setup().await;

    let unknown_stake = create_unknown_validator_stake(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts.stake_pool.pubkey(),
        0,
    )
    .await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::decrease_validator_stake_with_reserve(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.staker.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &stake_pool_accounts.reserve_stake.pubkey(),
            &unknown_stake.stake_account,
            &unknown_stake.transient_stake_account,
            decrease_lamports,
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

#[test_case(DecreaseInstruction::Additional; "additional")]
#[test_case(DecreaseInstruction::Reserve; "reserve")]
#[test_case(DecreaseInstruction::Deprecated; "deprecated")]
#[tokio::test]
async fn fail_twice_diff_seed(instruction_type: DecreaseInstruction) {
    let (mut context, stake_pool_accounts, validator_stake, _deposit_info, decrease_lamports, _) =
        setup().await;

    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            decrease_lamports / 3,
            validator_stake.transient_stake_seed,
            instruction_type,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let transient_stake_seed = validator_stake.transient_stake_seed * 100;
    let transient_stake_address = find_transient_stake_program_address(
        &id(),
        &validator_stake.vote.pubkey(),
        &stake_pool_accounts.stake_pool.pubkey(),
        transient_stake_seed,
    )
    .0;
    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &transient_stake_address,
            decrease_lamports / 2,
            transient_stake_seed,
            instruction_type,
        )
        .await
        .unwrap()
        .unwrap();
    if instruction_type == DecreaseInstruction::Additional {
        assert_eq!(
            error,
            TransactionError::InstructionError(0, InstructionError::InvalidSeeds)
        );
    } else {
        assert_matches!(
            error,
            TransactionError::InstructionError(
                _,
                InstructionError::Custom(code)
            ) if code == StakePoolError::TransientAccountInUse as u32
        );
    }
}

#[test_case(true, DecreaseInstruction::Additional, DecreaseInstruction::Additional; "success-all-additional")]
#[test_case(true, DecreaseInstruction::Reserve, DecreaseInstruction::Additional; "success-with-additional")]
#[test_case(false, DecreaseInstruction::Additional, DecreaseInstruction::Reserve; "fail-without-additional")]
#[test_case(false, DecreaseInstruction::Reserve, DecreaseInstruction::Reserve; "fail-no-additional")]
#[tokio::test]
async fn twice(
    success: bool,
    first_instruction: DecreaseInstruction,
    second_instruction: DecreaseInstruction,
) {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake,
        _deposit_info,
        decrease_lamports,
        reserve_lamports,
    ) = setup().await;

    let pre_stake_account =
        get_account(&mut context.banks_client, &validator_stake.stake_account).await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());

    let first_decrease = decrease_lamports / 3;
    let second_decrease = decrease_lamports / 2;
    let total_decrease = first_decrease + second_decrease;
    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            first_decrease,
            validator_stake.transient_stake_seed,
            first_instruction,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            second_decrease,
            validator_stake.transient_stake_seed,
            second_instruction,
        )
        .await;

    if success {
        assert!(error.is_none(), "{:?}", error);
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

        // Check validator stake account balance
        let stake_account =
            get_account(&mut context.banks_client, &validator_stake.stake_account).await;
        let stake_state = deserialize::<stake::state::StakeStateV2>(&stake_account.data).unwrap();
        assert_eq!(
            pre_stake_account.lamports - total_decrease,
            stake_account.lamports
        );
        assert_eq!(
            stake_state.delegation().unwrap().deactivation_epoch,
            Epoch::MAX
        );

        // Check transient stake account state and balance
        let transient_stake_account = get_account(
            &mut context.banks_client,
            &validator_stake.transient_stake_account,
        )
        .await;
        let transient_stake_state =
            deserialize::<stake::state::StakeStateV2>(&transient_stake_account.data).unwrap();
        let mut transient_lamports = total_decrease + stake_rent;
        if second_instruction == DecreaseInstruction::Additional {
            transient_lamports += stake_rent;
        }
        assert_eq!(transient_stake_account.lamports, transient_lamports);
        assert_ne!(
            transient_stake_state
                .delegation()
                .unwrap()
                .deactivation_epoch,
            Epoch::MAX
        );

        // marked correctly in the list
        let validator_list = stake_pool_accounts
            .get_validator_list(&mut context.banks_client)
            .await;
        let entry = validator_list.find(&validator_stake.vote.pubkey()).unwrap();
        assert_eq!(
            u64::from(entry.transient_stake_lamports),
            transient_lamports
        );

        // reserve deducted properly
        let mut reserve_lamports = reserve_lamports - stake_rent;
        if second_instruction == DecreaseInstruction::Additional {
            reserve_lamports -= stake_rent;
        }
        let reserve_stake_account = get_account(
            &mut context.banks_client,
            &stake_pool_accounts.reserve_stake.pubkey(),
        )
        .await;
        assert_eq!(reserve_stake_account.lamports, reserve_lamports);
    } else {
        let error = error.unwrap().unwrap();
        assert_matches!(
            error,
            TransactionError::InstructionError(
                _,
                InstructionError::Custom(code)
            ) if code == StakePoolError::TransientAccountInUse as u32
        );
    }
}

#[test_case(DecreaseInstruction::Additional; "additional")]
#[test_case(DecreaseInstruction::Reserve; "reserve")]
#[test_case(DecreaseInstruction::Deprecated; "deprecated")]
#[tokio::test]
async fn fail_with_small_lamport_amount(instruction_type: DecreaseInstruction) {
    let (mut context, stake_pool_accounts, validator_stake, _deposit_info, _decrease_lamports, _) =
        setup().await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let lamports = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());

    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            lamports,
            validator_stake.transient_stake_seed,
            instruction_type,
        )
        .await
        .unwrap()
        .unwrap();

    assert_matches!(
        error,
        TransactionError::InstructionError(_, InstructionError::AccountNotRentExempt)
    );
}

#[test_case(DecreaseInstruction::Additional; "additional")]
#[test_case(DecreaseInstruction::Reserve; "reserve")]
#[test_case(DecreaseInstruction::Deprecated; "deprecated")]
#[tokio::test]
async fn fail_big_overdraw(instruction_type: DecreaseInstruction) {
    let (mut context, stake_pool_accounts, validator_stake, deposit_info, _decrease_lamports, _) =
        setup().await;

    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            deposit_info.stake_lamports * 1_000_000,
            validator_stake.transient_stake_seed,
            instruction_type,
        )
        .await
        .unwrap()
        .unwrap();

    assert_matches!(
        error,
        TransactionError::InstructionError(_, InstructionError::InsufficientFunds)
    );
}

#[test_case(DecreaseInstruction::Additional; "additional")]
#[test_case(DecreaseInstruction::Reserve; "reserve")]
#[test_case(DecreaseInstruction::Deprecated; "deprecated")]
#[tokio::test]
async fn fail_overdraw(instruction_type: DecreaseInstruction) {
    let (mut context, stake_pool_accounts, validator_stake, deposit_info, _decrease_lamports, _) =
        setup().await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());

    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            deposit_info.stake_lamports + stake_rent + 1,
            validator_stake.transient_stake_seed,
            instruction_type,
        )
        .await
        .unwrap()
        .unwrap();

    assert_matches!(
        error,
        TransactionError::InstructionError(_, InstructionError::InsufficientFunds)
    );
}

#[tokio::test]
async fn fail_additional_with_increasing() {
    let (mut context, stake_pool_accounts, validator_stake, _, decrease_lamports, _) =
        setup().await;

    let current_minimum_delegation = stake_pool_get_minimum_delegation(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;

    // warp forward to activation
    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    context.warp_to_slot(first_normal_slot + 1).unwrap();
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

    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.stake_account,
            &validator_stake.vote.pubkey(),
            current_minimum_delegation,
            validator_stake.transient_stake_seed,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            decrease_lamports / 2,
            validator_stake.transient_stake_seed,
            DecreaseInstruction::Additional,
        )
        .await
        .unwrap()
        .unwrap();

    assert_matches!(
        error,
        TransactionError::InstructionError(
            _,
            InstructionError::Custom(code)
        ) if code == StakePoolError::WrongStakeStake as u32
    );
}
