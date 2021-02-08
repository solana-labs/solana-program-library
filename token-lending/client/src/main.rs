use solana_client::rpc_client::RpcClient;
use solana_program::program_pack::Pack;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    system_instruction::create_account,
    transaction::Transaction,
};
use spl_token::{
    instruction::approve,
    state::{Account as Token, Mint},
};
use spl_token_lending::{
    instruction::{init_lending_market, init_reserve},
    state::{LendingMarket, Reserve, ReserveConfig, ReserveFees},
};
use std::str::FromStr;

// -------- UPDATE START -------
const KEYPAIR_PATH: &str = "/your/path";
const SRM_TOKEN_ACCOUNT: &str = "BASE58_ADDRESS";
const USDC_TOKEN_ACCOUNT: &str = "BASE58_ADDRESS";
const WRAPPED_SOL_TOKEN_ACCOUNT: &str = "BASE58_ADDRESS";
solana_program::declare_id!("TokenLend1ng1111111111111111111111111111111");
// -------- UPDATE END ---------

pub struct DexMarket {
    pub name: &'static str,
    pub pubkey: Pubkey,
}

pub fn main() {
    let mut client = RpcClient::new("https://api.mainnet-beta.solana.com".to_owned());

    let payer = read_keypair_file(&format!("{}/payer.json", KEYPAIR_PATH)).unwrap();
    let usdc_mint_pubkey =
        Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();

    let sol_usdc_dex_market = DexMarket {
        name: "sol_usdc",
        pubkey: Pubkey::from_str("7xMDbYTCqQEcK2aM9LbetGtNFJpzKdfXzLL5juaLh4GJ").unwrap(),
    };

    let srm_usdc_dex_market = DexMarket {
        name: "srm_usdc",
        pubkey: Pubkey::from_str("CDdR97S8y96v3To93aKvi3nCnjUrbuVSuumw8FLvbVeg").unwrap(),
    };

    let quote_token_mint = usdc_mint_pubkey;
    let (lending_market_keypair, _lending_market) =
        create_lending_market(&mut client, quote_token_mint, &payer);

    let usdc_liquidity_source = Pubkey::from_str(USDC_TOKEN_ACCOUNT).unwrap();
    let usdc_reserve_config = ReserveConfig {
        optimal_utilization_rate: 80,
        loan_to_value_ratio: 75,
        liquidation_bonus: 5,
        liquidation_threshold: 80,
        min_borrow_rate: 0,
        optimal_borrow_rate: 4,
        max_borrow_rate: 30,
        fees: ReserveFees {
            borrow_fee_wad: 100_000_000_000_000, // 1 bp
            host_fee_percentage: 20,
        },
    };

    let (usdc_reserve_pubkey, _usdc_reserve) = create_reserve(
        &mut client,
        usdc_reserve_config,
        &lending_market_keypair,
        None,
        usdc_liquidity_source,
        &payer,
    );

    println!("Created usdc reserve with pubkey: {}", usdc_reserve_pubkey);

    let sol_liquidity_source = Pubkey::from_str(WRAPPED_SOL_TOKEN_ACCOUNT).unwrap();
    let sol_reserve_config = ReserveConfig {
        optimal_utilization_rate: 0,
        loan_to_value_ratio: 75,
        liquidation_bonus: 10,
        liquidation_threshold: 80,
        min_borrow_rate: 0,
        optimal_borrow_rate: 2,
        max_borrow_rate: 15,
        fees: ReserveFees {
            borrow_fee_wad: 1_000_000_000_000, // 0.01 bp
            host_fee_percentage: 20,
        },
    };

    let (sol_reserve_pubkey, _sol_reserve) = create_reserve(
        &mut client,
        sol_reserve_config,
        &lending_market_keypair,
        Some(sol_usdc_dex_market.pubkey),
        sol_liquidity_source,
        &payer,
    );

    println!("Created sol reserve with pubkey: {}", sol_reserve_pubkey);

    let srm_liquidity_source = Pubkey::from_str(SRM_TOKEN_ACCOUNT).unwrap();
    let srm_reserve_config = ReserveConfig {
        optimal_utilization_rate: 0,
        loan_to_value_ratio: 75,
        liquidation_bonus: 10,
        liquidation_threshold: 80,
        min_borrow_rate: 0,
        optimal_borrow_rate: 2,
        max_borrow_rate: 15,
        fees: ReserveFees {
            borrow_fee_wad: 10_000_000_000_000, // 0.1 bp
            host_fee_percentage: 25,
        },
    };

    let (srm_reserve_pubkey, _srm_reserve) = create_reserve(
        &mut client,
        srm_reserve_config,
        &lending_market_keypair,
        Some(srm_usdc_dex_market.pubkey),
        srm_liquidity_source,
        &payer,
    );

    println!("Created srm reserve with pubkey: {}", srm_reserve_pubkey);
}

