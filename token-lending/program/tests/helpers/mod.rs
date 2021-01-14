#![allow(dead_code)]

use assert_matches::*;
use solana_program::{program_option::COption, program_pack::Pack, pubkey::Pubkey};
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    signature::{read_keypair_file, Keypair, Signer},
    system_instruction::create_account,
    transaction::{Transaction, TransactionError},
};
use spl_token::{
    instruction::approve,
    state::{Account as Token, AccountState, Mint},
};
use spl_token_lending::{
    instruction::{
        borrow_reserve_liquidity, deposit_reserve_liquidity, init_lending_market, init_reserve,
        liquidate_obligation, BorrowAmountType,
    },
    math::Decimal,
    processor::process_instruction,
    state::{
        LendingMarket, Obligation, Reserve, ReserveConfig, ReserveFees, ReserveState,
        INITIAL_COLLATERAL_RATE, PROGRAM_VERSION,
    },
};
use std::str::FromStr;
pub mod genesis;
use genesis::GenesisAccounts;

pub const TEST_RESERVE_CONFIG: ReserveConfig = ReserveConfig {
    optimal_utilization_rate: 80,
    loan_to_value_ratio: 50,
    liquidation_bonus: 5,
    liquidation_threshold: 50,
    min_borrow_rate: 0,
    optimal_borrow_rate: 4,
    max_borrow_rate: 30,
    fees: ReserveFees {
        borrow_fee_wad: 100_000_000_000,
        /// 0.00001% (Aave borrow fee)
        host_fee_percentage: 20,
    },
};

pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const SRM_MINT: &str = "SRMuApVNdxXokk5GT7XD5cUUgXMBCoAz2LHeuAoKWRt";

pub const SOL_USDC_MARKET: &str = "7xMDbYTCqQEcK2aM9LbetGtNFJpzKdfXzLL5juaLh4GJ";
pub const SOL_USDC_BIDS: &str = "4VndUfHkmh6RWTQbXSVjY3wbSfqGjoPbuPHMoatV272H";
pub const SOL_USDC_ASKS: &str = "6LTxKpMyGnbHM5rRx7f3eZHF9q3gnUBV5ucXF9LvrB3M";
pub const SRM_USDC_MARKET: &str = "CDdR97S8y96v3To93aKvi3nCnjUrbuVSuumw8FLvbVeg";
pub const SRM_USDC_BIDS: &str = "DkxpXtF1EyjHomQcEhH54498gdhUN3t1sZCjReFNYZZn";
pub const SRM_USDC_ASKS: &str = "DRqgRZqfdD6PLHKSU7ydyVXWMUpvkqhzLZ1JSKn1iB1K";

#[allow(non_camel_case_types)]
pub enum TestDexMarketPair {
    SRM_USDC,
    SOL_USDC,
}

pub struct LendingTest {
    pub sol_usdc_dex_market: TestDexMarket,
    pub srm_usdc_dex_market: TestDexMarket,
    pub usdc_mint: TestQuoteMint,
    pub srm_mint: TestQuoteMint,
}

pub fn setup_test() -> (ProgramTest, LendingTest) {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    let usdc_mint = add_usdc_mint(&mut test);
    let srm_mint = add_srm_mint(&mut test);

    let sol_usdc_dex_market = TestDexMarket::setup(&mut test, TestDexMarketPair::SOL_USDC);
    let srm_usdc_dex_market = TestDexMarket::setup(&mut test, TestDexMarketPair::SRM_USDC);

    (
        test,
        LendingTest {
            sol_usdc_dex_market,
            srm_usdc_dex_market,
            usdc_mint,
            srm_mint,
        },
    )
}

trait AddPacked {
    fn add_packable_account<T: Pack>(
        &mut self,
        pubkey: Pubkey,
        amount: u64,
        data: &T,
        owner: &Pubkey,
    );
}

impl AddPacked for ProgramTest {
    fn add_packable_account<T: Pack>(
        &mut self,
        pubkey: Pubkey,
        amount: u64,
        data: &T,
        owner: &Pubkey,
    ) {
        let mut account = Account::new(amount, T::get_packed_len(), owner);
        data.pack_into_slice(&mut account.data);
        self.add_account(pubkey, account);
    }
}

pub fn add_lending_market(test: &mut ProgramTest, quote_token_mint: Pubkey) -> TestLendingMarket {
    let keypair = Keypair::new();
    let pubkey = keypair.pubkey();
    let (authority, bump_seed) =
        Pubkey::find_program_address(&[pubkey.as_ref()], &spl_token_lending::id());

    test.add_packable_account(
        pubkey,
        u32::MAX as u64,
        &LendingMarket {
            version: PROGRAM_VERSION,
            bump_seed,
            quote_token_mint,
            token_program_id: spl_token::id(),
        },
        &spl_token_lending::id(),
    );

    TestLendingMarket {
        keypair,
        authority,
        quote_token_mint,
    }
}

