#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;

use solana_program_test::*;
use solana_sdk::transaction::TransactionError;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, transport::TransportError};
use spl_token_lending::{
    error::LendingError,
    math::Decimal,
    processor::process_instruction,
    state::{INITIAL_COLLATERAL_RATE, SLOTS_PER_YEAR},
};

const LAMPORTS_TO_SOL: u64 = 1_000_000_000;
const FRACTIONAL_TO_USDC: u64 = 1_000_000;

const INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS: u64 = 100 * LAMPORTS_TO_SOL;
const INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL: u64 = 100 * FRACTIONAL_TO_USDC;

// set loan values to about 90% of collateral value so that it gets liquidated
const USDC_LOAN: u64 = 2 * FRACTIONAL_TO_USDC;
const USDC_LOAN_SOL_COLLATERAL: u64 = INITIAL_COLLATERAL_RATE * LAMPORTS_TO_SOL;

const SOL_LOAN: u64 = LAMPORTS_TO_SOL;
const SOL_LOAN_USDC_COLLATERAL: u64 = 2 * INITIAL_COLLATERAL_RATE * FRACTIONAL_TO_USDC;
const NUMBER_OF_TESTS: u64 = 3;

struct TestReturn {
    banks_client: BanksClient,
    usdc_reserve: TestReserve,
    sol_reserve: TestReserve,
    obligation: TestObligation,
    result: Result<(), TransactionError>,
}

enum ObligationType {
    USDC,
    SOL,
}
struct TestConfig {
    obligation_type: ObligationType,
    amount: u64,
}

async fn setup(config: TestConfig) -> TestReturn {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    let TestConfig {
        amount,
        obligation_type,
    } = config;

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(NUMBER_OF_TESTS * 80_000);

    let user_accounts_owner = Keypair::new();
    let sol_usdc_dex_market = TestDexMarket::setup(&mut test, TestDexMarketPair::SOL_USDC);
    let usdc_mint = add_usdc_mint(&mut test);
    let lending_market = add_lending_market(&mut test, usdc_mint.pubkey);

    // Loans are unhealthy if borrow is more than 80% of collateral
    let mut reserve_config = TEST_RESERVE_CONFIG;
    reserve_config.liquidation_threshold = 80;

    let obligation: TestObligation;
    let result: Result<(), TransportError>;
    let usdc_borrow_amount = match obligation_type {
        ObligationType::USDC => amount,
        _ => 0,
    };
    let sol_borrow_amount = match obligation_type {
        ObligationType::SOL => amount,
        _ => 0,
    };

    let usdc_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            config: reserve_config,
            slots_elapsed: SLOTS_PER_YEAR,
            liquidity_amount: INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            borrow_amount: usdc_borrow_amount,
            user_liquidity_amount: usdc_borrow_amount,
            collateral_amount: SOL_LOAN_USDC_COLLATERAL,
            ..AddReserveArgs::default()
        },
    );

    let sol_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            config: reserve_config,
            slots_elapsed: SLOTS_PER_YEAR,
            liquidity_amount: INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS,
            liquidity_mint_decimals: 9,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            dex_market_pubkey: Some(sol_usdc_dex_market.pubkey),
            collateral_amount: USDC_LOAN_SOL_COLLATERAL,
            borrow_amount: sol_borrow_amount,
            user_liquidity_amount: sol_borrow_amount,
            ..AddReserveArgs::default()
        },
    );

    match obligation_type {
        ObligationType::USDC => {
            obligation = add_obligation(
                &mut test,
                &user_accounts_owner,
                &lending_market,
                AddObligationArgs {
                    slots_elapsed: SLOTS_PER_YEAR,
                    borrow_reserve: &usdc_reserve,
                    collateral_reserve: &sol_reserve,
                    collateral_amount: USDC_LOAN_SOL_COLLATERAL,
                    borrowed_liquidity_wads: Decimal::from(usdc_borrow_amount),
                },
            );
        }
        ObligationType::SOL => {
            obligation = add_obligation(
                &mut test,
                &user_accounts_owner,
                &lending_market,
                AddObligationArgs {
                    slots_elapsed: SLOTS_PER_YEAR,
                    borrow_reserve: &sol_reserve,
                    collateral_reserve: &usdc_reserve,
                    collateral_amount: SOL_LOAN_USDC_COLLATERAL,
                    borrowed_liquidity_wads: Decimal::from(sol_borrow_amount),
                },
            );
        }
    }

    let (mut banks_client, payer, _recent_blockhash) = test.start().await;

    match obligation_type {
        ObligationType::USDC => {
            result = lending_market
                .liquidate(
                    &mut banks_client,
                    &payer,
                    LiquidateArgs {
                        repay_reserve: &usdc_reserve,
                        withdraw_reserve: &sol_reserve,
                        dex_market: &sol_usdc_dex_market,
                        amount: usdc_borrow_amount,
                        user_accounts_owner: &user_accounts_owner,
                        obligation: &obligation,
                    },
                )
                .await;
        }
        ObligationType::SOL => {
            result = lending_market
                .liquidate(
                    &mut banks_client,
                    &payer,
                    LiquidateArgs {
                        repay_reserve: &sol_reserve,
                        withdraw_reserve: &usdc_reserve,
                        dex_market: &sol_usdc_dex_market,
                        amount: sol_borrow_amount,
                        user_accounts_owner: &user_accounts_owner,
                        obligation: &obligation,
                    },
                )
                .await;
        }
    }

    let unwrapped_result = match result {
        Ok(t) => Ok(t),
        Err(t) => Err(t.unwrap()),
    };
    TestReturn {
        banks_client,
        usdc_reserve,
        sol_reserve,
        obligation,
        result: unwrapped_result,
    }
}

