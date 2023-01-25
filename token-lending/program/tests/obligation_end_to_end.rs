#![cfg(feature = "test-bpf")]

use crate::solend_program_test::TokenBalanceChange;
use solend_program::math::TryMul;
use solend_program::math::TrySub;
use solend_program::state::ReserveConfig;
use solend_program::state::ReserveFees;
mod helpers;

use std::collections::HashSet;

use crate::solend_program_test::setup_world;
use crate::solend_program_test::BalanceChecker;
use crate::solend_program_test::Info;
use crate::solend_program_test::SolendProgramTest;
use crate::solend_program_test::User;
use helpers::*;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solend_program::math::Decimal;
use solend_program::state::LendingMarket;
use solend_program::state::Reserve;

async fn setup() -> (
    SolendProgramTest,
    Info<LendingMarket>,
    Info<Reserve>,
    Info<Reserve>,
    User,
) {
    let (test, lending_market, usdc_reserve, wsol_reserve, _, user) = setup_world(
        &test_reserve_config(),
        &ReserveConfig {
            fees: ReserveFees {
                borrow_fee_wad: 100_000_000_000,
                flash_loan_fee_wad: 0,
                host_fee_percentage: 20,
            },
            ..test_reserve_config()
        },
    )
    .await;

    (test, lending_market, usdc_reserve, wsol_reserve, user)
}

#[tokio::test]
async fn test_success() {
    let (mut test, lending_market, usdc_reserve, wsol_reserve, user) = setup().await;

    let host_fee_receiver = User::new_with_balances(&mut test, &[(&wsol_mint::id(), 0)]).await;
    let obligation = lending_market
        .init_obligation(&mut test, Keypair::new(), &user)
        .await
        .unwrap();

    let balance_checker = BalanceChecker::start(
        &mut test,
        &[&usdc_reserve, &wsol_reserve, &user, &host_fee_receiver],
    )
    .await;

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

    let obligation = test.load_account(obligation.pubkey).await;
    lending_market
        .borrow_obligation_liquidity(
            &mut test,
            &wsol_reserve,
            &obligation,
            &user,
            &host_fee_receiver.get_account(&wsol_mint::id()).unwrap(),
            LAMPORTS_TO_SOL / 2,
        )
        .await
        .unwrap();

    lending_market
        .repay_obligation_liquidity(&mut test, &wsol_reserve, &obligation, &user, u64::MAX)
        .await
        .unwrap();

    let obligation = test.load_account(obligation.pubkey).await;
    lending_market
        .withdraw_obligation_collateral_and_redeem_reserve_collateral(
            &mut test,
            &usdc_reserve,
            &obligation,
            &user,
            100 * FRACTIONAL_TO_USDC,
        )
        .await
        .unwrap();

    let (balance_changes, mint_supply_changes) =
        balance_checker.find_balance_changes(&mut test).await;
    let borrow_fee = Decimal::from(LAMPORTS_TO_SOL / 2)
        .try_mul(Decimal::from_scaled_val(
            wsol_reserve.account.config.fees.borrow_fee_wad as u128,
        ))
        .unwrap();
    let host_fee = borrow_fee
        .try_mul(Decimal::from_percent(
            wsol_reserve.account.config.fees.host_fee_percentage,
        ))
        .unwrap();

    let expected_balance_changes = HashSet::from([
        TokenBalanceChange {
            token_account: user.get_account(&wsol_mint::id()).unwrap(),
            mint: wsol_mint::id(),
            diff: -(borrow_fee.try_round_u64().unwrap() as i128),
        },
        TokenBalanceChange {
            token_account: host_fee_receiver.get_account(&wsol_mint::id()).unwrap(),
            mint: wsol_mint::id(),
            diff: host_fee.try_round_u64().unwrap() as i128,
        },
        TokenBalanceChange {
            token_account: wsol_reserve.account.config.fee_receiver,
            mint: wsol_mint::id(),
            diff: borrow_fee
                .try_sub(host_fee)
                .unwrap()
                .try_round_u64()
                .unwrap() as i128,
        },
    ]);
    assert_eq!(balance_changes, expected_balance_changes);
    assert_eq!(mint_supply_changes, HashSet::new());
}