pub struct AddObligationArgs<'a> {
    pub slots_elapsed: u64,
    pub borrow_reserve: &'a TestReserve,
    pub collateral_reserve: &'a TestReserve,
    pub collateral_amount: u64,
    pub borrowed_liquidity_wads: Decimal,
}

pub fn add_obligation(
    test: &mut ProgramTest,
    user_accounts_owner: &Keypair,
    lending_market: &TestLendingMarket,
    args: AddObligationArgs,
) -> TestObligation {
    let AddObligationArgs {
        slots_elapsed,
        borrow_reserve,
        collateral_reserve,
        collateral_amount,
        borrowed_liquidity_wads,
    } = args;

    let token_mint_pubkey = Pubkey::new_unique();
    test.add_packable_account(
        token_mint_pubkey,
        u32::MAX as u64,
        &Mint {
            is_initialized: true,
            decimals: collateral_reserve.liquidity_mint_decimals,
            mint_authority: COption::Some(lending_market.authority),
            supply: collateral_amount,
            ..Mint::default()
        },
        &spl_token::id(),
    );

    let token_account_pubkey = Pubkey::new_unique();
    test.add_packable_account(
        token_account_pubkey,
        u32::MAX as u64,
        &Token {
            mint: token_mint_pubkey,
            owner: user_accounts_owner.pubkey(),
            state: AccountState::Initialized,
            amount: collateral_amount,
            ..Token::default()
        },
        &spl_token::id(),
    );

    let obligation_keypair = Keypair::new();
    let obligation_pubkey = obligation_keypair.pubkey();
    test.add_packable_account(
        obligation_pubkey,
        u32::MAX as u64,
        &Obligation {
            version: PROGRAM_VERSION,
            last_update_slot: 1u64.wrapping_sub(slots_elapsed),
            deposited_collateral_tokens: collateral_amount,
            collateral_reserve: collateral_reserve.pubkey,
            cumulative_borrow_rate_wads: Decimal::one(),
            borrowed_liquidity_wads,
            borrow_reserve: borrow_reserve.pubkey,
            token_mint: token_mint_pubkey,
        },
        &spl_token_lending::id(),
    );

    TestObligation {
        keypair: obligation_keypair,
        token_mint: token_mint_pubkey,
        token_account: token_account_pubkey,
    }
}

#[derive(Default)]
pub struct AddReserveArgs {
    pub name: String,
    pub slots_elapsed: u64,
    pub config: ReserveConfig,
    pub liquidity_amount: u64,
    pub liquidity_mint_pubkey: Pubkey,
    pub liquidity_mint_decimals: u8,
    pub user_liquidity_amount: u64,
    pub borrow_amount: u64,
    pub collateral_amount: u64,
    pub fees_amount: u64,
    pub dex_market_pubkey: Option<Pubkey>,
}

