#![cfg(feature = "test-bpf")]

mod helpers;

use std::collections::HashSet;

use helpers::*;

use flash_loan_proxy::proxy_program;
use helpers::solend_program_test::{
    setup_world, BalanceChecker, Info, SolendProgramTest, TokenBalanceChange, User,
};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::sysvar;
use solana_program_test::*;
use solana_sdk::{
    instruction::InstructionError,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::TransactionError,
};
use solend_program::instruction::LendingInstruction;
use solend_program::state::LastUpdate;
use solend_program::{
    error::LendingError,
    instruction::{flash_borrow_reserve_liquidity, flash_repay_reserve_liquidity},
    state::{LendingMarket, Reserve, ReserveConfig, ReserveFees},
};
use spl_token::error::TokenError;
use spl_token::instruction::approve;

async fn setup(
    usdc_reserve_config: &ReserveConfig,
) -> (
    SolendProgramTest,
    Info<LendingMarket>,
    Info<Reserve>,
    User,
    User,
    User,
) {
    let (mut test, lending_market, usdc_reserve, _, lending_market_owner, user) =
        setup_world(usdc_reserve_config, &test_reserve_config()).await;

    // deposit 100k USDC
    lending_market
        .deposit(&mut test, &usdc_reserve, &user, 100_000_000_000)
        .await
        .expect("This should succeed");

    let usdc_reserve = test.load_account(usdc_reserve.pubkey).await;

    let host_fee_receiver = User::new_with_balances(&mut test, &[(&usdc_mint::id(), 0)]).await;

    (
        test,
        lending_market,
        usdc_reserve,
        user,
        host_fee_receiver,
        lending_market_owner,
    )
}

#[tokio::test]
async fn test_success() {
    let (mut test, lending_market, usdc_reserve, user, host_fee_receiver, _) =
        setup(&ReserveConfig {
            deposit_limit: u64::MAX,
            fees: ReserveFees {
                borrow_fee_wad: 100_000_000_000,
                host_fee_percentage: 20,
                flash_loan_fee_wad: 3_000_000_000_000_000,
            },
            ..test_reserve_config()
        })
        .await;

    let balance_checker =
        BalanceChecker::start(&mut test, &[&usdc_reserve, &user, &host_fee_receiver]).await;

    const FLASH_LOAN_AMOUNT: u64 = 1_000 * FRACTIONAL_TO_USDC;
    const FEE_AMOUNT: u64 = 3_000_000;
    const HOST_FEE_AMOUNT: u64 = 600_000;
    test.process_transaction(
        &[
            flash_borrow_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                usdc_reserve.account.liquidity.supply_pubkey,
                user.get_account(&usdc_mint::id()).unwrap(),
                usdc_reserve.pubkey,
                lending_market.pubkey,
            ),
            flash_repay_reserve_liquidity(
                solend_program::id(),
                FLASH_LOAN_AMOUNT,
                0,
                user.get_account(&usdc_mint::id()).unwrap(),
                usdc_reserve.account.liquidity.supply_pubkey,
                usdc_reserve.account.config.fee_receiver,
                host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                usdc_reserve.pubkey,
                lending_market.pubkey,
                user.keypair.pubkey(),
            ),
        ],
        Some(&[&user.keypair]),
    )
    .await
    .unwrap();

    // check balance changes
    let (balance_changes, mint_supply_changes) =
        balance_checker.find_balance_changes(&mut test).await;
    let expected_balance_changes = HashSet::from([
        TokenBalanceChange {
            token_account: user.get_account(&usdc_mint::id()).unwrap(),
            mint: usdc_mint::id(),
            diff: -(FEE_AMOUNT as i128),
        },
        TokenBalanceChange {
            token_account: usdc_reserve.account.config.fee_receiver,
            mint: usdc_mint::id(),
            diff: (FEE_AMOUNT - HOST_FEE_AMOUNT) as i128,
        },
        TokenBalanceChange {
            token_account: host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
            mint: usdc_mint::id(),
            diff: HOST_FEE_AMOUNT as i128,
        },
    ]);
    assert_eq!(balance_changes, expected_balance_changes);
    assert_eq!(mint_supply_changes, HashSet::new());

    // check program state changes
    let lending_market_post = test
        .load_account::<LendingMarket>(lending_market.pubkey)
        .await;
    assert_eq!(lending_market, lending_market_post);

    let usdc_reserve_post = test.load_account::<Reserve>(usdc_reserve.pubkey).await;
    assert_eq!(
        usdc_reserve_post.account,
        Reserve {
            last_update: LastUpdate {
                slot: 1000,
                stale: true
            },
            ..usdc_reserve.account
        }
    );
}

