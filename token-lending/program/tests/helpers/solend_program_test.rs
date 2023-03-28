use super::{
    flash_loan_proxy::proxy_program,
    mock_pyth::{init_switchboard, set_switchboard_price},
};
use crate::helpers::*;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solend_program::state::RateLimiterConfig;
use solend_sdk::{instruction::update_reserve_config, NULL_PUBKEY};

use pyth_sdk_solana::state::PROD_ACCT_SIZE;
use solana_program::{
    clock::Clock,
    instruction::Instruction,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, sysvar,
};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    signature::{Keypair, Signer},
    system_instruction::create_account,
    transaction::Transaction,
};
use solend_program::{
    instruction::{
        deposit_obligation_collateral, deposit_reserve_liquidity, init_lending_market,
        init_reserve, liquidate_obligation_and_redeem_reserve_collateral, redeem_fees,
        redeem_reserve_collateral, repay_obligation_liquidity, set_lending_market_owner_and_config,
        withdraw_obligation_collateral,
    },
    processor::process_instruction,
    state::{LendingMarket, Reserve, ReserveConfig},
};

use spl_token::state::{Account as Token, Mint};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use super::mock_pyth::{init, mock_pyth_program, set_price};

pub struct SolendProgramTest {
    pub context: ProgramTestContext,
    rent: Rent,

    // authority of all mints
    authority: Keypair,

    pub mints: HashMap<Pubkey, Option<Oracle>>,
}

#[derive(Debug, Clone, Copy)]
pub struct Oracle {
    pub pyth_product_pubkey: Pubkey,
    pub pyth_price_pubkey: Pubkey,
    pub switchboard_feed_pubkey: Option<Pubkey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Info<T> {
    pub pubkey: Pubkey,
    pub account: T,
}

impl SolendProgramTest {
    pub async fn start_new() -> Self {
        let mut test = ProgramTest::new(
            "solend_program",
            solend_program::id(),
            processor!(process_instruction),
        );

        test.prefer_bpf(false);
        test.add_program(
            "mock_pyth",
            mock_pyth_program::id(),
            processor!(mock_pyth::process_instruction),
        );

        test.add_program(
            "flash_loan_proxy",
            proxy_program::id(),
            processor!(flash_loan_proxy::process_instruction),
        );

        let authority = Keypair::new();

        add_mint(&mut test, usdc_mint::id(), 6, authority.pubkey());
        add_mint(&mut test, usdt_mint::id(), 6, authority.pubkey());
        add_mint(&mut test, wsol_mint::id(), 9, authority.pubkey());

        let mut context = test.start_with_context().await;
        let rent = context.banks_client.get_rent().await.unwrap();

        SolendProgramTest {
            context,
            rent,
            authority,
            mints: HashMap::from([
                (usdc_mint::id(), None),
                (wsol_mint::id(), None),
                (usdt_mint::id(), None),
            ]),
        }
    }

    pub async fn process_transaction(
        &mut self,
        instructions: &[Instruction],
        signers: Option<&[&Keypair]>,
    ) -> Result<(), BanksClientError> {
        let mut transaction =
            Transaction::new_with_payer(instructions, Some(&self.context.payer.pubkey()));

        let mut all_signers = vec![&self.context.payer];

        if let Some(signers) = signers {
            all_signers.extend_from_slice(signers);
        }

        // This fails when warping is involved - https://gitmemory.com/issue/solana-labs/solana/18201/868325078
        // let recent_blockhash = self.context.banks_client.get_recent_blockhash().await.unwrap();

        transaction.sign(&all_signers, self.context.last_blockhash);

        self.context
            .banks_client
            .process_transaction(transaction)
            .await
    }

    pub async fn load_optional_account<T: Pack + IsInitialized>(
        &mut self,
        acc_pk: Pubkey,
    ) -> Info<Option<T>> {
        self.context
            .banks_client
            .get_account(acc_pk)
            .await
            .unwrap()
            .map(|acc| Info {
                pubkey: acc_pk,
                account: T::unpack(&acc.data).ok(),
            })
            .unwrap()
    }

    pub async fn load_account<T: Pack + IsInitialized>(&mut self, acc_pk: Pubkey) -> Info<T> {
        let acc = self
            .context
            .banks_client
            .get_account(acc_pk)
            .await
            .unwrap()
            .unwrap();

        Info {
            pubkey: acc_pk,
            account: T::unpack(&acc.data).unwrap(),
        }
    }

    pub async fn get_bincode_account<T: serde::de::DeserializeOwned>(
        &mut self,
        address: &Pubkey,
    ) -> T {
        self.context
            .banks_client
            .get_account(*address)
            .await
            .unwrap()
            .map(|a| bincode::deserialize::<T>(&a.data).unwrap())
            .unwrap_or_else(|| panic!("GET-TEST-ACCOUNT-ERROR"))
    }

    #[allow(dead_code)]
    pub async fn get_clock(&mut self) -> Clock {
        self.get_bincode_account::<Clock>(&sysvar::clock::id())
            .await
    }

    /// Advances clock by x slots. note that transactions don't automatically increment the slot
    /// value in Clock, so this function must be explicitly called whenever you want time to move
    /// forward.
    pub async fn advance_clock_by_slots(&mut self, slots: u64) {
        let clock: Clock = self.get_clock().await;
        self.context.warp_to_slot(clock.slot + slots).unwrap();
    }

    pub async fn create_account(
        &mut self,
        size: usize,
        owner: &Pubkey,
        keypair: Option<&Keypair>,
    ) -> Pubkey {
        let rent = self.rent.minimum_balance(size);

        let new_keypair = Keypair::new();
        let keypair = match keypair {
            None => &new_keypair,
            Some(kp) => kp,
        };

        let instructions = [system_instruction::create_account(
            &self.context.payer.pubkey(),
            &keypair.pubkey(),
            rent as u64,
            size as u64,
            owner,
        )];

        self.process_transaction(&instructions, Some(&[keypair]))
            .await
            .unwrap();

        keypair.pubkey()
    }

