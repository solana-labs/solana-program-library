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
    instruction::deposit_obligation_collateral, math::Decimal, processor::process_instruction,
    state::INITIAL_COLLATERAL_RATIO,
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
    test.set_bpf_compute_max_units(25_000);

    const INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS: u64 = 100 * LAMPORTS_TO_SOL;
    const INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL: u64 = 100 * FRACTIONAL_TO_USDC;

    const DEPOSIT_COLLATERAL: u64 = 1 * LAMPORTS_TO_SOL;

    const OBLIGATION_LOAN: u64 = 10 * FRACTIONAL_TO_USDC;
    const OBLIGATION_COLLATERAL: u64 = 10 * LAMPORTS_TO_SOL * INITIAL_COLLATERAL_RATIO;

    let user_accounts_owner = Keypair::new();
    let user_transfer_authority = Keypair::new();
    let usdc_mint = add_usdc_mint(&mut test);
    let lending_market = add_lending_market(&mut test, usdc_mint.pubkey);

    let sol_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            liquidity_amount: INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS,
            liquidity_mint_decimals: 9,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            collateral_amount: OBLIGATION_COLLATERAL,
            config: TEST_RESERVE_CONFIG,
            ..AddReserveArgs::default()
        },
    );

    let usdc_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            initial_borrow_rate: 1,
            liquidity_amount: INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            borrow_amount: OBLIGATION_LOAN * 101 / 100,
            user_liquidity_amount: OBLIGATION_LOAN,
            config: TEST_RESERVE_CONFIG,
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

    let initial_collateral_supply_balance =
        get_token_balance(&mut banks_client, sol_reserve.collateral_supply).await;
    let initial_user_collateral_balance =
        get_token_balance(&mut banks_client, sol_reserve.user_collateral_account).await;
    let initial_obligation_token_balance =
        get_token_balance(&mut banks_client, obligation.token_account).await;

    let mut transaction = Transaction::new_with_payer(
        &[
            approve(
                &spl_token::id(),
                &sol_reserve.user_collateral_account,
                &user_transfer_authority.pubkey(),
                &user_accounts_owner.pubkey(),
                &[],
                DEPOSIT_COLLATERAL,
            )
            .unwrap(),
            deposit_obligation_collateral(
                spl_token_lending::id(),
                DEPOSIT_COLLATERAL,
                sol_reserve.user_collateral_account,
                sol_reserve.collateral_supply,
                sol_reserve.pubkey,
                obligation.pubkey,
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

    // check that collateral tokens were transferred
    let collateral_supply_balance =
        get_token_balance(&mut banks_client, sol_reserve.collateral_supply).await;
    assert_eq!(
        collateral_supply_balance,
        initial_collateral_supply_balance + DEPOSIT_COLLATERAL
    );
    let user_collateral_balance =
        get_token_balance(&mut banks_client, sol_reserve.user_collateral_account).await;
    assert_eq!(
        user_collateral_balance,
        initial_user_collateral_balance - DEPOSIT_COLLATERAL
    );

    // check that obligation tokens were minted
    let obligation_token_balance =
        get_token_balance(&mut banks_client, obligation.token_account).await;
    assert_eq!(
        obligation_token_balance,
        initial_obligation_token_balance + DEPOSIT_COLLATERAL
    );
}
