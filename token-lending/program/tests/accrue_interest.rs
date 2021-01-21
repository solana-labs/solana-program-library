#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_token_lending::{
    instruction::accrue_reserve_interest,
    math::{Decimal, Rate, TryMul},
    processor::process_instruction,
    state::SLOTS_PER_YEAR,
};

const LAMPORTS_TO_SOL: u64 = 1_000_000_000;
const FRACTIONAL_TO_USDC: u64 = 1_000_000;

const INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS: u64 = 100 * LAMPORTS_TO_SOL;
const INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL: u64 = 100 * FRACTIONAL_TO_USDC;

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(80_000);

    let user_accounts_owner = Keypair::new();
    let usdc_mint = add_usdc_mint(&mut test);
    let lending_market = add_lending_market(&mut test, usdc_mint.pubkey);

    let mut reserve_config = TEST_RESERVE_CONFIG;
    reserve_config.loan_to_value_ratio = 80;

    // Configure reserve to a fixed borrow rate of 1%
    const BORROW_RATE: u8 = 1;
    reserve_config.min_borrow_rate = BORROW_RATE;
    reserve_config.optimal_borrow_rate = BORROW_RATE;
    reserve_config.optimal_utilization_rate = 100;

    let usdc_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            borrow_amount: 100,
            liquidity_amount: INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL,
            liquidity_mint_decimals: usdc_mint.decimals,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            slots_elapsed: SLOTS_PER_YEAR,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let sol_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            borrow_amount: 100,
            liquidity_amount: INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS,
            liquidity_mint_decimals: 9,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            slots_elapsed: SLOTS_PER_YEAR,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;
    let mut transaction = Transaction::new_with_payer(
        &[accrue_reserve_interest(
            spl_token_lending::id(),
            vec![usdc_reserve.pubkey, sol_reserve.pubkey],
        )],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer], recent_blockhash);
    assert!(banks_client.process_transaction(transaction).await.is_ok());

    let sol_reserve = sol_reserve.get_state(&mut banks_client).await;
    let usdc_reserve = usdc_reserve.get_state(&mut banks_client).await;

    let borrow_rate = Rate::from_percent(100u8 + BORROW_RATE);
    assert!(sol_reserve.cumulative_borrow_rate_wads > borrow_rate.into());
    assert_eq!(
        sol_reserve.cumulative_borrow_rate_wads,
        usdc_reserve.cumulative_borrow_rate_wads
    );
    assert!(
        sol_reserve.liquidity.borrowed_amount_wads
            > Decimal::from(100u64).try_mul(borrow_rate).unwrap()
    );
    assert_eq!(
        sol_reserve.liquidity.borrowed_amount_wads,
        usdc_reserve.liquidity.borrowed_amount_wads
    );
}
