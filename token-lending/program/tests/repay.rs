#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_token::instruction::approve;
use spl_token_lending::{
    instruction::repay_reserve_liquidity, math::Decimal, processor::process_instruction,
    state::SLOTS_PER_YEAR,
};

const LAMPORTS_TO_SOL: u64 = 1_000_000_000;
const FRACTIONAL_TO_USDC: u64 = 1_000_000;

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

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(80_000);

    const OBLIGATION_LOAN: u64 = 1;
    const OBLIGATION_COLLATERAL: u64 = 500;

    let user_accounts_owner = Keypair::new();
    let user_transfer_authority = Keypair::new();
    let sol_usdc_dex_market = TestDexMarket::setup(&mut test, TestDexMarketPair::SOL_USDC);
    let usdc_mint = add_usdc_mint(&mut test);
    let lending_market = add_lending_market(&mut test, usdc_mint.pubkey);

    let usdc_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            config: TEST_RESERVE_CONFIG,
            slots_elapsed: SLOTS_PER_YEAR,
            liquidity_amount: INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            borrow_amount: OBLIGATION_LOAN,
            user_liquidity_amount: OBLIGATION_LOAN,
            ..AddReserveArgs::default()
        },
    );

    let sol_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            config: TEST_RESERVE_CONFIG,
            slots_elapsed: SLOTS_PER_YEAR,
            liquidity_amount: INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS,
            liquidity_mint_decimals: 9,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            dex_market_pubkey: Some(sol_usdc_dex_market.pubkey),
            collateral_amount: OBLIGATION_COLLATERAL,
            ..AddReserveArgs::default()
        },
    );

    let obligation = add_obligation(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddObligationArgs {
            slots_elapsed: SLOTS_PER_YEAR,
            borrow_reserve: &usdc_reserve,
            collateral_reserve: &sol_reserve,
            collateral_amount: OBLIGATION_COLLATERAL,
            borrowed_liquidity_wads: Decimal::from(OBLIGATION_LOAN),
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[
            approve(
                &spl_token::id(),
                &usdc_reserve.user_liquidity_account,
                &user_transfer_authority.pubkey(),
                &user_accounts_owner.pubkey(),
                &[],
                OBLIGATION_LOAN,
            )
            .unwrap(),
            approve(
                &spl_token::id(),
                &obligation.token_account,
                &user_transfer_authority.pubkey(),
                &user_accounts_owner.pubkey(),
                &[],
                OBLIGATION_COLLATERAL,
            )
            .unwrap(),
            repay_reserve_liquidity(
                spl_token_lending::id(),
                OBLIGATION_LOAN,
                usdc_reserve.user_liquidity_account,
                sol_reserve.user_collateral_account,
                usdc_reserve.pubkey,
                usdc_reserve.liquidity_supply,
                sol_reserve.pubkey,
                sol_reserve.collateral_supply,
                obligation.keypair.pubkey(),
                obligation.token_mint,
                obligation.token_account,
                lending_market.keypair.pubkey(),
                lending_market.authority,
                user_transfer_authority.pubkey(),
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(
        &[&payer, &user_accounts_owner, &user_transfer_authority],
        recent_blockhash,
    );
    assert!(banks_client.process_transaction(transaction).await.is_ok());
}
