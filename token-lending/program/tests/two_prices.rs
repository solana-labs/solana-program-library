#![cfg(feature = "test-bpf")]

use crate::solend_program_test::custom_scenario;
use crate::solend_program_test::find_reserve;
use crate::solend_program_test::User;

use crate::solend_program_test::BalanceChecker;
use crate::solend_program_test::ObligationArgs;
use crate::solend_program_test::PriceArgs;
use crate::solend_program_test::ReserveArgs;
use crate::solend_program_test::TokenBalanceChange;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_sdk::instruction::InstructionError;
use solana_sdk::transaction::TransactionError;
use solend_program::error::LendingError;

use solend_program::state::ReserveConfig;
use solend_program::NULL_PUBKEY;
use solend_sdk::state::ReserveFees;
mod helpers;

use helpers::*;
use solana_program_test::*;

use std::collections::HashSet;

/// the two prices feature affects a bunch of instructions. All of those instructions are tested
/// here for correctness.

#[tokio::test]
async fn test_borrow() {
    let (mut test, lending_market, reserves, obligation, user) = custom_scenario(
        &[
            ReserveArgs {
                mint: usdc_mint::id(),
                config: test_reserve_config(),
                liquidity_amount: 100_000 * FRACTIONAL_TO_USDC,
                price: PriceArgs {
                    price: 10,
                    conf: 0,
                    expo: -1,
                    ema_price: 10,
                    ema_conf: 1,
                },
            },
            ReserveArgs {
                mint: wsol_mint::id(),
                config: ReserveConfig {
                    loan_to_value_ratio: 50,
                    liquidation_threshold: 55,
                    fees: ReserveFees::default(),
                    optimal_borrow_rate: 0,
                    max_borrow_rate: 0,
                    ..test_reserve_config()
                },
                liquidity_amount: 100 * LAMPORTS_PER_SOL,
                price: PriceArgs {
                    price: 10,
                    conf: 0,
                    expo: 0,
                    ema_price: 10,
                    ema_conf: 0,
                },
            },
        ],
        &ObligationArgs {
            deposits: vec![(usdc_mint::id(), 100 * FRACTIONAL_TO_USDC)],
            borrows: vec![(wsol_mint::id(), LAMPORTS_PER_SOL)],
        },
    )
    .await;

    // update prices
    test.set_price(
        &usdc_mint::id(),
        &PriceArgs {
            price: 9,
            conf: 0,
            expo: -1,
            ema_price: 10,
            ema_conf: 0,
        },
    )
    .await;

    test.set_price(
        &wsol_mint::id(),
        &PriceArgs {
            price: 10,
            conf: 0,
            expo: 0,
            ema_price: 20,
            ema_conf: 0,
        },
    )
    .await;

    test.advance_clock_by_slots(1).await;

    let balance_checker = BalanceChecker::start(&mut test, &[&user]).await;

    // obligation currently has 100 USDC deposited and 1 sol borrowed
    // if we try to borrow the max amount, how much SOL should we receive?
    // allowed borrow value = 100 * min(1, 0.9) * 0.5 = $45
    // borrow value upper bound: 1 * max(10, 20) = $20
    // max SOL that can be borrowed is: ($45 - $20) / $20 = 1.25 SOL
    lending_market
        .borrow_obligation_liquidity(
            &mut test,
            &find_reserve(&reserves, &wsol_mint::id()).unwrap(),
            &obligation,
            &user,
            &NULL_PUBKEY,
            u64::MAX,
        )
        .await
        .unwrap();

    let (balance_changes, _) = balance_checker.find_balance_changes(&mut test).await;
    let expected_balance_changes = HashSet::from([TokenBalanceChange {
        token_account: user.get_account(&wsol_mint::id()).unwrap(),
        mint: wsol_mint::id(),
        diff: (LAMPORTS_PER_SOL * 125 / 100) as i128,
    }]);

    assert_eq!(balance_changes, expected_balance_changes);

    test.advance_clock_by_slots(1).await;

    // shouldn't be able to borrow any more
    let err = lending_market
        .borrow_obligation_liquidity(
            &mut test,
            &find_reserve(&reserves, &wsol_mint::id()).unwrap(),
            &obligation,
            &user,
            &NULL_PUBKEY,
            u64::MAX,
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        err,
        TransactionError::InstructionError(
            3,
            InstructionError::Custom(LendingError::BorrowTooLarge as u32)
        )
    );
}

