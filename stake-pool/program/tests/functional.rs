#![cfg(feature = "test-bpf")]

use solana_program::{hash::Hash, program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_stake_pool::*;

fn program_test() -> ProgramTest {
    let mut pc = ProgramTest::new(
        "spl_stake_pool",
        id(),
        processor!(processor::Processor::process),
    );

    // Add SPL Token program
    pc.add_program(
        "spl_token",
        spl_token::id(),
        processor!(spl_token::processor::Processor::process),
    );

    pc
}

async fn create_mint(banks_client: &mut BanksClient, recent_blockhash: &Hash, payer: &Keypair, pool_mint: &Keypair, owner: &Pubkey) {
    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::create_account(
            &payer.pubkey(),
            &pool_mint.pubkey(),
            mint_rent,
            spl_token::state::Mint::LEN as u64,
            &spl_token::id(),
        ),
        spl_token::instruction::initialize_mint(
            &spl_token::id(),
            &pool_mint.pubkey(),
            &owner,
            None,
            0,
        ).unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, pool_mint], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

async fn create_token_account(banks_client: &mut BanksClient, recent_blockhash: &Hash, payer: &Keypair, account: &Keypair, pool_mint: &Pubkey, owner: &Pubkey) {
    let rent = banks_client.get_rent().await.unwrap();
    let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::create_account(
            &payer.pubkey(),
            &account.pubkey(),
            account_rent,
            spl_token::state::Account::LEN as u64,
            &spl_token::id(),
        ),
        spl_token::instruction::initialize_account(
            &spl_token::id(),
            &account.pubkey(),
            pool_mint,
            owner,
        ).unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, account], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

async fn create_stake_pool(banks_client: &mut BanksClient, recent_blockhash: &Hash, payer: &Keypair, stake_pool: &Keypair, pool_mint: &Pubkey, pool_token_account: &Pubkey, owner: &Pubkey) {
    let rent = banks_client.get_rent().await.unwrap();
    let rent = rent.minimum_balance(state::State::LEN);
    let numerator = 1;
    let denominator = 100;
    let fee = instruction::Fee { numerator, denominator };
    let init_args = instruction::InitArgs { fee };

    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::create_account(
            &payer.pubkey(),
            &stake_pool.pubkey(),
            rent,
            state::State::LEN as u64,
            &id(),
        ),
        instruction::initialize(
            &id(),
            &stake_pool.pubkey(),
            owner,
            pool_mint,
            pool_token_account,
            &spl_token::id(),
            init_args,
        ).unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, stake_pool], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn test_stake_pool_initialize() {
    let stake_pool = Keypair::new();
    let stake_pool_address = &stake_pool.pubkey();
    let (withdraw_authority, _) = Pubkey::find_program_address(
        &[&stake_pool_address.to_bytes()[..32], b"withdraw"],
        &id(),
    );
    let pool_mint = Keypair::new();
    let pool_token_account = Keypair::new();
    let owner_address = Pubkey::new_unique();

    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    create_mint(&mut banks_client, &recent_blockhash, &payer, &pool_mint, &withdraw_authority).await;
    create_token_account(&mut banks_client, &recent_blockhash, &payer, &pool_token_account, &pool_mint.pubkey(), &owner_address).await;
    create_stake_pool(&mut banks_client, &recent_blockhash, &payer, &stake_pool, &pool_mint.pubkey(), &pool_token_account.pubkey(), &owner_address).await;

    // Stake pool now exists
    let stake_pool = banks_client
        .get_account(*stake_pool_address)
        .await
        .expect("get_account")
        .expect("stake pool not none");
    assert_eq!(
        stake_pool.data.len(),
        state::State::LEN
    );
    assert_eq!(stake_pool.owner, id());
}
