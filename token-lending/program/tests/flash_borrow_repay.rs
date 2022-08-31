#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program::instruction::{
    AccountMeta, Instruction, InstructionError::PrivilegeEscalation,
};
use solana_program::sysvar;
use solana_program_test::*;
use solana_sdk::{
    instruction::InstructionError,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::{Transaction, TransactionError},
};
use solend_program::{
    error::LendingError,
    instruction::{
        flash_borrow_reserve_liquidity, flash_repay_reserve_liquidity, LendingInstruction,
    },
    processor::process_instruction,
};
use spl_token::error::TokenError;
use spl_token::instruction::approve;

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "solend_program",
        solend_program::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_compute_max_units(60_000);

    const FLASH_LOAN_AMOUNT: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const FEE_AMOUNT: u64 = 3_000_000;
    const HOST_FEE_AMOUNT: u64 = 600_000;

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut reserve_config = test_reserve_config();
    reserve_config.fees.host_fee_percentage = 20;
    reserve_config.fees.flash_loan_fee_wad = 3_000_000_000_000_000;

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: FEE_AMOUNT,
            liquidity_amount: FLASH_LOAN_AMOUNT,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;
    let mut transaction = Transaction::new_with_payer(
        &[
            flash_borrow_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
            ),
            flash_repay_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                0,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.config.fee_receiver,
                usdc_test_reserve.liquidity_host_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
                user_accounts_owner.pubkey(),
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);
    assert!(banks_client.process_transaction(transaction).await.is_ok());

    let usdc_reserve = usdc_test_reserve.get_state(&mut banks_client).await;
    assert_eq!(usdc_reserve.liquidity.available_amount, FLASH_LOAN_AMOUNT);
    assert!(usdc_reserve.last_update.stale);

    let liquidity_supply =
        get_token_balance(&mut banks_client, usdc_test_reserve.liquidity_supply_pubkey).await;
    assert_eq!(liquidity_supply, FLASH_LOAN_AMOUNT);

    let token_balance =
        get_token_balance(&mut banks_client, usdc_test_reserve.user_liquidity_pubkey).await;
    assert_eq!(token_balance, 0);

    let fee_balance =
        get_token_balance(&mut banks_client, usdc_test_reserve.config.fee_receiver).await;
    assert_eq!(fee_balance, FEE_AMOUNT - HOST_FEE_AMOUNT);

    let host_fee_balance =
        get_token_balance(&mut banks_client, usdc_test_reserve.liquidity_host_pubkey).await;
    assert_eq!(host_fee_balance, HOST_FEE_AMOUNT);
}

#[tokio::test]
async fn test_fail_disable_flash_loans() {
    let mut test = ProgramTest::new(
        "solend_program",
        solend_program::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_compute_max_units(60_000);

    const FLASH_LOAN_AMOUNT: u64 = 3_000_000;
    const LIQUIDITY_AMOUNT: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const FEE_AMOUNT: u64 = 3_000_000;

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut reserve_config = test_reserve_config();
    reserve_config.fees.flash_loan_fee_wad = u64::MAX; // disabled

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: FEE_AMOUNT,
            liquidity_amount: LIQUIDITY_AMOUNT,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;
    let mut transaction = Transaction::new_with_payer(
        &[
            flash_borrow_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
            ),
            flash_repay_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                0,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.config.fee_receiver,
                usdc_test_reserve.liquidity_host_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
                user_accounts_owner.pubkey(),
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);

    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::FlashLoansDisabled as u32)
        )
    );
}

#[tokio::test]
async fn test_fail_borrow_over_borrow_limit() {
    let mut test = ProgramTest::new(
        "solend_program",
        solend_program::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_compute_max_units(60_000);

    const FLASH_LOAN_AMOUNT: u64 = 3_000_000;
    const LIQUIDITY_AMOUNT: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const FEE_AMOUNT: u64 = 3_000_000;

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut reserve_config = test_reserve_config();
    reserve_config.borrow_limit = 2_000_000;

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: FEE_AMOUNT,
            liquidity_amount: LIQUIDITY_AMOUNT,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;
    let mut transaction = Transaction::new_with_payer(
        &[
            flash_borrow_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
            ),
            flash_repay_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                0,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.config.fee_receiver,
                usdc_test_reserve.liquidity_host_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
                user_accounts_owner.pubkey(),
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);

    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::InvalidAmount as u32)
        )
    );
}

