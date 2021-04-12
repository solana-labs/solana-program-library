mod helpers;

use {
    bincode::deserialize,
    helpers::*,
    solana_program::{clock::Epoch, hash::Hash, instruction::InstructionError, pubkey::Pubkey},
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
    },
    spl_stake_pool::{error::StakePoolError, id, instruction, stake_program},
};

async fn setup() -> (
    BanksClient,
    Keypair,
    Hash,
    StakePoolAccounts,
    ValidatorStakeAccount,
    DepositInfo,
    u64,
) {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    let reserve_lamports = 5_000_000;
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, reserve_lamports)
        .await
        .unwrap();

    let validator_stake_account = simple_add_validator_to_pool(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let deposit_info = simple_deposit(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
        &validator_stake_account,
        5_000_000,
    )
    .await;

    (
        banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        reserve_lamports,
    )
}

#[tokio::test]
async fn success() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake,
        _deposit_info,
        reserve_lamports,
    ) = setup().await;

    // Save reserve stake
    let pre_reserve_stake_account =
        get_account(&mut banks_client, &stake_pool_accounts.reserve_stake.pubkey()).await;

    // Check no transient stake
    let transient_account = banks_client
        .get_account(validator_stake.transient_stake_account)
        .await
        .unwrap();
    assert!(transient_account.is_none());

    let rent = banks_client.get_rent().await.unwrap();
    let lamports = rent.minimum_balance(std::mem::size_of::<stake_program::StakeState>());
    let reserve_lamports = reserve_lamports - lamports;
    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &validator_stake.transient_stake_account,
            &validator_stake.vote.pubkey(),
            reserve_lamports,
        )
        .await;
    assert!(error.is_none());

    // Check reserve stake account balance
    let reserve_stake_account =
        get_account(&mut banks_client, &stake_pool_accounts.reserve_stake.pubkey()).await;
    let reserve_stake_state =
        deserialize::<stake_program::StakeState>(&reserve_stake_account.data).unwrap();
    assert_eq!(
        pre_reserve_stake_account.lamports - reserve_lamports,
        reserve_stake_account.lamports
    );
    assert!(reserve_stake_state.delegation().is_none());

    // Check transient stake account state and balance
    let transient_stake_account =
        get_account(&mut banks_client, &validator_stake.transient_stake_account).await;
    let transient_stake_state =
        deserialize::<stake_program::StakeState>(&transient_stake_account.data).unwrap();
    assert_eq!(transient_stake_account.lamports, reserve_lamports);
    assert_ne!(
        transient_stake_state
            .delegation()
            .unwrap()
            .activation_epoch,
        Epoch::MAX
    );
}
