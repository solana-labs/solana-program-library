#![cfg(feature = "test-bpf")]

mod helpers;

use crate::solend_program_test::setup_world;
use crate::solend_program_test::Info;
use crate::solend_program_test::SolendProgramTest;
use crate::solend_program_test::User;
use helpers::*;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program_test::*;
use solana_sdk::{
    instruction::InstructionError,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::TransactionError,
};
use solend_program::state::LendingMarket;
use solend_program::state::RateLimiterConfig;
use solend_sdk::state::RateLimiter;

use solend_program::{error::LendingError, instruction::LendingInstruction};

async fn setup() -> (SolendProgramTest, Info<LendingMarket>, User) {
    let (test, lending_market, _usdc_reserve, _, lending_market_owner, _user) =
        setup_world(&test_reserve_config(), &test_reserve_config()).await;

    (test, lending_market, lending_market_owner)
}

#[tokio::test]
async fn test_success() {
    let (mut test, lending_market, lending_market_owner) = setup().await;
    let new_owner = Keypair::new();
    let new_config = RateLimiterConfig {
        max_outflow: 100,
        window_duration: 5,
    };

    lending_market
        .set_lending_market_owner_and_config(
            &mut test,
            &lending_market_owner,
            &new_owner.pubkey(),
            new_config,
        )
        .await
        .unwrap();

    let lending_market_post = test
        .load_account::<LendingMarket>(lending_market.pubkey)
        .await;

    assert_eq!(
        lending_market_post.account,
        LendingMarket {
            owner: new_owner.pubkey(),
            rate_limiter: RateLimiter::new(new_config, 1000),
            ..lending_market_post.account
        }
    );
}

#[tokio::test]
async fn test_invalid_owner() {
    let (mut test, lending_market, _lending_market_owner) = setup().await;
    let invalid_owner = User::new_with_keypair(Keypair::new());
    let new_owner = Keypair::new();

    let res = lending_market
        .set_lending_market_owner_and_config(
            &mut test,
            &invalid_owner,
            &new_owner.pubkey(),
            RateLimiterConfig::default(),
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        res,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::InvalidMarketOwner as u32)
        )
    );
}

#[tokio::test]
async fn test_owner_not_signer() {
    let (mut test, lending_market, _lending_market_owner) = setup().await;
    let new_owner = Pubkey::new_unique();
    let res = test
        .process_transaction(
            &[Instruction {
                program_id: solend_program::id(),
                accounts: vec![
                    AccountMeta::new(lending_market.pubkey, false),
                    AccountMeta::new_readonly(lending_market.account.owner, false),
                ],
                data: LendingInstruction::SetLendingMarketOwnerAndConfig {
                    new_owner,
                    rate_limiter_config: RateLimiterConfig::default(),
                }
                .pack(),
            }],
            None,
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        res,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LendingError::InvalidSigner as u32)
        )
    );
}
