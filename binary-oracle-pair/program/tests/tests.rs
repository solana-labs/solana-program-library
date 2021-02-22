#![cfg(feature = "test-bpf")]

use solana_program::{hash::Hash, program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signer},
    transaction::Transaction,
    transport::TransportError,
};
use spl_binary_oracle_pair::*;

pub fn program_test() -> ProgramTest {
    ProgramTest::new(
        "spl_binary_oracle_pair",
        id(),
        processor!(processor::Processor::process_instruction),
    )
}

pub async fn create_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    mint_account: &Keypair,
    mint_rent: u64,
    owner: &Pubkey,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &mint_account.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint_account.pubkey(),
                &owner,
                None,
                0,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, mint_account], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn create_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Keypair,
    rent: u64,
    space: u64,
    owner: &Pubkey,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::create_account(
            &payer.pubkey(),
            &account.pubkey(),
            rent,
            space,
            owner,
        )],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[payer, account], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

async fn get_account(banks_client: &mut BanksClient, pubkey: &Pubkey) -> Account {
    banks_client
        .get_account(*pubkey)
        .await
        .expect("account not found")
        .expect("account empty")
}

#[tokio::test]
async fn test_init_pool() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    let pool_account = Keypair::new();

    let rent = banks_client.get_rent().await.unwrap();
    let pool_rent = rent.minimum_balance(state::Pool::LEN);
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);
    let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);

    // create pool account
    create_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &pool_account,
        pool_rent,
        state::Pool::LEN as u64,
        &id(),
    )
    .await
    .unwrap();

    // create authority program key
    let (authority, bump_seed) =
        Pubkey::find_program_address(&[&pool_account.pubkey().to_bytes()[..32]], &id());
    let decider = Keypair::new();

    // create mint of deposit token
    let mint_owner = Keypair::new();
    let deposit_token_mint = Keypair::new();
    create_mint(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &deposit_token_mint,
        mint_rent,
        &mint_owner.pubkey(),
    )
    .await
    .unwrap();

    let deposit_account = Keypair::new();
    let token_pass_mint = Keypair::new();
    let token_fail_mint = Keypair::new();

    let init_args = instruction::InitArgs {
        mint_end_slot: 1000,
        decide_end_slot: 2000,
        bump_seed,
    };

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &deposit_account.pubkey(),
                account_rent,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &token_pass_mint.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &token_fail_mint.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            instruction::init_pool(
                &id(),
                &pool_account.pubkey(),
                &authority,
                &decider.pubkey(),
                &deposit_token_mint.pubkey(),
                &deposit_account.pubkey(),
                &token_pass_mint.pubkey(),
                &token_fail_mint.pubkey(),
                &spl_token::id(),
                init_args,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(
        &[&payer, &deposit_account, &token_pass_mint, &token_fail_mint],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let pool_account_data = get_account(&mut banks_client, &pool_account.pubkey()).await;

    assert_eq!(pool_account_data.data.len(), state::Pool::LEN);
    assert_eq!(pool_account_data.owner, id());

    // check if Pool is initialized
    state::Pool::unpack(pool_account_data.data.as_slice()).unwrap();
}