#[tokio::test]
async fn test_fail_disable_flash_loans() {
    let (mut test, lending_market, usdc_reserve, user, host_fee_receiver, _) =
        setup(&ReserveConfig {
            deposit_limit: u64::MAX,
            fees: ReserveFees {
                borrow_fee_wad: 1,
                host_fee_percentage: 20,
                flash_loan_fee_wad: u64::MAX,
            },
            ..test_reserve_config()
        })
        .await;

    const FLASH_LOAN_AMOUNT: u64 = 3_000_000;
    let res = test
        .process_transaction(
            &[
                flash_borrow_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    usdc_reserve.account.liquidity.supply_pubkey,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                ),
                flash_repay_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    0,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.account.liquidity.supply_pubkey,
                    usdc_reserve.account.config.fee_receiver,
                    host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                    user.keypair.pubkey(),
                ),
            ],
            Some(&[&user.keypair]),
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        res,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::FlashLoansDisabled as u32)
        )
    );
}

#[tokio::test]
async fn test_fail_borrow_over_borrow_limit() {
    let (mut test, lending_market, usdc_reserve, user, host_fee_receiver, _) =
        setup(&ReserveConfig {
            deposit_limit: u64::MAX,
            borrow_limit: 2_000_000,
            fees: ReserveFees {
                borrow_fee_wad: 1,
                host_fee_percentage: 20,
                flash_loan_fee_wad: 1,
            },
            ..test_reserve_config()
        })
        .await;

    const FLASH_LOAN_AMOUNT: u64 = 3_000_000;
    let res = test
        .process_transaction(
            &[
                flash_borrow_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    usdc_reserve.account.liquidity.supply_pubkey,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                ),
                flash_repay_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    0,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.account.liquidity.supply_pubkey,
                    usdc_reserve.account.config.fee_receiver,
                    host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                    user.keypair.pubkey(),
                ),
            ],
            Some(&[&user.keypair]),
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        res,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::InvalidAmount as u32)
        )
    );
}

#[tokio::test]
async fn test_fail_double_borrow() {
    let (mut test, lending_market, usdc_reserve, user, host_fee_receiver, _) =
        setup(&ReserveConfig {
            deposit_limit: u64::MAX,
            borrow_limit: u64::MAX,
            fees: ReserveFees {
                borrow_fee_wad: 1,
                host_fee_percentage: 20,
                flash_loan_fee_wad: 1,
            },
            ..test_reserve_config()
        })
        .await;

    const FLASH_LOAN_AMOUNT: u64 = 3_000_000;
    let res = test
        .process_transaction(
            &[
                flash_borrow_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    usdc_reserve.account.liquidity.supply_pubkey,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                ),
                flash_borrow_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    usdc_reserve.account.liquidity.supply_pubkey,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                ),
                flash_repay_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    0,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.account.liquidity.supply_pubkey,
                    usdc_reserve.account.config.fee_receiver,
                    host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                    user.keypair.pubkey(),
                ),
            ],
            Some(&[&user.keypair]),
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        res,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::MultipleFlashBorrows as u32)
        )
    );
}

