#![cfg(feature = "test-bpf")]

mod helpers;

use crate::solend_program_test::MintSupplyChange;
use std::collections::HashSet;

use helpers::solend_program_test::{
    setup_world, BalanceChecker, Info, SolendProgramTest, TokenBalanceChange, User,
};
use helpers::*;
use solana_program_test::*;
use solana_sdk::signature::Keypair;

use solend_program::math::Decimal;
use solend_program::state::{
    LastUpdate, LendingMarket, Obligation, ObligationCollateral, Reserve, ReserveCollateral,
    ReserveLiquidity,
};

async fn setup() -> (
    SolendProgramTest,
    Info<LendingMarket>,
    Info<Reserve>,
    User,
    Info<Obligation>,
) {
    let (mut test, lending_market, usdc_reserve, _, _, user) =
        setup_world(&test_reserve_config(), &test_reserve_config()).await;

    let obligation = lending_market
        .init_obligation(&mut test, Keypair::new(), &user)
        .await
        .expect("This should succeed");

    (test, lending_market, usdc_reserve, user, obligation)
}

#[tokio::test]
async fn test_success() {
    let (mut test, lending_market, usdc_reserve, user, obligation) = setup().await;

    test.advance_clock_by_slots(1).await;

    let balance_checker = BalanceChecker::start(&mut test, &[&usdc_reserve, &user]).await;

    // deposit
    lending_market
        .deposit_reserve_liquidity_and_obligation_collateral(
            &mut test,
            &usdc_reserve,
            &obligation,
            &user,
            1_000_000,
        )
        .await
        .expect("this should succeed");

    // check token balances
    let (token_balance_changes, mint_supply_changes) =
        balance_checker.find_balance_changes(&mut test).await;

    assert_eq!(
        token_balance_changes,
        HashSet::from([
            TokenBalanceChange {
                token_account: user.get_account(&usdc_mint::id()).unwrap(),
                mint: usdc_mint::id(),
                diff: -1_000_000,
            },
            TokenBalanceChange {
                token_account: usdc_reserve.account.collateral.supply_pubkey,
                mint: usdc_reserve.account.collateral.mint_pubkey,
                diff: 1_000_000,
            },
            TokenBalanceChange {
                token_account: usdc_reserve.account.liquidity.supply_pubkey,
                mint: usdc_reserve.account.liquidity.mint_pubkey,
                diff: 1_000_000,
            },
        ]),
        "{:#?}",
        token_balance_changes
    );

    assert_eq!(
        mint_supply_changes,
        HashSet::from([MintSupplyChange {
            mint: usdc_reserve.account.collateral.mint_pubkey,
            diff: 1_000_000,
        },]),
        "{:#?}",
        mint_supply_changes
    );

    // check program state
    let lending_market_post = test
        .load_account::<LendingMarket>(lending_market.pubkey)
        .await;
    assert_eq!(lending_market.account, lending_market_post.account);

    let usdc_reserve_post = test.load_account::<Reserve>(usdc_reserve.pubkey).await;
    assert_eq!(
        usdc_reserve_post.account,
        Reserve {
            last_update: LastUpdate {
                slot: 1001,
                stale: false,
            },
            liquidity: ReserveLiquidity {
                available_amount: usdc_reserve.account.liquidity.available_amount + 1_000_000,
                ..usdc_reserve.account.liquidity
            },
            collateral: ReserveCollateral {
                mint_total_supply: usdc_reserve.account.collateral.mint_total_supply + 1_000_000,
                ..usdc_reserve.account.collateral
            },
            ..usdc_reserve.account
        }
    );

    let obligation_post = test.load_account::<Obligation>(obligation.pubkey).await;
    assert_eq!(
        obligation_post.account,
        Obligation {
            last_update: LastUpdate {
                slot: 1000,
                stale: true
            },
            deposits: [ObligationCollateral {
                deposit_reserve: usdc_reserve.pubkey,
                deposited_amount: 1_000_000,
                market_value: Decimal::zero()
            }]
            .to_vec(),
            ..obligation.account
        }
    );
}
