#![cfg(feature = "test-bpf")]
/// the borrow weight feature affects a bunch of instructions. All of those instructions are tested
/// here for correctness.
use crate::solend_program_test::setup_world;
use crate::solend_program_test::BalanceChecker;
use crate::solend_program_test::TokenBalanceChange;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_sdk::instruction::InstructionError;
use solana_sdk::transaction::TransactionError;
use solend_program::error::LendingError;
use solend_program::state::ReserveConfig;
use solend_sdk::state::ReserveFees;
mod helpers;

use crate::solend_program_test::scenario_1;
use crate::solend_program_test::User;
use helpers::*;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solend_program::math::Decimal;
use solend_program::state::Obligation;
use std::collections::HashSet;

#[tokio::test]
async fn test_refresh_obligation() {
    let (mut test, lending_market, _, _, _, obligation) = scenario_1(
        &test_reserve_config(),
        &ReserveConfig {
            added_borrow_weight_bps: 10_000,
            ..test_reserve_config()
        },
    )
    .await;

    lending_market
        .refresh_obligation(&mut test, &obligation)
        .await
        .unwrap();

    let obligation_post = test.load_account::<Obligation>(obligation.pubkey).await;

    // obligation has borrowed 10 sol and sol = $10 but since borrow weight == 2, the
    // borrowed_value is 200 instead of 100.
    assert_eq!(
        obligation_post.account,
        Obligation {
            borrowed_value: Decimal::from(200u64),
            ..obligation.account
        }
    );
}

#[tokio::test]
async fn test_borrow() {
    let (mut test, lending_market, usdc_reserve, wsol_reserve, _, _) = setup_world(
        &test_reserve_config(),
        &ReserveConfig {
            added_borrow_weight_bps: 10_000,
            fees: ReserveFees {
                borrow_fee_wad: 10_000_000_000_000_000, // 1%
                host_fee_percentage: 20,
                flash_loan_fee_wad: 0,
            },
            ..test_reserve_config()
        },
    )
    .await;

    // create obligation with 100 USDC deposited.
    let (user, obligation) = {
        let user = User::new_with_balances(
            &mut test,
            &[
                (&usdc_mint::id(), 200 * FRACTIONAL_TO_USDC),
                (&usdc_reserve.account.collateral.mint_pubkey, 0),
                (&wsol_mint::id(), 0),
            ],
        )
        .await;

        let obligation = lending_market
            .init_obligation(&mut test, Keypair::new(), &user)
            .await
            .expect("This should succeed");

        lending_market
            .deposit_reserve_liquidity_and_obligation_collateral(
                &mut test,
                &usdc_reserve,
                &obligation,
                &user,
                100 * FRACTIONAL_TO_USDC,
            )
            .await
            .unwrap();
        (user, obligation)
    };

    // deposit 100 WSOL into reserve
    let host_fee_receiver = {
        let wsol_depositor = User::new_with_balances(
            &mut test,
            &[
                (&wsol_mint::id(), 5 * LAMPORTS_PER_SOL),
                (&wsol_reserve.account.collateral.mint_pubkey, 0),
            ],
        )
        .await;

        lending_market
            .deposit(
                &mut test,
                &wsol_reserve,
                &wsol_depositor,
                5 * LAMPORTS_PER_SOL,
            )
            .await
            .unwrap();

        wsol_depositor.get_account(&wsol_mint::id()).unwrap()
    };

    // borrow max amount of SOL
    {
        lending_market
            .borrow_obligation_liquidity(
                &mut test,
                &wsol_reserve,
                &obligation,
                &user,
                &host_fee_receiver,
                u64::MAX,
            )
            .await
            .unwrap();

        let obligation_post = test.load_account::<Obligation>(obligation.pubkey).await;
        // - usdc ltv is 0.5,
        // - sol borrow weight is 2
        // max you can borrow is 100 * 0.5 / 2 = 2.5 SOL
        assert_eq!(
            obligation_post.account.borrows[0].borrowed_amount_wads,
            Decimal::from(LAMPORTS_PER_SOL * 25 / 10)
        );
    }

    // check that we shouldn't be able to withdraw anything
    {
        let res = lending_market
            .withdraw_obligation_collateral(&mut test, &usdc_reserve, &obligation, &user, u64::MAX)
            .await
            .err()
            .unwrap()
            .unwrap();

        assert_eq!(
            res,
            TransactionError::InstructionError(
                3,
                InstructionError::Custom(LendingError::WithdrawTooLarge as u32)
            )
        );
    }

    // deposit another 50 USDC
    lending_market
        .deposit_reserve_liquidity_and_obligation_collateral(
            &mut test,
            &usdc_reserve,
            &obligation,
            &user,
            50 * FRACTIONAL_TO_USDC,
        )
        .await
        .unwrap();

    test.advance_clock_by_slots(1).await;

    // max withdraw
    {
        let balance_checker = BalanceChecker::start(&mut test, &[&user]).await;

        lending_market
            .withdraw_obligation_collateral(&mut test, &usdc_reserve, &obligation, &user, u64::MAX)
            .await
            .unwrap();

        let (balance_changes, _) = balance_checker.find_balance_changes(&mut test).await;
        // should only be able to withdraw 50 USDC because the rest is needed to collateralize the
        // SOL borrow
        assert_eq!(
            balance_changes,
            HashSet::from([TokenBalanceChange {
                token_account: user
                    .get_account(&usdc_reserve.account.collateral.mint_pubkey)
                    .unwrap(),
                mint: usdc_reserve.account.collateral.mint_pubkey,
                diff: (50 * FRACTIONAL_TO_USDC - 1) as i128,
            }])
        );
    }
}