#[tokio::test]
async fn test_fail_double_repay() {
    let (mut test, lending_market, usdc_reserve, user, host_fee_receiver, _) =
        setup(&ReserveConfig {
            deposit_limit: u64::MAX,
            borrow_limit: u64::MAX,
            fees: ReserveFees {
                borrow_fee_wad: 1,
                host_fee_percentage: 20,
                flash_loan_fee_wad: 1,
            },
            ..test_reserve_config()
        })
        .await;

    const FLASH_LOAN_AMOUNT: u64 = 3_000_000;
    let res = test
        .process_transaction(
            &[
                flash_borrow_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    usdc_reserve.account.liquidity.supply_pubkey,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                ),
                flash_repay_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    0,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.account.liquidity.supply_pubkey,
                    usdc_reserve.account.config.fee_receiver,
                    host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                    user.keypair.pubkey(),
                ),
                flash_repay_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    0,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.account.liquidity.supply_pubkey,
                    usdc_reserve.account.config.fee_receiver,
                    host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                    user.keypair.pubkey(),
                ),
            ],
            Some(&[&user.keypair]),
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        res,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::MultipleFlashBorrows as u32)
        )
    );
}

#[tokio::test]
async fn test_fail_only_one_flash_ix_pair_per_tx() {
    let (mut test, lending_market, usdc_reserve, user, host_fee_receiver, _) =
        setup(&ReserveConfig {
            deposit_limit: u64::MAX,
            borrow_limit: u64::MAX,
            fees: ReserveFees {
                borrow_fee_wad: 1,
                host_fee_percentage: 20,
                flash_loan_fee_wad: 3_000_000_000_000_000,
            },
            ..test_reserve_config()
        })
        .await;

    const FLASH_LOAN_AMOUNT: u64 = 3_000_000;
    let res = test
        .process_transaction(
            &[
                flash_borrow_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    usdc_reserve.account.liquidity.supply_pubkey,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                ),
                flash_repay_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    0,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.account.liquidity.supply_pubkey,
                    usdc_reserve.account.config.fee_receiver,
                    host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                    user.keypair.pubkey(),
                ),
                flash_borrow_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    usdc_reserve.account.liquidity.supply_pubkey,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                ),
                flash_repay_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    2,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.account.liquidity.supply_pubkey,
                    usdc_reserve.account.config.fee_receiver,
                    host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                    user.keypair.pubkey(),
                ),
            ],
            Some(&[&user.keypair]),
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        res,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::MultipleFlashBorrows as u32)
        )
    );
}

