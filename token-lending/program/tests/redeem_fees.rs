#![cfg(feature = "test-bpf")]

mod helpers;

use crate::solend_program_test::scenario_1;
use crate::solend_program_test::BalanceChecker;
use crate::solend_program_test::PriceArgs;
use crate::solend_program_test::TokenBalanceChange;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solend_program::state::LastUpdate;
use solend_program::state::ReserveLiquidity;
use solend_program::state::{Reserve, ReserveConfig};
use std::collections::HashSet;

use helpers::*;
use solana_program_test::*;
use solend_program::{
    math::{Decimal, TrySub},
    state::SLOTS_PER_YEAR,
};

#[tokio::test]
async fn test_success() {
    let (mut test, lending_market, _, wsol_reserve, user, _) = scenario_1(
        &test_reserve_config(),
        &ReserveConfig {
            protocol_take_rate: 10,
            ..test_reserve_config()
        },
    )
    .await;

    test.advance_clock_by_slots(SLOTS_PER_YEAR).await;

    test.set_price(
        &wsol_mint::id(),
        &PriceArgs {
            price: 10,
            expo: 0,
            conf: 0,
            ema_price: 10,
            ema_conf: 0,
        },
    )
    .await;

    lending_market
        .refresh_reserve(&mut test, &wsol_reserve)
        .await
        .unwrap();

    // deposit some liquidity so we can actually redeem the fees later
    lending_market
        .deposit(&mut test, &wsol_reserve, &user, LAMPORTS_PER_SOL)
        .await
        .unwrap();

    let wsol_reserve = test.load_account::<Reserve>(wsol_reserve.pubkey).await;

    // redeem fees
    let balance_checker = BalanceChecker::start(&mut test, &[&wsol_reserve]).await;

    lending_market
        .redeem_fees(&mut test, &wsol_reserve)
        .await
        .unwrap();

    let expected_fees = wsol_reserve.account.calculate_redeem_fees().unwrap();

    // check token balances
    let (balance_changes, mint_supply_changes) =
        balance_checker.find_balance_changes(&mut test).await;
    let expected_balance_changes = HashSet::from([
        TokenBalanceChange {
            token_account: wsol_reserve.account.config.fee_receiver,
            mint: wsol_mint::id(),
            diff: expected_fees as i128,
        },
        TokenBalanceChange {
            token_account: wsol_reserve.account.liquidity.supply_pubkey,
            mint: wsol_mint::id(),
            diff: -(expected_fees as i128),
        },
    ]);
    assert_eq!(balance_changes, expected_balance_changes);
    assert_eq!(mint_supply_changes, HashSet::new());

    // check program state
    let wsol_reserve_post = test.load_account::<Reserve>(wsol_reserve.pubkey).await;
    assert_eq!(
        wsol_reserve_post.account,
        Reserve {
            last_update: LastUpdate {
                slot: 1000 + SLOTS_PER_YEAR,
                stale: true
            },
            liquidity: ReserveLiquidity {
                available_amount: wsol_reserve.account.liquidity.available_amount - expected_fees,
                accumulated_protocol_fees_wads: wsol_reserve
                    .account
                    .liquidity
                    .accumulated_protocol_fees_wads
                    .try_sub(Decimal::from(expected_fees))
                    .unwrap(),
                ..wsol_reserve.account.liquidity
            },
            ..wsol_reserve.account
        }
    );
}
