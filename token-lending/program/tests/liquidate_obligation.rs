#![cfg(feature = "test-bpf")]

mod helpers;

use crate::solend_program_test::scenario_1;

use helpers::*;
use solana_program::instruction::InstructionError;

use solana_program_test::*;

use solana_sdk::transaction::TransactionError;
use solend_program::error::LendingError;

#[tokio::test]
async fn test_fail_deprecated() {
    let (mut test, lending_market, usdc_reserve, wsol_reserve, user, obligation) =
        scenario_1(&test_reserve_config(), &test_reserve_config()).await;

    let res = lending_market
        .liquidate_obligation(
            &mut test,
            &wsol_reserve,
            &usdc_reserve,
            &obligation,
            &user,
            1,
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        res,
        TransactionError::InstructionError(
            3,
            InstructionError::Custom(LendingError::DeprecatedInstruction as u32)
        )
    );
}
