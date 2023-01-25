#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::solend_program_test::{SolendProgramTest, User};
use helpers::*;
use mock_pyth::mock_pyth_program;
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::TransactionError;
use solend_program::error::LendingError;
use solend_program::instruction::init_lending_market;
use solend_program::state::{LendingMarket, PROGRAM_VERSION};

#[tokio::test]
async fn test_success() {
    let mut test = SolendProgramTest::start_new().await;
    let lending_market_owner = User::new_with_balances(&mut test, &[]).await;

    let lending_market = test
        .init_lending_market(&lending_market_owner, &Keypair::new())
        .await
        .unwrap();
    assert_eq!(
        lending_market.account,
        LendingMarket {
            version: PROGRAM_VERSION,
            bump_seed: lending_market.account.bump_seed, // TODO test this field
            owner: lending_market_owner.keypair.pubkey(),
            quote_currency: QUOTE_CURRENCY,
            token_program_id: spl_token::id(),
            oracle_program_id: mock_pyth_program::id(),
            switchboard_oracle_program_id: mock_pyth_program::id(),
        }
    );
}

#[tokio::test]
async fn test_already_initialized() {
    let mut test = SolendProgramTest::start_new().await;
    let lending_market_owner = User::new_with_balances(&mut test, &[]).await;

    let keypair = Keypair::new();
    test.init_lending_market(&lending_market_owner, &keypair)
        .await
        .unwrap();

    test.advance_clock_by_slots(1).await;

    let res = test
        .process_transaction(
            &[init_lending_market(
                solend_program::id(),
                lending_market_owner.keypair.pubkey(),
                QUOTE_CURRENCY,
                keypair.pubkey(),
                mock_pyth_program::id(),
                mock_pyth_program::id(),
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
            InstructionError::Custom(LendingError::AlreadyInitialized as u32)
        )
    );
}
