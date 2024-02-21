#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::items_after_test_module)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    bincode::deserialize,
    helpers::*,
    solana_program::{
        borsh1::try_from_slice_unchecked, instruction::InstructionError, pubkey::Pubkey, stake,
    },
    solana_program_test::*,
    solana_sdk::{signature::Signer, transaction::TransactionError},
    spl_stake_pool::{error::StakePoolError, instruction, state},
    test_case::test_case,
};

#[tokio::test]
async fn fail_remove_validator() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        _,
    ) = setup_for_withdraw(spl_token::id(), STAKE_ACCOUNT_RENT_EXEMPTION).await;

    // decrease a little stake, not all
    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            deposit_info.stake_lamports / 2,
            validator_stake.transient_stake_seed,
            DecreaseInstruction::Reserve,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // warp forward to deactivation
    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    context.warp_to_slot(first_normal_slot + 1).unwrap();

    // update to merge deactivated stake into reserve
    let error = stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            false,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // Withdraw entire account, fail because some stake left
    let validator_stake_account =
        get_account(&mut context.banks_client, &validator_stake.stake_account).await;
    let remaining_lamports = validator_stake_account.lamports;
    let new_user_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake.stake_account,
            &new_user_authority,
            remaining_lamports,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::StakeLamportsNotEqualToMinimum as u32)
        )
    );
}

#[test_case(0; "equal")]
#[test_case(5; "big")]
#[test_case(11; "bigger")]
#[test_case(29; "biggest")]
#[tokio::test]
async fn success_remove_validator(multiple: u64) {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        _,
    ) = setup_for_withdraw(spl_token::id(), STAKE_ACCOUNT_RENT_EXEMPTION).await;

    // make pool tokens very valuable, so it isn't possible to exactly get down to
    // the minimum
    transfer(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts.reserve_stake.pubkey(),
        deposit_info.stake_lamports * multiple, // each pool token is worth more than one lamport
    )
    .await;
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            false,
        )
        .await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());
    let stake_pool = stake_pool_accounts
        .get_stake_pool(&mut context.banks_client)
        .await;
    let lamports_per_pool_token = stake_pool.get_lamports_per_pool_token().unwrap();

    // decrease all of stake except for lamports_per_pool_token lamports, must be
    // withdrawable
    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            deposit_info.stake_lamports + stake_rent - lamports_per_pool_token,
            validator_stake.transient_stake_seed,
            DecreaseInstruction::Reserve,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // warp forward to deactivation
    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    context.warp_to_slot(first_normal_slot + 1).unwrap();

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();

    // update to merge deactivated stake into reserve
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            false,
        )
        .await;

    let validator_stake_account =
        get_account(&mut context.banks_client, &validator_stake.stake_account).await;
    let remaining_lamports = validator_stake_account.lamports;
    let stake_minimum_delegation =
        stake_get_minimum_delegation(&mut context.banks_client, &context.payer, &last_blockhash)
            .await;
    // make sure it's actually more than the minimum
    assert!(remaining_lamports > stake_rent + stake_minimum_delegation);

    // round up to force one more pool token if needed
    let pool_tokens_post_fee =
        (remaining_lamports * stake_pool.pool_token_supply + stake_pool.total_lamports - 1)
            / stake_pool.total_lamports;
    let new_user_authority = Pubkey::new_unique();
    let pool_tokens = stake_pool_accounts.calculate_inverse_withdrawal_fee(pool_tokens_post_fee);
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake.stake_account,
            &new_user_authority,
            pool_tokens,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // Check validator stake account gone
    let validator_stake_account = context
        .banks_client
        .get_account(validator_stake.stake_account)
        .await
        .unwrap();
    assert!(validator_stake_account.is_none());

    // Check user recipient stake account balance
    let user_stake_recipient_account =
        get_account(&mut context.banks_client, &user_stake_recipient.pubkey()).await;
    assert_eq!(
        user_stake_recipient_account.lamports,
        remaining_lamports + stake_rent
    );

    // Check that cleanup happens correctly
    stake_pool_accounts
        .cleanup_removed_validator_entries(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
        )
        .await;

    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    let validator_stake_item = validator_list.find(&validator_stake.vote.pubkey());
    assert!(validator_stake_item.is_none());
}