    pub async fn create_mint(&mut self, mint_authority: &Pubkey) -> Pubkey {
        let keypair = Keypair::new();
        let rent = self.rent.minimum_balance(Mint::LEN);

        let instructions = [
            system_instruction::create_account(
                &self.context.payer.pubkey(),
                &keypair.pubkey(),
                rent,
                Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &keypair.pubkey(),
                mint_authority,
                None,
                0,
            )
            .unwrap(),
        ];

        self.process_transaction(&instructions, Some(&[&keypair]))
            .await
            .unwrap();

        keypair.pubkey()
    }

    pub async fn create_token_account(&mut self, owner: &Pubkey, mint: &Pubkey) -> Pubkey {
        let keypair = Keypair::new();
        let instructions = [
            system_instruction::create_account(
                &self.context.payer.pubkey(),
                &keypair.pubkey(),
                self.rent.minimum_balance(Token::LEN),
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &keypair.pubkey(),
                mint,
                owner,
            )
            .unwrap(),
        ];

        self.process_transaction(&instructions, Some(&[&keypair]))
            .await
            .unwrap();

        keypair.pubkey()
    }

    pub async fn mint_to(&mut self, mint: &Pubkey, dst: &Pubkey, amount: u64) {
        assert!(self.mints.contains_key(mint));

        let instructions = [spl_token::instruction::mint_to(
            &spl_token::id(),
            mint,
            dst,
            &self.authority.pubkey(),
            &[],
            amount,
        )
        .unwrap()];

        let authority = Keypair::from_bytes(&self.authority.to_bytes()).unwrap(); // hack
        self.process_transaction(&instructions, Some(&[&authority]))
            .await
            .unwrap();
    }

    // wrappers around solend instructions. these should be used to test logic things (eg you can't
    // borrow more than the borrow limit, but these methods can't be used to test account-level
    // security of an instruction (eg what happens if im not the lending market owner but i try to
    // add a reserve anyways).

    pub async fn init_lending_market(
        &mut self,
        owner: &User,
        lending_market_key: &Keypair,
    ) -> Result<Info<LendingMarket>, BanksClientError> {
        let payer = self.context.payer.pubkey();
        let lamports = Rent::minimum_balance(&self.rent, LendingMarket::LEN);

        let res = self
            .process_transaction(
                &[
                    create_account(
                        &payer,
                        &lending_market_key.pubkey(),
                        lamports,
                        LendingMarket::LEN as u64,
                        &solend_program::id(),
                    ),
                    init_lending_market(
                        solend_program::id(),
                        owner.keypair.pubkey(),
                        QUOTE_CURRENCY,
                        lending_market_key.pubkey(),
                        mock_pyth_program::id(),
                        mock_pyth_program::id(), // TODO suspicious
                    ),
                ],
                Some(&[lending_market_key]),
            )
            .await;

        match res {
            Ok(()) => Ok(self
                .load_account::<LendingMarket>(lending_market_key.pubkey())
                .await),
            Err(e) => Err(e),
        }
    }

    pub async fn init_pyth_feed(&mut self, mint: &Pubkey) {
        let pyth_price_pubkey = self
            .create_account(3312, &mock_pyth_program::id(), None)
            .await;
        let pyth_product_pubkey = self
            .create_account(PROD_ACCT_SIZE, &mock_pyth_program::id(), None)
            .await;

        self.process_transaction(
            &[init(
                mock_pyth_program::id(),
                pyth_price_pubkey,
                pyth_product_pubkey,
            )],
            None,
        )
        .await
        .unwrap();

        self.mints.insert(
            *mint,
            Some(Oracle {
                pyth_product_pubkey,
                pyth_price_pubkey,
                switchboard_feed_pubkey: None,
            }),
        );
    }

    pub async fn set_price(&mut self, mint: &Pubkey, price: &PriceArgs) {
        let oracle = self.mints.get(mint).unwrap().unwrap();
        self.process_transaction(
            &[set_price(
                mock_pyth_program::id(),
                oracle.pyth_price_pubkey,
                price.price,
                price.conf,
                price.expo,
                price.ema_price,
                price.ema_conf,
            )],
            None,
        )
        .await
        .unwrap();
    }

    pub async fn init_switchboard_feed(&mut self, mint: &Pubkey) -> Pubkey {
        let switchboard_feed_pubkey = self
            .create_account(
                std::mem::size_of::<AggregatorAccountData>() + 8,
                &mock_pyth_program::id(),
                None,
            )
            .await;

        self.process_transaction(
            &[init_switchboard(
                mock_pyth_program::id(),
                switchboard_feed_pubkey,
            )],
            None,
        )
        .await
        .unwrap();

        let oracle = self.mints.get_mut(mint).unwrap();
        if let Some(ref mut oracle) = oracle {
            oracle.switchboard_feed_pubkey = Some(switchboard_feed_pubkey);
            switchboard_feed_pubkey
        } else {
            panic!("oracle not initialized");
        }
    }