pub fn add_reserve(
    test: &mut ProgramTest,
    user_accounts_owner: &Keypair,
    lending_market: &TestLendingMarket,
    args: AddReserveArgs,
) -> TestReserve {
    let AddReserveArgs {
        name,
        slots_elapsed,
        config,
        liquidity_amount,
        liquidity_mint_pubkey,
        liquidity_mint_decimals,
        user_liquidity_amount,
        borrow_amount,
        collateral_amount,
        fees_amount,
        dex_market_pubkey,
    } = args;

    let is_native = if liquidity_mint_pubkey == spl_token::native_mint::id() {
        COption::Some(1)
    } else {
        COption::None
    };

    let collateral_mint_pubkey = Pubkey::new_unique();
    test.add_packable_account(
        collateral_mint_pubkey,
        u32::MAX as u64,
        &Mint {
            is_initialized: true,
            decimals: liquidity_mint_decimals,
            mint_authority: COption::Some(lending_market.authority),
            supply: collateral_amount,
            ..Mint::default()
        },
        &spl_token::id(),
    );

    let collateral_supply_pubkey = Pubkey::new_unique();
    test.add_packable_account(
        collateral_supply_pubkey,
        u32::MAX as u64,
        &Token {
            mint: collateral_mint_pubkey,
            owner: lending_market.authority,
            amount: collateral_amount,
            state: AccountState::Initialized,
            ..Token::default()
        },
        &spl_token::id(),
    );

    let collateral_fees_receiver_pubkey = Pubkey::new_unique();
    test.add_packable_account(
        collateral_fees_receiver_pubkey,
        u32::MAX as u64,
        &Token {
            mint: collateral_mint_pubkey,
            owner: lending_market.keypair.pubkey(),
            amount: fees_amount,
            state: AccountState::Initialized,
            ..Token::default()
        },
        &spl_token::id(),
    );

    let collateral_host_pubkey = Pubkey::new_unique();
    test.add_packable_account(
        collateral_host_pubkey,
        u32::MAX as u64,
        &Token {
            mint: collateral_mint_pubkey,
            owner: user_accounts_owner.pubkey(),
            amount: fees_amount,
            state: AccountState::Initialized,
            ..Token::default()
        },
        &spl_token::id(),
    );

    let amount = if let COption::Some(rent_reserve) = is_native {
        liquidity_amount + rent_reserve
    } else {
        u32::MAX as u64
    };

    let liquidity_supply_pubkey = Pubkey::new_unique();
    test.add_packable_account(
        liquidity_supply_pubkey,
        amount,
        &Token {
            mint: liquidity_mint_pubkey,
            owner: lending_market.authority,
            amount: liquidity_amount,
            state: AccountState::Initialized,
            is_native,
            ..Token::default()
        },
        &spl_token::id(),
    );

    let reserve_keypair = Keypair::new();
    let reserve_pubkey = reserve_keypair.pubkey();
    let mut reserve_state = ReserveState::new(1u64.wrapping_sub(slots_elapsed), liquidity_amount);
    reserve_state.add_borrow(borrow_amount).unwrap();
    test.add_packable_account(
        reserve_pubkey,
        u32::MAX as u64,
        &Reserve {
            version: PROGRAM_VERSION,
            lending_market: lending_market.keypair.pubkey(),
            liquidity_mint: liquidity_mint_pubkey,
            liquidity_mint_decimals,
            liquidity_supply: liquidity_supply_pubkey,
            collateral_mint: collateral_mint_pubkey,
            collateral_supply: collateral_supply_pubkey,
            collateral_fees_receiver: collateral_fees_receiver_pubkey,
            dex_market: dex_market_pubkey.into(),
            config,
            state: reserve_state,
        },
        &spl_token_lending::id(),
    );

    let amount = if let COption::Some(rent_reserve) = is_native {
        user_liquidity_amount + rent_reserve
    } else {
        u32::MAX as u64
    };

    let user_liquidity_pubkey = Pubkey::new_unique();
    test.add_packable_account(
        user_liquidity_pubkey,
        amount,
        &Token {
            mint: liquidity_mint_pubkey,
            owner: user_accounts_owner.pubkey(),
            amount: user_liquidity_amount,
            state: AccountState::Initialized,
            is_native,
            ..Token::default()
        },
        &spl_token::id(),
    );
    let user_collateral_pubkey = Pubkey::new_unique();
    test.add_packable_account(
        user_collateral_pubkey,
        u32::MAX as u64,
        &Token {
            mint: collateral_mint_pubkey,
            owner: user_accounts_owner.pubkey(),
            amount: liquidity_amount * INITIAL_COLLATERAL_RATE,
            state: AccountState::Initialized,
            ..Token::default()
        },
        &spl_token::id(),
    );

    TestReserve {
        name,
        pubkey: reserve_pubkey,
        lending_market: lending_market.keypair.pubkey(),
        config,
        liquidity_mint: liquidity_mint_pubkey,
        liquidity_mint_decimals,
        liquidity_supply: liquidity_supply_pubkey,
        collateral_mint: collateral_mint_pubkey,
        collateral_supply: collateral_supply_pubkey,
        collateral_fees_receiver: collateral_fees_receiver_pubkey,
        collateral_host: collateral_host_pubkey,
        user_liquidity_account: user_liquidity_pubkey,
        user_collateral_account: user_collateral_pubkey,
        dex_market: dex_market_pubkey,
    }
}

pub struct TestLendingMarket {
    pub keypair: Keypair,
    pub authority: Pubkey,
    pub quote_token_mint: Pubkey,
}

pub struct BorrowArgs<'a> {
    pub deposit_reserve: &'a TestReserve,
    pub borrow_reserve: &'a TestReserve,
    pub borrow_amount_type: BorrowAmountType,
    pub amount: u64,
    pub dex_market: &'a TestDexMarket,
    pub user_accounts_owner: &'a Keypair,
    pub obligation: Option<TestObligation>,
}

pub struct LiquidateArgs<'a> {
    pub repay_reserve: &'a TestReserve,
    pub withdraw_reserve: &'a TestReserve,
    pub obligation: &'a TestObligation,
    pub amount: u64,
    pub dex_market: &'a TestDexMarket,
    pub user_accounts_owner: &'a Keypair,
}