#[tokio::test]
async fn test_liquidation() {
    let (mut test, lending_market, usdc_reserve, wsol_reserve, lending_market_owner, _) =
        setup_world(
            &test_reserve_config(),
            &ReserveConfig {
                added_borrow_weight_bps: 0,
                fees: ReserveFees {
                    borrow_fee_wad: 10_000_000_000_000_000, // 1%
                    host_fee_percentage: 20,
                    flash_loan_fee_wad: 0,
                },
                ..test_reserve_config()
            },
        )
        .await;

    // create obligation with 100 USDC deposited.
    let (user, obligation) = {
        let user = User::new_with_balances(
            &mut test,
            &[
                (&usdc_mint::id(), 200 * FRACTIONAL_TO_USDC),
                (&usdc_reserve.account.collateral.mint_pubkey, 0),
                (&wsol_mint::id(), 0),
            ],
        )
        .await;

        let obligation = lending_market
            .init_obligation(&mut test, Keypair::new(), &user)
            .await
            .expect("This should succeed");

        lending_market
            .deposit_reserve_liquidity_and_obligation_collateral(
                &mut test,
                &usdc_reserve,
                &obligation,
                &user,
                100 * FRACTIONAL_TO_USDC,
            )
            .await
            .unwrap();
        (user, obligation)
    };

    // deposit 100 WSOL into reserve
    let host_fee_receiver = {
        let wsol_depositor = User::new_with_balances(
            &mut test,
            &[
                (&wsol_mint::id(), 5 * LAMPORTS_PER_SOL),
                (&wsol_reserve.account.collateral.mint_pubkey, 0),
            ],
        )
        .await;

        lending_market
            .deposit(
                &mut test,
                &wsol_reserve,
                &wsol_depositor,
                5 * LAMPORTS_PER_SOL,
            )
            .await
            .unwrap();

        wsol_depositor.get_account(&wsol_mint::id()).unwrap()
    };

    // borrow max amount of SOL
    {
        lending_market
            .borrow_obligation_liquidity(
                &mut test,
                &wsol_reserve,
                &obligation,
                &user,
                &host_fee_receiver,
                u64::MAX,
            )
            .await
            .unwrap();

        let obligation_post = test.load_account::<Obligation>(obligation.pubkey).await;
        // - usdc ltv is 0.5,
        // - sol borrow weight is 1
        // max you can borrow is 100 * 0.5 = 5 SOL
        assert_eq!(
            obligation_post.account.borrows[0].borrowed_amount_wads,
            Decimal::from(LAMPORTS_PER_SOL * 5)
        );
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

    // liquidating now would clearly fail because the obligation is healthy
    {
        let res = lending_market
            .liquidate_obligation_and_redeem_reserve_collateral(
                &mut test,
                &wsol_reserve,
                &usdc_reserve,
                &obligation,
                &liquidator,
                u64::MAX,
            )
            .await
            .err()
            .unwrap()
            .unwrap();
        assert_eq!(
            res,
            TransactionError::InstructionError(
                3,
                InstructionError::Custom(LendingError::ObligationHealthy as u32)
            )
        );
    }

    // what is the minimum borrow weight we need for the obligation to be eligible for liquidation?
    // 100 * 0.55 = 5 * 10 * borrow_weight
    // => borrow_weight = 1.1

    // set borrow weight to 1.1
    lending_market
        .update_reserve_config(
            &mut test,
            &lending_market_owner,
            &wsol_reserve,
            ReserveConfig {
                added_borrow_weight_bps: 1_000,
                ..wsol_reserve.account.config
            },
            wsol_reserve.account.rate_limiter.config,
            None,
        )
        .await
        .unwrap();

    test.advance_clock_by_slots(1).await;

    // liquidating now should work
    {
        let balance_checker = BalanceChecker::start(&mut test, &[&liquidator]).await;
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

        // how much should be liquidated?
        // => borrow value * close factor
        // (5 sol * $10 * 1.1) * 0.2 = 11 usd worth of sol => repay ~1.1 sol (approximate because
        // there is 1 slot worth of interest that is unaccounted for)
        // note that if there were no borrow weight, we would only liquidate 10 usdc.
        let (balance_changes, _) = balance_checker.find_balance_changes(&mut test).await;
        assert!(balance_changes.contains(&TokenBalanceChange {
            token_account: liquidator.get_account(&wsol_mint::id()).unwrap(),
            mint: wsol_mint::id(),
            diff: -1100000002 // ~1.1 SOL
        }));
    }
}
