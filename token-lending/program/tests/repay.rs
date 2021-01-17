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
    instruction::repay_reserve_liquidity,
    math::{Decimal, TryAdd, TryDiv, TryMul, TrySub},
    processor::process_instruction,
    state::{INITIAL_COLLATERAL_RATIO, SLOTS_PER_YEAR},
};

const LAMPORTS_TO_SOL: u64 = 1_000_000_000;
const FRACTIONAL_TO_USDC: u64 = 1_000_000;

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(85_000);

    const INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS: u64 = 100 * LAMPORTS_TO_SOL;
    const INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL: u64 = 100 * FRACTIONAL_TO_USDC;

    const OBLIGATION_LOAN: u64 = 10 * FRACTIONAL_TO_USDC;
    const OBLIGATION_COLLATERAL: u64 = 10 * LAMPORTS_TO_SOL * INITIAL_COLLATERAL_RATIO;

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
            borrow_reserve: &usdc_reserve,
            collateral_reserve: &sol_reserve,
            collateral_amount: OBLIGATION_COLLATERAL,
            borrowed_liquidity_wads: Decimal::from(OBLIGATION_LOAN),
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let initial_user_collateral_balance =
        get_token_balance(&mut banks_client, sol_reserve.user_collateral_account).await;

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
                lending_market.pubkey,
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

    let collateral_received =
        get_token_balance(&mut banks_client, sol_reserve.user_collateral_account).await
            - initial_user_collateral_balance;
    assert!(collateral_received > 0);

    let borrow_reserve_state = usdc_reserve.get_state(&mut banks_client).await;
    assert!(borrow_reserve_state.state.cumulative_borrow_rate_wads > Decimal::one());

    let obligation_state = obligation.get_state(&mut banks_client).await;
    assert_eq!(
        obligation_state.cumulative_borrow_rate_wads,
        borrow_reserve_state.state.cumulative_borrow_rate_wads
    );
    assert_eq!(
        obligation_state.borrowed_liquidity_wads,
        borrow_reserve_state.state.borrowed_liquidity_wads
    );

    // use cumulative borrow rate directly since test rate starts at 1.0
    let expected_obligation_interest = obligation_state
        .cumulative_borrow_rate_wads
        .try_mul(OBLIGATION_LOAN)
        .unwrap()
        .try_sub(Decimal::from(OBLIGATION_LOAN))
        .unwrap();
    assert_eq!(
        obligation_state.borrowed_liquidity_wads,
        expected_obligation_interest
    );

    let expected_obligation_total = Decimal::from(OBLIGATION_LOAN)
        .try_add(expected_obligation_interest)
        .unwrap();

    let expected_obligation_repaid_percent = Decimal::from(OBLIGATION_LOAN)
        .try_div(expected_obligation_total)
        .unwrap();

    let expected_collateral_received = expected_obligation_repaid_percent
        .try_mul(OBLIGATION_COLLATERAL)
        .unwrap()
        .try_round_u64()
        .unwrap();
    assert_eq!(collateral_received, expected_collateral_received);

    let expected_collateral_remaining = OBLIGATION_COLLATERAL - expected_collateral_received;
    assert_eq!(
        obligation_state.deposited_collateral_tokens,
        expected_collateral_remaining
    );
}
