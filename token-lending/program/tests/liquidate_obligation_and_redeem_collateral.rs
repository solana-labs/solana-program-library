#![cfg(feature = "test-bpf")]

use crate::solend_program_test::MintSupplyChange;
use solend_program::math::TrySub;
use solend_program::state::LastUpdate;
use solend_program::state::ObligationCollateral;
use solend_program::state::ObligationLiquidity;
use solend_program::state::ReserveConfig;
mod helpers;

use crate::solend_program_test::scenario_1;
use crate::solend_program_test::BalanceChecker;
use crate::solend_program_test::PriceArgs;
use crate::solend_program_test::TokenBalanceChange;
use crate::solend_program_test::User;
use helpers::*;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solend_program::math::Decimal;
use solend_program::state::LendingMarket;
use solend_program::state::Obligation;
use solend_program::state::Reserve;
use solend_program::state::ReserveCollateral;
use solend_program::state::ReserveLiquidity;
use solend_program::state::LIQUIDATION_CLOSE_FACTOR;

use std::collections::HashSet;

#[tokio::test]
async fn test_success_new() {
    let (mut test, lending_market, usdc_reserve, wsol_reserve, user, obligation) = scenario_1(
        &ReserveConfig {
            protocol_liquidation_fee: 30,
            ..test_reserve_config()
        },
        &test_reserve_config(),
    )
    .await;

    let liquidator = User::new_with_balances(
        &mut test,
        &[
            (&wsol_mint::id(), 100 * LAMPORTS_TO_SOL),
            (&usdc_reserve.account.collateral.mint_pubkey, 0),
            (&usdc_mint::id(), 0),
        ],
    )
    .await;

    let balance_checker = BalanceChecker::start(
        &mut test,
        &[
            &usdc_reserve,
            &user,
            &wsol_reserve,
            &usdc_reserve,
            &liquidator,
        ],
    )
    .await;

    // close LTV is 0.55, we've deposited 100k USDC and borrowed 10 SOL.
    // obligation gets liquidated if 100k * 0.55 = 10 SOL * sol_price => sol_price = 5.5k
    test.set_price(
        &wsol_mint::id(),
        PriceArgs {
            price: 5500,
            conf: 0,
            expo: 0,
        },
    )
    .await;

    lending_market
        .liquidate_obligation_and_redeem_reserve_collateral(
            &mut test,
            &wsol_reserve,
            &usdc_reserve,
            &obligation,
            &liquidator,
            u64::MAX,
        )
        .await
        .unwrap();

    let (balance_changes, mint_supply_changes) =
        balance_checker.find_balance_changes(&mut test).await;

    let bonus = usdc_reserve.account.config.liquidation_bonus as u64;
    let protocol_liquidation_fee_pct = usdc_reserve.account.config.protocol_liquidation_fee as u64;

    let expected_borrow_repaid = 10 * (LIQUIDATION_CLOSE_FACTOR as u64) / 100;
    let expected_usdc_withdrawn = expected_borrow_repaid * 5500 * (100 + bonus) / 100;

    let expected_total_bonus = expected_usdc_withdrawn - expected_borrow_repaid * 5500;
    let expected_protocol_liquidation_fee =
        expected_total_bonus * protocol_liquidation_fee_pct / 100;

    let expected_balance_changes = HashSet::from([
        // liquidator
        TokenBalanceChange {
            token_account: liquidator.get_account(&usdc_mint::id()).unwrap(),
            mint: usdc_mint::id(),
            diff: ((expected_usdc_withdrawn - expected_protocol_liquidation_fee)
                * FRACTIONAL_TO_USDC) as i128,
        },
        TokenBalanceChange {
            token_account: liquidator.get_account(&wsol_mint::id()).unwrap(),
            mint: wsol_mint::id(),
            diff: -((expected_borrow_repaid * LAMPORTS_TO_SOL) as i128),
        },
        // usdc reserve
        TokenBalanceChange {
            token_account: usdc_reserve.account.collateral.supply_pubkey,
            mint: usdc_reserve.account.collateral.mint_pubkey,
            diff: -((expected_usdc_withdrawn * FRACTIONAL_TO_USDC) as i128),
        },
        TokenBalanceChange {
            token_account: usdc_reserve.account.liquidity.supply_pubkey,
            mint: usdc_mint::id(),
            diff: -((expected_usdc_withdrawn * FRACTIONAL_TO_USDC) as i128),
        },
        TokenBalanceChange {
            token_account: usdc_reserve.account.config.fee_receiver,
            mint: usdc_mint::id(),
            diff: (expected_protocol_liquidation_fee * FRACTIONAL_TO_USDC) as i128,
        },
        // wsol reserve
        TokenBalanceChange {
            token_account: wsol_reserve.account.liquidity.supply_pubkey,
            mint: wsol_mint::id(),
            diff: (expected_borrow_repaid * LAMPORTS_TO_SOL) as i128,
        },
    ]);
    assert_eq!(balance_changes, expected_balance_changes);
    assert_eq!(
        mint_supply_changes,
        HashSet::from([MintSupplyChange {
            mint: usdc_reserve.account.collateral.mint_pubkey,
            diff: -((expected_usdc_withdrawn * FRACTIONAL_TO_USDC) as i128)
        }])
    );

    // check program state
    let lending_market_post = test
        .load_account::<LendingMarket>(lending_market.pubkey)
        .await;
    assert_eq!(lending_market_post.account, lending_market.account);

    let usdc_reserve_post = test.load_account::<Reserve>(usdc_reserve.pubkey).await;
    assert_eq!(
        usdc_reserve_post.account,
        Reserve {
            liquidity: ReserveLiquidity {
                available_amount: usdc_reserve.account.liquidity.available_amount
                    - expected_usdc_withdrawn * FRACTIONAL_TO_USDC,
                ..usdc_reserve.account.liquidity
            },
            collateral: ReserveCollateral {
                mint_total_supply: usdc_reserve.account.collateral.mint_total_supply
                    - expected_usdc_withdrawn * FRACTIONAL_TO_USDC,
                ..usdc_reserve.account.collateral
            },
            ..usdc_reserve.account
        }
    );

    let wsol_reserve_post = test.load_account::<Reserve>(wsol_reserve.pubkey).await;
    assert_eq!(
        wsol_reserve_post.account,
        Reserve {
            liquidity: ReserveLiquidity {
                available_amount: wsol_reserve.account.liquidity.available_amount
                    + expected_borrow_repaid * LAMPORTS_TO_SOL,
                borrowed_amount_wads: wsol_reserve
                    .account
                    .liquidity
                    .borrowed_amount_wads
                    .try_sub(Decimal::from(expected_borrow_repaid * LAMPORTS_TO_SOL))
                    .unwrap(),
                market_price: Decimal::from(5500u64),
                ..wsol_reserve.account.liquidity
            },
            ..wsol_reserve.account
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
                deposited_amount: (100_000 - expected_usdc_withdrawn) * FRACTIONAL_TO_USDC,
                market_value: Decimal::from(100_000u64) // old value
            }]
            .to_vec(),
            borrows: [ObligationLiquidity {
                borrow_reserve: wsol_reserve.pubkey,
                cumulative_borrow_rate_wads: Decimal::one(),
                borrowed_amount_wads: Decimal::from(10 * LAMPORTS_TO_SOL)
                    .try_sub(Decimal::from(expected_borrow_repaid * LAMPORTS_TO_SOL))
                    .unwrap(),
                market_value: Decimal::from(55_000u64),
            }]
            .to_vec(),
            deposited_value: Decimal::from(100_000u64),
            borrowed_value: Decimal::from(55_000u64),
            allowed_borrow_value: Decimal::from(50_000u64),
            unhealthy_borrow_value: Decimal::from(55_000u64),
            ..obligation.account
        }
    );
}

