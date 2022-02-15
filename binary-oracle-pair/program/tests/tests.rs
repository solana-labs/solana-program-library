#![cfg(feature = "test-bpf")]

use borsh::de::BorshDeserialize;
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

pub async fn create_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Result<(), TransportError> {
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
                mint,
                owner,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, account], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn mint_tokens_to(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
            mint,
            destination,
            &authority.pubkey(),
            &[&authority.pubkey()],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, authority], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn approve_delegate(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    source: &Pubkey,
    delegate: &Pubkey,
    source_owner: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[spl_token::instruction::approve(
            &spl_token::id(),
            source,
            delegate,
            &source_owner.pubkey(),
            &[&source_owner.pubkey()],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, source_owner], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn make_decision(
    program_context: &mut ProgramTestContext,
    pool_account: &Pubkey,
    decider: &Keypair,
    decision: bool,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[instruction::decide(&id(), pool_account, &decider.pubkey(), decision).unwrap()],
        Some(&program_context.payer.pubkey()),
    );

    transaction.sign(
        &[&program_context.payer, decider],
        program_context.last_blockhash,
    );
    program_context
        .banks_client
        .process_transaction(transaction)
        .await?;
    Ok(())
}

pub async fn make_withdraw(
    program_context: &mut ProgramTestContext,
    pool_account: &Pubkey,
    authority: &Pubkey,
    user_transfer_authority: &Pubkey,
    pool_deposit_account: &Pubkey,
    user_pass_account: &Pubkey,
    user_fail_account: &Pubkey,
    token_pass_mint: &Pubkey,
    token_fail_mint: &Pubkey,
    user_account: &Pubkey,
    withdraw_amount: u64,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[instruction::withdraw(
            &id(),
            pool_account,
            authority,
            user_transfer_authority,
            pool_deposit_account,
            user_pass_account,
            user_fail_account,
            token_pass_mint,
            token_fail_mint,
            user_account,
            &spl_token::id(),
            withdraw_amount,
        )
        .unwrap()],
        Some(&program_context.payer.pubkey()),
    );
    transaction.sign(&[&program_context.payer], program_context.last_blockhash);
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
    Ok(())
}

pub async fn get_token_balance(banks_client: &mut BanksClient, token: &Pubkey) -> u64 {
    let token_account = banks_client.get_account(*token).await.unwrap().unwrap();
    let account_info: spl_token::state::Account =
        spl_token::state::Account::unpack_from_slice(token_account.data.as_slice()).unwrap();
    account_info.amount
}

pub struct TestPool {
    pub pool_account: Keypair,
    pub authority: Pubkey,
    pub bump_seed: u8,
    pub deposit_token_mint: Keypair,
    pub deposit_token_mint_owner: Keypair,
    pub pool_deposit_account: Keypair,
    pub token_pass_mint: Keypair,
    pub token_fail_mint: Keypair,
    pub decider: Keypair,
    pub mint_end_slot: u64,
    pub decide_end_slot: u64,
}

impl TestPool {
    pub fn new() -> Self {
        let pool_account = Keypair::new();
        let (authority, bump_seed) =
            Pubkey::find_program_address(&[&pool_account.pubkey().to_bytes()[..32]], &id());
        Self {
            pool_account,
            authority,
            bump_seed,
            deposit_token_mint: Keypair::new(),
            deposit_token_mint_owner: Keypair::new(),
            pool_deposit_account: Keypair::new(),
            token_pass_mint: Keypair::new(),
            token_fail_mint: Keypair::new(),
            decider: Keypair::new(),
            mint_end_slot: 2,
            decide_end_slot: 2000,
        }
    }

