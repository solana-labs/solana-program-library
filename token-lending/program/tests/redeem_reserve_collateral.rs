#![cfg(feature = "test-bpf")]

mod helpers;

use crate::solend_program_test::MintSupplyChange;
use std::collections::HashSet;

use helpers::solend_program_test::{
    setup_world, BalanceChecker, Info, SolendProgramTest, TokenBalanceChange, User,
};
use helpers::*;
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::transaction::TransactionError;
use solend_program::state::{
    LastUpdate, LendingMarket, Reserve, ReserveCollateral, ReserveLiquidity,
};

pub async fn setup() -> (SolendProgramTest, Info<LendingMarket>, Info<Reserve>, User) {
    let (mut test, lending_market, usdc_reserve, _, _, user) =
        setup_world(&test_reserve_config(), &test_reserve_config()).await;

    lending_market
        .deposit(&mut test, &usdc_reserve, &user, 1_000_000)
        .await
        .expect("this should succeed");

    let lending_market = test
        .load_account::<LendingMarket>(lending_market.pubkey)
        .await;

    let usdc_reserve = test.load_account::<Reserve>(usdc_reserve.pubkey).await;

    (test, lending_market, usdc_reserve, user)
}

#[tokio::test]
async fn test_success() {
    let (mut test, lending_market, usdc_reserve, user) = setup().await;

    let balance_checker = BalanceChecker::start(&mut test, &[&usdc_reserve, &user]).await;

    lending_market
        .redeem(&mut test, &usdc_reserve, &user, 1_000_000)
        .await
        .expect("This should succeed");

    // check token balances
    let (balance_changes, mint_supply_changes) =
        balance_checker.find_balance_changes(&mut test).await;

    assert_eq!(
        balance_changes,
        HashSet::from([
            TokenBalanceChange {
                token_account: user.get_account(&usdc_mint::id()).unwrap(),
                mint: usdc_mint::id(),
                diff: 1_000_000,
            },
            TokenBalanceChange {
                token_account: user
                    .get_account(&usdc_reserve.account.collateral.mint_pubkey)
                    .unwrap(),
                mint: usdc_reserve.account.collateral.mint_pubkey,
                diff: -1_000_000,
            },
            TokenBalanceChange {
                token_account: usdc_reserve.account.liquidity.supply_pubkey,
                mint: usdc_reserve.account.liquidity.mint_pubkey,
                diff: -1_000_000,
            },
        ]),
        "{:#?}",
        balance_changes
    );
    assert_eq!(
        mint_supply_changes,
        HashSet::from([MintSupplyChange {
            mint: usdc_reserve.account.collateral.mint_pubkey,
            diff: -1_000_000,
        },]),
        "{:#?}",
        mint_supply_changes
    );

    // check program state changes
    let lending_market_post = test
        .load_account::<LendingMarket>(lending_market.pubkey)
        .await;
    assert_eq!(lending_market.account, lending_market_post.account);

    let usdc_reserve_post = test.load_account::<Reserve>(usdc_reserve.pubkey).await;
    assert_eq!(
        usdc_reserve_post.account,
        Reserve {
            last_update: LastUpdate {
                slot: 1000,
                stale: true
            },
            liquidity: ReserveLiquidity {
                available_amount: usdc_reserve.account.liquidity.available_amount - 1_000_000,
                ..usdc_reserve.account.liquidity
            },
            collateral: ReserveCollateral {
                mint_total_supply: usdc_reserve.account.collateral.mint_total_supply - 1_000_000,
                ..usdc_reserve.account.collateral
            },
            ..usdc_reserve.account
        }
    );
}

#[tokio::test]
async fn test_fail_redeem_too_much() {
    let (mut test, lending_market, usdc_reserve, user) = setup().await;

    let res = lending_market
        .redeem(&mut test, &usdc_reserve, &user, 1_000_001)
        .await
        .err()
        .unwrap()
        .unwrap();

    match res {
        // TokenError::Insufficient Funds
        TransactionError::InstructionError(0, InstructionError::Custom(1)) => (),
        // LendingError::TokenBurnFailed
        TransactionError::InstructionError(0, InstructionError::Custom(19)) => (),
        _ => panic!("Unexpected error: {:#?}", res),
    };
}
