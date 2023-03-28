#![cfg(feature = "test-bpf")]

use solana_program::instruction::InstructionError;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::Signer;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::transaction::TransactionError;

mod helpers;

use helpers::solend_program_test::{setup_world, Info, SolendProgramTest, User};
use solend_sdk::error::LendingError;

use solend_sdk::state::{LendingMarket, RateLimiterConfig, Reserve, ReserveConfig};

use helpers::*;

use solana_program_test::*;

use solend_sdk::state::Obligation;

async fn setup(
    wsol_reserve_config: &ReserveConfig,
) -> (
    SolendProgramTest,
    Info<LendingMarket>,
    Info<Reserve>,
    Info<Reserve>,
    User,
    Info<Obligation>,
    User,
    User,
    User,
) {
    let (mut test, lending_market, usdc_reserve, wsol_reserve, lending_market_owner, user) =
        setup_world(&test_reserve_config(), wsol_reserve_config).await;

    let obligation = lending_market
        .init_obligation(&mut test, Keypair::new(), &user)
        .await
        .expect("This should succeed");

    lending_market
        .deposit(&mut test, &usdc_reserve, &user, 100_000_000)
        .await
        .expect("This should succeed");

    let usdc_reserve = test.load_account(usdc_reserve.pubkey).await;

    lending_market
        .deposit_obligation_collateral(&mut test, &usdc_reserve, &obligation, &user, 100_000_000)
        .await
        .expect("This should succeed");

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

    // populate market price correctly
    lending_market
        .refresh_reserve(&mut test, &wsol_reserve)
        .await
        .unwrap();

    // populate deposit value correctly.
    let obligation = test.load_account::<Obligation>(obligation.pubkey).await;
    lending_market
        .refresh_obligation(&mut test, &obligation)
        .await
        .unwrap();

    let lending_market = test.load_account(lending_market.pubkey).await;
    let usdc_reserve = test.load_account(usdc_reserve.pubkey).await;
    let wsol_reserve = test.load_account(wsol_reserve.pubkey).await;
    let obligation = test.load_account::<Obligation>(obligation.pubkey).await;

    let host_fee_receiver = User::new_with_balances(&mut test, &[(&wsol_mint::id(), 0)]).await;
    (
        test,
        lending_market,
        usdc_reserve,
        wsol_reserve,
        user,
        obligation,
        host_fee_receiver,
        lending_market_owner,
        wsol_depositor,
    )
}

#[tokio::test]
async fn test_outflow_reserve() {
    let (
        mut test,
        lending_market,
        usdc_reserve,
        wsol_reserve,
        user,
        obligation,
        host_fee_receiver,
        lending_market_owner,
        wsol_depositor,
    ) = setup(&ReserveConfig {
        ..test_reserve_config()
    })
    .await;

    // ie, within 10 slots, the maximum outflow is $10
    lending_market
        .set_lending_market_owner_and_config(
            &mut test,
            &lending_market_owner,
            &lending_market_owner.keypair.pubkey(),
            RateLimiterConfig {
                window_duration: 10,
                max_outflow: 10,
            },
        )
        .await
        .unwrap();

    // borrow max amount
    lending_market
        .borrow_obligation_liquidity(
            &mut test,
            &wsol_reserve,
            &obligation,
            &user,
            &host_fee_receiver.get_account(&wsol_mint::id()).unwrap(),
            LAMPORTS_PER_SOL,
        )
        .await
        .unwrap();

    // for the next 10 slots, we shouldn't be able to withdraw, borrow, or redeem anything.
    let cur_slot = test.get_clock().await.slot;
    for _ in cur_slot..(cur_slot + 10) {
        let res = lending_market
            .borrow_obligation_liquidity(
                &mut test,
                &wsol_reserve,
                &obligation,
                &user,
                &host_fee_receiver.get_account(&wsol_mint::id()).unwrap(),
                1,
            )
            .await
            .err()
            .unwrap()
            .unwrap();

        assert_eq!(
            res,
            TransactionError::InstructionError(
                3,
                InstructionError::Custom(LendingError::OutflowRateLimitExceeded as u32)
            )
        );

        let res = lending_market
            .withdraw_obligation_collateral_and_redeem_reserve_collateral(
                &mut test,
                &usdc_reserve,
                &obligation,
                &user,
                1,
            )
            .await
            .err()
            .unwrap()
            .unwrap();

        assert_eq!(
            res,
            TransactionError::InstructionError(
                3,
                InstructionError::Custom(LendingError::OutflowRateLimitExceeded as u32)
            )
        );

        let res = lending_market
            .redeem(&mut test, &wsol_reserve, &wsol_depositor, 1)
            .await
            .err()
            .unwrap()
            .unwrap();

        assert_eq!(
            res,
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(LendingError::OutflowRateLimitExceeded as u32)
            )
        );

        test.advance_clock_by_slots(1).await;
    }
}