#[tokio::test]
async fn fail_with_reserve() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup_for_withdraw(spl_token::id(), STAKE_ACCOUNT_RENT_EXEMPTION).await;

    // decrease a little stake, not all
    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            deposit_info.stake_lamports / 2,
            validator_stake.transient_stake_seed,
            DecreaseInstruction::Reserve,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // warp forward to deactivation
    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    context.warp_to_slot(first_normal_slot + 1).unwrap();

    // update to merge deactivated stake into reserve
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            false,
        )
        .await;

    // Withdraw directly from reserve, fail because some stake left
    let new_user_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &stake_pool_accounts.reserve_stake.pubkey(),
            &new_user_authority,
            tokens_to_burn,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::StakeLamportsNotEqualToMinimum as u32)
        )
    );
}

#[tokio::test]
async fn success_with_reserve() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        _,
    ) = setup_for_withdraw(spl_token::id(), STAKE_ACCOUNT_RENT_EXEMPTION).await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());

    // decrease all of stake
    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            deposit_info.stake_lamports + stake_rent,
            validator_stake.transient_stake_seed,
            DecreaseInstruction::Reserve,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // warp forward to deactivation
    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    context.warp_to_slot(first_normal_slot + 1).unwrap();

    // update to merge deactivated stake into reserve
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            false,
        )
        .await;

    // now it works
    let new_user_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &stake_pool_accounts.reserve_stake.pubkey(),
            &new_user_authority,
            deposit_info.pool_tokens,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // first and only deposit, lamports:pool 1:1
    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool =
        try_from_slice_unchecked::<state::StakePool>(stake_pool.data.as_slice()).unwrap();
    // the entire deposit is actually stake since it isn't activated, so only
    // the stake deposit fee is charged
    let deposit_fee = stake_pool
        .calc_pool_tokens_stake_deposit_fee(stake_rent + deposit_info.stake_lamports)
        .unwrap();
    assert_eq!(
        deposit_info.stake_lamports + stake_rent - deposit_fee,
        deposit_info.pool_tokens,
        "stake {} rent {} deposit fee {} pool tokens {}",
        deposit_info.stake_lamports,
        stake_rent,
        deposit_fee,
        deposit_info.pool_tokens
    );

    let withdrawal_fee = stake_pool_accounts.calculate_withdrawal_fee(deposit_info.pool_tokens);

    // Check tokens used
    let user_token_balance = get_token_balance(
        &mut context.banks_client,
        &deposit_info.pool_account.pubkey(),
    )
    .await;
    assert_eq!(user_token_balance, 0);

    // Check reserve stake account balance
    let reserve_stake_account = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await;
    let stake_state =
        deserialize::<stake::state::StakeStateV2>(&reserve_stake_account.data).unwrap();
    let meta = stake_state.meta().unwrap();
    assert_eq!(
        meta.rent_exempt_reserve + withdrawal_fee + deposit_fee + stake_rent,
        reserve_stake_account.lamports
    );

    // Check user recipient stake account balance
    let user_stake_recipient_account =
        get_account(&mut context.banks_client, &user_stake_recipient.pubkey()).await;
    assert_eq!(
        user_stake_recipient_account.lamports,
        deposit_info.stake_lamports + stake_rent * 2 - withdrawal_fee - deposit_fee
    );
}

#[tokio::test]
async fn success_with_empty_preferred_withdraw() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup_for_withdraw(spl_token::id(), 0).await;

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
            instruction::PreferredValidatorType::Withdraw,
            Some(preferred_validator.vote.pubkey()),
        )
        .await;

    // preferred is empty, withdrawing from non-preferred works
    let new_authority = Pubkey::new_unique();
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
            tokens_to_burn / 2,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);
}

#[tokio::test]
async fn success_and_fail_with_preferred_withdraw() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup_for_withdraw(spl_token::id(), 0).await;

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();

    let preferred_validator = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts,
        None,
    )
    .await;

    stake_pool_accounts
        .set_preferred_validator(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            instruction::PreferredValidatorType::Withdraw,
            Some(preferred_validator.vote.pubkey()),
        )
        .await;

    let _preferred_deposit = simple_deposit_stake(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts,
        &preferred_validator,
        TEST_STAKE_AMOUNT,
    )
    .await
    .unwrap();

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake.stake_account,
            &new_authority,
            tokens_to_burn / 2 + 1,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::IncorrectWithdrawVoteAddress as u32)
        )
    );

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();

    // success from preferred
    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &preferred_validator.stake_account,
            &new_authority,
            tokens_to_burn / 2,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);
}

