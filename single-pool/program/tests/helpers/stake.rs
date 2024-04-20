#![allow(dead_code)] // needed because cargo doesn't understand test usage

use {
    crate::get_account,
    bincode::deserialize,
    solana_program_test::BanksClient,
    solana_sdk::{
        hash::Hash,
        native_token::LAMPORTS_PER_SOL,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        stake::{
            self,
            state::{Meta, Stake, StakeStateV2},
        },
        system_instruction,
        transaction::Transaction,
    },
    std::convert::TryInto,
};

pub const TEST_STAKE_AMOUNT: u64 = 10_000_000_000; // 10 sol

pub async fn get_stake_account(
    banks_client: &mut BanksClient,
    pubkey: &Pubkey,
) -> (Meta, Option<Stake>, u64) {
    let stake_account = get_account(banks_client, pubkey).await;
    let lamports = stake_account.lamports;
    match deserialize::<StakeStateV2>(&stake_account.data).unwrap() {
        StakeStateV2::Initialized(meta) => (meta, None, lamports),
        StakeStateV2::Stake(meta, stake, _) => (meta, Some(stake), lamports),
        _ => unimplemented!(),
    }
}

pub async fn get_stake_account_rent(banks_client: &mut BanksClient) -> u64 {
    let rent = banks_client.get_rent().await.unwrap();
    rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>())
}

pub async fn get_pool_minimum_delegation(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
) -> u64 {
    let transaction = Transaction::new_signed_with_payer(
        &[stake::instruction::get_minimum_delegation()],
        Some(&payer.pubkey()),
        &[payer],
        *recent_blockhash,
    );
    let mut data = banks_client
        .simulate_transaction(transaction)
        .await
        .unwrap()
        .simulation_details
        .unwrap()
        .return_data
        .unwrap()
        .data;
    data.resize(8, 0);
    let stake_program_minimum = data.try_into().map(u64::from_le_bytes).unwrap();

    std::cmp::max(stake_program_minimum, LAMPORTS_PER_SOL)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_independent_stake_account(
    banks_client: &mut BanksClient,
    fee_payer: &Keypair,
    rent_payer: &Keypair,
    recent_blockhash: &Hash,
    stake: &Keypair,
    authorized: &stake::state::Authorized,
    lockup: &stake::state::Lockup,
    stake_amount: u64,
) -> u64 {
    let lamports = get_stake_account_rent(banks_client).await + stake_amount;
    let transaction = Transaction::new_signed_with_payer(
        &stake::instruction::create_account(
            &rent_payer.pubkey(),
            &stake.pubkey(),
            authorized,
            lockup,
            lamports,
        ),
        Some(&fee_payer.pubkey()),
        &[fee_payer, rent_payer, stake],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    lamports
}

pub async fn create_blank_stake_account(
    banks_client: &mut BanksClient,
    fee_payer: &Keypair,
    rent_payer: &Keypair,
    recent_blockhash: &Hash,
    stake: &Keypair,
) -> u64 {
    let lamports = get_stake_account_rent(banks_client).await;
    let transaction = Transaction::new_signed_with_payer(
        &[system_instruction::create_account(
            &rent_payer.pubkey(),
            &stake.pubkey(),
            lamports,
            std::mem::size_of::<stake::state::StakeStateV2>() as u64,
            &stake::program::id(),
        )],
        Some(&fee_payer.pubkey()),
        &[fee_payer, rent_payer, stake],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    lamports
}

pub async fn delegate_stake_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake: &Pubkey,
    authorized: &Keypair,
    vote: &Pubkey,
) {
    let mut transaction = Transaction::new_with_payer(
        &[stake::instruction::delegate_stake(
            stake,
            &authorized.pubkey(),
            vote,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, authorized], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}
