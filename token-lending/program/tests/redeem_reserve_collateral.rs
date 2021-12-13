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
    instruction::redeem_reserve_collateral, processor::process_instruction,
    state::INITIAL_COLLATERAL_RATIO,
};

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(40_000);

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    const USDC_RESERVE_LIQUIDITY_FRACTIONAL: u64 = 10 * FRACTIONAL_TO_USDC;
    const COLLATERAL_AMOUNT: u64 = USDC_RESERVE_LIQUIDITY_FRACTIONAL * INITIAL_COLLATERAL_RATIO;

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            collateral_amount: COLLATERAL_AMOUNT,
            liquidity_amount: USDC_RESERVE_LIQUIDITY_FRACTIONAL,
            liquidity_mint_decimals: usdc_mint.decimals,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            config: TEST_RESERVE_CONFIG,
            mark_fresh: true,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let user_transfer_authority = Keypair::new();
    let mut transaction = Transaction::new_with_payer(
        &[
            approve(
                &spl_token::id(),
                &usdc_test_reserve.user_collateral_pubkey,
                &user_transfer_authority.pubkey(),
                &user_accounts_owner.pubkey(),
                &[],
                COLLATERAL_AMOUNT,
            )
            .unwrap(),
            redeem_reserve_collateral(
                spl_token_lending::id(),
                COLLATERAL_AMOUNT,
                usdc_test_reserve.user_collateral_pubkey,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.pubkey,
                usdc_test_reserve.collateral_mint_pubkey,
                usdc_test_reserve.liquidity_supply_pubkey,
                lending_market.pubkey,
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