#[tokio::test]
async fn test_fail_double_borrow() {
    let mut test = ProgramTest::new(
        "solend_program",
        solend_program::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_compute_max_units(60_000);

    const FLASH_LOAN_AMOUNT: u64 = 3_000_000;
    const LIQUIDITY_AMOUNT: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const FEE_AMOUNT: u64 = 3_000_000;

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut reserve_config = test_reserve_config();
    reserve_config.fees.host_fee_percentage = 20;
    reserve_config.fees.flash_loan_fee_wad = 3_000_000_000_000_000;

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: FEE_AMOUNT,
            liquidity_amount: LIQUIDITY_AMOUNT,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;
    let mut transaction = Transaction::new_with_payer(
        &[
            flash_borrow_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
            ),
            flash_borrow_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
            ),
            flash_repay_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                0,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.config.fee_receiver,
                usdc_test_reserve.liquidity_host_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
                user_accounts_owner.pubkey(),
            ),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);

    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::MultipleFlashBorrows as u32)
        )
    );
}

/// idk why anyone would do this but w/e
#[tokio::test]
async fn test_fail_double_repay() {
    let mut test = ProgramTest::new(
        "solend_program",
        solend_program::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_compute_max_units(60_000);

    const FLASH_LOAN_AMOUNT: u64 = 3_000_000;
    const LIQUIDITY_AMOUNT: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const FEE_AMOUNT: u64 = 3_000_000;

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut reserve_config = test_reserve_config();
    reserve_config.fees.host_fee_percentage = 20;
    reserve_config.fees.flash_loan_fee_wad = 3_000_000_000_000_000;

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: FEE_AMOUNT,
            liquidity_amount: LIQUIDITY_AMOUNT,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;
    let mut transaction = Transaction::new_with_payer(
        &[
            flash_borrow_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
            ),
            flash_repay_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                0,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.config.fee_receiver,
                usdc_test_reserve.liquidity_host_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
                user_accounts_owner.pubkey(),
            ),
            flash_repay_reserve_liquidity(
                solend_program::id(),
                0,
                0,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.config.fee_receiver,
                usdc_test_reserve.liquidity_host_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
                user_accounts_owner.pubkey(),
            ),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);

    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::MultipleFlashBorrows as u32)
        )
    );
}

#[tokio::test]
async fn test_fail_only_one_flash_ix_pair_per_tx() {
    let mut test = ProgramTest::new(
        "solend_program",
        solend_program::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_compute_max_units(60_000);

    const FLASH_LOAN_AMOUNT: u64 = 3_000_000;
    const LIQUIDITY_AMOUNT: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const FEE_AMOUNT: u64 = 3_000_000;

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut reserve_config = test_reserve_config();
    reserve_config.fees.host_fee_percentage = 20;
    reserve_config.fees.flash_loan_fee_wad = 3_000_000_000_000_000;

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: FEE_AMOUNT,
            liquidity_amount: LIQUIDITY_AMOUNT,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    // eventually this will be valid. but for v1 implementation, we only let 1 flash ix pair per tx
    let (mut banks_client, payer, recent_blockhash) = test.start().await;
    let mut transaction = Transaction::new_with_payer(
        &[
            flash_borrow_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
            ),
            flash_repay_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                0,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.config.fee_receiver,
                usdc_test_reserve.liquidity_host_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
                user_accounts_owner.pubkey(),
            ),
            flash_borrow_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
            ),
            flash_repay_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                2,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.config.fee_receiver,
                usdc_test_reserve.liquidity_host_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
                user_accounts_owner.pubkey(),
            ),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);

    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::MultipleFlashBorrows as u32)
        )
    );
}