impl TestLendingMarket {
    pub async fn init(
        banks_client: &mut BanksClient,
        quote_token_mint: Pubkey,
        payer: &Keypair,
    ) -> Self {
        let keypair = read_keypair_file("tests/fixtures/lending_market.json").unwrap();
        let pubkey = keypair.pubkey();
        let (authority_pubkey, _bump_seed) =
            Pubkey::find_program_address(&[&pubkey.to_bytes()[..32]], &spl_token_lending::id());

        let rent = banks_client.get_rent().await.unwrap();
        let mut transaction = Transaction::new_with_payer(
            &[
                create_account(
                    &payer.pubkey(),
                    &pubkey,
                    rent.minimum_balance(LendingMarket::LEN),
                    LendingMarket::LEN as u64,
                    &spl_token_lending::id(),
                ),
                init_lending_market(spl_token_lending::id(), pubkey, quote_token_mint),
            ],
            Some(&payer.pubkey()),
        );

        let recent_blockhash = banks_client.get_recent_blockhash().await.unwrap();
        transaction.sign(&[&payer, &keypair], recent_blockhash);
        assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));

        TestLendingMarket {
            keypair,
            authority: authority_pubkey,
            quote_token_mint,
        }
    }

    pub async fn deposit(
        &self,
        banks_client: &mut BanksClient,
        user_accounts_owner: &Keypair,
        payer: &Keypair,
        reserve: &TestReserve,
        amount: u64,
    ) {
        let user_transfer_authority = Keypair::new();
        let mut transaction = Transaction::new_with_payer(
            &[
                approve(
                    &spl_token::id(),
                    &reserve.user_liquidity_account,
                    &user_transfer_authority.pubkey(),
                    &user_accounts_owner.pubkey(),
                    &[],
                    amount,
                )
                .unwrap(),
                deposit_reserve_liquidity(
                    spl_token_lending::id(),
                    amount,
                    reserve.user_liquidity_account,
                    reserve.user_collateral_account,
                    reserve.pubkey,
                    reserve.liquidity_supply,
                    reserve.collateral_mint,
                    self.keypair.pubkey(),
                    self.authority,
                    user_transfer_authority.pubkey(),
                ),
            ],
            Some(&payer.pubkey()),
        );

        let recent_blockhash = banks_client.get_recent_blockhash().await.unwrap();
        transaction.sign(
            &[payer, user_accounts_owner, &user_transfer_authority],
            recent_blockhash,
        );

        assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));
    }

    pub async fn liquidate(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        args: LiquidateArgs<'_>,
    ) {
        let LiquidateArgs {
            repay_reserve,
            withdraw_reserve,
            obligation,
            amount,
            dex_market,
            user_accounts_owner,
        } = args;

        let dex_market_orders_pubkey = if repay_reserve.dex_market.is_none() {
            dex_market.asks_pubkey
        } else {
            dex_market.bids_pubkey
        };

        let memory_keypair = Keypair::new();
        let user_transfer_authority = Keypair::new();
        let mut transaction = Transaction::new_with_payer(
            &[
                create_account(
                    &payer.pubkey(),
                    &memory_keypair.pubkey(),
                    0,
                    65548,
                    &spl_token_lending::id(),
                ),
                approve(
                    &spl_token::id(),
                    &repay_reserve.user_liquidity_account,
                    &user_transfer_authority.pubkey(),
                    &user_accounts_owner.pubkey(),
                    &[],
                    amount,
                )
                .unwrap(),
                liquidate_obligation(
                    spl_token_lending::id(),
                    amount,
                    repay_reserve.user_liquidity_account,
                    withdraw_reserve.user_collateral_account,
                    repay_reserve.pubkey,
                    repay_reserve.liquidity_supply,
                    withdraw_reserve.pubkey,
                    withdraw_reserve.collateral_supply,
                    obligation.keypair.pubkey(),
                    self.keypair.pubkey(),
                    self.authority,
                    user_transfer_authority.pubkey(),
                    dex_market.pubkey,
                    dex_market_orders_pubkey,
                    memory_keypair.pubkey(),
                ),
            ],
            Some(&payer.pubkey()),
        );

        let recent_blockhash = banks_client.get_recent_blockhash().await.unwrap();
        transaction.sign(
            &[
                &payer,
                &memory_keypair,
                &user_accounts_owner,
                &user_transfer_authority,
            ],
            recent_blockhash,
        );
        assert!(banks_client.process_transaction(transaction).await.is_ok());
    }

    pub async fn borrow(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        args: BorrowArgs<'_>,
    ) -> TestObligation {
        let rent = banks_client.get_rent().await.unwrap();
        let memory_keypair = Keypair::new();
        let user_transfer_authority = Keypair::new();

        let BorrowArgs {
            borrow_reserve,
            deposit_reserve,
            borrow_amount_type,
            amount,
            dex_market,
            user_accounts_owner,
            obligation,
        } = args;

        let dex_market_orders_pubkey = if deposit_reserve.dex_market.is_none() {
            dex_market.asks_pubkey
        } else {
            dex_market.bids_pubkey
        };

        let approve_amount = if borrow_amount_type == BorrowAmountType::CollateralDepositAmount {
            amount
        } else {
            get_token_balance(banks_client, deposit_reserve.user_collateral_account).await
        };

        let obligation = if let Some(obligation) = obligation {
            obligation
        } else {
            let obligation_token_mint_keypair = Keypair::new();
            let obligation_token_account_keypair = Keypair::new();
            let obligation = TestObligation {
                keypair: Keypair::new(),
                token_mint: obligation_token_mint_keypair.pubkey(),
                token_account: obligation_token_account_keypair.pubkey(),
            };

            let mut transaction = Transaction::new_with_payer(
                &[
                    create_account(
                        &payer.pubkey(),
                        &obligation_token_mint_keypair.pubkey(),
                        rent.minimum_balance(Mint::LEN),
                        Mint::LEN as u64,
                        &spl_token::id(),
                    ),
                    create_account(
                        &payer.pubkey(),
                        &obligation_token_account_keypair.pubkey(),
                        rent.minimum_balance(Token::LEN),
                        Token::LEN as u64,
                        &spl_token::id(),
                    ),
                    create_account(
                        &payer.pubkey(),
                        &obligation.keypair.pubkey(),
                        rent.minimum_balance(Obligation::LEN),
                        Obligation::LEN as u64,
                        &spl_token_lending::id(),
                    ),
                ],
                Some(&payer.pubkey()),
            );

            let recent_blockhash = banks_client.get_recent_blockhash().await.unwrap();
            transaction.sign(
                &vec![
                    payer,
                    &obligation.keypair,
                    &obligation_token_account_keypair,
                    &obligation_token_mint_keypair,
                ],
                recent_blockhash,
            );

            assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));

            obligation
        };

        let mut transaction = Transaction::new_with_payer(
            &[
                approve(
                    &spl_token::id(),
                    &deposit_reserve.user_collateral_account,
                    &user_transfer_authority.pubkey(),
                    &user_accounts_owner.pubkey(),
                    &[],
                    approve_amount,
                )
                .unwrap(),
                create_account(
                    &payer.pubkey(),
                    &memory_keypair.pubkey(),
                    0,
                    65548,
                    &spl_token_lending::id(),
                ),
                borrow_reserve_liquidity(
                    spl_token_lending::id(),
                    amount,
                    borrow_amount_type,
                    deposit_reserve.user_collateral_account,
                    borrow_reserve.user_liquidity_account,
                    deposit_reserve.pubkey,
                    deposit_reserve.collateral_supply,
                    deposit_reserve.collateral_fees_receiver,
                    borrow_reserve.pubkey,
                    borrow_reserve.liquidity_supply,
                    self.keypair.pubkey(),
                    self.authority,
                    user_transfer_authority.pubkey(),
                    obligation.keypair.pubkey(),
                    obligation.token_mint,
                    obligation.token_account,
                    user_accounts_owner.pubkey(),
                    dex_market.pubkey,
                    dex_market_orders_pubkey,
                    memory_keypair.pubkey(),
                    Some(deposit_reserve.collateral_host),
                ),
            ],
            Some(&payer.pubkey()),
        );

        let recent_blockhash = banks_client.get_recent_blockhash().await.unwrap();
        transaction.sign(
            &vec![
                payer,
                user_accounts_owner,
                &memory_keypair,
                &user_transfer_authority,
            ],
            recent_blockhash,
        );

        assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));
        obligation
    }

    pub async fn get_state(&self, banks_client: &mut BanksClient) -> LendingMarket {
        let lending_market_account: Account = banks_client
            .get_account(self.keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        LendingMarket::unpack(&lending_market_account.data[..]).unwrap()
    }

    pub async fn add_to_genesis(
        &self,
        banks_client: &mut BanksClient,
        genesis_accounts: &mut GenesisAccounts,
    ) {
        println!("lending_market: {}", self.keypair.pubkey());
        genesis_accounts
            .fetch_and_insert(banks_client, self.keypair.pubkey())
            .await;
    }
}