#[tokio::test]
async fn test_fail_invalid_repay_ix() {
    let (mut test, lending_market, usdc_reserve, user, host_fee_receiver, _) =
        setup(&ReserveConfig {
            deposit_limit: u64::MAX,
            borrow_limit: u64::MAX,
            fees: ReserveFees {
                borrow_fee_wad: 1,
                host_fee_percentage: 20,
                flash_loan_fee_wad: 1,
            },
            ..test_reserve_config()
        })
        .await;

    const FLASH_LOAN_AMOUNT: u64 = 3_000_000;
    // case 1: invalid reserve in repay
    {
        let res = test
            .process_transaction(
                &[
                    flash_borrow_reserve_liquidity(
                        solend_program::id(),
                        FLASH_LOAN_AMOUNT,
                        usdc_reserve.account.liquidity.supply_pubkey,
                        user.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.pubkey,
                        lending_market.pubkey,
                    ),
                    flash_repay_reserve_liquidity(
                        solend_program::id(),
                        FLASH_LOAN_AMOUNT,
                        0,
                        user.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.account.liquidity.supply_pubkey,
                        usdc_reserve.account.config.fee_receiver,
                        host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                        Pubkey::new_unique(),
                        lending_market.pubkey,
                        user.keypair.pubkey(),
                    ),
                ],
                Some(&[&user.keypair]),
            )
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            res,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(LendingError::InvalidFlashRepay as u32)
            )
        );
    }

    // case 2: invalid liquidity amount
    {
        let res = test
            .process_transaction(
                &[
                    flash_borrow_reserve_liquidity(
                        solend_program::id(),
                        FLASH_LOAN_AMOUNT,
                        usdc_reserve.account.liquidity.supply_pubkey,
                        user.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.pubkey,
                        lending_market.pubkey,
                    ),
                    flash_repay_reserve_liquidity(
                        solend_program::id(),
                        FLASH_LOAN_AMOUNT - 1,
                        0,
                        user.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.account.liquidity.supply_pubkey,
                        usdc_reserve.account.config.fee_receiver,
                        host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.pubkey,
                        lending_market.pubkey,
                        user.keypair.pubkey(),
                    ),
                ],
                Some(&[&user.keypair]),
            )
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            res,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(LendingError::InvalidFlashRepay as u32)
            )
        );
    }

    // case 3: no repay
    {
        let res = test
            .process_transaction(
                &[flash_borrow_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    usdc_reserve.account.liquidity.supply_pubkey,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                )],
                None,
            )
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            res,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(LendingError::NoFlashRepayFound as u32)
            )
        );
    }

    // case 4: cpi repay
    {
        let res = test
            .process_transaction(
                &[
                    flash_borrow_reserve_liquidity(
                        solend_program::id(),
                        FLASH_LOAN_AMOUNT,
                        usdc_reserve.account.liquidity.supply_pubkey,
                        user.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.pubkey,
                        lending_market.pubkey,
                    ),
                    helpers::flash_loan_proxy::repay_proxy(
                        proxy_program::id(),
                        FLASH_LOAN_AMOUNT,
                        0,
                        user.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.account.liquidity.supply_pubkey,
                        usdc_reserve.account.config.fee_receiver,
                        host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.pubkey,
                        solend_program::id(),
                        lending_market.pubkey,
                        user.keypair.pubkey(),
                    ),
                ],
                Some(&[&user.keypair]),
            )
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            res,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(LendingError::NoFlashRepayFound as u32)
            )
        );
    }

    // case 5: insufficient funds to pay fees on repay.
    {
        let new_user = User::new_with_balances(&mut test, &[(&usdc_mint::id(), 0)]).await;
        let res = test
            .process_transaction(
                &[
                    flash_borrow_reserve_liquidity(
                        solend_program::id(),
                        100_000_000_000,
                        usdc_reserve.account.liquidity.supply_pubkey,
                        new_user.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.pubkey,
                        lending_market.pubkey,
                    ),
                    flash_repay_reserve_liquidity(
                        solend_program::id(),
                        100_000_000_000,
                        0,
                        new_user.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.account.liquidity.supply_pubkey,
                        usdc_reserve.account.config.fee_receiver,
                        host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.pubkey,
                        lending_market.pubkey,
                        new_user.keypair.pubkey(),
                    ),
                ],
                Some(&[&new_user.keypair]),
            )
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
        let res = test
            .process_transaction(
                &[flash_repay_reserve_liquidity(
                    solend_program::id(),
                    FLASH_LOAN_AMOUNT,
                    0,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.account.liquidity.supply_pubkey,
                    usdc_reserve.account.config.fee_receiver,
                    host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                    user.keypair.pubkey(),
                )],
                Some(&[&user.keypair]),
            )
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            res,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(LendingError::InvalidFlashRepay as u32)
            )
        );
    }

    // case 7: Incorrect borrow instruction index -- points to itself
    {
        let res = test
            .process_transaction(
                &[
                    flash_borrow_reserve_liquidity(
                        solend_program::id(),
                        FLASH_LOAN_AMOUNT,
                        usdc_reserve.account.liquidity.supply_pubkey,
                        user.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.pubkey,
                        lending_market.pubkey,
                    ),
                    flash_repay_reserve_liquidity(
                        solend_program::id(),
                        FLASH_LOAN_AMOUNT,
                        1, // should be 0
                        user.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.account.liquidity.supply_pubkey,
                        usdc_reserve.account.config.fee_receiver,
                        host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.pubkey,
                        lending_market.pubkey,
                        user.keypair.pubkey(),
                    ),
                ],
                Some(&[&user.keypair]),
            )
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            res,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(LendingError::InvalidFlashRepay as u32)
            )
        );
    }

    // case 8: Incorrect borrow instruction index -- points to some other program
    {
        let user_transfer_authority = Keypair::new();
        let res = test
            .process_transaction(
                &[
                    approve(
                        &spl_token::id(),
                        &user.get_account(&usdc_mint::id()).unwrap(),
                        &user_transfer_authority.pubkey(),
                        &user.keypair.pubkey(),
                        &[],
                        1,
                    )
                    .unwrap(),
                    flash_borrow_reserve_liquidity(
                        solend_program::id(),
                        FLASH_LOAN_AMOUNT,
                        usdc_reserve.account.liquidity.supply_pubkey,
                        user.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.pubkey,
                        lending_market.pubkey,
                    ),
                    flash_repay_reserve_liquidity(
                        solend_program::id(),
                        FLASH_LOAN_AMOUNT,
                        0,
                        user.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.account.liquidity.supply_pubkey,
                        usdc_reserve.account.config.fee_receiver,
                        host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.pubkey,
                        lending_market.pubkey,
                        user.keypair.pubkey(),
                    ),
                ],
                Some(&[&user.keypair]),
            )
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            res,
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(LendingError::InvalidFlashRepay as u32)
            )
        );
    }
    // case 9: Incorrect borrow instruction index -- points to a later borrow
    {
        let res = test
            .process_transaction(
                &[
                    flash_repay_reserve_liquidity(
                        solend_program::id(),
                        FLASH_LOAN_AMOUNT,
                        1,
                        user.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.account.liquidity.supply_pubkey,
                        usdc_reserve.account.config.fee_receiver,
                        host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.pubkey,
                        lending_market.pubkey,
                        user.keypair.pubkey(),
                    ),
                    flash_borrow_reserve_liquidity(
                        solend_program::id(),
                        FLASH_LOAN_AMOUNT,
                        usdc_reserve.account.liquidity.supply_pubkey,
                        user.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.pubkey,
                        lending_market.pubkey,
                    ),
                    flash_repay_reserve_liquidity(
                        solend_program::id(),
                        FLASH_LOAN_AMOUNT,
                        1,
                        user.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.account.liquidity.supply_pubkey,
                        usdc_reserve.account.config.fee_receiver,
                        host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                        usdc_reserve.pubkey,
                        lending_market.pubkey,
                        user.keypair.pubkey(),
                    ),
                ],
                Some(&[&user.keypair]),
            )
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            res,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(LendingError::InvalidFlashRepay as u32)
            )
        );
    }
}