#[tokio::test]
async fn test_fail_invalid_repay_ix() {
    let mut test = ProgramTest::new(
        "solend_program",
        solend_program::id(),
        processor!(process_instruction),
    );

    let proxy_program_id = Pubkey::new_unique();
    test.prefer_bpf(false);
    test.add_program(
        "flash_loan_proxy",
        proxy_program_id,
        processor!(helpers::flash_loan_proxy::process_instruction),
    );

    const FLASH_LOAN_AMOUNT: u64 = 3_000_000;
    const LIQUIDITY_AMOUNT: u64 = 1_000_000 * FRACTIONAL_TO_USDC;
    const FEE_AMOUNT: u64 = 3_000_000;

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut reserve_config = test_reserve_config();
    reserve_config.fees.host_fee_percentage = 20;
    reserve_config.fees.flash_loan_fee_wad = 3_000_000_000_000_000;

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: FEE_AMOUNT,
            liquidity_amount: LIQUIDITY_AMOUNT,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    // case 1: invalid reserve in repay
    {
        let mut transaction = Transaction::new_with_payer(
            &[
                flash_borrow_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    usdc_test_reserve.liquidity_supply_pubkey,
                    usdc_test_reserve.user_liquidity_pubkey,
                    usdc_test_reserve.pubkey,
                    lending_market.pubkey,
                ),
                flash_repay_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    0,
                    usdc_test_reserve.user_liquidity_pubkey,
                    usdc_test_reserve.liquidity_supply_pubkey,
                    usdc_test_reserve.config.fee_receiver,
                    usdc_test_reserve.liquidity_host_pubkey,
                    Pubkey::new_unique(),
                    lending_market.pubkey,
                    user_accounts_owner.pubkey(),
                ),
            ],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);

        assert_eq!(
            banks_client
                .process_transaction(transaction)
                .await
                .unwrap_err()
                .unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(LendingError::InvalidFlashRepay as u32)
            )
        );
    }

    // case 2: invalid liquidity amount
    {
        let mut transaction = Transaction::new_with_payer(
            &[
                flash_borrow_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    usdc_test_reserve.liquidity_supply_pubkey,
                    usdc_test_reserve.user_liquidity_pubkey,
                    usdc_test_reserve.pubkey,
                    lending_market.pubkey,
                ),
                flash_repay_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT - 1,
                    0,
                    usdc_test_reserve.user_liquidity_pubkey,
                    usdc_test_reserve.liquidity_supply_pubkey,
                    usdc_test_reserve.config.fee_receiver,
                    usdc_test_reserve.liquidity_host_pubkey,
                    usdc_test_reserve.pubkey,
                    lending_market.pubkey,
                    user_accounts_owner.pubkey(),
                ),
            ],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);

        assert_eq!(
            banks_client
                .process_transaction(transaction)
                .await
                .unwrap_err()
                .unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(LendingError::InvalidFlashRepay as u32)
            )
        );
    }

    // case 3: no repay
    {
        let mut transaction = Transaction::new_with_payer(
            &[flash_borrow_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
            )],
            Some(&payer.pubkey()),
        );

        transaction.sign(&[&payer], recent_blockhash);

        assert_eq!(
            banks_client
                .process_transaction(transaction)
                .await
                .unwrap_err()
                .unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(LendingError::NoFlashRepayFound as u32)
            )
        );
    }

    // case 4: cpi repay
    {
        let mut transaction = Transaction::new_with_payer(
            &[
                flash_borrow_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    usdc_test_reserve.liquidity_supply_pubkey,
                    usdc_test_reserve.user_liquidity_pubkey,
                    usdc_test_reserve.pubkey,
                    lending_market.pubkey,
                ),
                helpers::flash_loan_proxy::repay_proxy(
                    proxy_program_id,
                    FLASH_LOAN_AMOUNT,
                    0,
                    usdc_test_reserve.user_liquidity_pubkey,
                    usdc_test_reserve.liquidity_supply_pubkey,
                    usdc_test_reserve.config.fee_receiver,
                    usdc_test_reserve.liquidity_host_pubkey,
                    usdc_test_reserve.pubkey,
                    solend_program::id(),
                    lending_market.pubkey,
                    user_accounts_owner.pubkey(),
                ),
            ],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);

        assert_eq!(
            banks_client
                .process_transaction(transaction)
                .await
                .unwrap_err()
                .unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(LendingError::NoFlashRepayFound as u32)
            )
        );
    }

    // case 5: insufficient funds to pay fees on repay. FEE_AMOUNT was calculated using
    // FLASH_LOAN_AMOUNT, not LIQUIDITY_AMOUNT.
    {
        let mut transaction = Transaction::new_with_payer(
            &[
                flash_borrow_reserve_liquidity(
                    solend_program::id(),
                    LIQUIDITY_AMOUNT,
                    usdc_test_reserve.liquidity_supply_pubkey,
                    usdc_test_reserve.user_liquidity_pubkey,
                    usdc_test_reserve.pubkey,
                    lending_market.pubkey,
                ),
                flash_repay_reserve_liquidity(
                    solend_program::id(),
                    LIQUIDITY_AMOUNT,
                    0,
                    usdc_test_reserve.user_liquidity_pubkey,
                    usdc_test_reserve.liquidity_supply_pubkey,
                    usdc_test_reserve.config.fee_receiver,
                    usdc_test_reserve.liquidity_host_pubkey,
                    usdc_test_reserve.pubkey,
                    lending_market.pubkey,
                    user_accounts_owner.pubkey(),
                ),
            ],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);

        let res = banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap();

        // weird glitch. depending on cargo version the error type is different. idek.
        assert!(
            res == TransactionError::InstructionError(
                1,
                InstructionError::Custom(TokenError::InsufficientFunds as u32)
            ) || res
                == TransactionError::InstructionError(
                    1,
                    InstructionError::Custom(LendingError::TokenTransferFailed as u32)
                )
        );
    }

    // case 6: Sole repay instruction
    {
        let mut transaction = Transaction::new_with_payer(
            &[flash_repay_reserve_liquidity(
                solend_program::id(),
                LIQUIDITY_AMOUNT,
                0,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.config.fee_receiver,
                usdc_test_reserve.liquidity_host_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
                user_accounts_owner.pubkey(),
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);

        assert_eq!(
            banks_client
                .process_transaction(transaction)
                .await
                .unwrap_err()
                .unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(LendingError::InvalidFlashRepay as u32)
            )
        );
    }

    // case 7: Incorrect borrow instruction index -- points to itself
    {
        let mut transaction = Transaction::new_with_payer(
            &[
                flash_borrow_reserve_liquidity(
                    solend_program::id(),
                    LIQUIDITY_AMOUNT,
                    usdc_test_reserve.liquidity_supply_pubkey,
                    usdc_test_reserve.user_liquidity_pubkey,
                    usdc_test_reserve.pubkey,
                    lending_market.pubkey,
                ),
                flash_repay_reserve_liquidity(
                    solend_program::id(),
                    LIQUIDITY_AMOUNT,
                    1, // should be zero
                    usdc_test_reserve.user_liquidity_pubkey,
                    usdc_test_reserve.liquidity_supply_pubkey,
                    usdc_test_reserve.config.fee_receiver,
                    usdc_test_reserve.liquidity_host_pubkey,
                    usdc_test_reserve.pubkey,
                    lending_market.pubkey,
                    user_accounts_owner.pubkey(),
                ),
            ],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);

        assert_eq!(
            banks_client
                .process_transaction(transaction)
                .await
                .unwrap_err()
                .unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(LendingError::InvalidFlashRepay as u32)
            )
        );
    }

    // case 8: Incorrect borrow instruction index -- points to some other program
    {
        let user_transfer_authority = Keypair::new();
        let mut transaction = Transaction::new_with_payer(
            &[
                approve(
                    &spl_token::id(),
                    &usdc_test_reserve.user_liquidity_pubkey,
                    &user_transfer_authority.pubkey(),
                    &user_accounts_owner.pubkey(),
                    &[],
                    1,
                )
                .unwrap(),
                flash_repay_reserve_liquidity(
                    solend_program::id(),
                    LIQUIDITY_AMOUNT,
                    0, // should be zero
                    usdc_test_reserve.user_liquidity_pubkey,
                    usdc_test_reserve.liquidity_supply_pubkey,
                    usdc_test_reserve.config.fee_receiver,
                    usdc_test_reserve.liquidity_host_pubkey,
                    usdc_test_reserve.pubkey,
                    lending_market.pubkey,
                    user_accounts_owner.pubkey(),
                ),
            ],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);

        assert_eq!(
            banks_client
                .process_transaction(transaction)
                .await
                .unwrap_err()
                .unwrap(),
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(LendingError::InvalidFlashRepay as u32)
            )
        );
    }
    // case 9: Incorrect borrow instruction index -- points to a later borrow
    {
        let mut transaction = Transaction::new_with_payer(
            &[
                flash_repay_reserve_liquidity(
                    solend_program::id(),
                    FRACTIONAL_TO_USDC,
                    1,
                    usdc_test_reserve.user_liquidity_pubkey,
                    usdc_test_reserve.liquidity_supply_pubkey,
                    usdc_test_reserve.config.fee_receiver,
                    usdc_test_reserve.liquidity_host_pubkey,
                    usdc_test_reserve.pubkey,
                    lending_market.pubkey,
                    user_accounts_owner.pubkey(),
                ),
                flash_borrow_reserve_liquidity(
                    solend_program::id(),
                    FEE_AMOUNT,
                    usdc_test_reserve.liquidity_supply_pubkey,
                    usdc_test_reserve.user_liquidity_pubkey,
                    usdc_test_reserve.pubkey,
                    lending_market.pubkey,
                ),
                flash_repay_reserve_liquidity(
                    solend_program::id(),
                    FEE_AMOUNT,
                    1,
                    usdc_test_reserve.user_liquidity_pubkey,
                    usdc_test_reserve.liquidity_supply_pubkey,
                    usdc_test_reserve.config.fee_receiver,
                    usdc_test_reserve.liquidity_host_pubkey,
                    usdc_test_reserve.pubkey,
                    lending_market.pubkey,
                    user_accounts_owner.pubkey(),
                ),
            ],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);

        assert_eq!(
            banks_client
                .process_transaction(transaction)
                .await
                .unwrap_err()
                .unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(LendingError::InvalidFlashRepay as u32)
            )
        );
    }
}

