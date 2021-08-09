use {
    solana_program::{hash::Hash, program_pack::Pack, pubkey::Pubkey, system_instruction},
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
        transport::TransportError,
    },
    spl_token_swap::{
        curve::{base::SwapCurve, fees::Fees},
        id, instruction, processor,
        state::PoolRegistry,
    },
    std::convert::TryInto,
};

pub fn program_test() -> ProgramTest {
    ProgramTest::new(
        "spl_token_swap",
        id(),
        processor!(processor::Processor::process),
    )
}

pub async fn create_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Keypair,
    owner: &Pubkey,
    len: u64,
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let rent_amt = rent.minimum_balance(len.try_into().unwrap());

    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::create_account(
            &payer.pubkey(),
            &account.pubkey(),
            rent_amt,
            0,
            owner,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, account], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn create_account_with_seed(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Pubkey,
    base: &Keypair,
    seed: &str,
    owner: &Pubkey,
    len: u64,
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let rent_amt = rent.minimum_balance(len.try_into().unwrap());

    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::create_account_with_seed(
            &payer.pubkey(),
            &account,
            &base.pubkey(),
            seed,
            rent_amt,
            len,
            owner,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, base], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn create_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    mint: &Keypair,
    manager: &Pubkey,
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint.pubkey(),
                manager,
                None,
                0,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, mint], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn create_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Keypair,
    pool_mint: &Pubkey,
    manager: &Pubkey,
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
                pool_mint,
                manager,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, account], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn mint_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    mint: &Pubkey,
    account: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
            mint,
            account,
            &mint_authority.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer, mint_authority],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn create_pool_registry(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    pool_registry_key: &Pubkey,
    pool_registry_payer: &Keypair,
) -> Result<(), TransportError> {
    let size = std::mem::size_of::<PoolRegistry>().try_into().unwrap();
    assert_eq!(size, 10485733);

    create_account_with_seed(
        banks_client,
        payer,
        recent_blockhash,
        pool_registry_key,
        pool_registry_payer,
        "poolregistry",
        &id(),
        size,
    )
    .await
    .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::initialize_registry(
            &id(),
            &pool_registry_payer.pubkey(),
            pool_registry_key,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer, pool_registry_payer],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub struct TokenSwapAccounts {
    pub fees: Fees,
    pub swap_curve: SwapCurve,
    pub swap_pubkey: Pubkey,
    pub authority_pubkey: Pubkey,
    pub nonce: u8,
    pub token_a_key: Keypair,
    pub token_a_mint_key: Keypair,
    pub token_b_key: Keypair,
    pub token_b_mint_key: Keypair,
    pub pool_mint_key: Keypair,
    pub pool_fee_key: Keypair,
    pub pool_token_key: Keypair,
    pub pool_registry_pubkey: Pubkey,
}

impl TokenSwapAccounts {
    pub async fn new(
        mut banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        pool_registry_pubkey: Pubkey,
        fees: Fees,
        swap_curve: SwapCurve,
        token_a_amount: u64,
        token_b_amount: u64,
    ) -> Self {
        //random keys for these
        let token_a_mint_key = Keypair::new();
        let token_a_key = Keypair::new();
        let token_b_mint_key = Keypair::new();
        let token_b_key = Keypair::new();
        let pool_mint_key = Keypair::new();
        let pool_fee_key = Keypair::new();
        let pool_token_key = Keypair::new();

        let mut seed_key_vec = Vec::new();
        seed_key_vec.push(token_a_mint_key.pubkey().to_bytes());
        seed_key_vec.push(token_b_mint_key.pubkey().to_bytes());
        seed_key_vec.sort();

        //pda for swap account
        let (swap_pubkey, _pool_pda_seed_nonce) = Pubkey::find_program_address(
            &[
                &seed_key_vec[0][..32],
                &seed_key_vec[1][..32],
                &[swap_curve.curve_type as u8],
            ],
            &id(),
        );

        //create the pda for the authority
        let (authority_pubkey, nonce) =
            Pubkey::find_program_address(&[&swap_pubkey.to_bytes()[..]], &spl_token_swap::id());

        //create the pool mint
        create_mint(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &pool_mint_key,
            &authority_pubkey,
        )
        .await
        .unwrap();

        //create the pool fee account
        create_token_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &pool_fee_key,
            &pool_mint_key.pubkey(),
            &payer.pubkey(),
        )
        .await
        .unwrap();

        //create the pool token account
        create_token_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &pool_token_key,
            &pool_mint_key.pubkey(),
            &payer.pubkey(),
        )
        .await
        .unwrap();

        //create the A mint and pool's token account
        create_mint(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &token_a_mint_key,
            &payer.pubkey(),
        )
        .await
        .unwrap();
        create_token_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &token_a_key,
            &token_a_mint_key.pubkey(),
            &authority_pubkey,
        )
        .await
        .unwrap();
        mint_tokens(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &token_a_mint_key.pubkey(),
            &token_a_key.pubkey(),
            &payer,
            token_a_amount,
        )
        .await
        .unwrap();
        //create the B mint and pool's token account
        create_mint(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &token_b_mint_key,
            &payer.pubkey(),
        )
        .await
        .unwrap();
        create_token_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &token_b_key,
            &token_b_mint_key.pubkey(),
            &authority_pubkey,
        )
        .await
        .unwrap();
        mint_tokens(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &token_b_mint_key.pubkey(),
            &token_b_key.pubkey(),
            &payer,
            token_b_amount,
        )
        .await
        .unwrap();
        TokenSwapAccounts {
            fees,
            swap_curve,
            swap_pubkey,
            authority_pubkey,
            nonce,
            token_a_key,
            token_a_mint_key,
            token_b_key,
            token_b_mint_key,
            pool_mint_key,
            pool_fee_key,
            pool_token_key,
            pool_registry_pubkey,
        }
    }

    pub async fn initialize_swap(
        &mut self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
    ) -> Result<(), TransportError> {
        let mut transaction = Transaction::new_with_payer(
            &[instruction::initialize(
                &id(),
                &spl_token::id(),
                &payer.pubkey(),
                &self.swap_pubkey,
                &self.authority_pubkey,
                &self.token_a_key.pubkey(),
                &self.token_b_key.pubkey(),
                &self.pool_mint_key.pubkey(),
                &self.pool_fee_key.pubkey(),
                &self.pool_token_key.pubkey(),
                self.nonce,
                self.fees.clone(),
                self.swap_curve.clone(),
                &self.pool_registry_pubkey,
            )
            .unwrap()],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[payer], *recent_blockhash);
        banks_client.process_transaction(transaction).await?;
        Ok(())
    }
}