    pub async fn set_switchboard_price(&mut self, mint: &Pubkey, price: SwitchboardPriceArgs) {
        let oracle = self.mints.get(mint).unwrap().unwrap();
        self.process_transaction(
            &[set_switchboard_price(
                mock_pyth_program::id(),
                oracle.switchboard_feed_pubkey.unwrap(),
                price.price,
                price.expo,
            )],
            None,
        )
        .await
        .unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn init_reserve(
        &mut self,
        lending_market: &Info<LendingMarket>,
        lending_market_owner: &User,
        mint: &Pubkey,
        reserve_config: &ReserveConfig,
        reserve_keypair: &Keypair,
        liquidity_amount: u64,
        oracle: Option<Oracle>,
    ) -> Result<Info<Reserve>, BanksClientError> {
        let destination_collateral_pubkey = self
            .create_account(Token::LEN, &spl_token::id(), None)
            .await;
        let reserve_liquidity_supply_pubkey = self
            .create_account(Token::LEN, &spl_token::id(), None)
            .await;
        let reserve_pubkey = self
            .create_account(Reserve::LEN, &solend_program::id(), Some(reserve_keypair))
            .await;

        let reserve_liquidity_fee_receiver = self
            .create_account(Token::LEN, &spl_token::id(), None)
            .await;

        let reserve_collateral_mint_pubkey =
            self.create_account(Mint::LEN, &spl_token::id(), None).await;
        let reserve_collateral_supply_pubkey = self
            .create_account(Token::LEN, &spl_token::id(), None)
            .await;

        let oracle = if let Some(o) = oracle {
            o
        } else {
            self.mints.get(mint).unwrap().unwrap()
        };

        let res = self
            .process_transaction(
                &[
                    ComputeBudgetInstruction::set_compute_unit_limit(70_000),
                    init_reserve(
                        solend_program::id(),
                        liquidity_amount,
                        ReserveConfig {
                            fee_receiver: reserve_liquidity_fee_receiver,
                            ..*reserve_config
                        },
                        lending_market_owner.get_account(mint).unwrap(),
                        destination_collateral_pubkey,
                        reserve_pubkey,
                        *mint,
                        reserve_liquidity_supply_pubkey,
                        reserve_collateral_mint_pubkey,
                        reserve_collateral_supply_pubkey,
                        oracle.pyth_product_pubkey,
                        oracle.pyth_price_pubkey,
                        Pubkey::from_str("nu11111111111111111111111111111111111111111").unwrap(),
                        lending_market.pubkey,
                        lending_market_owner.keypair.pubkey(),
                        lending_market_owner.keypair.pubkey(),
                    ),
                ],
                Some(&[&lending_market_owner.keypair]),
            )
            .await;

        match res {
            Ok(()) => Ok(self.load_account::<Reserve>(reserve_pubkey).await),
            Err(e) => Err(e),
        }
    }
}

/// 1 User holds many token accounts
#[derive(Debug)]
pub struct User {
    pub keypair: Keypair,
    pub token_accounts: Vec<Info<Token>>,
}

impl User {
    pub fn new_with_keypair(keypair: Keypair) -> Self {
        User {
            keypair,
            token_accounts: Vec::new(),
        }
    }

    /// Creates a user with specified token accounts and balances. This function only works if the
    /// SolendProgramTest object owns the mint authorities. eg this won't work for native SOL.
    pub async fn new_with_balances(
        test: &mut SolendProgramTest,
        mints_and_balances: &[(&Pubkey, u64)],
    ) -> Self {
        let mut user = User {
            keypair: Keypair::new(),
            token_accounts: Vec::new(),
        };

        for (mint, balance) in mints_and_balances {
            let token_account = user.create_token_account(mint, test).await;
            if *balance > 0 {
                test.mint_to(mint, &token_account.pubkey, *balance).await;
            }
        }

        user
    }

    pub fn get_account(&self, mint: &Pubkey) -> Option<Pubkey> {
        self.token_accounts.iter().find_map(|ta| {
            if ta.account.mint == *mint {
                Some(ta.pubkey)
            } else {
                None
            }
        })
    }

    pub async fn get_balance(&self, test: &mut SolendProgramTest, mint: &Pubkey) -> Option<u64> {
        match self.get_account(mint) {
            None => None,
            Some(pubkey) => {
                let token_account = test.load_account::<Token>(pubkey).await;
                Some(token_account.account.amount)
            }
        }
    }

    pub async fn create_token_account(
        &mut self,
        mint: &Pubkey,
        test: &mut SolendProgramTest,
    ) -> Info<Token> {
        match self
            .token_accounts
            .iter()
            .find(|ta| ta.account.mint == *mint)
        {
            None => {
                let pubkey = test
                    .create_token_account(&self.keypair.pubkey(), mint)
                    .await;
                let account = test.load_account::<Token>(pubkey).await;

                self.token_accounts.push(account.clone());

                account
            }
            Some(t) => t.clone(),
        }
    }

    pub async fn transfer(
        &self,
        mint: &Pubkey,
        destination_pubkey: Pubkey,
        amount: u64,
        test: &mut SolendProgramTest,
    ) {
        let instruction = [spl_token::instruction::transfer(
            &spl_token::id(),
            &self.get_account(mint).unwrap(),
            &destination_pubkey,
            &self.keypair.pubkey(),
            &[],
            amount,
        )
        .unwrap()];

        test.process_transaction(&instruction, Some(&[&self.keypair]))
            .await
            .unwrap();
    }
}

pub struct PriceArgs {
    pub price: i64,
    pub conf: u64,
    pub expo: i32,
    pub ema_price: i64,
    pub ema_conf: u64,
}

pub struct SwitchboardPriceArgs {
    pub price: i64,
    pub expo: i32,
}

impl Info<LendingMarket> {
    pub async fn deposit(
        &self,
        test: &mut SolendProgramTest,
        reserve: &Info<Reserve>,
        user: &User,
        liquidity_amount: u64,
    ) -> Result<(), BanksClientError> {
        let instructions = [deposit_reserve_liquidity(
            solend_program::id(),
            liquidity_amount,
            user.get_account(&reserve.account.liquidity.mint_pubkey)
                .unwrap(),
            user.get_account(&reserve.account.collateral.mint_pubkey)
                .unwrap(),
            reserve.pubkey,
            reserve.account.liquidity.supply_pubkey,
            reserve.account.collateral.mint_pubkey,
            self.pubkey,
            user.keypair.pubkey(),
        )];

        test.process_transaction(&instructions, Some(&[&user.keypair]))
            .await
    }