#[tokio::test]
async fn test_fail_insufficient_liquidity_for_borrow() {
    let (mut test, lending_market, usdc_reserve, user, host_fee_receiver, _) =
        setup(&ReserveConfig {
            deposit_limit: u64::MAX,
            fees: ReserveFees {
                borrow_fee_wad: 100_000_000_000,
                host_fee_percentage: 20,
                flash_loan_fee_wad: 3_000_000_000_000_000,
            },
            ..test_reserve_config()
        })
        .await;

    let res = test
        .process_transaction(
            &[
                flash_borrow_reserve_liquidity(
                    solend_program::id(),
                    1_000_000_000_000,
                    usdc_reserve.account.liquidity.supply_pubkey,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                ),
                flash_repay_reserve_liquidity(
                    solend_program::id(),
                    1_000_000_000_000,
                    0,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.account.liquidity.supply_pubkey,
                    usdc_reserve.account.config.fee_receiver,
                    host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                    user.keypair.pubkey(),
                ),
            ],
            Some(&[&user.keypair]),
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        res,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::InsufficientLiquidity as u32)
        )
    );
}

#[tokio::test]
async fn test_fail_cpi_borrow() {
    let (mut test, lending_market, usdc_reserve, user, _, _) = setup(&ReserveConfig {
        deposit_limit: u64::MAX,
        borrow_limit: u64::MAX,
        fees: ReserveFees {
            borrow_fee_wad: 1,
            host_fee_percentage: 20,
            flash_loan_fee_wad: 1,
        },
        ..test_reserve_config()
    })
    .await;

    const FLASH_LOAN_AMOUNT: u64 = 3_000_000;
    let res = test
        .process_transaction(
            &[helpers::flash_loan_proxy::borrow_proxy(
                proxy_program::id(),
                FLASH_LOAN_AMOUNT,
                usdc_reserve.account.liquidity.supply_pubkey,
                user.get_account(&usdc_mint::id()).unwrap(),
                usdc_reserve.pubkey,
                solend_program::id(),
                lending_market.pubkey,
                Pubkey::find_program_address(
                    &[lending_market.pubkey.as_ref()],
                    &solend_program::id(),
                )
                .0,
            )],
            None,
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        res,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::FlashBorrowCpi as u32)
        )
    );
}

