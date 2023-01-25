#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::solend_program_test::{setup_world, Info, SolendProgramTest, User};
use helpers::*;
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::signature::Keypair;

use solana_sdk::signer::Signer;
use solana_sdk::transaction::TransactionError;
use solend_program::error::LendingError;
use solend_program::instruction::init_obligation;
use solend_program::math::Decimal;
use solend_program::state::{LastUpdate, LendingMarket, Obligation, PROGRAM_VERSION};

async fn setup() -> (SolendProgramTest, Info<LendingMarket>, User) {
    let (test, lending_market, _, _, _, user) =
        setup_world(&test_reserve_config(), &test_reserve_config()).await;

    (test, lending_market, user)
}

#[tokio::test]
async fn test_success() {
    let (mut test, lending_market, user) = setup().await;

    let obligation = lending_market
        .init_obligation(&mut test, Keypair::new(), &user)
        .await
        .expect("This should succeed");

    assert_eq!(
        obligation.account,
        Obligation {
            version: PROGRAM_VERSION,
            last_update: LastUpdate {
                slot: 1000,
                stale: true
            },
            lending_market: lending_market.pubkey,
            owner: user.keypair.pubkey(),
            deposits: Vec::new(),
            borrows: Vec::new(),
            deposited_value: Decimal::zero(),
            borrowed_value: Decimal::zero(),
            allowed_borrow_value: Decimal::zero(),
            unhealthy_borrow_value: Decimal::zero()
        }
    );
}

#[tokio::test]
async fn test_already_initialized() {
    let (mut test, lending_market, user) = setup().await;

    let keypair = Keypair::new();
    let keypair_clone = Keypair::from_bytes(&keypair.to_bytes().clone()).unwrap();

    lending_market
        .init_obligation(&mut test, keypair, &user)
        .await
        .expect("This should succeed");

    test.advance_clock_by_slots(1).await;

    let res = test
        .process_transaction(
            &[init_obligation(
                solend_program::id(),
                keypair_clone.pubkey(),
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
            InstructionError::Custom(LendingError::AlreadyInitialized as u32)
        )
    );
}
