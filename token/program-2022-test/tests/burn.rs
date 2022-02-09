#![cfg(feature = "test-bpf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError, pubkey::Pubkey, signature::Signer, signer::keypair::Keypair,
        transaction::TransactionError, transport::TransportError,
    },
    spl_token_2022::error::TokenError,
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
};

async fn run_basic(context: TestContext) {
    let TokenContext {
        decimals,
        mint_authority,
        token,
        alice,
        bob,
        ..
    } = context.token_context.unwrap();

    let alice_account = Keypair::new();
    let alice_account = token
        .create_auxiliary_token_account(&alice_account, &alice.pubkey())
        .await
        .unwrap();

    // mint a token
    let amount = 10;
    token
        .mint_to(&alice_account, &mint_authority, amount)
        .await
        .unwrap();

    // unchecked is ok
    token.burn(&alice_account, &alice, 1).await.unwrap();

    // checked is ok
    token
        .burn_checked(&alice_account, &alice, 1, decimals)
        .await
        .unwrap();

    // burn too much is not ok
    let error = token
        .burn_checked(&alice_account, &alice, amount, decimals)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::InsufficientFunds as u32)
            )
        )))
    );

    // wrong signer
    let error = token
        .burn_checked(&alice_account, &bob, 1, decimals)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::OwnerMismatch as u32)
            )
        )))
    );
}

#[tokio::test]
async fn basic() {
    let mut context = TestContext::new().await;
    context.init_token_with_mint(vec![]).await.unwrap();
    run_basic(context).await;
}

#[tokio::test]
async fn basic_with_extension() {
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: Some(Pubkey::new_unique()),
            withdraw_withheld_authority: Some(Pubkey::new_unique()),
            transfer_fee_basis_points: 100u16,
            maximum_fee: 1_000u64,
        }])
        .await
        .unwrap();
    run_basic(context).await;
}

async fn run_self_owned(context: TestContext) {
    let TokenContext {
        decimals,
        mint_authority,
        token,
        alice,
        ..
    } = context.token_context.unwrap();

    let alice_account = token
        .create_auxiliary_token_account(&alice, &alice.pubkey())
        .await
        .unwrap();

    // mint a token
    let amount = 10;
    token
        .mint_to(&alice_account, &mint_authority, amount)
        .await
        .unwrap();

    // unchecked is ok
    token.burn(&alice_account, &alice, 1).await.unwrap();

    // checked is ok
    token
        .burn_checked(&alice_account, &alice, 1, decimals)
        .await
        .unwrap();
}

#[tokio::test]
async fn self_owned() {
    let mut context = TestContext::new().await;
    context.init_token_with_mint(vec![]).await.unwrap();
    run_self_owned(context).await;
}

#[tokio::test]
async fn self_owned_with_extension() {
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: Some(Pubkey::new_unique()),
            withdraw_withheld_authority: Some(Pubkey::new_unique()),
            transfer_fee_basis_points: 100u16,
            maximum_fee: 1_000u64,
        }])
        .await
        .unwrap();
    run_self_owned(context).await;
}