#[derive(Debug)]
pub struct TestReserve {
    pub name: String,
    pub pubkey: Pubkey,
    pub lending_market: Pubkey,
    pub config: ReserveConfig,
    pub liquidity_mint: Pubkey,
    pub liquidity_mint_decimals: u8,
    pub liquidity_supply: Pubkey,
    pub collateral_mint: Pubkey,
    pub collateral_supply: Pubkey,
    pub collateral_fees_receiver: Pubkey,
    pub collateral_host: Pubkey,
    pub user_liquidity_account: Pubkey,
    pub user_collateral_account: Pubkey,
    pub dex_market: Option<Pubkey>,
}

impl TestReserve {
    #[allow(clippy::too_many_arguments)]
    pub async fn init(
        name: String,
        banks_client: &mut BanksClient,
        lending_market: &TestLendingMarket,
        reserve_amount: u64,
        config: ReserveConfig,
        liquidity_mint_pubkey: Pubkey,
        user_liquidity_account: Pubkey,
        payer: &Keypair,
        user_accounts_owner: &Keypair,
        dex_market: &TestDexMarket,
    ) -> Result<Self, TransactionError> {
        let reserve_keypair = Keypair::new();
        let reserve_pubkey = reserve_keypair.pubkey();
        let collateral_mint_keypair = Keypair::new();
        let collateral_supply_keypair = Keypair::new();
        let collateral_fees_receiver_keypair = Keypair::new();
        let collateral_host_keypair = Keypair::new();
        let liquidity_supply_keypair = Keypair::new();
        let user_collateral_token_keypair = Keypair::new();
        let user_transfer_authority_keypair = Keypair::new();

        let dex_market_pubkey = if liquidity_mint_pubkey != lending_market.quote_token_mint {
            Some(dex_market.pubkey)
        } else {
            None
        };

        let liquidity_mint_account = banks_client
            .get_account(liquidity_mint_pubkey)
            .await
            .unwrap()
            .unwrap();
        let liquidity_mint = Mint::unpack(&liquidity_mint_account.data[..]).unwrap();

        let rent = banks_client.get_rent().await.unwrap();
        let mut transaction = Transaction::new_with_payer(
            &[
                approve(
                    &spl_token::id(),
                    &user_liquidity_account,
                    &user_transfer_authority_keypair.pubkey(),
                    &user_accounts_owner.pubkey(),
                    &[],
                    reserve_amount,
                )
                .unwrap(),
                create_account(
                    &payer.pubkey(),
                    &collateral_mint_keypair.pubkey(),
                    rent.minimum_balance(Mint::LEN),
                    Mint::LEN as u64,
                    &spl_token::id(),
                ),
                create_account(
                    &payer.pubkey(),
                    &collateral_supply_keypair.pubkey(),
                    rent.minimum_balance(Token::LEN),
                    Token::LEN as u64,
                    &spl_token::id(),
                ),
                create_account(
                    &payer.pubkey(),
                    &collateral_fees_receiver_keypair.pubkey(),
                    rent.minimum_balance(Token::LEN),
                    Token::LEN as u64,
                    &spl_token::id(),
                ),
                create_account(
                    &payer.pubkey(),
                    &collateral_host_keypair.pubkey(),
                    rent.minimum_balance(Token::LEN),
                    Token::LEN as u64,
                    &spl_token::id(),
                ),
                create_account(
                    &payer.pubkey(),
                    &liquidity_supply_keypair.pubkey(),
                    rent.minimum_balance(Token::LEN),
                    Token::LEN as u64,
                    &spl_token::id(),
                ),
                create_account(
                    &payer.pubkey(),
                    &user_collateral_token_keypair.pubkey(),
                    rent.minimum_balance(Token::LEN),
                    Token::LEN as u64,
                    &spl_token::id(),
                ),
                create_account(
                    &payer.pubkey(),
                    &reserve_pubkey,
                    rent.minimum_balance(Reserve::LEN),
                    Reserve::LEN as u64,
                    &spl_token_lending::id(),
                ),
                init_reserve(
                    spl_token_lending::id(),
                    reserve_amount,
                    config,
                    user_liquidity_account,
                    user_collateral_token_keypair.pubkey(),
                    reserve_pubkey,
                    liquidity_mint_pubkey,
                    liquidity_supply_keypair.pubkey(),
                    collateral_mint_keypair.pubkey(),
                    collateral_supply_keypair.pubkey(),
                    collateral_fees_receiver_keypair.pubkey(),
                    lending_market.keypair.pubkey(),
                    user_transfer_authority_keypair.pubkey(),
                    dex_market_pubkey,
                ),
            ],
            Some(&payer.pubkey()),
        );

        let recent_blockhash = banks_client.get_recent_blockhash().await.unwrap();
        transaction.sign(
            &vec![
                payer,
                user_accounts_owner,
                &reserve_keypair,
                &lending_market.keypair,
                &collateral_mint_keypair,
                &collateral_supply_keypair,
                &collateral_fees_receiver_keypair,
                &collateral_host_keypair,
                &liquidity_supply_keypair,
                &user_collateral_token_keypair,
                &user_transfer_authority_keypair,
            ],
            recent_blockhash,
        );

        banks_client
            .process_transaction(transaction)
            .await
            .map(|_| Self {
                name,
                pubkey: reserve_pubkey,
                lending_market: lending_market.keypair.pubkey(),
                config,
                liquidity_mint: liquidity_mint_pubkey,
                liquidity_mint_decimals: liquidity_mint.decimals,
                liquidity_supply: liquidity_supply_keypair.pubkey(),
                collateral_mint: collateral_mint_keypair.pubkey(),
                collateral_supply: collateral_supply_keypair.pubkey(),
                collateral_fees_receiver: collateral_fees_receiver_keypair.pubkey(),
                collateral_host: collateral_host_keypair.pubkey(),
                user_liquidity_account,
                user_collateral_account: user_collateral_token_keypair.pubkey(),
                dex_market: dex_market_pubkey,
            })
            .map_err(|e| e.unwrap())
    }