#[tokio::test]
async fn test_fail_insufficient_liquidity_for_borrow() {
    let mut test = ProgramTest::new(
        "solend_program",
        solend_program::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_compute_max_units(60_000);

    const LIQUIDITY_AMOUNT: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const FEE_AMOUNT: u64 = 3_000_000;

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut reserve_config = test_reserve_config();
    reserve_config.fees.host_fee_percentage = 20;
    reserve_config.fees.flash_loan_fee_wad = 3_000_000_000_000_000;

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: FEE_AMOUNT,
            liquidity_amount: LIQUIDITY_AMOUNT,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;
    let mut transaction = Transaction::new_with_payer(
        &[
            flash_borrow_reserve_liquidity(
                solend_program::id(),
                LIQUIDITY_AMOUNT + 1,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
            ),
            flash_repay_reserve_liquidity(
                solend_program::id(),
                LIQUIDITY_AMOUNT + 1,
                0,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.config.fee_receiver,
                usdc_test_reserve.liquidity_host_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
                user_accounts_owner.pubkey(),
            ),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);

    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::InsufficientLiquidity as u32)
        )
    );
}

#[tokio::test]
async fn test_fail_cpi_borrow() {
    let mut test = ProgramTest::new(
        "solend_program",
        solend_program::id(),
        processor!(process_instruction),
    );

    let proxy_program_id = Pubkey::new_unique();
    test.prefer_bpf(false);
    test.add_program(
        "flash_loan_proxy",
        proxy_program_id,
        processor!(helpers::flash_loan_proxy::process_instruction),
    );

    // limit to track compute unit increase
    test.set_compute_max_units(60_000);

    const FLASH_LOAN_AMOUNT: u64 = 3_000_000;
    const LIQUIDITY_AMOUNT: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const FEE_AMOUNT: u64 = 3_000_000;

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut reserve_config = test_reserve_config();
    reserve_config.fees.host_fee_percentage = 20;
    reserve_config.fees.flash_loan_fee_wad = 3_000_000_000_000_000;

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: FEE_AMOUNT,
            liquidity_amount: LIQUIDITY_AMOUNT,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;
    let mut transaction = Transaction::new_with_payer(
        &[helpers::flash_loan_proxy::borrow_proxy(
            proxy_program_id,
            FLASH_LOAN_AMOUNT,
            usdc_test_reserve.liquidity_supply_pubkey,
            usdc_test_reserve.user_liquidity_pubkey,
            usdc_test_reserve.pubkey,
            solend_program::id(),
            lending_market.pubkey,
            lending_market.authority,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);

    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::FlashBorrowCpi as u32)
        )
    );
}

