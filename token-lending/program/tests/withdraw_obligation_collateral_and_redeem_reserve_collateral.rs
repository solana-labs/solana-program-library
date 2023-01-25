#![cfg(feature = "test-bpf")]

mod helpers;

use crate::solend_program_test::MintSupplyChange;
use solend_sdk::state::ObligationCollateral;
use solend_sdk::state::ReserveCollateral;
use std::collections::HashSet;

use crate::solend_program_test::scenario_1;
use crate::solend_program_test::BalanceChecker;
use crate::solend_program_test::TokenBalanceChange;
use helpers::*;

use solana_program_test::*;

use solend_sdk::state::LastUpdate;
use solend_sdk::state::Obligation;

use solend_sdk::state::Reserve;
use solend_sdk::state::ReserveLiquidity;

#[tokio::test]
async fn test_success() {
    let (mut test, lending_market, usdc_reserve, wsol_reserve, user, obligation) =
        scenario_1(&test_reserve_config(), &test_reserve_config()).await;

    let balance_checker =
        BalanceChecker::start(&mut test, &[&usdc_reserve, &user, &wsol_reserve]).await;

    lending_market
        .withdraw_obligation_collateral_and_redeem_reserve_collateral(
            &mut test,
            &usdc_reserve,
            &obligation,
            &user,
            u64::MAX,
        )
        .await
        .unwrap();

    // check token balances
    let (balance_changes, mint_supply_changes) =
        balance_checker.find_balance_changes(&mut test).await;
    let withdraw_amount = (100_000 * FRACTIONAL_TO_USDC - 200 * FRACTIONAL_TO_USDC) as i128;

    let expected_balance_changes = HashSet::from([
        TokenBalanceChange {
            token_account: user.get_account(&usdc_mint::id()).unwrap(),
            mint: usdc_mint::id(),
            diff: withdraw_amount,
        },
        TokenBalanceChange {
            token_account: usdc_reserve.account.liquidity.supply_pubkey,
            mint: usdc_mint::id(),
            diff: -withdraw_amount,
        },
        TokenBalanceChange {
            token_account: usdc_reserve.account.collateral.supply_pubkey,
            mint: usdc_reserve.account.collateral.mint_pubkey,
            diff: -withdraw_amount,
        },
    ]);
    assert_eq!(balance_changes, expected_balance_changes);
    assert_eq!(
        mint_supply_changes,
        HashSet::from([MintSupplyChange {
            mint: usdc_reserve.account.collateral.mint_pubkey,
            diff: -withdraw_amount
        }])
    );

    // check program state
    let usdc_reserve_post = test.load_account::<Reserve>(usdc_reserve.pubkey).await;
    assert_eq!(
        usdc_reserve_post.account,
        Reserve {
            last_update: LastUpdate {
                slot: 1000,
                stale: true
            },
            liquidity: ReserveLiquidity {
                available_amount: usdc_reserve.account.liquidity.available_amount
                    - withdraw_amount as u64,
                ..usdc_reserve.account.liquidity
            },
            collateral: ReserveCollateral {
                mint_total_supply: usdc_reserve.account.collateral.mint_total_supply
                    - withdraw_amount as u64,
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
                deposited_amount: 200 * FRACTIONAL_TO_USDC,
                ..obligation.account.deposits[0]
            }]
            .to_vec(),
            ..obligation.account
        }
    );
}