    pub async fn add_to_genesis(
        &self,
        banks_client: &mut BanksClient,
        genesis_accounts: &mut GenesisAccounts,
    ) {
        println!("{}_reserve: {}", self.name, self.pubkey);
        genesis_accounts
            .fetch_and_insert(banks_client, self.pubkey)
            .await;
        println!("{}_collateral_mint: {}", self.name, self.collateral_mint);
        genesis_accounts
            .fetch_and_insert(banks_client, self.collateral_mint)
            .await;
        println!(
            "{}_collateral_supply: {}",
            self.name, self.collateral_supply
        );
        genesis_accounts
            .fetch_and_insert(banks_client, self.collateral_fees_receiver)
            .await;
        println!(
            "{}_collateral_fees_receiver: {}",
            self.name, self.collateral_fees_receiver
        );
        genesis_accounts
            .fetch_and_insert(banks_client, self.collateral_supply)
            .await;
        if &self.name != "sol" {
            println!("{}_liquidity_mint: {}", self.name, self.liquidity_mint);
            genesis_accounts
                .fetch_and_insert(banks_client, self.liquidity_mint)
                .await;
        }
        println!("{}_liquidity_supply: {}", self.name, self.liquidity_supply);
        genesis_accounts
            .fetch_and_insert(banks_client, self.liquidity_supply)
            .await;
        println!(
            "{}_user_collateral: {}",
            self.name, self.user_collateral_account
        );
        genesis_accounts
            .fetch_and_insert(banks_client, self.user_collateral_account)
            .await;
        println!(
            "{}_user_liquidity: {}",
            self.name, self.user_liquidity_account
        );
        genesis_accounts
            .fetch_and_insert(banks_client, self.user_liquidity_account)
            .await;
    }