#[tokio::test]
async fn test_fail_cpi_repay() {
    let mut test = ProgramTest::new(
        "solend_program",
        solend_program::id(),
        processor!(process_instruction),
    );

    let proxy_program_id = Pubkey::new_unique();
    test.prefer_bpf(false);
    test.add_program(
        "flash_loan_proxy",
        proxy_program_id,
        processor!(helpers::flash_loan_proxy::process_instruction),
    );

    // limit to track compute unit increase
    test.set_compute_max_units(60_000);

    const FLASH_LOAN_AMOUNT: u64 = 3_000_000;
    const LIQUIDITY_AMOUNT: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const FEE_AMOUNT: u64 = 3_000_000;

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut reserve_config = test_reserve_config();
    reserve_config.fees.host_fee_percentage = 20;
    reserve_config.fees.flash_loan_fee_wad = 3_000_000_000_000_000;

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: FEE_AMOUNT,
            liquidity_amount: LIQUIDITY_AMOUNT,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;
    let mut transaction = Transaction::new_with_payer(
        &[helpers::flash_loan_proxy::repay_proxy(
            proxy_program_id,
            FLASH_LOAN_AMOUNT,
            0,
            usdc_test_reserve.user_liquidity_pubkey,
            usdc_test_reserve.liquidity_supply_pubkey,
            usdc_test_reserve.config.fee_receiver,
            usdc_test_reserve.liquidity_host_pubkey,
            usdc_test_reserve.pubkey,
            solend_program::id(),
            lending_market.pubkey,
            user_accounts_owner.pubkey(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &user_accounts_owner], recent_blockhash);

    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::FlashRepayCpi as u32)
        )
    );
}