#[tokio::test]
async fn fail_withdraw_from_transient() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_withdraw,
    ) = setup_for_withdraw(spl_token::id(), STAKE_ACCOUNT_RENT_EXEMPTION).await;

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();

    // add a preferred withdraw validator, keep it empty, to be sure that this works
    let preferred_validator = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts,
        None,
    )
    .await;

    stake_pool_accounts
        .set_preferred_validator(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            instruction::PreferredValidatorType::Withdraw,
            Some(preferred_validator.vote.pubkey()),
        )
        .await;

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());

    // decrease to minimum stake + 2 lamports
    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &validator_stake_account.stake_account,
            &validator_stake_account.transient_stake_account,
            deposit_info.stake_lamports + stake_rent - 2,
            validator_stake_account.transient_stake_seed,
            DecreaseInstruction::Reserve,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // fail withdrawing from transient, still a lamport in the validator stake
    // account
    let new_user_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake_account.transient_stake_account,
            &new_user_authority,
            tokens_to_withdraw,
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
}

#[tokio::test]
async fn success_withdraw_from_transient() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_withdraw,
    ) = setup_for_withdraw(spl_token::id(), STAKE_ACCOUNT_RENT_EXEMPTION).await;

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();

    // add a preferred withdraw validator, keep it empty, to be sure that this works
    let preferred_validator = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts,
        None,
    )
    .await;

    stake_pool_accounts
        .set_preferred_validator(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            instruction::PreferredValidatorType::Withdraw,
            Some(preferred_validator.vote.pubkey()),
        )
        .await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();

    // decrease all of stake
    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &validator_stake_account.stake_account,
            &validator_stake_account.transient_stake_account,
            deposit_info.stake_lamports + stake_rent,
            validator_stake_account.transient_stake_seed,
            DecreaseInstruction::Reserve,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // nothing left in the validator stake account (or any others), so withdrawing
    // from the transient account is ok!
    let new_user_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake_account.transient_stake_account,
            &new_user_authority,
            tokens_to_withdraw / 2,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);
}

#[tokio::test]
async fn success_with_small_preferred_withdraw() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup_for_withdraw(spl_token::id(), 0).await;

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();

    // make pool tokens very valuable, so it isn't possible to exactly get down to
    // the minimum
    transfer(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts.reserve_stake.pubkey(),
        deposit_info.stake_lamports * 5, // each pool token is worth more than one lamport
    )
    .await;
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            false,
        )
        .await;

    let preferred_validator = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts,
        None,
    )
    .await;

    stake_pool_accounts
        .set_preferred_validator(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            instruction::PreferredValidatorType::Withdraw,
            Some(preferred_validator.vote.pubkey()),
        )
        .await;

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();

    // add a tiny bit of stake, less than lamports per pool token to preferred
    // validator
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_exempt = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());
    let stake_minimum_delegation =
        stake_get_minimum_delegation(&mut context.banks_client, &context.payer, &last_blockhash)
            .await;
    let minimum_lamports = stake_minimum_delegation + rent_exempt;

    simple_deposit_stake(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts,
        &preferred_validator,
        stake_minimum_delegation + 1, // stake_rent gets deposited too
    )
    .await
    .unwrap();

    // decrease all stake except for 1 lamport
    let error = stake_pool_accounts
        .decrease_validator_stake_either(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &preferred_validator.stake_account,
            &preferred_validator.transient_stake_account,
            minimum_lamports,
            preferred_validator.transient_stake_seed,
            DecreaseInstruction::Reserve,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // warp forward to deactivation
    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    context.warp_to_slot(first_normal_slot + 1).unwrap();

    // update to merge deactivated stake into reserve
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            false,
        )
        .await;

    // withdraw from preferred fails
    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &preferred_validator.stake_account,
            &new_authority,
            1,
        )
        .await;
    assert!(error.is_some());

    // preferred is empty, withdrawing from non-preferred works
    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake.stake_account,
            &new_authority,
            tokens_to_burn / 6,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);
}