    pub async fn update_reserve_config(
        &self,
        test: &mut SolendProgramTest,
        lending_market_owner: &User,
        reserve: &Info<Reserve>,
        config: ReserveConfig,
        rate_limiter_config: RateLimiterConfig,
        oracle: Option<&Oracle>,
    ) -> Result<(), BanksClientError> {
        let default_oracle = test
            .mints
            .get(&reserve.account.liquidity.mint_pubkey)
            .unwrap()
            .unwrap();
        let oracle = oracle.unwrap_or(&default_oracle);

        let instructions = [update_reserve_config(
            solend_program::id(),
            config,
            rate_limiter_config,
            reserve.pubkey,
            self.pubkey,
            lending_market_owner.keypair.pubkey(),
            oracle.pyth_product_pubkey,
            oracle.pyth_price_pubkey,
            oracle.switchboard_feed_pubkey.unwrap_or(NULL_PUBKEY),
        )];

        test.process_transaction(&instructions, Some(&[&lending_market_owner.keypair]))
            .await
    }

    pub async fn deposit_reserve_liquidity_and_obligation_collateral(
        &self,
        test: &mut SolendProgramTest,
        reserve: &Info<Reserve>,
        obligation: &Info<Obligation>,
        user: &User,
        liquidity_amount: u64,
    ) -> Result<(), BanksClientError> {
        let instructions = [deposit_reserve_liquidity_and_obligation_collateral(
            solend_program::id(),
            liquidity_amount,
            user.get_account(&reserve.account.liquidity.mint_pubkey)
                .unwrap(),
            user.get_account(&reserve.account.collateral.mint_pubkey)
                .unwrap(),
            reserve.pubkey,
            reserve.account.liquidity.supply_pubkey,
            reserve.account.collateral.mint_pubkey,
            self.pubkey,
            reserve.account.collateral.supply_pubkey,
            obligation.pubkey,
            user.keypair.pubkey(),
            reserve.account.liquidity.pyth_oracle_pubkey,
            reserve.account.liquidity.switchboard_oracle_pubkey,
            user.keypair.pubkey(),
        )];

        test.process_transaction(&instructions, Some(&[&user.keypair]))
            .await
    }

    pub async fn redeem(
        &self,
        test: &mut SolendProgramTest,
        reserve: &Info<Reserve>,
        user: &User,
        collateral_amount: u64,
    ) -> Result<(), BanksClientError> {
        let instructions = [
            refresh_reserve(
                solend_program::id(),
                reserve.pubkey,
                reserve.account.liquidity.pyth_oracle_pubkey,
                reserve.account.liquidity.switchboard_oracle_pubkey,
            ),
            redeem_reserve_collateral(
                solend_program::id(),
                collateral_amount,
                user.get_account(&reserve.account.collateral.mint_pubkey)
                    .unwrap(),
                user.get_account(&reserve.account.liquidity.mint_pubkey)
                    .unwrap(),
                reserve.pubkey,
                reserve.account.collateral.mint_pubkey,
                reserve.account.liquidity.supply_pubkey,
                self.pubkey,
                user.keypair.pubkey(),
            ),
        ];

        test.process_transaction(&instructions, Some(&[&user.keypair]))
            .await
    }

    pub async fn init_obligation(
        &self,
        test: &mut SolendProgramTest,
        obligation_keypair: Keypair,
        user: &User,
    ) -> Result<Info<Obligation>, BanksClientError> {
        let instructions = [
            system_instruction::create_account(
                &test.context.payer.pubkey(),
                &obligation_keypair.pubkey(),
                Rent::minimum_balance(&Rent::default(), Obligation::LEN),
                Obligation::LEN as u64,
                &solend_program::id(),
            ),
            init_obligation(
                solend_program::id(),
                obligation_keypair.pubkey(),
                self.pubkey,
                user.keypair.pubkey(),
            ),
        ];

        match test
            .process_transaction(&instructions, Some(&[&obligation_keypair, &user.keypair]))
            .await
        {
            Ok(()) => Ok(test
                .load_account::<Obligation>(obligation_keypair.pubkey())
                .await),
            Err(e) => Err(e),
        }
    }

    pub async fn deposit_obligation_collateral(
        &self,
        test: &mut SolendProgramTest,
        reserve: &Info<Reserve>,
        obligation: &Info<Obligation>,
        user: &User,
        collateral_amount: u64,
    ) -> Result<(), BanksClientError> {
        let instructions = [deposit_obligation_collateral(
            solend_program::id(),
            collateral_amount,
            user.get_account(&reserve.account.collateral.mint_pubkey)
                .unwrap(),
            reserve.account.collateral.supply_pubkey,
            reserve.pubkey,
            obligation.pubkey,
            self.pubkey,
            user.keypair.pubkey(),
            user.keypair.pubkey(),
        )];

        test.process_transaction(&instructions, Some(&[&user.keypair]))
            .await
    }

    pub async fn refresh_reserve(
        &self,
        test: &mut SolendProgramTest,
        reserve: &Info<Reserve>,
    ) -> Result<(), BanksClientError> {
        test.process_transaction(
            &[refresh_reserve(
                solend_program::id(),
                reserve.pubkey,
                reserve.account.liquidity.pyth_oracle_pubkey,
                reserve.account.liquidity.switchboard_oracle_pubkey,
            )],
            None,
        )
        .await
    }

