#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_sdk::signature::Keypair;
use spl_token_lending::{
    instruction::BorrowAmountType,
    state::{INITIAL_COLLATERAL_RATIO, PROGRAM_VERSION},
};

#[tokio::test]
async fn test_success() {
    let (mut test, lending) = setup_test();

    let LendingTest {
        sol_usdc_dex_market,
        srm_usdc_dex_market,
        usdc_mint,
        srm_mint,
    } = lending;

    // Initialize Lending Market
    let lending_market = add_lending_market(&mut test, usdc_mint.pubkey);

    const LAMPORTS_TO_SOL: u64 = 1_000_000_000;
    const FRACTIONAL_TO_USDC: u64 = 1_000_000;
    const FRACTIONAL_TO_SRM: u64 = 1_000_000;

    // Market and collateral are setup to fill two orders in the dex market at an average
    // price of 2210.5
    const fn lamports_to_usdc_fractional(lamports: u64) -> u64 {
        lamports / LAMPORTS_TO_SOL * (2210 + 2211) / 2 * FRACTIONAL_TO_USDC / 1000
    };

    const USER_SOL_DEPOSIT_LAMPORTS: u64 = 10_000 * LAMPORTS_TO_SOL;
    const USER_SOL_COLLATERAL_LAMPORTS: u64 = 8_500 * LAMPORTS_TO_SOL;
    const INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS: u64 = 32_500 * LAMPORTS_TO_SOL;
    const TOTAL_SOL: u64 = USER_SOL_DEPOSIT_LAMPORTS + INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS;
    const INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL: u64 = lamports_to_usdc_fractional(TOTAL_SOL);
    const INITIAL_SRM_RESERVE_SUPPLY_FRACTIONAL: u64 = 20_000 * FRACTIONAL_TO_SRM;

    let user_accounts_owner = Keypair::new();

    let usdc_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            name: "usdc".to_owned(),
            dex_market_pubkey: None,
            liquidity_amount: INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL,
            liquidity_mint_decimals: usdc_mint.decimals,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            user_liquidity_amount: USER_SOL_DEPOSIT_LAMPORTS,
            config: TEST_RESERVE_CONFIG,
            ..AddReserveArgs::default()
        },
    );

    let sol_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            name: "sol".to_owned(),
            dex_market_pubkey: Some(sol_usdc_dex_market.pubkey),
            liquidity_amount: INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS,
            liquidity_mint_pubkey: spl_token::native_mint::id(),
            liquidity_mint_decimals: 9,
            user_liquidity_amount: USER_SOL_DEPOSIT_LAMPORTS,
            config: TEST_RESERVE_CONFIG,
            ..AddReserveArgs::default()
        },
    );

    let srm_reserve = add_reserve(
        &mut test,
        &user_accounts_owner,
        &lending_market,
        AddReserveArgs {
            name: "srm".to_owned(),
            dex_market_pubkey: Some(srm_usdc_dex_market.pubkey),
            liquidity_amount: INITIAL_SRM_RESERVE_SUPPLY_FRACTIONAL,
            liquidity_mint_decimals: srm_mint.decimals,
            liquidity_mint_pubkey: srm_mint.pubkey,
            config: TEST_RESERVE_CONFIG,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, _recent_blockhash) = test.start().await;

    // Verify lending market
    let lending_market_info = lending_market.get_state(&mut banks_client).await;
    assert_eq!(lending_market_info.version, PROGRAM_VERSION);
    assert_eq!(lending_market_info.quote_token_mint, usdc_mint.pubkey);

    // Verify reserves
    usdc_reserve.validate_state(&mut banks_client).await;
    sol_reserve.validate_state(&mut banks_client).await;
    srm_reserve.validate_state(&mut banks_client).await;

    let usdc_liquidity_supply =
        get_token_balance(&mut banks_client, usdc_reserve.liquidity_supply).await;
    assert_eq!(
        usdc_liquidity_supply,
        INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL
    );
    let user_usdc_collateral_balance =
        get_token_balance(&mut banks_client, usdc_reserve.user_collateral_account).await;
    assert_eq!(
        user_usdc_collateral_balance,
        INITIAL_COLLATERAL_RATIO * INITIAL_USDC_RESERVE_SUPPLY_FRACTIONAL
    );

    let sol_liquidity_supply =
        get_token_balance(&mut banks_client, sol_reserve.liquidity_supply).await;
    assert_eq!(sol_liquidity_supply, INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS);
    let user_sol_balance =
        get_token_balance(&mut banks_client, sol_reserve.user_liquidity_account).await;
    assert_eq!(user_sol_balance, USER_SOL_DEPOSIT_LAMPORTS);
    let user_sol_collateral_balance =
        get_token_balance(&mut banks_client, sol_reserve.user_collateral_account).await;
    assert_eq!(
        user_sol_collateral_balance,
        INITIAL_COLLATERAL_RATIO * INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS
    );

    // Deposit SOL
    lending_market
        .deposit(
            &mut banks_client,
            &user_accounts_owner,
            &payer,
            &sol_reserve,
            USER_SOL_DEPOSIT_LAMPORTS,
        )
        .await;

    // Verify deposit
    let sol_liquidity_supply =
        get_token_balance(&mut banks_client, sol_reserve.liquidity_supply).await;
    assert_eq!(
        sol_liquidity_supply,
        INITIAL_SOL_RESERVE_SUPPLY_LAMPORTS + USER_SOL_DEPOSIT_LAMPORTS
    );
    let user_sol_balance =
        get_token_balance(&mut banks_client, sol_reserve.user_liquidity_account).await;
    assert_eq!(user_sol_balance, 0);
    let user_sol_collateral_balance =
        get_token_balance(&mut banks_client, sol_reserve.user_collateral_account).await;
    assert_eq!(
        user_sol_collateral_balance,
        INITIAL_COLLATERAL_RATIO * TOTAL_SOL
    );

    // Borrow USDC with SOL collateral
    let obligation = lending_market
        .borrow(
            &mut banks_client,
            &payer,
            BorrowArgs {
                deposit_reserve: &sol_reserve,
                borrow_reserve: &usdc_reserve,
                dex_market: &sol_usdc_dex_market,
                borrow_amount_type: BorrowAmountType::CollateralDepositAmount,
                amount: INITIAL_COLLATERAL_RATIO * USER_SOL_COLLATERAL_LAMPORTS,
                user_accounts_owner: &user_accounts_owner,
                obligation: None,
            },
        )
        .await;

    // Borrow more USDC using existing obligation account
    lending_market
        .borrow(
            &mut banks_client,
            &payer,
            BorrowArgs {
                deposit_reserve: &sol_reserve,
                borrow_reserve: &usdc_reserve,
                dex_market: &sol_usdc_dex_market,
                borrow_amount_type: BorrowAmountType::CollateralDepositAmount,
                amount: lamports_to_usdc_fractional(
                    usdc_reserve.config.loan_to_value_ratio as u64 * USER_SOL_COLLATERAL_LAMPORTS
                        / 100,
                ),
                user_accounts_owner: &user_accounts_owner,
                obligation: Some(obligation),
            },
        )
        .await;

    // Deposit USDC
    lending_market
        .deposit(
            &mut banks_client,
            &user_accounts_owner,
            &payer,
            &usdc_reserve,
            2 * INITIAL_COLLATERAL_RATIO
                * lamports_to_usdc_fractional(
                    usdc_reserve.config.loan_to_value_ratio as u64 * USER_SOL_COLLATERAL_LAMPORTS
                        / 100,
                ),
        )
        .await;

    // Borrow SOL with USDC collateral
    lending_market
        .borrow(
            &mut banks_client,
            &payer,
            BorrowArgs {
                deposit_reserve: &usdc_reserve,
                borrow_reserve: &sol_reserve,
                dex_market: &sol_usdc_dex_market,
                borrow_amount_type: BorrowAmountType::CollateralDepositAmount,
                amount: INITIAL_COLLATERAL_RATIO
                    * lamports_to_usdc_fractional(
                        usdc_reserve.config.loan_to_value_ratio as u64
                            * USER_SOL_COLLATERAL_LAMPORTS
                            / 100,
                    ),
                user_accounts_owner: &user_accounts_owner,
                obligation: None,
            },
        )
        .await;

    // Borrow SRM with USDC collateral
    lending_market
        .borrow(
            &mut banks_client,
            &payer,
            BorrowArgs {
                deposit_reserve: &usdc_reserve,
                borrow_reserve: &srm_reserve,
                dex_market: &srm_usdc_dex_market,
                borrow_amount_type: BorrowAmountType::CollateralDepositAmount,
                amount: INITIAL_COLLATERAL_RATIO
                    * lamports_to_usdc_fractional(
                        usdc_reserve.config.loan_to_value_ratio as u64
                            * USER_SOL_COLLATERAL_LAMPORTS
                            / 100,
                    ),
                user_accounts_owner: &user_accounts_owner,
                obligation: None,
            },
        )
        .await;

    // Only dump the accounts if the feature is specified
    #[cfg(feature = "test-dump-genesis-accounts")]
    {
        use helpers::genesis::GenesisAccounts;
        let mut genesis_accounts = GenesisAccounts::default();
        lending_market
            .add_to_genesis(&mut banks_client, &mut genesis_accounts)
            .await;
        sol_reserve
            .add_to_genesis(&mut banks_client, &mut genesis_accounts)
            .await;
        srm_reserve
            .add_to_genesis(&mut banks_client, &mut genesis_accounts)
            .await;
        usdc_reserve
            .add_to_genesis(&mut banks_client, &mut genesis_accounts)
            .await;
        sol_usdc_dex_market
            .add_to_genesis(&mut banks_client, &mut genesis_accounts)
            .await;
        srm_usdc_dex_market
            .add_to_genesis(&mut banks_client, &mut genesis_accounts)
            .await;
        genesis_accounts
            .insert_upgradeable_program(spl_token_lending::id(), "spl_token_lending.so");
        genesis_accounts.write_yaml();
    }
}