#[tokio::test]
async fn test_fail_repay_from_diff_reserve() {
    let mut test = ProgramTest::new(
        "solend_program",
        solend_program::id(),
        processor!(process_instruction),
    );

    // limit to track compute unit increase
    test.set_compute_max_units(61_000);

    const FLASH_LOAN_AMOUNT: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const FEE_AMOUNT: u64 = 3_000_000;

    let user_accounts_owner = Keypair::new();
    let lending_market = add_lending_market(&mut test);

    let mut reserve_config = test_reserve_config();
    reserve_config.fees.host_fee_percentage = 20;
    reserve_config.fees.flash_loan_fee_wad = 3_000_000_000_000_000;

    let usdc_mint = add_usdc_mint(&mut test);
    let usdc_oracle = add_usdc_oracle(&mut test);
    let usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: FEE_AMOUNT,
            liquidity_amount: FLASH_LOAN_AMOUNT,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );
    let another_usdc_test_reserve = add_reserve(
        &mut test,
        &lending_market,
        &usdc_oracle,
        &user_accounts_owner,
        AddReserveArgs {
            user_liquidity_amount: FEE_AMOUNT,
            liquidity_amount: FLASH_LOAN_AMOUNT,
            liquidity_mint_pubkey: usdc_mint.pubkey,
            liquidity_mint_decimals: usdc_mint.decimals,
            config: reserve_config,
            ..AddReserveArgs::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    // this transaction fails because the repay token transfers aren't signed by the
    // lending_market_authority PDA.
    let mut transaction = Transaction::new_with_payer(
        &[
            flash_borrow_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.user_liquidity_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
            ),
            malicious_flash_repay_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                0,
                another_usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.liquidity_supply_pubkey,
                usdc_test_reserve.config.fee_receiver,
                usdc_test_reserve.liquidity_host_pubkey,
                usdc_test_reserve.pubkey,
                lending_market.pubkey,
                lending_market.authority,
            ),
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer], recent_blockhash);
    let err = banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        err,
        TransactionError::InstructionError(1, PrivilegeEscalation)
    );
}

// don't explicitly check user_transfer_authority signer
#[allow(clippy::too_many_arguments)]
pub fn malicious_flash_repay_reserve_liquidity(
    program_id: Pubkey,
    liquidity_amount: u64,
    borrow_instruction_index: u8,
    source_liquidity_pubkey: Pubkey,
    destination_liquidity_pubkey: Pubkey,
    reserve_liquidity_fee_receiver_pubkey: Pubkey,
    host_fee_receiver_pubkey: Pubkey,
    reserve_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    user_transfer_authority_pubkey: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(source_liquidity_pubkey, false),
            AccountMeta::new(destination_liquidity_pubkey, false),
            AccountMeta::new(reserve_liquidity_fee_receiver_pubkey, false),
            AccountMeta::new(host_fee_receiver_pubkey, false),
            AccountMeta::new(reserve_pubkey, false),
            AccountMeta::new_readonly(lending_market_pubkey, false),
            AccountMeta::new_readonly(user_transfer_authority_pubkey, false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::FlashRepayReserveLiquidity {
            liquidity_amount,
            borrow_instruction_index,
        }
        .pack(),
    }
}
