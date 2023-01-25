#![cfg(feature = "test-bpf")]

mod helpers;

use crate::solend_program_test::scenario_1;
use std::collections::HashSet;

use helpers::solend_program_test::{BalanceChecker, TokenBalanceChange};
use helpers::*;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program_test::*;

use solend_program::math::TryDiv;
use solend_program::state::{LastUpdate, ObligationLiquidity, ReserveLiquidity, SLOTS_PER_YEAR};
use solend_program::{
    math::{Decimal, TryAdd, TryMul, TrySub},
    state::{Obligation, Reserve},
};

#[tokio::test]
async fn test_success() {
    let (mut test, lending_market, usdc_reserve, wsol_reserve, user, obligation) =
        scenario_1(&test_reserve_config(), &test_reserve_config()).await;

    test.advance_clock_by_slots(1).await;

    let balance_checker =
        BalanceChecker::start(&mut test, &[&usdc_reserve, &user, &wsol_reserve]).await;

    lending_market
        .repay_obligation_liquidity(
            &mut test,
            &wsol_reserve,
            &obligation,
            &user,
            10 * LAMPORTS_PER_SOL,
        )
        .await
        .unwrap();

    // check token balances
    let (balance_changes, mint_supply_changes) =
        balance_checker.find_balance_changes(&mut test).await;
    let expected_balance_changes = HashSet::from([
        TokenBalanceChange {
            token_account: user.get_account(&wsol_mint::id()).unwrap(),
            mint: wsol_mint::id(),
            diff: -(10 * LAMPORTS_PER_SOL as i128),
        },
        TokenBalanceChange {
            token_account: wsol_reserve.account.liquidity.supply_pubkey,
            mint: wsol_mint::id(),
            diff: (10 * LAMPORTS_PER_SOL as i128),
        },
    ]);
    assert_eq!(balance_changes, expected_balance_changes);
    assert_eq!(mint_supply_changes, HashSet::new());

    // check program state
    let wsol_reserve_post = test.load_account::<Reserve>(wsol_reserve.pubkey).await;

    // 1 + 0.3/SLOTS_PER_YEAR
    let new_cumulative_borrow_rate = Decimal::one()
        .try_add(
            Decimal::from_percent(wsol_reserve.account.config.max_borrow_rate)
                .try_div(Decimal::from(SLOTS_PER_YEAR))
                .unwrap(),
        )
        .unwrap();
    let new_borrowed_amount_wads = new_cumulative_borrow_rate
        .try_mul(Decimal::from(10 * LAMPORTS_PER_SOL))
        .unwrap()
        .try_sub(Decimal::from(10 * LAMPORTS_TO_SOL))
        .unwrap();

    assert_eq!(
        wsol_reserve_post.account,
        Reserve {
            last_update: LastUpdate {
                slot: 1001,
                stale: true
            },
            liquidity: ReserveLiquidity {
                available_amount: 10 * LAMPORTS_PER_SOL,
                borrowed_amount_wads: new_borrowed_amount_wads,
                cumulative_borrow_rate_wads: new_cumulative_borrow_rate,
                ..wsol_reserve.account.liquidity
            },
            ..wsol_reserve.account
        }
    );

    let obligation_post = test.load_account::<Obligation>(obligation.pubkey).await;
    assert_eq!(
        obligation_post.account,
        Obligation {
            // we don't require obligation to be refreshed for repay
            last_update: LastUpdate {
                slot: 1000,
                stale: true
            },
            borrows: [ObligationLiquidity {
                borrow_reserve: wsol_reserve.pubkey,
                cumulative_borrow_rate_wads: new_cumulative_borrow_rate,
                borrowed_amount_wads: new_borrowed_amount_wads,
                ..obligation.account.borrows[0]
            }]
            .to_vec(),
            ..obligation.account
        }
    );
}