#[tokio::test]
async fn test_withdraw() {
    let (mut test, lending_market, reserves, obligation, user) = custom_scenario(
        &[
            ReserveArgs {
                mint: usdc_mint::id(),
                config: test_reserve_config(),
                liquidity_amount: 100_000 * FRACTIONAL_TO_USDC,
                price: PriceArgs {
                    price: 10,
                    conf: 0,
                    expo: -1,
                    ema_price: 10,
                    ema_conf: 1,
                },
            },
            ReserveArgs {
                mint: usdt_mint::id(),
                config: test_reserve_config(),
                liquidity_amount: 100_000 * FRACTIONAL_TO_USDC,
                price: PriceArgs {
                    price: 10,
                    conf: 0,
                    expo: -1,
                    ema_price: 10,
                    ema_conf: 1,
                },
            },
            ReserveArgs {
                mint: wsol_mint::id(),
                config: ReserveConfig {
                    loan_to_value_ratio: 50,
                    liquidation_threshold: 55,
                    optimal_borrow_rate: 0,
                    max_borrow_rate: 0,
                    fees: ReserveFees::default(),
                    ..test_reserve_config()
                },
                liquidity_amount: 100 * LAMPORTS_PER_SOL,
                price: PriceArgs {
                    price: 10,
                    conf: 0,
                    expo: 0,
                    ema_price: 10,
                    ema_conf: 0,
                },
            },
        ],
        &ObligationArgs {
            deposits: vec![
                (usdc_mint::id(), 100 * FRACTIONAL_TO_USDC),
                (usdt_mint::id(), 20 * FRACTIONAL_TO_USDC),
            ],
            borrows: vec![(wsol_mint::id(), LAMPORTS_PER_SOL)],
        },
    )
    .await;

    // update prices
    test.set_price(
        &usdc_mint::id(),
        &PriceArgs {
            price: 100, // massive price increase
            conf: 0,
            expo: 0,
            ema_price: 1,
            ema_conf: 0,
        },
    )
    .await;

    test.set_price(
        &wsol_mint::id(),
        &PriceArgs {
            price: 10, // big price decrease
            conf: 0,
            expo: 0,
            ema_price: 20,
            ema_conf: 0,
        },
    )
    .await;

    test.advance_clock_by_slots(1).await;

    let balance_checker = BalanceChecker::start(&mut test, &[&user]).await;

    lending_market
        .withdraw_obligation_collateral_and_redeem_reserve_collateral(
            &mut test,
            &find_reserve(&reserves, &usdc_mint::id()).unwrap(),
            &obligation,
            &user,
            u64::MAX,
        )
        .await
        .unwrap();

    // how much usdc should we able to withdraw?
    // current allowed borrow value: 100 * min(100, 1) * 0.5 + 20 * min(1, 1) * 0.5 = $60
    // borrow value upper bound = 1 SOL * max($20, $10) = $20
    // max withdraw value = ($60 - $20) / 0.5 = $80
    // max withdraw liquidity amount = $80 / min(100, 1) = *80 USDC*
    // note that if we didn't have this two prices feature, you could withdraw all of the USDC
    // cUSDC/USDC exchange rate = 1 => max withdraw is 80 cUSDC
    //
    // reconciliation:
    // after withdraw, we are left with 20 USDC, 20 USDT
    // allowed borrow value is now 20 * min(100, 1) * 0.5 + 20 * min(1, 1) * 0.5 = $20
    // borrow value upper bound = $20
    // we have successfully borrowed the max amount
    let (balance_changes, _) = balance_checker.find_balance_changes(&mut test).await;
    let expected_balance_changes = HashSet::from([TokenBalanceChange {
        token_account: user.get_account(&usdc_mint::id()).unwrap(),
        mint: usdc_mint::id(),
        diff: (80 * FRACTIONAL_TO_USDC) as i128,
    }]);

    assert_eq!(balance_changes, expected_balance_changes);

    test.advance_clock_by_slots(1).await;

    // we shouldn't be able to withdraw anything else
    for mint in [usdc_mint::id(), usdt_mint::id()] {
        let err = lending_market
            .withdraw_obligation_collateral_and_redeem_reserve_collateral(
                &mut test,
                &find_reserve(&reserves, &mint).unwrap(),
                &obligation,
                &user,
                u64::MAX,
            )
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            err,
            TransactionError::InstructionError(
                4,
                InstructionError::Custom(LendingError::WithdrawTooLarge as u32)
            )
        );
    }
}

