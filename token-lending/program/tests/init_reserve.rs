#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program_test::*;
use solana_sdk::{
    instruction::InstructionError,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::{Transaction, TransactionError},
};
use spl_token_lending::{
    error::LendingError,
    instruction::{init_reserve, update_reserve_config},
    processor::process_instruction,
    state::{ReserveConfig, ReserveFees, INITIAL_COLLATERAL_RATIO},
};

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_bpf_compute_max_units(70_000);

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);
    let sol_oracle = add_sol_oracle(&mut test);

    let (mut banks_client, payer, _recent_blockhash) = test.start().await;

    const RESERVE_AMOUNT: u64 = 42;

    let sol_user_liquidity_account = create_and_mint_to_token_account(
        &mut banks_client,
        spl_token::native_mint::id(),
        None,
        &payer,
        user_accounts_owner.pubkey(),
        RESERVE_AMOUNT,
    )
    .await;

    let sol_reserve = TestReserve::init(
        "sol".to_owned(),
        &mut banks_client,
        &lending_market,
        &sol_oracle,
        RESERVE_AMOUNT,
        TEST_RESERVE_CONFIG,
        spl_token::native_mint::id(),
        sol_user_liquidity_account,
        &payer,
        &user_accounts_owner,
    )
    .await
    .unwrap();

    sol_reserve.validate_state(&mut banks_client).await;

    let sol_liquidity_supply =
        get_token_balance(&mut banks_client, sol_reserve.liquidity_supply_pubkey).await;
    assert_eq!(sol_liquidity_supply, RESERVE_AMOUNT);
    let user_sol_balance =
        get_token_balance(&mut banks_client, sol_reserve.user_liquidity_pubkey).await;
    assert_eq!(user_sol_balance, 0);
    let user_sol_collateral_balance =
        get_token_balance(&mut banks_client, sol_reserve.user_collateral_pubkey).await;
    assert_eq!(
        user_sol_collateral_balance,
        RESERVE_AMOUNT * INITIAL_COLLATERAL_RATIO
    );
}

#[tokio::test]
async fn test_already_initialized() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    let user_accounts_owner = Keypair::new();
    let user_transfer_authority = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            liquidity_amount: 42,
            liquidity_mint_decimals: usdc_mint.decimals,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            config: TEST_RESERVE_CONFIG,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[init_reserve(
            spl_token_lending::id(),
            42,
            usdc_test_reserve.config,
            usdc_test_reserve.user_liquidity_pubkey,
            usdc_test_reserve.user_collateral_pubkey,
            usdc_test_reserve.pubkey,
            usdc_test_reserve.liquidity_mint_pubkey,
            usdc_test_reserve.liquidity_supply_pubkey,
            usdc_test_reserve.liquidity_fee_receiver_pubkey,
            usdc_test_reserve.collateral_mint_pubkey,
            usdc_test_reserve.collateral_supply_pubkey,
            usdc_oracle.pyth_product_pubkey,
            usdc_oracle.pyth_price_pubkey,
            usdc_oracle.switchboard_feed_pubkey,
            lending_market.pubkey,
            lending_market.owner.pubkey(),
            user_transfer_authority.pubkey(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(
        &[&payer, &lending_market.owner, &user_transfer_authority],
        recent_blockhash,
    );
    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::AlreadyInitialized as u32)
        )
    );
}