    pub async fn get_state(&self, banks_client: &mut BanksClient) -> Reserve {
        let reserve_account: Account = banks_client
            .get_account(self.pubkey)
            .await
            .unwrap()
            .unwrap();
        Reserve::unpack(&reserve_account.data[..]).unwrap()
    }

    pub async fn validate_state(&self, banks_client: &mut BanksClient) {
        let reserve_state = self.get_state(banks_client).await;
        assert!(reserve_state.state.last_update_slot > 0);
        assert_eq!(PROGRAM_VERSION, reserve_state.version);
        assert_eq!(self.lending_market, reserve_state.lending_market);
        assert_eq!(self.liquidity_mint, reserve_state.liquidity_mint);
        assert_eq!(self.liquidity_supply, reserve_state.liquidity_supply);
        assert_eq!(self.collateral_mint, reserve_state.collateral_mint);
        assert_eq!(self.collateral_supply, reserve_state.collateral_supply);
        assert_eq!(self.config, reserve_state.config);

        let dex_market_coption = if let Some(dex_market_pubkey) = self.dex_market {
            COption::Some(dex_market_pubkey)
        } else {
            COption::None
        };

        assert_eq!(dex_market_coption, reserve_state.dex_market);
        assert_eq!(
            reserve_state.state.cumulative_borrow_rate_wads,
            Decimal::one()
        );
        assert_eq!(reserve_state.state.borrowed_liquidity_wads, Decimal::zero());
        assert!(reserve_state.state.available_liquidity > 0);
        assert!(reserve_state.state.collateral_mint_supply > 0);
    }
}

pub struct TestObligation {
    pub keypair: Keypair,
    pub token_mint: Pubkey,
    pub token_account: Pubkey,
}

impl TestObligation {
    pub async fn get_state(&self, banks_client: &mut BanksClient) -> Obligation {
        let obligation_account: Account = banks_client
            .get_account(self.keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        Obligation::unpack(&obligation_account.data[..]).unwrap()
    }
}

pub struct TestQuoteMint {
    pub pubkey: Pubkey,
    pub authority: Keypair,
    pub decimals: u8,
}

pub fn add_usdc_mint(test: &mut ProgramTest) -> TestQuoteMint {
    let authority = Keypair::new();
    let pubkey = Pubkey::from_str(USDC_MINT).unwrap();
    let decimals = 6;
    test.add_packable_account(
        pubkey,
        u32::MAX as u64,
        &Mint {
            is_initialized: true,
            mint_authority: COption::Some(authority.pubkey()),
            decimals,
            ..Mint::default()
        },
        &spl_token::id(),
    );
    TestQuoteMint {
        pubkey,
        authority,
        decimals,
    }
}

pub fn add_srm_mint(test: &mut ProgramTest) -> TestQuoteMint {
    let authority = Keypair::new();
    let pubkey = Pubkey::from_str(SRM_MINT).unwrap();
    let decimals = 6;
    test.add_packable_account(
        pubkey,
        u32::MAX as u64,
        &Mint {
            is_initialized: true,
            mint_authority: COption::Some(authority.pubkey()),
            decimals,
            ..Mint::default()
        },
        &spl_token::id(),
    );
    TestQuoteMint {
        pubkey,
        authority,
        decimals,
    }
}

pub struct TestDexMarket {
    pub name: String,
    pub pubkey: Pubkey,
    pub bids_pubkey: Pubkey,
    pub asks_pubkey: Pubkey,
}

impl TestDexMarket {
    pub fn setup(test: &mut ProgramTest, market_pair: TestDexMarketPair) -> TestDexMarket {
        let (name, pubkey, bids_pubkey, asks_pubkey) = match market_pair {
            TestDexMarketPair::SOL_USDC => {
                ("sol_usdc", SOL_USDC_MARKET, SOL_USDC_BIDS, SOL_USDC_ASKS)
            }
            TestDexMarketPair::SRM_USDC => {
                ("srm_usdc", SRM_USDC_MARKET, SRM_USDC_BIDS, SRM_USDC_ASKS)
            }
        };

        let pubkey = Pubkey::from_str(pubkey).unwrap();
        let bids_pubkey = Pubkey::from_str(bids_pubkey).unwrap();
        let asks_pubkey = Pubkey::from_str(asks_pubkey).unwrap();

        test.add_account_with_file_data(
            pubkey,
            u32::MAX as u64,
            Pubkey::new(&[0; 32]),
            &format!("{}_dex_market.bin", name),
        );

        test.add_account_with_file_data(
            bids_pubkey,
            u32::MAX as u64,
            Pubkey::new(&[0; 32]),
            &format!("{}_dex_market_bids.bin", name),
        );

        test.add_account_with_file_data(
            asks_pubkey,
            u32::MAX as u64,
            Pubkey::new(&[0; 32]),
            &format!("{}_dex_market_asks.bin", name),
        );

        Self {
            name: name.to_string(),
            pubkey,
            bids_pubkey,
            asks_pubkey,
        }
    }