    pub async fn init_pool(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
    ) {
        let rent = banks_client.get_rent().await.unwrap();
        let pool_rent = rent.minimum_balance(state::Pool::LEN);
        let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);
        let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);

        // create pool account
        create_account(
            banks_client,
            payer,
            recent_blockhash,
            &self.pool_account,
            pool_rent,
            state::Pool::LEN as u64,
            &id(),
        )
        .await
        .unwrap();

        // create mint of deposit token
        create_mint(
            banks_client,
            payer,
            recent_blockhash,
            &self.deposit_token_mint,
            mint_rent,
            &self.deposit_token_mint_owner.pubkey(),
        )
        .await
        .unwrap();

        let init_args = instruction::InitArgs {
            mint_end_slot: self.mint_end_slot,
            decide_end_slot: self.decide_end_slot,
            bump_seed: self.bump_seed,
        };

        let mut transaction = Transaction::new_with_payer(
            &[
                system_instruction::create_account(
                    &payer.pubkey(),
                    &self.pool_deposit_account.pubkey(),
                    account_rent,
                    spl_token::state::Account::LEN as u64,
                    &spl_token::id(),
                ),
                system_instruction::create_account(
                    &payer.pubkey(),
                    &self.token_pass_mint.pubkey(),
                    mint_rent,
                    spl_token::state::Mint::LEN as u64,
                    &spl_token::id(),
                ),
                system_instruction::create_account(
                    &payer.pubkey(),
                    &self.token_fail_mint.pubkey(),
                    mint_rent,
                    spl_token::state::Mint::LEN as u64,
                    &spl_token::id(),
                ),
                instruction::init_pool(
                    &id(),
                    &self.pool_account.pubkey(),
                    &self.authority,
                    &self.decider.pubkey(),
                    &self.deposit_token_mint.pubkey(),
                    &self.pool_deposit_account.pubkey(),
                    &self.token_pass_mint.pubkey(),
                    &self.token_fail_mint.pubkey(),
                    &spl_token::id(),
                    init_args,
                )
                .unwrap(),
            ],
            Some(&payer.pubkey()),
        );

        transaction.sign(
            &[
                payer,
                &self.pool_deposit_account,
                &self.token_pass_mint,
                &self.token_fail_mint,
            ],
            *recent_blockhash,
        );
        banks_client.process_transaction(transaction).await.unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn prepare_accounts_for_deposit(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        deposit_tokens_to_mint: u64,
        deposit_tokens_for_allowance: u64,
        user_account: &Keypair,
        authority: &Pubkey,
        user_account_owner: &Keypair,
        user_pass_account: &Keypair,
        user_fail_account: &Keypair,
    ) {
        // Create user account
        create_token_account(
            banks_client,
            payer,
            recent_blockhash,
            user_account,
            &self.deposit_token_mint.pubkey(),
            &user_account_owner.pubkey(),
        )
        .await
        .unwrap();

        // Mint to him some deposit tokens
        mint_tokens_to(
            banks_client,
            payer,
            recent_blockhash,
            &self.deposit_token_mint.pubkey(),
            &user_account.pubkey(),
            &self.deposit_token_mint_owner,
            deposit_tokens_to_mint,
        )
        .await
        .unwrap();

        // Give allowance to pool authority
        approve_delegate(
            banks_client,
            payer,
            recent_blockhash,
            &user_account.pubkey(),
            authority,
            user_account_owner,
            deposit_tokens_for_allowance,
        )
        .await
        .unwrap();

        // Create token accounts for PASS and FAIL tokens
        create_token_account(
            banks_client,
            payer,
            recent_blockhash,
            user_pass_account,
            &self.token_pass_mint.pubkey(),
            &user_account_owner.pubkey(),
        )
        .await
        .unwrap();

        create_token_account(
            banks_client,
            payer,
            recent_blockhash,
            user_fail_account,
            &self.token_fail_mint.pubkey(),
            &user_account_owner.pubkey(),
        )
        .await
        .unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn make_deposit(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        user_account: &Keypair,
        user_pass_account: &Keypair,
        user_fail_account: &Keypair,
        deposit_amount: u64,
    ) {
        let mut transaction = Transaction::new_with_payer(
            &[instruction::deposit(
                &id(),
                &self.pool_account.pubkey(),
                &self.authority,
                &self.authority,
                &user_account.pubkey(),
                &self.pool_deposit_account.pubkey(),
                &self.token_pass_mint.pubkey(),
                &self.token_fail_mint.pubkey(),
                &user_pass_account.pubkey(),
                &user_fail_account.pubkey(),
                &spl_token::id(),
                deposit_amount,
            )
            .unwrap()],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[payer], *recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn make_deposit_with_user_transfer_authority(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        user_account: &Keypair,
        user_authority: &Keypair,
        user_pass_account: &Keypair,
        user_fail_account: &Keypair,
        deposit_amount: u64,
    ) {
        let mut transaction = Transaction::new_with_payer(
            &[instruction::deposit(
                &id(),
                &self.pool_account.pubkey(),
                &self.authority,
                &user_authority.pubkey(),
                &user_account.pubkey(),
                &self.pool_deposit_account.pubkey(),
                &self.token_pass_mint.pubkey(),
                &self.token_fail_mint.pubkey(),
                &user_pass_account.pubkey(),
                &user_fail_account.pubkey(),
                &spl_token::id(),
                deposit_amount,
            )
            .unwrap()],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[payer, user_authority], *recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();
    }
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

    let pool = TestPool::new();

    pool.init_pool(&mut banks_client, &payer, &recent_blockhash)
        .await;

    let pool_account_data = get_account(&mut banks_client, &pool.pool_account.pubkey()).await;

    assert_eq!(pool_account_data.data.len(), state::Pool::LEN);
    assert_eq!(pool_account_data.owner, id());

    // check if Pool is initialized
    let pool = state::Pool::try_from_slice(pool_account_data.data.as_slice()).unwrap();
    assert!(pool.is_initialized());
}

#[tokio::test]
async fn test_deposit_with_program_authority() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    let deposit_amount = 100;

    let pool = TestPool::new();

    pool.init_pool(&mut banks_client, &payer, &recent_blockhash)
        .await;

    let user_account = Keypair::new();
    let user_account_owner = Keypair::new();
    let user_pass_account = Keypair::new();
    let user_fail_account = Keypair::new();

    pool.prepare_accounts_for_deposit(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        deposit_amount,
        deposit_amount,
        &user_account,
        &pool.authority,
        &user_account_owner,
        &user_pass_account,
        &user_fail_account,
    )
    .await;

    let user_balance_before = get_token_balance(&mut banks_client, &user_account.pubkey()).await;
    assert_eq!(user_balance_before, deposit_amount);

    // Make deposit
    pool.make_deposit(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_account,
        &user_pass_account,
        &user_fail_account,
        deposit_amount,
    )
    .await;

    // Check balance of user account
    let user_balance_after = get_token_balance(&mut banks_client, &user_account.pubkey()).await;
    assert_eq!(user_balance_after, 0);

    // Check balance of pool deposit account
    let pool_deposit_account_balance =
        get_token_balance(&mut banks_client, &pool.pool_deposit_account.pubkey()).await;
    assert_eq!(pool_deposit_account_balance, deposit_amount);

    // Check if user has PASS and FAIL tokens
    let user_pass_tokens = get_token_balance(&mut banks_client, &user_pass_account.pubkey()).await;
    assert_eq!(user_pass_tokens, deposit_amount);

    let user_fail_tokens = get_token_balance(&mut banks_client, &user_fail_account.pubkey()).await;
    assert_eq!(user_fail_tokens, deposit_amount);
}

#[tokio::test]
async fn test_deposit_with_user_authority() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    let deposit_amount = 100;

    let pool = TestPool::new();

    pool.init_pool(&mut banks_client, &payer, &recent_blockhash)
        .await;

    let user_account = Keypair::new();
    let user_account_owner = Keypair::new();
    let user_transfer_authority = Keypair::new();
    let user_pass_account = Keypair::new();
    let user_fail_account = Keypair::new();

    pool.prepare_accounts_for_deposit(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        deposit_amount,
        deposit_amount,
        &user_account,
        &user_transfer_authority.pubkey(),
        &user_account_owner,
        &user_pass_account,
        &user_fail_account,
    )
    .await;

    let user_balance_before = get_token_balance(&mut banks_client, &user_account.pubkey()).await;
    assert_eq!(user_balance_before, deposit_amount);

    // Make deposit
    pool.make_deposit_with_user_transfer_authority(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_account,
        &user_transfer_authority,
        &user_pass_account,
        &user_fail_account,
        deposit_amount,
    )
    .await;

    // Check balance of user account
    let user_balance_after = get_token_balance(&mut banks_client, &user_account.pubkey()).await;
    assert_eq!(user_balance_after, 0);

    // Check balance of pool deposit account
    let pool_deposit_account_balance =
        get_token_balance(&mut banks_client, &pool.pool_deposit_account.pubkey()).await;
    assert_eq!(pool_deposit_account_balance, deposit_amount);

    // Check if user has PASS and FAIL tokens
    let user_pass_tokens = get_token_balance(&mut banks_client, &user_pass_account.pubkey()).await;
    assert_eq!(user_pass_tokens, deposit_amount);

    let user_fail_tokens = get_token_balance(&mut banks_client, &user_fail_account.pubkey()).await;
    assert_eq!(user_fail_tokens, deposit_amount);
}

#[tokio::test]
async fn test_withdraw_no_decision() {
    let mut program_context = program_test().start_with_context().await;

    let deposit_amount = 100;
    let withdraw_amount = 50;

    let pool = TestPool::new();

    pool.init_pool(
        &mut program_context.banks_client,
        &program_context.payer,
        &program_context.last_blockhash,
    )
    .await;

    let user_account = Keypair::new();
    let user_account_owner = Keypair::new();
    let user_pass_account = Keypair::new();
    let user_fail_account = Keypair::new();

    pool.prepare_accounts_for_deposit(
        &mut program_context.banks_client,
        &program_context.payer,
        &program_context.last_blockhash,
        deposit_amount,
        deposit_amount,
        &user_account,
        &pool.authority,
        &user_account_owner,
        &user_pass_account,
        &user_fail_account,
    )
    .await;

    // Make deposit
    pool.make_deposit(
        &mut program_context.banks_client,
        &program_context.payer,
        &program_context.last_blockhash,
        &user_account,
        &user_pass_account,
        &user_fail_account,
        deposit_amount,
    )
    .await;

    // Set allowances to burn PASS and FAIL tokens
    approve_delegate(
        &mut program_context.banks_client,
        &program_context.payer,
        &program_context.last_blockhash,
        &user_pass_account.pubkey(),
        &pool.authority,
        &user_account_owner,
        deposit_amount,
    )
    .await
    .unwrap();
    approve_delegate(
        &mut program_context.banks_client,
        &program_context.payer,
        &program_context.last_blockhash,
        &user_fail_account.pubkey(),
        &pool.authority,
        &user_account_owner,
        deposit_amount,
    )
    .await
    .unwrap();

    let user_balance_before =
        get_token_balance(&mut program_context.banks_client, &user_account.pubkey()).await;
    assert_eq!(user_balance_before, 0);

    // Check balance of pool deposit account
    let pool_deposit_account_balance = get_token_balance(
        &mut program_context.banks_client,
        &pool.pool_deposit_account.pubkey(),
    )
    .await;
    assert_eq!(pool_deposit_account_balance, deposit_amount);

    // Check if user has PASS and FAIL tokens
    let user_pass_tokens = get_token_balance(
        &mut program_context.banks_client,
        &user_pass_account.pubkey(),
    )
    .await;
    assert_eq!(user_pass_tokens, deposit_amount);

    let user_fail_tokens = get_token_balance(
        &mut program_context.banks_client,
        &user_fail_account.pubkey(),
    )
    .await;
    assert_eq!(user_fail_tokens, deposit_amount);

    make_withdraw(
        &mut program_context,
        &pool.pool_account.pubkey(),
        &pool.authority,
        &pool.authority,
        &pool.pool_deposit_account.pubkey(),
        &user_pass_account.pubkey(),
        &user_fail_account.pubkey(),
        &pool.token_pass_mint.pubkey(),
        &pool.token_fail_mint.pubkey(),
        &user_account.pubkey(),
        withdraw_amount,
    )
    .await
    .unwrap();

    let user_balance_after =
        get_token_balance(&mut program_context.banks_client, &user_account.pubkey()).await;
    assert_eq!(user_balance_after, withdraw_amount);

    // Check balance of pool deposit account after withdraw
    let pool_deposit_account_balance_after = get_token_balance(
        &mut program_context.banks_client,
        &pool.pool_deposit_account.pubkey(),
    )
    .await;
    assert_eq!(
        pool_deposit_account_balance_after,
        deposit_amount - withdraw_amount
    );

    // Check if program burned PASS and FAIL tokens
    let user_pass_tokens_after = get_token_balance(
        &mut program_context.banks_client,
        &user_pass_account.pubkey(),
    )
    .await;
    assert_eq!(user_pass_tokens_after, deposit_amount - withdraw_amount);

    let user_fail_tokens_after = get_token_balance(
        &mut program_context.banks_client,
        &user_fail_account.pubkey(),
    )
    .await;
    assert_eq!(user_fail_tokens_after, deposit_amount - withdraw_amount);
}

#[tokio::test]
async fn test_withdraw_pass_decision() {
    let mut program_context = program_test().start_with_context().await;

    let deposit_amount = 100;
    let withdraw_amount = 50;

    let pool = TestPool::new();

    pool.init_pool(
        &mut program_context.banks_client,
        &program_context.payer,
        &program_context.last_blockhash,
    )
    .await;

    let user_account = Keypair::new();
    let user_account_owner = Keypair::new();
    let user_pass_account = Keypair::new();
    let user_fail_account = Keypair::new();

    pool.prepare_accounts_for_deposit(
        &mut program_context.banks_client,
        &program_context.payer,
        &program_context.last_blockhash,
        deposit_amount,
        deposit_amount,
        &user_account,
        &pool.authority,
        &user_account_owner,
        &user_pass_account,
        &user_fail_account,
    )
    .await;

    // Make deposit
    pool.make_deposit(
        &mut program_context.banks_client,
        &program_context.payer,
        &program_context.last_blockhash,
        &user_account,
        &user_pass_account,
        &user_fail_account,
        deposit_amount,
    )
    .await;

    // Set allowances to burn PASS and FAIL tokens
    approve_delegate(
        &mut program_context.banks_client,
        &program_context.payer,
        &program_context.last_blockhash,
        &user_pass_account.pubkey(),
        &pool.authority,
        &user_account_owner,
        deposit_amount,
    )
    .await
    .unwrap();
    approve_delegate(
        &mut program_context.banks_client,
        &program_context.payer,
        &program_context.last_blockhash,
        &user_fail_account.pubkey(),
        &pool.authority,
        &user_account_owner,
        deposit_amount,
    )
    .await
    .unwrap();

    let decision = true;

    program_context
        .warp_to_slot(pool.mint_end_slot + 1)
        .unwrap();

    make_decision(
        &mut program_context,
        &pool.pool_account.pubkey(),
        &pool.decider,
        decision,
    )
    .await
    .unwrap();

    make_withdraw(
        &mut program_context,
        &pool.pool_account.pubkey(),
        &pool.authority,
        &pool.authority,
        &pool.pool_deposit_account.pubkey(),
        &user_pass_account.pubkey(),
        &user_fail_account.pubkey(),
        &pool.token_pass_mint.pubkey(),
        &pool.token_fail_mint.pubkey(),
        &user_account.pubkey(),
        withdraw_amount,
    )
    .await
    .unwrap();

    let user_balance_after =
        get_token_balance(&mut program_context.banks_client, &user_account.pubkey()).await;
    assert_eq!(user_balance_after, withdraw_amount);

    // Check balance of pool deposit account after withdraw
    let pool_deposit_account_balance_after = get_token_balance(
        &mut program_context.banks_client,
        &pool.pool_deposit_account.pubkey(),
    )
    .await;
    assert_eq!(
        pool_deposit_account_balance_after,
        deposit_amount - withdraw_amount
    );

    // Check if program burned PASS and FAIL tokens
    let user_pass_tokens_after = get_token_balance(
        &mut program_context.banks_client,
        &user_pass_account.pubkey(),
    )
    .await;
    assert_eq!(user_pass_tokens_after, deposit_amount - withdraw_amount);

    let user_fail_tokens_after = get_token_balance(
        &mut program_context.banks_client,
        &user_fail_account.pubkey(),
    )
    .await;
    assert_eq!(user_fail_tokens_after, deposit_amount);
}

#[tokio::test]
async fn test_withdraw_fail_decision() {
    let mut program_context = program_test().start_with_context().await;

    let deposit_amount = 100;
    let withdraw_amount = 50;

    let pool = TestPool::new();

    pool.init_pool(
        &mut program_context.banks_client,
        &program_context.payer,
        &program_context.last_blockhash,
    )
    .await;

    let user_account = Keypair::new();
    let user_account_owner = Keypair::new();
    let user_pass_account = Keypair::new();
    let user_fail_account = Keypair::new();

    pool.prepare_accounts_for_deposit(
        &mut program_context.banks_client,
        &program_context.payer,
        &program_context.last_blockhash,
        deposit_amount,
        deposit_amount,
        &user_account,
        &pool.authority,
        &user_account_owner,
        &user_pass_account,
        &user_fail_account,
    )
    .await;

    // Make deposit
    pool.make_deposit(
        &mut program_context.banks_client,
        &program_context.payer,
        &program_context.last_blockhash,
        &user_account,
        &user_pass_account,
        &user_fail_account,
        deposit_amount,
    )
    .await;

    // Set allowances to burn PASS and FAIL tokens
    approve_delegate(
        &mut program_context.banks_client,
        &program_context.payer,
        &program_context.last_blockhash,
        &user_pass_account.pubkey(),
        &pool.authority,
        &user_account_owner,
        deposit_amount,
    )
    .await
    .unwrap();
    approve_delegate(
        &mut program_context.banks_client,
        &program_context.payer,
        &program_context.last_blockhash,
        &user_fail_account.pubkey(),
        &pool.authority,
        &user_account_owner,
        deposit_amount,
    )
    .await
    .unwrap();

    let decision = false;

    program_context
        .warp_to_slot(pool.mint_end_slot + 1)
        .unwrap();

    make_decision(
        &mut program_context,
        &pool.pool_account.pubkey(),
        &pool.decider,
        decision,
    )
    .await
    .unwrap();

    make_withdraw(
        &mut program_context,
        &pool.pool_account.pubkey(),
        &pool.authority,
        &pool.authority,
        &pool.pool_deposit_account.pubkey(),
        &user_pass_account.pubkey(),
        &user_fail_account.pubkey(),
        &pool.token_pass_mint.pubkey(),
        &pool.token_fail_mint.pubkey(),
        &user_account.pubkey(),
        withdraw_amount,
    )
    .await
    .unwrap();

    let user_balance_after =
        get_token_balance(&mut program_context.banks_client, &user_account.pubkey()).await;
    assert_eq!(user_balance_after, withdraw_amount);

    // Check balance of pool deposit account after withdraw
    let pool_deposit_account_balance_after = get_token_balance(
        &mut program_context.banks_client,
        &pool.pool_deposit_account.pubkey(),
    )
    .await;
    assert_eq!(
        pool_deposit_account_balance_after,
        deposit_amount - withdraw_amount
    );

    // Check if program burned PASS and FAIL tokens
    let user_pass_tokens_after = get_token_balance(
        &mut program_context.banks_client,
        &user_pass_account.pubkey(),
    )
    .await;
    assert_eq!(user_pass_tokens_after, deposit_amount);

    let user_fail_tokens_after = get_token_balance(
        &mut program_context.banks_client,
        &user_fail_account.pubkey(),
    )
    .await;
    assert_eq!(user_fail_tokens_after, deposit_amount - withdraw_amount);
}

#[tokio::test]
async fn test_decide() {
    let mut program_context = program_test().start_with_context().await;

    let pool = TestPool::new();

    pool.init_pool(
        &mut program_context.banks_client,
        &program_context.payer,
        &program_context.last_blockhash,
    )
    .await;

    let pool_account_data_before = program_context
        .banks_client
        .get_account(pool.pool_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let pool_data_before =
        state::Pool::try_from_slice(pool_account_data_before.data.as_slice()).unwrap();

    assert_eq!(pool_data_before.decision, state::Decision::Undecided);

    let decision = true;

    program_context
        .warp_to_slot(pool.mint_end_slot + 1)
        .unwrap();

    make_decision(
        &mut program_context,
        &pool.pool_account.pubkey(),
        &pool.decider,
        decision,
    )
    .await
    .unwrap();

    let pool_account_data_after = program_context
        .banks_client
        .get_account(pool.pool_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let pool_data_after =
        state::Pool::try_from_slice(pool_account_data_after.data.as_slice()).unwrap();

    assert_eq!(pool_data_after.decision, state::Decision::Pass);
}
