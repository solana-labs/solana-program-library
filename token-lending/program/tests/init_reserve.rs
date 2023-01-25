#![cfg(feature = "test-bpf")]

use crate::solend_program_test::BalanceChecker;
use crate::solend_program_test::MintAccount;
use crate::solend_program_test::MintSupplyChange;
use crate::solend_program_test::Oracle;
use crate::solend_program_test::TokenAccount;
use crate::solend_program_test::TokenBalanceChange;
use std::collections::HashSet;
use std::str::FromStr;
mod helpers;

use crate::solend_program_test::setup_world;
use crate::solend_program_test::Info;
use crate::solend_program_test::SolendProgramTest;
use crate::solend_program_test::User;
use helpers::*;
use solana_program::example_mocks::solana_sdk::Pubkey;
use solana_program::program_pack::Pack;
use solana_program_test::*;
use solana_sdk::{
    instruction::InstructionError,
    signature::{Keypair, Signer},
    transaction::TransactionError,
};
use solend_program::state::LastUpdate;
use solend_program::state::Reserve;
use solend_program::state::ReserveCollateral;
use solend_program::state::ReserveLiquidity;
use solend_program::state::PROGRAM_VERSION;
use solend_program::NULL_PUBKEY;
use solend_program::{
    error::LendingError,
    instruction::init_reserve,
    math::Decimal,
    state::{ReserveConfig, ReserveFees},
};
use solend_sdk::state::LendingMarket;
use spl_token::state::{Account as Token, Mint};

async fn setup() -> (SolendProgramTest, Info<LendingMarket>, User) {
    let (test, lending_market, _, _, lending_market_owner, _) =
        setup_world(&test_reserve_config(), &test_reserve_config()).await;

    (test, lending_market, lending_market_owner)
}

#[tokio::test]
async fn test_success() {
    let (mut test, lending_market, lending_market_owner) = setup().await;

    // create required pubkeys
    let reserve_keypair = Keypair::new();
    let destination_collateral_pubkey = test
        .create_account(Token::LEN, &spl_token::id(), None)
        .await;
    let reserve_liquidity_supply_pubkey = test
        .create_account(Token::LEN, &spl_token::id(), None)
        .await;
    let reserve_pubkey = test
        .create_account(Reserve::LEN, &solend_program::id(), Some(&reserve_keypair))
        .await;
    let reserve_liquidity_fee_receiver = test
        .create_account(Token::LEN, &spl_token::id(), None)
        .await;
    let reserve_collateral_mint_pubkey =
        test.create_account(Mint::LEN, &spl_token::id(), None).await;
    let reserve_collateral_supply_pubkey = test
        .create_account(Token::LEN, &spl_token::id(), None)
        .await;

    test.advance_clock_by_slots(1).await;

    let oracle = test.mints.get(&wsol_mint::id()).unwrap().unwrap();
    let reserve_config = ReserveConfig {
        fee_receiver: reserve_liquidity_fee_receiver,
        ..test_reserve_config()
    };

    let balance_checker = BalanceChecker::start(
        &mut test,
        &[
            &lending_market_owner,
            &TokenAccount(destination_collateral_pubkey),
            &TokenAccount(reserve_liquidity_supply_pubkey),
            &TokenAccount(reserve_liquidity_fee_receiver),
            &TokenAccount(reserve_collateral_supply_pubkey),
            &MintAccount(reserve_collateral_mint_pubkey),
        ],
    )
    .await;

    test.process_transaction(
        &[init_reserve(
            solend_program::id(),
            1000,
            reserve_config,
            lending_market_owner.get_account(&wsol_mint::id()).unwrap(),
            destination_collateral_pubkey,
            reserve_pubkey,
            wsol_mint::id(),
            reserve_liquidity_supply_pubkey,
            reserve_collateral_mint_pubkey,
            reserve_collateral_supply_pubkey,
            oracle.pyth_product_pubkey,
            oracle.pyth_price_pubkey,
            Pubkey::from_str("nu11111111111111111111111111111111111111111").unwrap(),
            lending_market.pubkey,
            lending_market_owner.keypair.pubkey(),
            lending_market_owner.keypair.pubkey(),
        )],
        Some(&[&lending_market_owner.keypair]),
    )
    .await
    .unwrap();

    // check token balances
    let (balance_changes, mint_supply_changes) =
        balance_checker.find_balance_changes(&mut test).await;
    let expected_balance_changes = HashSet::from([
        TokenBalanceChange {
            token_account: lending_market_owner.get_account(&wsol_mint::id()).unwrap(),
            mint: wsol_mint::id(),
            diff: -1000,
        },
        TokenBalanceChange {
            token_account: destination_collateral_pubkey,
            mint: reserve_collateral_mint_pubkey,
            diff: 1000,
        },
        TokenBalanceChange {
            token_account: reserve_liquidity_supply_pubkey,
            mint: wsol_mint::id(),
            diff: 1000,
        },
    ]);
    assert_eq!(balance_changes, expected_balance_changes);

    assert_eq!(
        mint_supply_changes,
        HashSet::from([MintSupplyChange {
            mint: reserve_collateral_mint_pubkey,
            diff: 1000,
        }])
    );

    // check program state
    let wsol_reserve = test.load_account::<Reserve>(reserve_pubkey).await;
    assert_eq!(
        wsol_reserve.account,
        Reserve {
            version: PROGRAM_VERSION,
            last_update: LastUpdate {
                slot: 1001,
                stale: true
            },
            lending_market: lending_market.pubkey,
            liquidity: ReserveLiquidity {
                mint_pubkey: wsol_mint::id(),
                mint_decimals: 9,
                supply_pubkey: reserve_liquidity_supply_pubkey,
                pyth_oracle_pubkey: oracle.pyth_price_pubkey,
                switchboard_oracle_pubkey: NULL_PUBKEY,
                available_amount: 1000,
                borrowed_amount_wads: Decimal::zero(),
                cumulative_borrow_rate_wads: Decimal::one(),
                accumulated_protocol_fees_wads: Decimal::zero(),
                market_price: Decimal::from(10u64),
            },
            collateral: ReserveCollateral {
                mint_pubkey: reserve_collateral_mint_pubkey,
                mint_total_supply: 1000,
                supply_pubkey: reserve_collateral_supply_pubkey,
            },
            config: reserve_config
        }
    );
}

