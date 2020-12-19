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
    instruction::withdraw_reserve_liquidity, processor::process_instruction,
    state::INITIAL_COLLATERAL_RATE,
};

const FRACTIONAL_TO_USDC: u64 = 1_000_000;
const INITIAL_USDC_RESERVE_SUPPLY_LAMPORTS: u64 = 10 * FRACTIONAL_TO_USDC;

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    let user_accounts_owner = Keypair::new();
    let usdc_mint = add_usdc_mint(&mut test);
    let lending_market = add_lending_market(&mut test, usdc_mint.pubkey);

    const WITHDRAW_COLLATERAL_AMOUNT: u64 =
        INITIAL_COLLATERAL_RATE * INITIAL_USDC_RESERVE_SUPPLY_LAMPORTS;

    let usdc_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            liquidity_amount: INITIAL_USDC_RESERVE_SUPPLY_LAMPORTS,
            liquidity_mint_decimals: usdc_mint.decimals,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            collateral_amount: WITHDRAW_COLLATERAL_AMOUNT,
            config: TEST_RESERVE_CONFIG,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[
            approve(
                &spl_token::id(),
                &usdc_reserve.user_collateral_account,
                &lending_market.authority,
                &user_accounts_owner.pubkey(),
                &[],
                WITHDRAW_COLLATERAL_AMOUNT,
            )
            .unwrap(),
            withdraw_reserve_liquidity(
                spl_token_lending::id(),
                WITHDRAW_COLLATERAL_AMOUNT,
                usdc_reserve.user_collateral_account,
                usdc_reserve.user_liquidity_account,
                usdc_reserve.pubkey,
                usdc_reserve.collateral_mint,
                usdc_reserve.liquidity_supply,
                lending_market.keypair.pubkey(),
                lending_market.authority,
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);
    assert!(banks_client.process_transaction(transaction).await.is_ok());
}