#[tokio::test]
async fn test_liquidation_doesnt_use_smoothed_price() {
    let (mut test, lending_market, reserves, obligation, user) = custom_scenario(
        &[
            ReserveArgs {
                mint: usdc_mint::id(),
                config: test_reserve_config(),
                liquidity_amount: 100_000 * FRACTIONAL_TO_USDC,
                price: PriceArgs {
                    price: 1,
                    conf: 0,
                    expo: 0,
                    ema_price: 1,
                    ema_conf: 0,
                },
            },
            ReserveArgs {
                mint: wsol_mint::id(),
                config: ReserveConfig {
                    loan_to_value_ratio: 50,
                    liquidation_threshold: 55,
                    fees: ReserveFees::default(),
                    optimal_borrow_rate: 0,
                    max_borrow_rate: 0,
                    protocol_liquidation_fee: 0,
                    ..test_reserve_config()
                },
                liquidity_amount: 100 * LAMPORTS_PER_SOL,
                price: PriceArgs {
                    price: 10,
                    conf: 0,
                    expo: 0,
                    ema_price: 10,
                    ema_conf: 0,
                },
            },
        ],
        &ObligationArgs {
            deposits: vec![(usdc_mint::id(), 100 * FRACTIONAL_TO_USDC)],
            borrows: vec![(wsol_mint::id(), LAMPORTS_PER_SOL)],
        },
    )
    .await;

    // set ema price to 100
    test.set_price(
        &wsol_mint::id(),
        &PriceArgs {
            price: 10,
            conf: 0,
            expo: 0,
            ema_price: 100,
            ema_conf: 0,
        },
    )
    .await;

    test.advance_clock_by_slots(1).await;

    // this should fail bc the obligation is still healthy wrt the current non-ema market prices
    let err = lending_market
        .liquidate_obligation_and_redeem_reserve_collateral(
            &mut test,
            &find_reserve(&reserves, &wsol_mint::id()).unwrap(),
            &find_reserve(&reserves, &usdc_mint::id()).unwrap(),
            &obligation,
            &user,
            u64::MAX,
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        err,
        TransactionError::InstructionError(
            3,
            InstructionError::Custom(LendingError::ObligationHealthy as u32)
        )
    );

    test.set_price(
        &usdc_mint::id(),
        &PriceArgs {
            price: 1,
            conf: 0,
            expo: 0,
            ema_price: 0,
            ema_conf: 0,
        },
    )
    .await;

    test.advance_clock_by_slots(1).await;

    // this should fail bc the obligation is still healthy wrt the current non-ema market prices
    let err = lending_market
        .liquidate_obligation_and_redeem_reserve_collateral(
            &mut test,
            &find_reserve(&reserves, &wsol_mint::id()).unwrap(),
            &find_reserve(&reserves, &usdc_mint::id()).unwrap(),
            &obligation,
            &user,
            u64::MAX,
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        err,
        TransactionError::InstructionError(
            3,
            InstructionError::Custom(LendingError::ObligationHealthy as u32)
        )
    );

    // now set the spot prices. this time, the liquidation should actually work
    test.set_price(
        &usdc_mint::id(),
        &PriceArgs {
            price: 1,
            conf: 0,
            expo: 0,
            ema_price: 10,
            ema_conf: 0,
        },
    )
    .await;

    test.set_price(
        &wsol_mint::id(),
        &PriceArgs {
            price: 100,
            conf: 0,
            expo: 0,
            ema_price: 10,
            ema_conf: 0,
        },
    )
    .await;

    test.advance_clock_by_slots(1).await;

    let usdc_reserve = find_reserve(&reserves, &usdc_mint::id()).unwrap();
    let wsol_reserve = find_reserve(&reserves, &wsol_mint::id()).unwrap();

    let liquidator = User::new_with_balances(
        &mut test,
        &[
            (&usdc_mint::id(), 100 * FRACTIONAL_TO_USDC),
            (&usdc_reserve.account.collateral.mint_pubkey, 0),
            (&wsol_mint::id(), 100 * LAMPORTS_PER_SOL),
            (&wsol_reserve.account.collateral.mint_pubkey, 0),
        ],
    )
    .await;

    let balance_checker = BalanceChecker::start(&mut test, &[&liquidator]).await;

    lending_market
        .liquidate_obligation_and_redeem_reserve_collateral(
            &mut test,
            &find_reserve(&reserves, &wsol_mint::id()).unwrap(),
            &find_reserve(&reserves, &usdc_mint::id()).unwrap(),
            &obligation,
            &liquidator,
            u64::MAX,
        )
        .await
        .unwrap();

    let (balance_changes, _) = balance_checker.find_balance_changes(&mut test).await;
    // make sure the liquidation amounts are also wrt spot prices
    let expected_balances_changes = HashSet::from([
        TokenBalanceChange {
            token_account: liquidator.get_account(&usdc_mint::id()).unwrap(),
            mint: usdc_mint::id(),
            diff: (20 * FRACTIONAL_TO_USDC * 105 / 100) as i128 - 1,
        },
        TokenBalanceChange {
            token_account: liquidator.get_account(&wsol_mint::id()).unwrap(),
            mint: wsol_mint::id(),
            diff: -((LAMPORTS_PER_SOL / 5) as i128),
        },
    ]);

    assert_eq!(balance_changes, expected_balances_changes);
}