    pub async fn build_refresh_instructions(
        &self,
        test: &mut SolendProgramTest,
        obligation: &Info<Obligation>,
        extra_reserve: Option<&Info<Reserve>>,
    ) -> Vec<Instruction> {
        let obligation = test.load_account::<Obligation>(obligation.pubkey).await;
        let reserve_pubkeys: Vec<Pubkey> = {
            let mut r = HashSet::new();
            r.extend(
                obligation
                    .account
                    .deposits
                    .iter()
                    .map(|d| d.deposit_reserve),
            );
            r.extend(obligation.account.borrows.iter().map(|b| b.borrow_reserve));

            if let Some(reserve) = extra_reserve {
                r.insert(reserve.pubkey);
            }

            r.into_iter().collect()
        };

        let mut reserves = Vec::new();
        for pubkey in reserve_pubkeys {
            reserves.push(test.load_account::<Reserve>(pubkey).await);
        }

        let mut instructions: Vec<Instruction> = reserves
            .into_iter()
            .map(|reserve| {
                refresh_reserve(
                    solend_program::id(),
                    reserve.pubkey,
                    reserve.account.liquidity.pyth_oracle_pubkey,
                    reserve.account.liquidity.switchboard_oracle_pubkey,
                )
            })
            .collect();

        let reserve_pubkeys: Vec<Pubkey> = {
            let mut r = Vec::new();
            r.extend(
                obligation
                    .account
                    .deposits
                    .iter()
                    .map(|d| d.deposit_reserve),
            );
            r.extend(obligation.account.borrows.iter().map(|b| b.borrow_reserve));
            r
        };

        instructions.push(refresh_obligation(
            solend_program::id(),
            obligation.pubkey,
            reserve_pubkeys,
        ));

        instructions
    }

    pub async fn refresh_obligation(
        &self,
        test: &mut SolendProgramTest,
        obligation: &Info<Obligation>,
    ) -> Result<(), BanksClientError> {
        let instructions = self
            .build_refresh_instructions(test, obligation, None)
            .await;

        test.process_transaction(&instructions, None).await
    }

    pub async fn borrow_obligation_liquidity(
        &self,
        test: &mut SolendProgramTest,
        borrow_reserve: &Info<Reserve>,
        obligation: &Info<Obligation>,
        user: &User,
        host_fee_receiver_pubkey: &Pubkey,
        liquidity_amount: u64,
    ) -> Result<(), BanksClientError> {
        let obligation = test.load_account::<Obligation>(obligation.pubkey).await;

        let mut instructions = self
            .build_refresh_instructions(test, &obligation, Some(borrow_reserve))
            .await;

        instructions.push(borrow_obligation_liquidity(
            solend_program::id(),
            liquidity_amount,
            borrow_reserve.account.liquidity.supply_pubkey,
            user.get_account(&borrow_reserve.account.liquidity.mint_pubkey)
                .unwrap(),
            borrow_reserve.pubkey,
            borrow_reserve.account.config.fee_receiver,
            obligation.pubkey,
            self.pubkey,
            user.keypair.pubkey(),
            Some(*host_fee_receiver_pubkey),
        ));

        test.process_transaction(&instructions, Some(&[&user.keypair]))
            .await
    }

    pub async fn repay_obligation_liquidity(
        &self,
        test: &mut SolendProgramTest,
        repay_reserve: &Info<Reserve>,
        obligation: &Info<Obligation>,
        user: &User,
        liquidity_amount: u64,
    ) -> Result<(), BanksClientError> {
        let instructions = [repay_obligation_liquidity(
            solend_program::id(),
            liquidity_amount,
            user.get_account(&repay_reserve.account.liquidity.mint_pubkey)
                .unwrap(),
            repay_reserve.account.liquidity.supply_pubkey,
            repay_reserve.pubkey,
            obligation.pubkey,
            self.pubkey,
            user.keypair.pubkey(),
        )];

        test.process_transaction(&instructions, Some(&[&user.keypair]))
            .await
    }

    pub async fn redeem_fees(
        &self,
        test: &mut SolendProgramTest,
        reserve: &Info<Reserve>,
    ) -> Result<(), BanksClientError> {
        let instructions = [
            refresh_reserve(
                solend_program::id(),
                reserve.pubkey,
                reserve.account.liquidity.pyth_oracle_pubkey,
                reserve.account.liquidity.switchboard_oracle_pubkey,
            ),
            redeem_fees(
                solend_program::id(),
                reserve.pubkey,
                reserve.account.config.fee_receiver,
                reserve.account.liquidity.supply_pubkey,
                self.pubkey,
            ),
        ];

        test.process_transaction(&instructions, None).await
    }

    pub async fn liquidate_obligation_and_redeem_reserve_collateral(
        &self,
        test: &mut SolendProgramTest,
        repay_reserve: &Info<Reserve>,
        withdraw_reserve: &Info<Reserve>,
        obligation: &Info<Obligation>,
        user: &User,
        liquidity_amount: u64,
    ) -> Result<(), BanksClientError> {
        let mut instructions = self
            .build_refresh_instructions(test, obligation, None)
            .await;

        instructions.push(liquidate_obligation_and_redeem_reserve_collateral(
            solend_program::id(),
            liquidity_amount,
            user.get_account(&repay_reserve.account.liquidity.mint_pubkey)
                .unwrap(),
            user.get_account(&withdraw_reserve.account.collateral.mint_pubkey)
                .unwrap(),
            user.get_account(&withdraw_reserve.account.liquidity.mint_pubkey)
                .unwrap(),
            repay_reserve.pubkey,
            repay_reserve.account.liquidity.supply_pubkey,
            withdraw_reserve.pubkey,
            withdraw_reserve.account.collateral.mint_pubkey,
            withdraw_reserve.account.collateral.supply_pubkey,
            withdraw_reserve.account.liquidity.supply_pubkey,
            withdraw_reserve.account.config.fee_receiver,
            obligation.pubkey,
            self.pubkey,
            user.keypair.pubkey(),
        ));

        test.process_transaction(&instructions, Some(&[&user.keypair]))
            .await
    }

    pub async fn liquidate_obligation(
        &self,
        test: &mut SolendProgramTest,
        repay_reserve: &Info<Reserve>,
        withdraw_reserve: &Info<Reserve>,
        obligation: &Info<Obligation>,
        user: &User,
        liquidity_amount: u64,
    ) -> Result<(), BanksClientError> {
        let mut instructions = self
            .build_refresh_instructions(test, obligation, None)
            .await;

        instructions.push(liquidate_obligation(
            solend_program::id(),
            liquidity_amount,
            user.get_account(&repay_reserve.account.liquidity.mint_pubkey)
                .unwrap(),
            user.get_account(&withdraw_reserve.account.collateral.mint_pubkey)
                .unwrap(),
            repay_reserve.pubkey,
            repay_reserve.account.liquidity.supply_pubkey,
            withdraw_reserve.pubkey,
            withdraw_reserve.account.collateral.supply_pubkey,
            obligation.pubkey,
            self.pubkey,
            user.keypair.pubkey(),
        ));

        test.process_transaction(&instructions, Some(&[&user.keypair]))
            .await
    }