#[tokio::test]
async fn test_liquidate_usdc_obligation() {
    let TestReturn {
        mut banks_client,
        usdc_reserve,
        obligation,
        result,
        ..
    } = setup(TestConfig {
        amount: USDC_LOAN,
        obligation_type: ObligationType::USDC,
    })
    .await;
    assert!(result.is_ok());
    let usdc_liquidity_supply =
        get_token_balance(&mut banks_client, usdc_reserve.liquidity_supply).await;
    let usdc_loan_state = obligation.get_state(&mut banks_client).await;
    let usdc_liquidated = usdc_liquidity_supply - INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL;
    assert!(usdc_liquidated > USDC_LOAN / 2);
    assert_eq!(
        usdc_liquidated,
        usdc_loan_state.borrowed_liquidity_wads.round_u64()
    );
    let collateral_liquidated =
        USDC_LOAN_SOL_COLLATERAL - usdc_loan_state.deposited_collateral_tokens;
    assert!(collateral_liquidated > 0)
}

#[tokio::test]
async fn test_liquidate_sol_obligation() {
    let TestReturn {
        mut banks_client,
        sol_reserve,
        obligation,
        result,
        ..
    } = setup(TestConfig {
        amount: SOL_LOAN,
        obligation_type: ObligationType::SOL,
    })
    .await;
    assert!(result.is_ok());
    let sol_liquidity_supply =
        get_token_balance(&mut banks_client, sol_reserve.liquidity_supply).await;
    let sol_loan_state = obligation.get_state(&mut banks_client).await;
    let sol_liquidated = sol_liquidity_supply - INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS;
    assert!(sol_liquidated > SOL_LOAN / 2);
    assert_eq!(
        sol_liquidated,
        sol_loan_state.borrowed_liquidity_wads.round_u64()
    );

    let collateral_liquidated =
        SOL_LOAN_USDC_COLLATERAL - sol_loan_state.deposited_collateral_tokens;
    assert!(collateral_liquidated > 0)
}

#[tokio::test]
async fn test_liquidate_healthy_obligation_failure() {
    let TestReturn { result, .. } = setup(TestConfig {
        amount: 100,
        obligation_type: ObligationType::USDC,
    })
    .await;
    let he_as_number = LendingError::HealthyObligation as u32;
    let unwrapped = result.unwrap_err();
    println!("WHAT");
    assert_eq!(
        solana_sdk::transaction::TransactionError::InstructionError(
            2,
            solana_sdk::instruction::InstructionError::Custom(he_as_number)
        ),
        unwrapped
    );
}
