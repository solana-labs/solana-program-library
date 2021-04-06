#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    instruction::InstructionError,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction::create_account,
    transaction::Transaction,
};
use spl_token::{
    instruction::approve,
    solana_program::program_pack::Pack,
    state::{Account as Token, Mint},
};
use spl_token_lending::{
    instruction::{
        borrow_obligation_liquidity, deposit_obligation_collateral, init_obligation,
        refresh_obligation, refresh_reserve, repay_obligation_liquidity,
        withdraw_obligation_collateral,
    },
    math::Decimal,
    processor::process_instruction,
    state::{Obligation, INITIAL_COLLATERAL_RATIO},
};

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(41_000);

    const FEE_AMOUNT: u64 = 100;
    const HOST_FEE_AMOUNT: u64 = 20;

    const SOL_DEPOSIT_AMOUNT_LAMPORTS: u64 = 100 * LAMPORTS_TO_SOL * INITIAL_COLLATERAL_RATIO;
    const SOL_RESERVE_COLLATERAL_LAMPORTS: u64 = SOL_DEPOSIT_AMOUNT_LAMPORTS;

    const USDC_BORROW_AMOUNT_FRACTIONAL: u64 = 1_000 * FRACTIONAL_TO_USDC - FEE_AMOUNT;
    const USDC_REPAY_AMOUNT_FRACTIONAL: u64 = USDC_BORROW_AMOUNT_FRACTIONAL + FEE_AMOUNT;
    const USDC_RESERVE_LIQUIDITY_FRACTIONAL: u64 =
        USDC_BORROW_AMOUNT_FRACTIONAL + USDC_REPAY_AMOUNT_FRACTIONAL;

    let user_accounts_owner = Keypair::new();
    let user_accounts_owner_pubkey = user_accounts_owner.pubkey();

    let user_transfer_authority = Keypair::new();
    let user_transfer_authority_pubkey = user_transfer_authority.pubkey();

    let obligation_keypair = Keypair::new();
    let obligation_pubkey = obligation_keypair.pubkey();

    let obligation_token_mint_keypair = Keypair::new();
    let obligation_token_mint_pubkey = obligation_token_mint_keypair.pubkey();

    let obligation_token_account_keypair = Keypair::new();
    let obligation_token_account_pubkey = obligation_token_account_keypair.pubkey();

    let usdc_mint = add_usdc_mint(&mut test);
    let lending_market = add_lending_market(&mut test, usdc_mint.pubkey);

    let mut reserve_config = TEST_RESERVE_CONFIG;
    reserve_config.loan_to_value_ratio = 50;

    let sol_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: SOL_RESERVE_COLLATERAL_LAMPORTS,
            liquidity_amount: SOL_RESERVE_COLLATERAL_LAMPORTS,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            liquidity_mint_decimals: 9,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: USDC_RESERVE_LIQUIDITY_FRACTIONAL,
            liquidity_amount: USDC_RESERVE_LIQUIDITY_FRACTIONAL,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;
    let payer_pubkey = payer.pubkey();

    let initial_collateral_supply_balance =
        get_token_balance(&mut banks_client, sol_test_reserve.collateral_supply_pubkey).await;
    let initial_user_collateral_balance =
        get_token_balance(&mut banks_client, sol_test_reserve.user_collateral_pubkey).await;
    let initial_liquidity_supply =
        get_token_balance(&mut banks_client, usdc_test_reserve.liquidity_supply_pubkey).await;
    let initial_user_liquidity_balance =
        get_token_balance(&mut banks_client, usdc_test_reserve.user_liquidity_pubkey).await;

    let rent = banks_client.get_rent().await.unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[
            // 0
            create_account(
                &payer.pubkey(),
                &obligation_keypair.pubkey(),
                rent.minimum_balance(Obligation::LEN),
                Obligation::LEN as u64,
                &spl_token_lending::id(),
            ),
            // 1
            init_obligation(
                spl_token_lending::id(),
                obligation_pubkey,
                lending_market.pubkey,
                user_accounts_owner_pubkey,
            ),
            // 2
            create_account(
                &payer_pubkey,
                &obligation_token_mint_pubkey,
                rent.minimum_balance(Mint::LEN),
                Mint::LEN as u64,
                &spl_token::id(),
            ),
            // 3
            create_account(
                &payer_pubkey,
                &obligation_token_account_pubkey,
                rent.minimum_balance(Token::LEN),
                Token::LEN as u64,
                &spl_token::id(),
            ),
            // 4
            refresh_reserve(
                spl_token_lending::id(),
                sol_test_reserve.pubkey,
                sol_test_reserve.liquidity_aggregator_pubkey,
            ),
            // 5
            approve(
                &spl_token::id(),
                &sol_test_reserve.user_collateral_pubkey,
                &user_transfer_authority_pubkey,
                &user_accounts_owner_pubkey,
                &[],
                SOL_DEPOSIT_AMOUNT_LAMPORTS,
            )
            .unwrap(),
            // 6
            deposit_obligation_collateral(
                spl_token_lending::id(),
                SOL_DEPOSIT_AMOUNT_LAMPORTS,
                sol_test_reserve.user_collateral_pubkey,
                sol_test_reserve.collateral_supply_pubkey,
                sol_test_reserve.pubkey,
                obligation_pubkey,
                obligation_token_mint_pubkey,
                obligation_token_account_pubkey,
                lending_market.pubkey,
                user_accounts_owner_pubkey,
                user_transfer_authority_pubkey,
            ),
            // 7
            refresh_obligation(
                spl_token_lending::id(),
                obligation_pubkey,
                vec![sol_test_reserve.pubkey],
            ),
            // 8
            refresh_reserve(spl_token_lending::id(), usdc_test_reserve.pubkey, None),
            // 9
            borrow_obligation_liquidity(
                spl_token_lending::id(),
                USDC_BORROW_AMOUNT_FRACTIONAL,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.pubkey,
                usdc_test_reserve.liquidity_fee_receiver_pubkey,
                obligation_pubkey,
                lending_market.pubkey,
                user_accounts_owner_pubkey,
                Some(usdc_test_reserve.liquidity_host_pubkey),
            ),
            // 10
            refresh_reserve(spl_token_lending::id(), usdc_test_reserve.pubkey, None),
            // 11
            refresh_obligation(
                spl_token_lending::id(),
                obligation_pubkey,
                vec![sol_test_reserve.pubkey, usdc_test_reserve.pubkey],
            ),
            // 12
            approve(
                &spl_token::id(),
                &usdc_test_reserve.user_liquidity_pubkey,
                &user_transfer_authority_pubkey,
                &user_accounts_owner_pubkey,
                &[],
                USDC_REPAY_AMOUNT_FRACTIONAL,
            )
            .unwrap(),
            // 13
            repay_obligation_liquidity(
                spl_token_lending::id(),
                USDC_REPAY_AMOUNT_FRACTIONAL,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.pubkey,
                obligation_pubkey,
                lending_market.pubkey,
                user_transfer_authority_pubkey,
            ),
            // 14
            refresh_obligation(
                spl_token_lending::id(),
                obligation_pubkey,
                vec![sol_test_reserve.pubkey],
            ),
            // 15
            approve(
                &spl_token::id(),
                &obligation_token_account_pubkey,
                &user_transfer_authority_pubkey,
                &user_accounts_owner_pubkey,
                &[],
                SOL_DEPOSIT_AMOUNT_LAMPORTS,
            )
            .unwrap(),
            // 16
            withdraw_obligation_collateral(
                spl_token_lending::id(),
                SOL_DEPOSIT_AMOUNT_LAMPORTS,
                sol_test_reserve.collateral_supply_pubkey,
                sol_test_reserve.user_collateral_pubkey,
                sol_test_reserve.pubkey,
                obligation_pubkey,
                obligation_token_mint_pubkey,
                obligation_token_account_pubkey,
                lending_market.pubkey,
                user_transfer_authority_pubkey,
            ),
        ],
        Some(&payer_pubkey),
    );

    transaction.sign(
        &vec![
            &payer,
            &obligation_keypair,
            &user_accounts_owner,
            &obligation_token_mint_keypair,
            &obligation_token_account_keypair,
            &user_transfer_authority,
        ],
        recent_blockhash,
    );
    assert!(banks_client.process_transaction(transaction).await.is_ok());

    let sol_reserve = sol_test_reserve.get_state(&mut banks_client).await;
    let usdc_reserve = usdc_test_reserve.get_state(&mut banks_client).await;

    let obligation = {
        let obligation_account: Account = banks_client
            .get_account(obligation_pubkey)
            .await
            .unwrap()
            .unwrap();
        Obligation::unpack(&obligation_account.data[..]).unwrap()
    };

    let collateral_supply_balance =
        get_token_balance(&mut banks_client, sol_test_reserve.collateral_supply_pubkey).await;
    let user_collateral_balance =
        get_token_balance(&mut banks_client, sol_test_reserve.user_collateral_pubkey).await;
    assert_eq!(collateral_supply_balance, initial_collateral_supply_balance);
    assert_eq!(user_collateral_balance, initial_user_collateral_balance);

    let liquidity_supply =
        get_token_balance(&mut banks_client, usdc_test_reserve.liquidity_supply_pubkey).await;
    let user_liquidity_balance =
        get_token_balance(&mut banks_client, usdc_test_reserve.user_liquidity_pubkey).await;
    assert_eq!(liquidity_supply, initial_liquidity_supply);
    assert_eq!(
        user_liquidity_balance,
        initial_user_liquidity_balance - FEE_AMOUNT
    );
    assert_eq!(usdc_reserve.liquidity.borrowed_amount_wads, Decimal::zero());
    assert_eq!(
        usdc_reserve.liquidity.available_amount,
        initial_liquidity_supply
    );

    let obligation_token_balance =
        get_token_balance(&mut banks_client, obligation_token_account_pubkey).await;
    assert_eq!(obligation_token_balance, 0);
    assert_eq!(obligation.deposits.len(), 0);
    assert_eq!(obligation.borrows.len(), 0);

    let fee_balance = get_token_balance(
        &mut banks_client,
        usdc_test_reserve.liquidity_fee_receiver_pubkey,
    )
    .await;
    assert_eq!(fee_balance, FEE_AMOUNT - HOST_FEE_AMOUNT);

    let host_fee_balance =
        get_token_balance(&mut banks_client, usdc_test_reserve.liquidity_host_pubkey).await;
    assert_eq!(host_fee_balance, HOST_FEE_AMOUNT);
}