#[tokio::test]
async fn test_invalid_fees() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);
    let sol_oracle = add_sol_oracle(&mut test);

    let (mut banks_client, payer, _recent_blockhash) = test.start().await;

    const RESERVE_AMOUNT: u64 = 42;

    let sol_user_liquidity_account = create_and_mint_to_token_account(
        &mut banks_client,
        spl_token::native_mint::id(),
        None,
        &payer,
        user_accounts_owner.pubkey(),
        RESERVE_AMOUNT,
    )
    .await;

    // fee above 100%
    {
        let mut config = TEST_RESERVE_CONFIG;
        config.fees = ReserveFees {
            borrow_fee_wad: 1_000_000_000_000_000_001,
            flash_loan_fee_wad: 1_000_000_000_000_000_001,
            host_fee_percentage: 0,
        };

        assert_eq!(
            TestReserve::init(
                "sol".to_owned(),
                &mut banks_client,
                &lending_market,
                &sol_oracle,
                RESERVE_AMOUNT,
                config,
                spl_token::native_mint::id(),
                sol_user_liquidity_account,
                &payer,
                &user_accounts_owner,
            )
            .await
            .unwrap_err(),
            TransactionError::InstructionError(
                8,
                InstructionError::Custom(LendingError::InvalidConfig as u32)
            )
        );
    }

    // host fee above 100%
    {
        let mut config = TEST_RESERVE_CONFIG;
        config.fees = ReserveFees {
            borrow_fee_wad: 10_000_000_000_000_000,
            flash_loan_fee_wad: 10_000_000_000_000_000,
            host_fee_percentage: 101,
        };

        assert_eq!(
            TestReserve::init(
                "sol".to_owned(),
                &mut banks_client,
                &lending_market,
                &sol_oracle,
                RESERVE_AMOUNT,
                config,
                spl_token::native_mint::id(),
                sol_user_liquidity_account,
                &payer,
                &user_accounts_owner,
            )
            .await
            .unwrap_err(),
            TransactionError::InstructionError(
                8,
                InstructionError::Custom(LendingError::InvalidConfig as u32)
            )
        );
    }
}

#[tokio::test]
async fn test_update_reserve_config() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    let user_accounts_owner = Keypair::new();
    let user_transfer_authority = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mint = add_usdc_mint(&mut test);
    let oracle = add_usdc_oracle(&mut test);
    let test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &oracle,
        &user_accounts_owner,
        AddReserveArgs {
            liquidity_amount: 42,
            liquidity_mint_decimals: mint.decimals,
            liquidity_mint_pubkey: mint.pubkey,
            config: TEST_RESERVE_CONFIG,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    // Create a reserve
    let mut transaction = Transaction::new_with_payer(
        &[init_reserve(
            spl_token_lending::id(),
            42,
            test_reserve.config,
            test_reserve.user_liquidity_pubkey,
            test_reserve.user_collateral_pubkey,
            test_reserve.pubkey,
            test_reserve.liquidity_mint_pubkey,
            test_reserve.liquidity_supply_pubkey,
            test_reserve.liquidity_fee_receiver_pubkey,
            test_reserve.collateral_mint_pubkey,
            test_reserve.collateral_supply_pubkey,
            oracle.pyth_product_pubkey,
            oracle.pyth_price_pubkey,
            oracle.switchboard_feed_pubkey,
            lending_market.pubkey,
            lending_market.owner.pubkey(),
            user_transfer_authority.pubkey(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(
        &[&payer, &lending_market.owner, &user_transfer_authority],
        recent_blockhash,
    );
    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::AlreadyInitialized as u32)
        )
    );

    // Update the reserve config
    let new_config: ReserveConfig = ReserveConfig {
        optimal_utilization_rate: 75,
        loan_to_value_ratio: 45,
        liquidation_bonus: 10,
        liquidation_threshold: 65,
        min_borrow_rate: 1,
        optimal_borrow_rate: 5,
        max_borrow_rate: 45,
        deposit_limit: 1_000_000,
        fees: ReserveFees {
            borrow_fee_wad: 200_000_000_000,
            flash_loan_fee_wad: 5_000_000_000_000_000,
            host_fee_percentage: 15,
        },
    };

    let mut transaction = Transaction::new_with_payer(
        &[update_reserve_config(
            spl_token_lending::id(),
            new_config,
            test_reserve.pubkey,
            lending_market.pubkey,
            lending_market.owner.pubkey(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &lending_market.owner], recent_blockhash);
    assert!(banks_client.process_transaction(transaction).await.is_ok());

    let test_reserve = test_reserve.get_state(&mut banks_client).await;
    assert_eq!(test_reserve.config, new_config);
}