#[tokio::test]
async fn test_init_reserve_null_oracles() {
    let (mut test, lending_market, lending_market_owner) = setup().await;

    let res = test
        .init_reserve(
            &lending_market,
            &lending_market_owner,
            &wsol_mint::id(),
            &test_reserve_config(),
            &Keypair::new(),
            1000,
            Some(Oracle {
                pyth_product_pubkey: NULL_PUBKEY,
                pyth_price_pubkey: NULL_PUBKEY,
                switchboard_feed_pubkey: Some(NULL_PUBKEY),
            }),
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        res,
        TransactionError::InstructionError(
            1,
            InstructionError::Custom(LendingError::InvalidOracleConfig as u32)
        )
    );
}

#[tokio::test]
async fn test_already_initialized() {
    let (mut test, lending_market, lending_market_owner) = setup().await;

    let keypair = Keypair::new();
    test.init_reserve(
        &lending_market,
        &lending_market_owner,
        &wsol_mint::id(),
        &test_reserve_config(),
        &keypair,
        1000,
        None,
    )
    .await
    .unwrap();

    let res = test
        .init_reserve(
            &lending_market,
            &lending_market_owner,
            &wsol_mint::id(),
            &test_reserve_config(),
            &keypair,
            1000,
            None,
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        res,
        TransactionError::InstructionError(
            1,
            InstructionError::Custom(LendingError::AlreadyInitialized as u32)
        )
    );
}

#[tokio::test]
async fn test_invalid_fees() {
    let (mut test, lending_market, lending_market_owner) = setup().await;

    let invalid_fees = [
        // borrow fee over 100%
        ReserveFees {
            borrow_fee_wad: 1_000_000_000_000_000_001,
            flash_loan_fee_wad: 1_000_000_000_000_000_001,
            host_fee_percentage: 0,
        },
        // host fee pct over 100%
        ReserveFees {
            borrow_fee_wad: 10_000_000_000_000_000,
            flash_loan_fee_wad: 10_000_000_000_000_000,
            host_fee_percentage: 101,
        },
    ];

    for fees in invalid_fees {
        let res = test
            .init_reserve(
                &lending_market,
                &lending_market_owner,
                &usdc_mint::id(),
                &ReserveConfig {
                    fees,
                    ..test_reserve_config()
                },
                &Keypair::new(),
                1000,
                None,
            )
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            res,
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(LendingError::InvalidConfig as u32)
            )
        );
    }
}

#[tokio::test]
async fn test_update_reserve_config() {
    let (mut test, lending_market, lending_market_owner) = setup().await;

    let wsol_reserve = test
        .init_reserve(
            &lending_market,
            &lending_market_owner,
            &wsol_mint::id(),
            &test_reserve_config(),
            &Keypair::new(),
            1000,
            None,
        )
        .await
        .unwrap();

    let new_reserve_config = test_reserve_config();
    lending_market
        .update_reserve_config(
            &mut test,
            &lending_market_owner,
            &wsol_reserve,
            new_reserve_config,
            None,
        )
        .await
        .unwrap();

    let wsol_reserve_post = test.load_account::<Reserve>(wsol_reserve.pubkey).await;
    assert_eq!(
        wsol_reserve_post.account,
        Reserve {
            config: new_reserve_config,
            ..wsol_reserve.account
        }
    );
}

#[tokio::test]
async fn test_update_invalid_oracle_config() {
    let (mut test, lending_market, lending_market_owner) = setup().await;
    let wsol_reserve = test
        .init_reserve(
            &lending_market,
            &lending_market_owner,
            &wsol_mint::id(),
            &test_reserve_config(),
            &Keypair::new(),
            1000,
            None,
        )
        .await
        .unwrap();

    let oracle = test.mints.get(&wsol_mint::id()).unwrap().unwrap();

    let new_reserve_config = test_reserve_config();

    // Try setting both of the oracles to null: Should fail
    let res = lending_market
        .update_reserve_config(
            &mut test,
            &lending_market_owner,
            &wsol_reserve,
            new_reserve_config,
            Some(&Oracle {
                pyth_product_pubkey: oracle.pyth_product_pubkey,
                pyth_price_pubkey: NULL_PUBKEY,
                switchboard_feed_pubkey: Some(NULL_PUBKEY),
            }),
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        res,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::InvalidOracleConfig as u32)
        )
    );
}
