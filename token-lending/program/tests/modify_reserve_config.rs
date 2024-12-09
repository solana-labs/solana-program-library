#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program::pubkey::Pubkey,
    solana_program_test::*,
    solana_sdk::{
        instruction::InstructionError,
        signature::{read_keypair_file, Keypair, Signer},
        transaction::{Transaction, TransactionError},
    },
    spl_token_lending::{
        error::LendingError,
        instruction::modify_reserve_config,
        processor::process_instruction,
        state::{
            InitLendingMarketParams, LendingMarket, ReserveConfig, ReserveFees,
            INITIAL_COLLATERAL_RATIO,
        },
    },
};

#[tokio::test]
async fn modify_reserve_config_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    test.set_compute_max_units(70_000);

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);
    let sol_oracle = add_sol_oracle(&mut test);

    const SOL_DEPOSIT_AMOUNT_LAMPORTS: u64 = 10 * LAMPORTS_TO_SOL * INITIAL_COLLATERAL_RATIO;
    const SOL_RESERVE_COLLATERAL_LAMPORTS: u64 = 2 * SOL_DEPOSIT_AMOUNT_LAMPORTS;

    let sol_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &sol_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: SOL_RESERVE_COLLATERAL_LAMPORTS,
            liquidity_amount: SOL_RESERVE_COLLATERAL_LAMPORTS,
            liquidity_mint_decimals: 9,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            config: TEST_RESERVE_CONFIG,
            mark_fresh: true,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    const OPTIMAL_UTILIZATION_RATE_CHANGE: u8 = 10;

    let new_config = ReserveConfig {
        optimal_utilization_rate: TEST_RESERVE_CONFIG.optimal_utilization_rate
            - OPTIMAL_UTILIZATION_RATE_CHANGE,
        loan_to_value_ratio: 50,
        liquidation_bonus: 5,
        liquidation_threshold: 55,
        min_borrow_rate: 0,
        optimal_borrow_rate: 4,
        max_borrow_rate: 30,
        fees: ReserveFees {
            borrow_fee_wad: 100_000_000_000,
            flash_loan_fee_wad: 3_000_000_000_000_000,
            host_fee_percentage: 20,
        },
    };

    let mut transaction = Transaction::new_with_payer(
        &[modify_reserve_config(
            spl_token_lending::id(),
            new_config,
            sol_test_reserve.pubkey,
            lending_market.pubkey,
            lending_market.owner.pubkey(),
        )],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &lending_market.owner], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.unwrap())
        .unwrap();

    let reserve_info = sol_test_reserve.get_state(&mut banks_client).await;
    assert_eq!(reserve_info.config, new_config);
}

#[tokio::test]
// Invalid Signer - Right owner, right market but owner is not a signer
async fn wrong_signer_of_lending_market_cannot_change_reserve_config() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    test.set_compute_max_units(70_000);

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);
    let sol_oracle = add_sol_oracle(&mut test);

    const SOL_DEPOSIT_AMOUNT_LAMPORTS: u64 = 10 * LAMPORTS_TO_SOL * INITIAL_COLLATERAL_RATIO;
    const SOL_RESERVE_COLLATERAL_LAMPORTS: u64 = 2 * SOL_DEPOSIT_AMOUNT_LAMPORTS;

    let sol_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &sol_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: SOL_RESERVE_COLLATERAL_LAMPORTS,
            liquidity_amount: SOL_RESERVE_COLLATERAL_LAMPORTS,
            liquidity_mint_decimals: 9,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            config: TEST_RESERVE_CONFIG,
            mark_fresh: true,
            ..AddReserveArgs::default()
        },
    );

    let mut other_lending_market = add_lending_market(&mut test);
    let other_lending_market_owner = Keypair::new();
    other_lending_market.owner = other_lending_market_owner;

    let (banks_client, payer, recent_blockhash) = test.start().await;

    const OPTIMAL_UTILIZATION_RATE_CHANGE: u8 = 10;

    let new_config = ReserveConfig {
        optimal_utilization_rate: TEST_RESERVE_CONFIG.optimal_utilization_rate
            - OPTIMAL_UTILIZATION_RATE_CHANGE,
        loan_to_value_ratio: 50,
        liquidation_bonus: 5,
        liquidation_threshold: 55,
        min_borrow_rate: 0,
        optimal_borrow_rate: 4,
        max_borrow_rate: 30,
        fees: ReserveFees {
            borrow_fee_wad: 100_000_000_000,
            flash_loan_fee_wad: 3_000_000_000_000_000,
            host_fee_percentage: 20,
        },
    };

    let mut instruction = modify_reserve_config(
        spl_token_lending::id(),
        new_config,
        sol_test_reserve.pubkey,
        lending_market.pubkey,
        lending_market.owner.pubkey(),
    );
    instruction.accounts[2].is_signer = false;

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));

    transaction.sign(&[&payer], recent_blockhash);

    let result = banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.unwrap());

    assert_eq!(
        result.unwrap_err(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::InvalidSigner as u32)
        )
    );
}