#[tokio::test]
async fn test_success_insufficient_liquidity() {
    let (mut test, lending_market, usdc_reserve, wsol_reserve, user, obligation) =
        scenario_1(&test_reserve_config(), &test_reserve_config()).await;

    // basically the same test as above, but now someone borrows a lot of USDC so the liquidatior
    // partially receives USDC and cUSDC
    {
        let usdc_borrower = User::new_with_balances(
            &mut test,
            &[
                (&usdc_mint::id(), 0),
                (&wsol_mint::id(), 20_000 * LAMPORTS_TO_SOL),
                (&wsol_reserve.account.collateral.mint_pubkey, 0),
            ],
        )
        .await;

        let obligation = lending_market
            .init_obligation(&mut test, Keypair::new(), &usdc_borrower)
            .await
            .unwrap();

        lending_market
            .deposit_reserve_liquidity_and_obligation_collateral(
                &mut test,
                &wsol_reserve,
                &obligation,
                &usdc_borrower,
                20_000 * LAMPORTS_TO_SOL,
            )
            .await
            .unwrap();

        let obligation = test.load_account::<Obligation>(obligation.pubkey).await;
        lending_market
            .borrow_obligation_liquidity(
                &mut test,
                &usdc_reserve,
                &obligation,
                &usdc_borrower,
                &usdc_borrower.get_account(&usdc_mint::id()).unwrap(),
                u64::MAX,
            )
            .await
            .unwrap()
    }

    let liquidator = User::new_with_balances(
        &mut test,
        &[
            (&wsol_mint::id(), 100 * LAMPORTS_TO_SOL),
            (&usdc_reserve.account.collateral.mint_pubkey, 0),
            (&usdc_mint::id(), 0),
        ],
    )
    .await;

    let balance_checker = BalanceChecker::start(
        &mut test,
        &[&usdc_reserve, &user, &wsol_reserve, &liquidator],
    )
    .await;

    // close LTV is 0.55, we've deposited 100k USDC and borrowed 10 SOL.
    // obligation gets liquidated if 100k * 0.55 = 10 SOL * sol_price => sol_price == 5.5k
    test.set_price(
        &wsol_mint::id(),
        PriceArgs {
            price: 5500,
            conf: 0,
            expo: 0,
        },
    )
    .await;

    let lending_market = test
        .load_account::<LendingMarket>(lending_market.pubkey)
        .await;
    let usdc_reserve = test.load_account::<Reserve>(usdc_reserve.pubkey).await;
    let wsol_reserve = test.load_account::<Reserve>(wsol_reserve.pubkey).await;

    let available_amount = usdc_reserve.account.liquidity.available_amount / FRACTIONAL_TO_USDC;

    lending_market
        .liquidate_obligation_and_redeem_reserve_collateral(
            &mut test,
            &wsol_reserve,
            &usdc_reserve,
            &obligation,
            &liquidator,
            u64::MAX,
        )
        .await
        .unwrap();

    let (balance_changes, mint_supply_changes) =
        balance_checker.find_balance_changes(&mut test).await;

    let bonus = usdc_reserve.account.config.liquidation_bonus as u64;

    let expected_borrow_repaid = 10 * (LIQUIDATION_CLOSE_FACTOR as u64) / 100;
    let expected_cusdc_withdrawn =
        expected_borrow_repaid * 5500 * (100 + bonus) / 100 - available_amount;
    let expected_protocol_liquidation_fee = usdc_reserve
        .account
        .calculate_protocol_liquidation_fee(available_amount * FRACTIONAL_TO_USDC)
        .unwrap();

    let expected_balance_changes = HashSet::from([
        // liquidator
        TokenBalanceChange {
            token_account: liquidator.get_account(&usdc_mint::id()).unwrap(),
            mint: usdc_mint::id(),
            diff: (available_amount * FRACTIONAL_TO_USDC - expected_protocol_liquidation_fee)
                as i128,
        },
        TokenBalanceChange {
            token_account: liquidator
                .get_account(&usdc_reserve.account.collateral.mint_pubkey)
                .unwrap(),
            mint: usdc_reserve.account.collateral.mint_pubkey,
            diff: (expected_cusdc_withdrawn * FRACTIONAL_TO_USDC) as i128,
        },
        TokenBalanceChange {
            token_account: liquidator.get_account(&wsol_mint::id()).unwrap(),
            mint: wsol_mint::id(),
            diff: -((expected_borrow_repaid * LAMPORTS_TO_SOL) as i128),
        },
        // usdc reserve
        TokenBalanceChange {
            token_account: usdc_reserve.account.collateral.supply_pubkey,
            mint: usdc_reserve.account.collateral.mint_pubkey,
            diff: -(((expected_cusdc_withdrawn + available_amount) * FRACTIONAL_TO_USDC) as i128),
        },
        TokenBalanceChange {
            token_account: usdc_reserve.account.liquidity.supply_pubkey,
            mint: usdc_mint::id(),
            diff: -((available_amount * FRACTIONAL_TO_USDC) as i128),
        },
        TokenBalanceChange {
            token_account: usdc_reserve.account.config.fee_receiver,
            mint: usdc_mint::id(),
            diff: expected_protocol_liquidation_fee as i128,
        },
        // wsol reserve
        TokenBalanceChange {
            token_account: wsol_reserve.account.liquidity.supply_pubkey,
            mint: wsol_mint::id(),
            diff: (expected_borrow_repaid * LAMPORTS_TO_SOL) as i128,
        },
    ]);
    assert_eq!(
        balance_changes, expected_balance_changes,
        "{:#?} {:#?}",
        balance_changes, expected_balance_changes
    );

    assert_eq!(
        mint_supply_changes,
        HashSet::from([MintSupplyChange {
            mint: usdc_reserve.account.collateral.mint_pubkey,
            diff: -((available_amount * FRACTIONAL_TO_USDC) as i128)
        }])
    );
}
