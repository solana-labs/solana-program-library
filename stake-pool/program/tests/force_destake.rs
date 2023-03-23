#![allow(clippy::integer_arithmetic)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program::{instruction::InstructionError, pubkey::Pubkey, stake},
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
        state::{StakeStatus, ValidatorStakeInfo},
        MINIMUM_ACTIVE_STAKE,
    },
    std::num::NonZeroU32,
};

async fn setup() -> (
    ProgramTestContext,
    StakePoolAccounts,
    Pubkey,
    Option<NonZeroU32>,
) {
    let mut program_test = program_test();
    let stake_pool_accounts = StakePoolAccounts::default();

    let stake_pool_pubkey = stake_pool_accounts.stake_pool.pubkey();
    let (mut stake_pool, mut validator_list) = stake_pool_accounts.state();

    let voter_pubkey = add_vote_account(&mut program_test);
    let meta = stake::state::Meta {
        rent_exempt_reserve: STAKE_ACCOUNT_RENT_EXEMPTION,
        authorized: stake::state::Authorized {
            staker: stake_pool_accounts.withdraw_authority,
            withdrawer: stake_pool_accounts.withdraw_authority,
        },
        lockup: stake_pool.lockup,
    };

    let stake_account = Account::create(
        TEST_STAKE_AMOUNT + STAKE_ACCOUNT_RENT_EXEMPTION,
        bincode::serialize::<stake::state::StakeState>(&stake::state::StakeState::Initialized(
            meta,
        ))
        .unwrap(),
        stake::program::id(),
        false,
        Epoch::default(),
    );

    let raw_validator_seed = 42;
    let validator_seed = NonZeroU32::new(raw_validator_seed);
    let (stake_address, _) =
        find_stake_program_address(&id(), &voter_pubkey, &stake_pool_pubkey, validator_seed);
    program_test.add_account(stake_address, stake_account);
    let active_stake_lamports = TEST_STAKE_AMOUNT - MINIMUM_ACTIVE_STAKE;
    // add to validator list
    validator_list.validators.push(ValidatorStakeInfo {
        status: StakeStatus::Active,
        vote_account_address: voter_pubkey,
        active_stake_lamports,
        transient_stake_lamports: 0,
        last_update_epoch: 0,
        transient_seed_suffix: 0,
        unused: 0,
        validator_seed_suffix: raw_validator_seed,
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
    (context, stake_pool_accounts, voter_pubkey, validator_seed)
}

#[tokio::test]
async fn success_update() {
    let (mut context, stake_pool_accounts, voter_pubkey, validator_seed) = setup().await;
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
            &[voter_pubkey],
            false,
        )
        .await;
    assert!(error.is_none());
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
    let (mut context, stake_pool_accounts, voter_pubkey, validator_seed) = setup().await;
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
            InstructionError::Custom(StakePoolError::WrongStakeState as u32)
        )
    );
}