#[tokio::test]
// Right lending market, wrong owner
async fn owner_of_different_lending_market_cannot_change_reserve_config() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    test.set_compute_max_units(70_000);

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);
    let sol_oracle = add_sol_oracle(&mut test);

    const SOL_DEPOSIT_AMOUNT_LAMPORTS: u64 = 10 * LAMPORTS_TO_SOL * INITIAL_COLLATERAL_RATIO;
    const SOL_RESERVE_COLLATERAL_LAMPORTS: u64 = 2 * SOL_DEPOSIT_AMOUNT_LAMPORTS;

    let sol_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &sol_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: SOL_RESERVE_COLLATERAL_LAMPORTS,
            liquidity_amount: SOL_RESERVE_COLLATERAL_LAMPORTS,
            liquidity_mint_decimals: 9,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            config: TEST_RESERVE_CONFIG,
            mark_fresh: true,
            ..AddReserveArgs::default()
        },
    );

    // Add a different lending market with a *different* owner
    let other_lending_market_pubkey = Pubkey::new_unique();
    let (other_lending_market_authority, bump_seed) = Pubkey::find_program_address(
        &[other_lending_market_pubkey.as_ref()],
        &spl_token_lending::id(),
    );

    let other_lending_market_owner = Keypair::new();
    let oracle_program_id = read_keypair_file("tests/fixtures/oracle_program_id.json")
        .unwrap()
        .pubkey();

    test.add_packable_account(
        other_lending_market_pubkey,
        u32::MAX as u64,
        &LendingMarket::new(InitLendingMarketParams {
            bump_seed,
            owner: other_lending_market_owner.pubkey(),
            quote_currency: QUOTE_CURRENCY,
            token_program_id: spl_token::id(),
            oracle_program_id,
        }),
        &spl_token_lending::id(),
    );

    let other_lending_market = TestLendingMarket {
        pubkey: other_lending_market_pubkey,
        owner: other_lending_market_owner,
        authority: other_lending_market_authority,
        quote_currency: QUOTE_CURRENCY,
        oracle_program_id,
    };

    let (mut banks_client, payer, recent_blockhash) = test.start().await;
    // Test modify reserve config instruction
    const OPTIMAL_UTILIZATION_RATE_CHANGE: u8 = 10;

    let new_config = ReserveConfig {
        optimal_utilization_rate: TEST_RESERVE_CONFIG.optimal_utilization_rate
            - OPTIMAL_UTILIZATION_RATE_CHANGE,
        loan_to_value_ratio: 50,
        liquidation_bonus: 5,
        liquidation_threshold: 55,
        min_borrow_rate: 0,
        optimal_borrow_rate: 4,
        max_borrow_rate: 30,
        fees: ReserveFees {
            borrow_fee_wad: 100_000_000_000,
            flash_loan_fee_wad: 3_000_000_000_000_000,
            host_fee_percentage: 20,
        },
    };

    let mut transaction = Transaction::new_with_payer(
        &[modify_reserve_config(
            spl_token_lending::id(),
            new_config,
            sol_test_reserve.pubkey,
            lending_market.pubkey,
            other_lending_market.owner.pubkey(),
        )],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &other_lending_market.owner], recent_blockhash);

    let result = banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.unwrap());

    assert_eq!(
        result.unwrap_err(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::InvalidMarketOwner as u32)
        )
    );

    let reserve_info = sol_test_reserve.get_state(&mut banks_client).await;
    assert_eq!(reserve_info.config, TEST_RESERVE_CONFIG);
}

#[tokio::test]
// Right owner, wrong lending market
async fn correct_owner_providing_wrong_lending_market_fails() {
    // When the correct owner of the lending market and reserve provides, perhaps
    // inadvertently, a lending market that is different from the given
    // reserve's corresponding lending market, then the transaction to modify
    // the current reserve config should fail.
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    test.set_compute_max_units(70_000);

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);
    let sol_oracle = add_sol_oracle(&mut test);

    const SOL_DEPOSIT_AMOUNT_LAMPORTS: u64 = 10 * LAMPORTS_TO_SOL * INITIAL_COLLATERAL_RATIO;
    const SOL_RESERVE_COLLATERAL_LAMPORTS: u64 = 2 * SOL_DEPOSIT_AMOUNT_LAMPORTS;

    let sol_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &sol_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: SOL_RESERVE_COLLATERAL_LAMPORTS,
            liquidity_amount: SOL_RESERVE_COLLATERAL_LAMPORTS,
            liquidity_mint_decimals: 9,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            config: TEST_RESERVE_CONFIG,
            mark_fresh: true,
            ..AddReserveArgs::default()
        },
    );

    let other_lending_market = add_lending_market(&mut test);

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    const OPTIMAL_UTILIZATION_RATE_CHANGE: u8 = 10;

    let new_config = ReserveConfig {
        optimal_utilization_rate: TEST_RESERVE_CONFIG.optimal_utilization_rate
            - OPTIMAL_UTILIZATION_RATE_CHANGE,
        loan_to_value_ratio: 50,
        liquidation_bonus: 5,
        liquidation_threshold: 55,
        min_borrow_rate: 0,
        optimal_borrow_rate: 4,
        max_borrow_rate: 30,
        fees: ReserveFees {
            borrow_fee_wad: 100_000_000_000,
            flash_loan_fee_wad: 3_000_000_000_000_000,
            host_fee_percentage: 20,
        },
    };

    let mut transaction = Transaction::new_with_payer(
        &[modify_reserve_config(
            spl_token_lending::id(),
            new_config,
            sol_test_reserve.pubkey,
            other_lending_market.pubkey,
            // lending_market.owner == other_lending_market.owner, defined by
            // `add_lending_market`
            lending_market.owner.pubkey(),
        )],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &lending_market.owner], recent_blockhash);

    let result = banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.unwrap());

    assert_eq!(
        result.unwrap_err(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::InvalidAccountInput as u32)
        )
    );

    let reserve_info = sol_test_reserve.get_state(&mut banks_client).await;
    assert_eq!(reserve_info.config, TEST_RESERVE_CONFIG);
}