    pub async fn withdraw_obligation_collateral_and_redeem_reserve_collateral(
        &self,
        test: &mut SolendProgramTest,
        withdraw_reserve: &Info<Reserve>,
        obligation: &Info<Obligation>,
        user: &User,
        collateral_amount: u64,
    ) -> Result<(), BanksClientError> {
        let obligation = test.load_account::<Obligation>(obligation.pubkey).await;

        let mut instructions = self
            .build_refresh_instructions(test, &obligation, Some(withdraw_reserve))
            .await;

        instructions.push(
            withdraw_obligation_collateral_and_redeem_reserve_collateral(
                solend_program::id(),
                collateral_amount,
                withdraw_reserve.account.collateral.supply_pubkey,
                user.get_account(&withdraw_reserve.account.collateral.mint_pubkey)
                    .unwrap(),
                withdraw_reserve.pubkey,
                obligation.pubkey,
                self.pubkey,
                user.get_account(&withdraw_reserve.account.liquidity.mint_pubkey)
                    .unwrap(),
                withdraw_reserve.account.collateral.mint_pubkey,
                withdraw_reserve.account.liquidity.supply_pubkey,
                user.keypair.pubkey(),
                user.keypair.pubkey(),
            ),
        );

        test.process_transaction(&instructions, Some(&[&user.keypair]))
            .await
    }

    pub async fn withdraw_obligation_collateral(
        &self,
        test: &mut SolendProgramTest,
        withdraw_reserve: &Info<Reserve>,
        obligation: &Info<Obligation>,
        user: &User,
        collateral_amount: u64,
    ) -> Result<(), BanksClientError> {
        let mut instructions = self
            .build_refresh_instructions(test, obligation, Some(withdraw_reserve))
            .await;

        instructions.push(withdraw_obligation_collateral(
            solend_program::id(),
            collateral_amount,
            withdraw_reserve.account.collateral.supply_pubkey,
            user.get_account(&withdraw_reserve.account.collateral.mint_pubkey)
                .unwrap(),
            withdraw_reserve.pubkey,
            obligation.pubkey,
            self.pubkey,
            user.keypair.pubkey(),
        ));

        test.process_transaction(&instructions, Some(&[&user.keypair]))
            .await
    }