pub fn create_lending_market(
    client: &mut RpcClient,
    quote_token_mint: Pubkey,
    payer: &Keypair,
) -> (Keypair, LendingMarket) {
    let keypair = read_keypair_file(&format!("{}/lending_market.json", KEYPAIR_PATH)).unwrap();
    let pubkey = keypair.pubkey();

    let mut transaction = Transaction::new_with_payer(
        &[
            create_account(
                &payer.pubkey(),
                &pubkey,
                client
                    .get_minimum_balance_for_rent_exemption(LendingMarket::LEN)
                    .unwrap(),
                LendingMarket::LEN as u64,
                &id(),
            ),
            init_lending_market(id(), pubkey, quote_token_mint),
        ],
        Some(&payer.pubkey()),
    );

    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    transaction.sign(&[&payer, &keypair], recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();

    let account = client.get_account(&pubkey).unwrap();
    let lending_market = LendingMarket::unpack(&account.data).unwrap();

    (keypair, lending_market)
}

pub fn create_reserve(
    client: &mut RpcClient,
    config: ReserveConfig,
    lending_market_keypair: &Keypair,
    dex_market_pubkey: Option<Pubkey>,
    liquidity_source_pubkey: Pubkey,
    payer: &Keypair,
) -> (Pubkey, Reserve) {
    let reserve_keypair = Keypair::new();
    let reserve_pubkey = reserve_keypair.pubkey();
    let collateral_mint_keypair = Keypair::new();
    let collateral_supply_keypair = Keypair::new();
    let collateral_fees_receiver_keypair = Keypair::new();
    let liquidity_supply_keypair = Keypair::new();
    let user_collateral_token_keypair = Keypair::new();
    let user_transfer_authority = Keypair::new();

    let liquidity_source_account = client.get_account(&liquidity_source_pubkey).unwrap();
    let liquidity_source_token = Token::unpack(&liquidity_source_account.data).unwrap();
    let liquidity_mint_pubkey = liquidity_source_token.mint;

    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let token_balance = client
        .get_minimum_balance_for_rent_exemption(Token::LEN)
        .unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[
            create_account(
                &payer.pubkey(),
                &collateral_mint_keypair.pubkey(),
                client
                    .get_minimum_balance_for_rent_exemption(Mint::LEN)
                    .unwrap(),
                Mint::LEN as u64,
                &spl_token::id(),
            ),
            create_account(
                &payer.pubkey(),
                &collateral_supply_keypair.pubkey(),
                token_balance,
                Token::LEN as u64,
                &spl_token::id(),
            ),
            create_account(
                &payer.pubkey(),
                &collateral_fees_receiver_keypair.pubkey(),
                token_balance,
                Token::LEN as u64,
                &spl_token::id(),
            ),
            create_account(
                &payer.pubkey(),
                &liquidity_supply_keypair.pubkey(),
                token_balance,
                Token::LEN as u64,
                &spl_token::id(),
            ),
            create_account(
                &payer.pubkey(),
                &user_collateral_token_keypair.pubkey(),
                token_balance,
                Token::LEN as u64,
                &spl_token::id(),
            ),
            create_account(
                &payer.pubkey(),
                &reserve_pubkey,
                client
                    .get_minimum_balance_for_rent_exemption(Reserve::LEN)
                    .unwrap(),
                Reserve::LEN as u64,
                &id(),
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(
        &vec![
            payer,
            &reserve_keypair,
            &collateral_mint_keypair,
            &collateral_supply_keypair,
            &liquidity_supply_keypair,
            &user_collateral_token_keypair,
        ],
        recent_blockhash,
    );

    client.send_and_confirm_transaction(&transaction).unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[
            approve(
                &spl_token::id(),
                &liquidity_source_pubkey,
                &user_transfer_authority.pubkey(),
                &payer.pubkey(),
                &[],
                liquidity_source_token.amount,
            )
            .unwrap(),
            init_reserve(
                id(),
                liquidity_source_token.amount,
                config,
                liquidity_source_pubkey,
                user_collateral_token_keypair.pubkey(),
                reserve_pubkey,
                liquidity_mint_pubkey,
                liquidity_supply_keypair.pubkey(),
                collateral_mint_keypair.pubkey(),
                collateral_supply_keypair.pubkey(),
                collateral_fees_receiver_keypair.pubkey(),
                lending_market_keypair.pubkey(),
                user_transfer_authority.pubkey(),
                dex_market_pubkey,
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(
        &vec![payer, &lending_market_keypair, &user_transfer_authority],
        recent_blockhash,
    );

    client.send_and_confirm_transaction(&transaction).unwrap();

    let account = client.get_account(&reserve_pubkey).unwrap();
    (reserve_pubkey, Reserve::unpack(&account.data).unwrap())
}