#[tokio::test]
async fn test_fail_cpi_repay() {
    let (mut test, lending_market, usdc_reserve, user, host_fee_receiver, _) =
        setup(&ReserveConfig {
            deposit_limit: u64::MAX,
            borrow_limit: u64::MAX,
            fees: ReserveFees {
                borrow_fee_wad: 1,
                host_fee_percentage: 20,
                flash_loan_fee_wad: 1,
            },
            ..test_reserve_config()
        })
        .await;

    const FLASH_LOAN_AMOUNT: u64 = 3_000_000;
    let res = test
        .process_transaction(
            &[helpers::flash_loan_proxy::repay_proxy(
                proxy_program::id(),
                FLASH_LOAN_AMOUNT,
                0,
                user.get_account(&usdc_mint::id()).unwrap(),
                usdc_reserve.account.liquidity.supply_pubkey,
                usdc_reserve.account.config.fee_receiver,
                host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                usdc_reserve.pubkey,
                solend_program::id(),
                lending_market.pubkey,
                user.keypair.pubkey(),
            )],
            Some(&[&user.keypair]),
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        res,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::FlashRepayCpi as u32)
        )
    );
}

#[tokio::test]
async fn test_fail_repay_from_diff_reserve() {
    let (mut test, lending_market, usdc_reserve, user, host_fee_receiver, lending_market_owner) =
        setup(&ReserveConfig {
            deposit_limit: u64::MAX,
            fees: ReserveFees {
                borrow_fee_wad: 1,
                host_fee_percentage: 20,
                flash_loan_fee_wad: 1,
            },
            ..test_reserve_config()
        })
        .await;

    let another_usdc_reserve = test
        .init_reserve(
            &lending_market,
            &lending_market_owner,
            &usdc_mint::id(),
            &test_reserve_config(),
            &Keypair::new(),
            10,
            None,
        )
        .await
        .unwrap();

    // this transaction fails because the repay token transfers aren't signed by the
    // lending_market_authority PDA.
    let res = test
        .process_transaction(
            &[
                flash_borrow_reserve_liquidity(
                    solend_program::id(),
                    1000,
                    usdc_reserve.account.liquidity.supply_pubkey,
                    user.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                ),
                malicious_flash_repay_reserve_liquidity(
                    solend_program::id(),
                    1000,
                    0,
                    another_usdc_reserve.account.liquidity.supply_pubkey,
                    usdc_reserve.account.liquidity.supply_pubkey,
                    usdc_reserve.account.config.fee_receiver,
                    host_fee_receiver.get_account(&usdc_mint::id()).unwrap(),
                    usdc_reserve.pubkey,
                    lending_market.pubkey,
                    Pubkey::find_program_address(
                        &[lending_market.pubkey.as_ref()],
                        &solend_program::id(),
                    )
                    .0,
                ),
            ],
            None, // Some(&[&user.keypair]),
        )
        .await
        .unwrap_err();

    match res {
        BanksClientError::RpcError(..) => (),
        BanksClientError::TransactionError(TransactionError::InstructionError(
            1,
            InstructionError::PrivilegeEscalation,
        )) => (),
        _ => panic!("Unexpected error: {:?}", res),
    };
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
