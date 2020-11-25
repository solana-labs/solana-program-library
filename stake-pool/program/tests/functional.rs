#![cfg(feature = "test-bpf")]

use solana_program::{hash::Hash, program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_stake_pool::*;

use bincode::deserialize;

const TEST_STAKE_AMOUNT: u64 = 100;

fn program_test() -> ProgramTest {
    ProgramTest::new(
        "spl_stake_pool",
        id(),
        processor!(processor::Processor::process),
    )
}

async fn create_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    pool_mint: &Keypair,
    owner: &Pubkey,
) {
    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
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
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, pool_mint], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

async fn create_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Keypair,
    pool_mint: &Pubkey,
    owner: &Pubkey,
) {
    let rent = banks_client.get_rent().await.unwrap();
    let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
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
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, account], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

async fn create_stake_pool(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake_pool: &Keypair,
    pool_mint: &Pubkey,
    pool_token_account: &Pubkey,
    owner: &Pubkey,
) {
    let rent = banks_client.get_rent().await.unwrap();
    let rent = rent.minimum_balance(state::State::LEN);
    let numerator = 1;
    let denominator = 100;
    let fee = instruction::Fee {
        numerator,
        denominator,
    };
    let init_args = instruction::InitArgs { fee };

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
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
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, stake_pool], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

async fn create_stake_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake: &Keypair,
    authorized: &stake::Authorized,
    lockup: &stake::Lockup,
) {
    let rent = banks_client.get_rent().await.unwrap();
    let lamports =
        rent.minimum_balance(std::mem::size_of::<stake::StakeState>()) + TEST_STAKE_AMOUNT;

    let mut transaction = Transaction::new_with_payer(
        &stake::create_account(
            &payer.pubkey(),
            &stake.pubkey(),
            authorized,
            lockup,
            lamports,
        ),
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, stake], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

struct StakePoolAccounts {
    pub stake_pool: Keypair,
    pub pool_mint: Keypair,
    pub pool_fee_account: Keypair,
    pub owner: Pubkey,
    pub withdraw_authority: Pubkey,
    pub deposit_authority: Pubkey,
}

impl StakePoolAccounts {
    pub fn new() -> Self {
        let stake_pool = Keypair::new();
        let stake_pool_address = &stake_pool.pubkey();
        let (withdraw_authority, _) = Pubkey::find_program_address(
            &[&stake_pool_address.to_bytes()[..32], b"withdraw"],
            &id(),
        );
        let (deposit_authority, _) = Pubkey::find_program_address(
            &[&stake_pool_address.to_bytes()[..32], b"deposit"],
            &id(),
        );
        let pool_mint = Keypair::new();
        let pool_fee_account = Keypair::new();
        let owner = Pubkey::new_unique();

        Self {
            stake_pool,
            pool_mint,
            pool_fee_account,
            owner,
            withdraw_authority,
            deposit_authority,
        }
    }

    pub async fn initialize_stake_pool(
        &self,
        mut banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
    ) {
        create_mint(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &self.pool_mint,
            &self.withdraw_authority,
        )
        .await;
        create_token_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &self.pool_fee_account,
            &self.pool_mint.pubkey(),
            &self.owner,
        )
        .await;
        create_stake_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &self.stake_pool,
            &self.pool_mint.pubkey(),
            &self.pool_fee_account.pubkey(),
            &self.owner,
        )
        .await;
    }

    pub async fn deposit_stake(
        &self,
        stake: &Pubkey,
        pool_account: &Pubkey,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
    ) {
        let mut transaction = Transaction::new_with_payer(
            &[instruction::deposit(
                &id(),
                &self.stake_pool.pubkey(),
                &self.deposit_authority,
                &self.withdraw_authority,
                stake,
                pool_account,
                &self.pool_fee_account.pubkey(),
                &self.pool_mint.pubkey(),
                &spl_token::id(),
                &stake::id(),
            )
            .unwrap()],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[payer], *recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();
    }
}

#[tokio::test]
async fn test_stake_pool_initialize() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await;

    // Stake pool now exists
    let stake_pool = banks_client
        .get_account(stake_pool_accounts.stake_pool.pubkey())
        .await
        .expect("get_account")
        .expect("stake pool not none");
    assert_eq!(stake_pool.data.len(), state::State::LEN);
    assert_eq!(stake_pool.owner, id());
}

#[tokio::test]
async fn test_stake_pool_deposit() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash)
        .await;

    let user = Keypair::new();
    // make stake account
    let user_stake = Keypair::new();
    let lockup = stake::Lockup::default();
    let authorized = stake::Authorized {
        staker: stake_pool_accounts.deposit_authority.clone(),
        withdrawer: stake_pool_accounts.deposit_authority.clone(),
    };
    create_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake,
        &authorized,
        &lockup,
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
    .await;
    stake_pool_accounts
        .deposit_stake(
            &user_stake.pubkey(),
            &user_pool_account.pubkey(),
            &mut banks_client,
            &payer,
            &recent_blockhash,
        )
        .await;

    let stake = banks_client
        .get_account(user_stake.pubkey())
        .await
        .expect("get_account")
        .expect("stake not none");
    assert_eq!(stake.data.len(), std::mem::size_of::<stake::StakeState>());
    assert_eq!(stake.owner, stake::id());

    let stake_state = deserialize::<stake::StakeState>(&stake.data).unwrap();
    match stake_state {
        stake::StakeState::Initialized(meta) => {
            assert_eq!(
                &meta.authorized.staker,
                &stake_pool_accounts.withdraw_authority
            );
            assert_eq!(
                &meta.authorized.withdrawer,
                &stake_pool_accounts.withdraw_authority
            );
        }
        _ => assert!(false),
    }
}
