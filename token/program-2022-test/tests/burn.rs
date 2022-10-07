#![cfg(feature = "test-sbf")]

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
        mint_authority,
        token,
        token_unchecked,
        alice,
        bob,
        ..
    } = context.token_context.unwrap();

    let alice_account = Keypair::new();
    token
        .create_auxiliary_token_account(&alice_account, &alice.pubkey())
        .await
        .unwrap();
    let alice_account = alice_account.pubkey();

    // mint a token
    let amount = 10;
    token
        .mint_to(
            &alice_account,
            &mint_authority.pubkey(),
            amount,
            &[&mint_authority],
        )
        .await
        .unwrap();

    // unchecked is ok
    token_unchecked
        .burn(&alice_account, &alice.pubkey(), 1, &[&alice])
        .await
        .unwrap();

    // checked is ok
    token
        .burn(&alice_account, &alice.pubkey(), 1, &[&alice])
        .await
        .unwrap();

    // burn too much is not ok
    let error = token
        .burn(&alice_account, &alice.pubkey(), amount, &[&alice])
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
        .burn(&alice_account, &bob.pubkey(), 1, &[&bob])
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
        mint_authority,
        token,
        token_unchecked,
        alice,
        ..
    } = context.token_context.unwrap();

    token
        .create_auxiliary_token_account(&alice, &alice.pubkey())
        .await
        .unwrap();
    let alice_account = alice.pubkey();

    // mint a token
    let amount = 10;
    token
        .mint_to(
            &alice_account,
            &mint_authority.pubkey(),
            amount,
            &[&mint_authority],
        )
        .await
        .unwrap();

    // unchecked is ok
    token_unchecked
        .burn(&alice_account, &alice.pubkey(), 1, &[&alice])
        .await
        .unwrap();

    // checked is ok
    token
        .burn(&alice_account, &alice.pubkey(), 1, &[&alice])
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

async fn run_burn_and_close_system_or_incinerator(context: TestContext, non_owner: &Pubkey) {
    let TokenContext {
        mint_authority,
        token,
        alice,
        ..
    } = context.token_context.unwrap();

    let alice_account = Keypair::new();
    token
        .create_auxiliary_token_account(&alice_account, &alice.pubkey())
        .await
        .unwrap();
    let alice_account = alice_account.pubkey();

    // mint a token
    token
        .mint_to(
            &alice_account,
            &mint_authority.pubkey(),
            1,
            &[&mint_authority],
        )
        .await
        .unwrap();

    // transfer token to incinerator/system
    let non_owner_account = Keypair::new();
    token
        .create_auxiliary_token_account(&non_owner_account, non_owner)
        .await
        .unwrap();
    let non_owner_account = non_owner_account.pubkey();
    token
        .transfer(
            &alice_account,
            &non_owner_account,
            &alice.pubkey(),
            1,
            &[&alice],
        )
        .await
        .unwrap();

    // can't close when holding tokens
    let carlos = Keypair::new();
    let error = token
        .close_account(
            &non_owner_account,
            &solana_program::incinerator::id(),
            &carlos.pubkey(),
            &[&carlos],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NonNativeHasBalance as u32)
            )
        )))
    );

    // but anyone can burn it
    token
        .burn(&non_owner_account, &carlos.pubkey(), 1, &[&carlos])
        .await
        .unwrap();

    // closing fails if destination is not the incinerator
    let error = token
        .close_account(
            &non_owner_account,
            &carlos.pubkey(),
            &carlos.pubkey(),
            &[&carlos],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::InvalidAccountData)
        )))
    );

    let error = token
        .close_account(
            &non_owner_account,
            &solana_program::system_program::id(),
            &carlos.pubkey(),
            &[&carlos],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::InvalidAccountData)
        )))
    );

    // ... and then close it
    token.get_new_latest_blockhash().await.unwrap();
    token
        .close_account(
            &non_owner_account,
            &solana_program::incinerator::id(),
            &carlos.pubkey(),
            &[&carlos],
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn burn_and_close_incinerator_tokens() {
    let mut context = TestContext::new().await;
    context.init_token_with_mint(vec![]).await.unwrap();
    run_burn_and_close_system_or_incinerator(context, &solana_program::incinerator::id()).await;
}

#[tokio::test]
async fn burn_and_close_system_tokens() {
    let mut context = TestContext::new().await;
    context.init_token_with_mint(vec![]).await.unwrap();
    run_burn_and_close_system_or_incinerator(context, &solana_program::system_program::id()).await;
}
