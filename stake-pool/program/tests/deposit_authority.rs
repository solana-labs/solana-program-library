#![cfg(feature = "test-bpf")]

mod helpers;

use {
    helpers::*,
    solana_program::{instruction::InstructionError, stake},
    solana_program_test::*,
    solana_sdk::{
        borsh::try_from_slice_unchecked,
        signature::{Keypair, Signer},
        transaction::TransactionError,
    },
    spl_stake_pool::{error::StakePoolError, state::StakePool},
};

#[tokio::test]
async fn success_initialize() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let deposit_authority = Keypair::new();
    let stake_pool_accounts = StakePoolAccounts::new_with_deposit_authority(deposit_authority);
    let deposit_authority = stake_pool_accounts.stake_deposit_authority;
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    // Stake pool now exists
    let stake_pool_account =
        get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool =
        try_from_slice_unchecked::<StakePool>(stake_pool_account.data.as_slice()).unwrap();
    assert_eq!(stake_pool.stake_deposit_authority, deposit_authority);
    assert_eq!(stake_pool.sol_deposit_authority.unwrap(), deposit_authority);
}

#[tokio::test]
async fn success_deposit() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_deposit_authority = Keypair::new();
    let stake_pool_accounts =
        StakePoolAccounts::new_with_deposit_authority(stake_deposit_authority);
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let validator_stake_account = simple_add_validator_to_pool(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let user = Keypair::new();
    let user_stake = Keypair::new();
    let lockup = stake::state::Lockup::default();
    let authorized = stake::state::Authorized {
        staker: user.pubkey(),
        withdrawer: user.pubkey(),
    };

    let _stake_lamports = create_independent_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake,
        &authorized,
        &lockup,
        TEST_STAKE_AMOUNT,
    )
    .await;

    delegate_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake.pubkey(),
        &user,
        &validator_stake_account.vote.pubkey(),
    )
    .await;

    // make pool token account
    let user_pool_account = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_pool_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &user.pubkey(),
    )
    .await
    .unwrap();

    let error = stake_pool_accounts
        .deposit_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.pubkey(),
            &user_pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &user,
        )
        .await;
    assert!(error.is_none());
}

#[tokio::test]
async fn fail_deposit_without_authority_signature() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_deposit_authority = Keypair::new();
    let mut stake_pool_accounts =
        StakePoolAccounts::new_with_deposit_authority(stake_deposit_authority);
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let validator_stake_account = simple_add_validator_to_pool(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let user = Keypair::new();
    let user_stake = Keypair::new();
    let lockup = stake::state::Lockup::default();
    let authorized = stake::state::Authorized {
        staker: user.pubkey(),
        withdrawer: user.pubkey(),
    };

    let _stake_lamports = create_independent_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake,
        &authorized,
        &lockup,
        TEST_STAKE_AMOUNT,
    )
    .await;

    delegate_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake.pubkey(),
        &user,
        &validator_stake_account.vote.pubkey(),
    )
    .await;

    // make pool token account
    let user_pool_account = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_pool_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &user.pubkey(),
    )
    .await
    .unwrap();

    let wrong_depositor = Keypair::new();
    stake_pool_accounts.stake_deposit_authority = wrong_depositor.pubkey();
    stake_pool_accounts.stake_deposit_authority_keypair = Some(wrong_depositor);

    let error = stake_pool_accounts
        .deposit_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.pubkey(),
            &user_pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &user,
        )
        .await
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            assert_eq!(
                error_index,
                StakePoolError::InvalidStakeDepositAuthority as u32
            );
        }
        _ => panic!("Wrong error occurs while try to make a deposit with wrong stake program ID"),
    }
}