    pub async fn set_lending_market_owner_and_config(
        &self,
        test: &mut SolendProgramTest,
        lending_market_owner: &User,
        new_owner: &Pubkey,
        config: RateLimiterConfig,
    ) -> Result<(), BanksClientError> {
        let instructions = [set_lending_market_owner_and_config(
            solend_program::id(),
            self.pubkey,
            lending_market_owner.keypair.pubkey(),
            *new_owner,
            config,
        )];

        test.process_transaction(&instructions, Some(&[&lending_market_owner.keypair]))
            .await
    }
}

/// Track token balance changes across transactions.
pub struct BalanceChecker {
    token_accounts: Vec<Info<Option<Token>>>,
    mint_accounts: Vec<Info<Option<Mint>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenBalanceChange {
    pub token_account: Pubkey,
    pub mint: Pubkey,
    pub diff: i128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MintSupplyChange {
    pub mint: Pubkey,
    pub diff: i128,
}

impl BalanceChecker {
    pub async fn start(test: &mut SolendProgramTest, objs: &[&dyn GetTokenAndMintPubkeys]) -> Self {
        let mut refreshed_token_accounts = Vec::new();
        let mut refreshed_mint_accounts = Vec::new();

        for obj in objs {
            let (token_pubkeys, mint_pubkeys) = obj.get_token_and_mint_pubkeys();

            for pubkey in token_pubkeys {
                let refreshed_account = test.load_optional_account::<Token>(pubkey).await;
                refreshed_token_accounts.push(refreshed_account);
            }

            for pubkey in mint_pubkeys {
                let refreshed_account = test.load_optional_account::<Mint>(pubkey).await;
                refreshed_mint_accounts.push(refreshed_account);
            }
        }

        BalanceChecker {
            token_accounts: refreshed_token_accounts,
            mint_accounts: refreshed_mint_accounts,
        }
    }

    pub async fn find_balance_changes(
        &self,
        test: &mut SolendProgramTest,
    ) -> (HashSet<TokenBalanceChange>, HashSet<MintSupplyChange>) {
        let mut token_balance_changes = HashSet::new();
        let mut mint_supply_changes = HashSet::new();

        for token_account in &self.token_accounts {
            let refreshed_token_account = test.load_account::<Token>(token_account.pubkey).await;
            match token_account.account {
                None => {
                    if refreshed_token_account.account.amount > 0 {
                        token_balance_changes.insert(TokenBalanceChange {
                            token_account: refreshed_token_account.pubkey,
                            mint: refreshed_token_account.account.mint,
                            diff: refreshed_token_account.account.amount as i128,
                        });
                    }
                }
                Some(token_account) => {
                    if refreshed_token_account.account.amount != token_account.amount {
                        token_balance_changes.insert(TokenBalanceChange {
                            token_account: refreshed_token_account.pubkey,
                            mint: token_account.mint,
                            diff: (refreshed_token_account.account.amount as i128)
                                - (token_account.amount as i128),
                        });
                    }
                }
            };
        }

        for mint_account in &self.mint_accounts {
            let refreshed_mint_account = test.load_account::<Mint>(mint_account.pubkey).await;
            match mint_account.account {
                None => {
                    if refreshed_mint_account.account.supply > 0 {
                        mint_supply_changes.insert(MintSupplyChange {
                            mint: refreshed_mint_account.pubkey,
                            diff: refreshed_mint_account.account.supply as i128,
                        });
                    }
                }
                Some(mint_account) => {
                    if refreshed_mint_account.account.supply != mint_account.supply {
                        mint_supply_changes.insert(MintSupplyChange {
                            mint: refreshed_mint_account.pubkey,
                            diff: (refreshed_mint_account.account.supply as i128)
                                - (mint_account.supply as i128),
                        });
                    }
                }
            };
        }

        (token_balance_changes, mint_supply_changes)
    }
}

/// trait that tracks token and mint accounts associated with a specific struct
pub trait GetTokenAndMintPubkeys {
    fn get_token_and_mint_pubkeys(&self) -> (Vec<Pubkey>, Vec<Pubkey>);
}

impl GetTokenAndMintPubkeys for User {
    fn get_token_and_mint_pubkeys(&self) -> (Vec<Pubkey>, Vec<Pubkey>) {
        (
            self.token_accounts.iter().map(|a| a.pubkey).collect(),
            vec![],
        )
    }
}

impl GetTokenAndMintPubkeys for Info<Reserve> {
    fn get_token_and_mint_pubkeys(&self) -> (Vec<Pubkey>, Vec<Pubkey>) {
        (
            vec![
                self.account.liquidity.supply_pubkey,
                self.account.collateral.supply_pubkey,
                self.account.config.fee_receiver,
            ],
            vec![
                self.account.liquidity.mint_pubkey,
                self.account.collateral.mint_pubkey,
            ],
        )
    }
}

pub struct MintAccount(pub Pubkey);
pub struct TokenAccount(pub Pubkey);

impl GetTokenAndMintPubkeys for MintAccount {
    fn get_token_and_mint_pubkeys(&self) -> (Vec<Pubkey>, Vec<Pubkey>) {
        (vec![], vec![self.0])
    }
}

impl GetTokenAndMintPubkeys for TokenAccount {
    fn get_token_and_mint_pubkeys(&self) -> (Vec<Pubkey>, Vec<Pubkey>) {
        (vec![self.0], vec![])
    }
}

/// Init's a lending market with a usdc reserve and wsol reserve.
pub async fn setup_world(
    usdc_reserve_config: &ReserveConfig,
    wsol_reserve_config: &ReserveConfig,
) -> (
    SolendProgramTest,
    Info<LendingMarket>,
    Info<Reserve>,
    Info<Reserve>,
    User,
    User,
) {
    let mut test = SolendProgramTest::start_new().await;

    let lending_market_owner = User::new_with_balances(
        &mut test,
        &[
            (&usdc_mint::id(), 2_000_000),
            (&wsol_mint::id(), 2 * LAMPORTS_TO_SOL),
        ],
    )
    .await;

    let lending_market = test
        .init_lending_market(&lending_market_owner, &Keypair::new())
        .await
        .unwrap();

    test.advance_clock_by_slots(999).await;

    test.init_pyth_feed(&usdc_mint::id()).await;
    test.set_price(
        &usdc_mint::id(),
        &PriceArgs {
            price: 1,
            conf: 0,
            expo: 0,
            ema_price: 1,
            ema_conf: 0,
        },
    )
    .await;

    test.init_pyth_feed(&wsol_mint::id()).await;
    test.set_price(
        &wsol_mint::id(),
        &PriceArgs {
            price: 10,
            conf: 0,
            expo: 0,
            ema_price: 10,
            ema_conf: 0,
        },
    )
    .await;

    let usdc_reserve = test
        .init_reserve(
            &lending_market,
            &lending_market_owner,
            &usdc_mint::id(),
            usdc_reserve_config,
            &Keypair::new(),
            1_000_000,
            None,
        )
        .await
        .unwrap();

    let wsol_reserve = test
        .init_reserve(
            &lending_market,
            &lending_market_owner,
            &wsol_mint::id(),
            wsol_reserve_config,
            &Keypair::new(),
            LAMPORTS_TO_SOL,
            None,
        )
        .await
        .unwrap();

    let user = User::new_with_balances(
        &mut test,
        &[
            (&usdc_mint::id(), 1_000_000_000_000),             // 1M USDC
            (&usdc_reserve.account.collateral.mint_pubkey, 0), // cUSDC
            (&wsol_mint::id(), 10 * LAMPORTS_TO_SOL),
            (&wsol_reserve.account.collateral.mint_pubkey, 0), // cSOL
        ],
    )
    .await;

    (
        test,
        lending_market,
        usdc_reserve,
        wsol_reserve,
        lending_market_owner,
        user,
    )
}

/// Scenario 1
/// sol = $10
/// usdc = $1
/// LendingMarket
/// - USDC Reserve
/// - WSOL Reserve
/// Obligation
/// - 100k USDC deposit
/// - 10 SOL borrowed
/// no interest has accrued on anything yet, ie:
/// - cUSDC/USDC = 1
/// - cSOL/SOL = 1
/// - Obligation owes _exactly_ 10 SOL
/// slot is 999, so the next tx that runs will be at slot 1000
pub async fn scenario_1(
    usdc_reserve_config: &ReserveConfig,
    wsol_reserve_config: &ReserveConfig,
) -> (
    SolendProgramTest,
    Info<LendingMarket>,
    Info<Reserve>,
    Info<Reserve>,
    User,
    Info<Obligation>,
) {
    let (mut test, lending_market, usdc_reserve, wsol_reserve, lending_market_owner, user) =
        setup_world(usdc_reserve_config, wsol_reserve_config).await;

    // init obligation
    let obligation = lending_market
        .init_obligation(&mut test, Keypair::new(), &user)
        .await
        .expect("This should succeed");

    // deposit 100k USDC
    lending_market
        .deposit(&mut test, &usdc_reserve, &user, 100_000_000_000)
        .await
        .expect("This should succeed");

    let usdc_reserve = test.load_account(usdc_reserve.pubkey).await;

    // deposit 100k cUSDC
    lending_market
        .deposit_obligation_collateral(
            &mut test,
            &usdc_reserve,
            &obligation,
            &user,
            100_000_000_000,
        )
        .await
        .expect("This should succeed");

    let wsol_depositor = User::new_with_balances(
        &mut test,
        &[
            (&wsol_mint::id(), 9 * LAMPORTS_PER_SOL),
            (&wsol_reserve.account.collateral.mint_pubkey, 0),
        ],
    )
    .await;

    // deposit 9 SOL. wSOL reserve now has 10 SOL.
    lending_market
        .deposit(
            &mut test,
            &wsol_reserve,
            &wsol_depositor,
            9 * LAMPORTS_PER_SOL,
        )
        .await
        .unwrap();

    // borrow 10 SOL against 100k cUSDC.
    let obligation = test.load_account::<Obligation>(obligation.pubkey).await;
    lending_market
        .borrow_obligation_liquidity(
            &mut test,
            &wsol_reserve,
            &obligation,
            &user,
            &lending_market_owner.get_account(&wsol_mint::id()).unwrap(),
            u64::MAX,
        )
        .await
        .unwrap();

    // populate market price correctly
    lending_market
        .refresh_reserve(&mut test, &wsol_reserve)
        .await
        .unwrap();

    // populate deposit value correctly.
    let obligation = test.load_account::<Obligation>(obligation.pubkey).await;
    lending_market
        .refresh_obligation(&mut test, &obligation)
        .await
        .unwrap();

    let lending_market = test.load_account(lending_market.pubkey).await;
    let usdc_reserve = test.load_account(usdc_reserve.pubkey).await;
    let wsol_reserve = test.load_account(wsol_reserve.pubkey).await;
    let obligation = test.load_account::<Obligation>(obligation.pubkey).await;

    (
        test,
        lending_market,
        usdc_reserve,
        wsol_reserve,
        user,
        obligation,
    )
}

pub struct ReserveArgs {
    pub mint: Pubkey,
    pub config: ReserveConfig,
    pub liquidity_amount: u64,
    pub price: PriceArgs,
}

pub struct ObligationArgs {
    pub deposits: Vec<(Pubkey, u64)>,
    pub borrows: Vec<(Pubkey, u64)>,
}

pub async fn custom_scenario(
    reserve_args: &[ReserveArgs],
    obligation_args: &ObligationArgs,
) -> (
    SolendProgramTest,
    Info<LendingMarket>,
    Vec<Info<Reserve>>,
    Info<Obligation>,
    User,
) {
    let mut test = SolendProgramTest::start_new().await;
    let mints_and_liquidity_amounts = reserve_args
        .iter()
        .map(|reserve_arg| (&reserve_arg.mint, reserve_arg.liquidity_amount))
        .collect::<Vec<_>>();

    let lending_market_owner =
        User::new_with_balances(&mut test, &mints_and_liquidity_amounts).await;

    let lending_market = test
        .init_lending_market(&lending_market_owner, &Keypair::new())
        .await
        .unwrap();

    let deposits_and_balances = obligation_args
        .deposits
        .iter()
        .map(|(mint, amount)| (mint, *amount))
        .collect::<Vec<_>>();

    let mut obligation_owner = User::new_with_balances(&mut test, &deposits_and_balances).await;

    let obligation = lending_market
        .init_obligation(&mut test, Keypair::new(), &obligation_owner)
        .await
        .unwrap();

    test.advance_clock_by_slots(999).await;

    let mut reserves = Vec::new();
    for reserve_arg in reserve_args {
        test.init_pyth_feed(&reserve_arg.mint).await;

        test.set_price(&reserve_arg.mint, &reserve_arg.price).await;

        let reserve = test
            .init_reserve(
                &lending_market,
                &lending_market_owner,
                &reserve_arg.mint,
                &reserve_arg.config,
                &Keypair::new(),
                reserve_arg.liquidity_amount,
                None,
            )
            .await
            .unwrap();

        let user = User::new_with_balances(
            &mut test,
            &[
                (&reserve_arg.mint, reserve_arg.liquidity_amount),
                (&reserve.account.collateral.mint_pubkey, 0),
            ],
        )
        .await;

        lending_market
            .deposit(&mut test, &reserve, &user, reserve_arg.liquidity_amount)
            .await
            .unwrap();

        obligation_owner
            .create_token_account(&reserve_arg.mint, &mut test)
            .await;

        reserves.push(reserve);
    }

    for (mint, amount) in obligation_args.deposits.iter() {
        let reserve = reserves
            .iter()
            .find(|reserve| reserve.account.liquidity.mint_pubkey == *mint)
            .unwrap();

        obligation_owner
            .create_token_account(&reserve.account.collateral.mint_pubkey, &mut test)
            .await;

        lending_market
            .deposit_reserve_liquidity_and_obligation_collateral(
                &mut test,
                reserve,
                &obligation,
                &obligation_owner,
                *amount,
            )
            .await
            .unwrap();
    }

    for (mint, amount) in obligation_args.borrows.iter() {
        let reserve = reserves
            .iter()
            .find(|reserve| reserve.account.liquidity.mint_pubkey == *mint)
            .unwrap();

        obligation_owner.create_token_account(mint, &mut test).await;
        let fee_receiver = User::new_with_balances(&mut test, &[(mint, 0)]).await;

        lending_market
            .borrow_obligation_liquidity(
                &mut test,
                reserve,
                &obligation,
                &obligation_owner,
                &fee_receiver.get_account(mint).unwrap(),
                *amount,
            )
            .await
            .unwrap();
    }

    (test, lending_market, reserves, obligation, obligation_owner)
}

pub fn find_reserve(reserves: &[Info<Reserve>], mint: &Pubkey) -> Option<Info<Reserve>> {
    reserves
        .iter()
        .find(|reserve| reserve.account.liquidity.mint_pubkey == *mint)
        .cloned()
}
