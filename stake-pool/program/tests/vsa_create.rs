#![cfg(feature = "test-bpf")]

mod helpers;

use crate::solana_program::pubkey::Pubkey;
use helpers::*;

use bincode::deserialize;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_stake_pool::*;

#[tokio::test]
async fn test_create_validator_stake_account() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await;

    let validator = Pubkey::new_unique();
    let user_stake_authority = Keypair::new();
    let user_withdraw_authority = Keypair::new();

    let (stake_account, _) = processor::Processor::find_stake_address_for_validator(
        &id(),
        &validator,
        &stake_pool_accounts.stake_pool.pubkey(),
    );

    let mut transaction = Transaction::new_with_payer(
        &[instruction::create_validator_stake_account(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &payer.pubkey(),
            &stake_account,
            &validator,
            &user_stake_authority.pubkey(),
            &user_withdraw_authority.pubkey(),
            &solana_program::system_program::id(),
            &stake::id(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Check authorities
    let stake = get_account(&mut banks_client, &stake_account).await;
    let stake_state = deserialize::<stake::StakeState>(&stake.data).unwrap();
    match stake_state {
        stake::StakeState::Initialized(meta) => {
            assert_eq!(&meta.authorized.staker, &user_stake_authority.pubkey());
            assert_eq!(
                &meta.authorized.withdrawer,
                &user_withdraw_authority.pubkey()
            );
        }
        _ => panic!(),
    }
}
