#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction::create_account,
    system_program,
    transaction::Transaction,
};
use spl_token::instruction::approve;
use spl_token_lending::{
    instruction::liquidate_obligation, math::Decimal, processor::process_instruction,
    state::INITIAL_COLLATERAL_RATE,
};

const LAMPORTS_TO_SOL: u64 = 1_000_000_000; // -> 2_210_000
const FRACTIONAL_TO_USDC: u64 = 1_000_000;
// 0.000001 USDC

// Market and collateral are setup to fill two orders in the dex market at an average
// price of 2210.5
const fn lamports_to_usdc_fractional(lamports: u64) -> u64 {
    lamports / LAMPORTS_TO_SOL * (2210 + 2211) / 2 * FRACTIONAL_TO_USDC / 1000
}

const INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS: u64 = 42_500 * LAMPORTS_TO_SOL;
const INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL: u64 =
    lamports_to_usdc_fractional(INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS);

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    const OBLIGATION_USDC_LOAN: u64 = FRACTIONAL_TO_USDC;
    const OBLIGATION_SOL_COLLATERAL: u64 = INITIAL_COLLATERAL_RATE * LAMPORTS_TO_SOL;

    let user_accounts_owner = Keypair::new();
    let sol_usdc_dex_market =
        TestDexMarket::setup(&mut test, "sol_usdc", SOL_USDC_BIDS, SOL_USDC_ASKS);
    let usdc_mint = add_usdc_mint(&mut test);
    let lending_market = add_lending_market(&mut test, usdc_mint.pubkey);

    let usdc_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            config: TEST_RESERVE_CONFIG,
            liquidity_amount: INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            borrow_amount: OBLIGATION_USDC_LOAN,
            user_liquidity_amount: OBLIGATION_USDC_LOAN,
            ..AddReserveArgs::default()
        },
    );

    let sol_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            config: TEST_RESERVE_CONFIG,
            liquidity_amount: INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS,
            liquidity_mint_decimals: 9,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            dex_market_pubkey: Some(sol_usdc_dex_market.pubkey),
            collateral_amount: OBLIGATION_SOL_COLLATERAL,
            ..AddReserveArgs::default()
        },
    );

    let obligation = add_obligation(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        &usdc_reserve,
        &sol_reserve,
        OBLIGATION_SOL_COLLATERAL,
        Decimal::from(OBLIGATION_USDC_LOAN),
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let memory_keypair = Keypair::new();
    let mut transaction = Transaction::new_with_payer(
        &[
            create_account(
                &payer.pubkey(),
                &memory_keypair.pubkey(),
                0,
                65548,
                &system_program::id(),
            ),
            approve(
                &spl_token::id(),
                &usdc_reserve.user_liquidity_account,
                &lending_market.authority,
                &user_accounts_owner.pubkey(),
                &[],
                OBLIGATION_USDC_LOAN,
            )
            .unwrap(),
            liquidate_obligation(
                spl_token_lending::id(),
                OBLIGATION_USDC_LOAN,
                usdc_reserve.user_liquidity_account,
                sol_reserve.user_collateral_account,
                usdc_reserve.pubkey,
                usdc_reserve.liquidity_supply,
                sol_reserve.pubkey,
                sol_reserve.collateral_supply,
                obligation.keypair.pubkey(),
                lending_market.keypair.pubkey(),
                lending_market.authority,
                sol_usdc_dex_market.pubkey,
                sol_usdc_dex_market.bids_pubkey,
                memory_keypair.pubkey(),
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(
        &[&payer, &memory_keypair, &user_accounts_owner],
        recent_blockhash,
    );
    assert!(banks_client.process_transaction(transaction).await.is_ok());
}
