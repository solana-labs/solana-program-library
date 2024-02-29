#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program::{
        borsh1::try_from_slice_unchecked,
        instruction::InstructionError,
        pubkey::Pubkey,
        stake::{
            self,
            stake_flags::StakeFlags,
            state::{Authorized, Delegation, Lockup, Meta, Stake, StakeStateV2},
        },
    },
    solana_program_test::*,
    solana_sdk::{
        account::{Account, WritableAccount},
        clock::Epoch,
        signature::Signer,
        transaction::TransactionError,
    },
    spl_stake_pool::{
        error::StakePoolError,
        find_stake_program_address, find_transient_stake_program_address, id,
        state::{AccountType, StakeStatus, ValidatorList, ValidatorListHeader, ValidatorStakeInfo},
        MINIMUM_ACTIVE_STAKE,
    },
    std::num::NonZeroU32,
};

async fn setup(
    stake_pool_accounts: &StakePoolAccounts,
    forced_stake: &StakeStateV2,
    voter_pubkey: &Pubkey,
) -> (ProgramTestContext, Option<NonZeroU32>) {
    let mut program_test = program_test();

    let stake_pool_pubkey = stake_pool_accounts.stake_pool.pubkey();
    let (mut stake_pool, mut validator_list) = stake_pool_accounts.state();

    let _ = add_vote_account_with_pubkey(voter_pubkey, &mut program_test);
    let mut data = vec![0; std::mem::size_of::<StakeStateV2>()];
    bincode::serialize_into(&mut data[..], forced_stake).unwrap();

    let stake_account = Account::create(
        TEST_STAKE_AMOUNT + STAKE_ACCOUNT_RENT_EXEMPTION,
        data,
        stake::program::id(),
        false,
        Epoch::default(),
    );

    let raw_validator_seed = 42;
    let validator_seed = NonZeroU32::new(raw_validator_seed);
    let (stake_address, _) =
        find_stake_program_address(&id(), voter_pubkey, &stake_pool_pubkey, validator_seed);
    program_test.add_account(stake_address, stake_account);
    let active_stake_lamports = TEST_STAKE_AMOUNT - MINIMUM_ACTIVE_STAKE;
    // add to validator list
    validator_list.validators.push(ValidatorStakeInfo {
        status: StakeStatus::Active.into(),
        vote_account_address: *voter_pubkey,
        active_stake_lamports: active_stake_lamports.into(),
        transient_stake_lamports: 0.into(),
        last_update_epoch: 0.into(),
        transient_seed_suffix: 0.into(),
        unused: 0.into(),
        validator_seed_suffix: raw_validator_seed.into(),
    });

    stake_pool.total_lamports += active_stake_lamports;
    stake_pool.pool_token_supply += active_stake_lamports;

    add_reserve_stake_account(
        &mut program_test,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.withdraw_authority,
        TEST_STAKE_AMOUNT,
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
        stake_pool_accounts.max_validators,
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

    let context = program_test.start_with_context().await;
    (context, validator_seed)
}

#[tokio::test]
async fn success_update() {
    let stake_pool_accounts = StakePoolAccounts::default();
    let meta = Meta {
        rent_exempt_reserve: STAKE_ACCOUNT_RENT_EXEMPTION,
        authorized: Authorized {
            staker: stake_pool_accounts.withdraw_authority,
            withdrawer: stake_pool_accounts.withdraw_authority,
        },
        lockup: Lockup::default(),
    };
    let voter_pubkey = Pubkey::new_unique();
    let (mut context, validator_seed) = setup(
        &stake_pool_accounts,
        &StakeStateV2::Initialized(meta),
        &voter_pubkey,
    )
    .await;
    let pre_reserve_lamports = context
        .banks_client
        .get_account(stake_pool_accounts.reserve_stake.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let (stake_address, _) = find_stake_program_address(
        &id(),
        &voter_pubkey,
        &stake_pool_accounts.stake_pool.pubkey(),
        validator_seed,
    );
    let validator_stake_lamports = context
        .banks_client
        .get_account(stake_address)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    // update should merge the destaked validator stake account into the reserve
    let error = stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            false,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);
    let post_reserve_lamports = context
        .banks_client
        .get_account(stake_pool_accounts.reserve_stake.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    assert_eq!(
        post_reserve_lamports,
        pre_reserve_lamports + validator_stake_lamports
    );
    // test no more validator stake account
    assert!(context
        .banks_client
        .get_account(stake_address)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn fail_increase() {
    let stake_pool_accounts = StakePoolAccounts::default();
    let meta = Meta {
        rent_exempt_reserve: STAKE_ACCOUNT_RENT_EXEMPTION,
        authorized: Authorized {
            staker: stake_pool_accounts.withdraw_authority,
            withdrawer: stake_pool_accounts.withdraw_authority,
        },
        lockup: Lockup::default(),
    };
    let voter_pubkey = Pubkey::new_unique();
    let (mut context, validator_seed) = setup(
        &stake_pool_accounts,
        &StakeStateV2::Initialized(meta),
        &voter_pubkey,
    )
    .await;
    let (stake_address, _) = find_stake_program_address(
        &id(),
        &voter_pubkey,
        &stake_pool_accounts.stake_pool.pubkey(),
        validator_seed,
    );
    let transient_stake_seed = 0;
    let transient_stake_address = find_transient_stake_program_address(
        &id(),
        &voter_pubkey,
        &stake_pool_accounts.stake_pool.pubkey(),
        transient_stake_seed,
    )
    .0;
    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &transient_stake_address,
            &stake_address,
            &voter_pubkey,
            MINIMUM_ACTIVE_STAKE,
            transient_stake_seed,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::WrongStakeStake as u32)
        )
    );
}

#[tokio::test]
async fn success_remove_validator() {
    let stake_pool_accounts = StakePoolAccounts::default();
    let meta = Meta {
        rent_exempt_reserve: STAKE_ACCOUNT_RENT_EXEMPTION,
        authorized: Authorized {
            staker: stake_pool_accounts.withdraw_authority,
            withdrawer: stake_pool_accounts.withdraw_authority,
        },
        lockup: Lockup::default(),
    };
    let voter_pubkey = Pubkey::new_unique();
    let stake = Stake {
        delegation: Delegation {
            voter_pubkey,
            stake: TEST_STAKE_AMOUNT,
            activation_epoch: 0,
            deactivation_epoch: 0,
            ..Delegation::default()
        },
        credits_observed: 1,
    };
    let (mut context, validator_seed) = setup(
        &stake_pool_accounts,
        &StakeStateV2::Stake(meta, stake, StakeFlags::empty()),
        &voter_pubkey,
    )
    .await;

    // move forward to after deactivation
    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    context.warp_to_slot(first_normal_slot + 1).unwrap();
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            false,
        )
        .await;

    let (stake_address, _) = find_stake_program_address(
        &id(),
        &voter_pubkey,
        &stake_pool_accounts.stake_pool.pubkey(),
        validator_seed,
    );
    let transient_stake_seed = 0;
    let transient_stake_address = find_transient_stake_program_address(
        &id(),
        &voter_pubkey,
        &stake_pool_accounts.stake_pool.pubkey(),
        transient_stake_seed,
    )
    .0;

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

    // Get a new blockhash for the next update to work
    context.get_new_latest_blockhash().await.unwrap();

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
        try_from_slice_unchecked::<ValidatorList>(validator_list.data.as_slice()).unwrap();
    assert_eq!(
        validator_list,
        ValidatorList {
            header: ValidatorListHeader {
                account_type: AccountType::ValidatorList,
                max_validators: stake_pool_accounts.max_validators,
            },
            validators: vec![]
        }
    );

    // Check stake account no longer exists
    let account = context
        .banks_client
        .get_account(stake_address)
        .await
        .unwrap();
    assert!(account.is_none());
}