    pub async fn add_to_genesis(
        &self,
        banks_client: &mut BanksClient,
        genesis_accounts: &mut GenesisAccounts,
    ) {
        println!("{}_dex_market: {}", self.name, self.pubkey);
        genesis_accounts
            .fetch_and_insert(banks_client, self.pubkey)
            .await;
        println!("{}_dex_market_bids: {}", self.name, self.bids_pubkey);
        genesis_accounts
            .fetch_and_insert(banks_client, self.bids_pubkey)
            .await;
        println!("{}_dex_market_asks: {}", self.name, self.asks_pubkey);
        genesis_accounts
            .fetch_and_insert(banks_client, self.asks_pubkey)
            .await;
    }
}

pub async fn create_and_mint_to_token_account(
    banks_client: &mut BanksClient,
    mint_pubkey: Pubkey,
    mint_authority: Option<&Keypair>,
    payer: &Keypair,
    authority: Pubkey,
    amount: u64,
) -> Pubkey {
    if let Some(mint_authority) = mint_authority {
        let account_pubkey =
            create_token_account(banks_client, mint_pubkey, &payer, Some(authority), None).await;

        mint_to(
            banks_client,
            mint_pubkey,
            &payer,
            account_pubkey,
            mint_authority,
            amount,
        )
        .await;

        account_pubkey
    } else {
        create_token_account(
            banks_client,
            mint_pubkey,
            &payer,
            Some(authority),
            Some(amount),
        )
        .await
    }
}

pub async fn create_token_account(
    banks_client: &mut BanksClient,
    mint_pubkey: Pubkey,
    payer: &Keypair,
    authority: Option<Pubkey>,
    native_amount: Option<u64>,
) -> Pubkey {
    let token_keypair = Keypair::new();
    let token_pubkey = token_keypair.pubkey();
    let authority_pubkey = authority.unwrap_or_else(|| payer.pubkey());

    let rent = banks_client.get_rent().await.unwrap();
    let lamports = rent.minimum_balance(Token::LEN) + native_amount.unwrap_or_default();
    let mut transaction = Transaction::new_with_payer(
        &[
            create_account(
                &payer.pubkey(),
                &token_pubkey,
                lamports,
                Token::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &token_pubkey,
                &mint_pubkey,
                &authority_pubkey,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );

    let recent_blockhash = banks_client.get_recent_blockhash().await.unwrap();
    transaction.sign(&[&payer, &token_keypair], recent_blockhash);

    assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));

    token_pubkey
}

pub async fn mint_to(
    banks_client: &mut BanksClient,
    mint_pubkey: Pubkey,
    payer: &Keypair,
    account_pubkey: Pubkey,
    authority: &Keypair,
    amount: u64,
) {
    let mut transaction = Transaction::new_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
            &mint_pubkey,
            &account_pubkey,
            &authority.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );

    let recent_blockhash = banks_client.get_recent_blockhash().await.unwrap();
    transaction.sign(&[payer, authority], recent_blockhash);

    assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));
}

pub async fn get_token_balance(banks_client: &mut BanksClient, pubkey: Pubkey) -> u64 {
    let token: Account = banks_client.get_account(pubkey).await.unwrap().unwrap();

    spl_token::state::Account::unpack(&token.data[..])
        .unwrap()
        .amount
}
