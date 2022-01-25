#![cfg(feature = "test-bpf")]

mod program_test;
use {
    program_test::TestContext,
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError, signature::Signer,
        signer::keypair::Keypair, transaction::TransactionError, transport::TransportError,
    },
    spl_token_2022::error::TokenError,
    spl_token_client::token::TokenError as TokenClientError,
};

#[tokio::test]
async fn basic() {
    let TestContext {
        token,
        mint_authority,
        alice,
        bob,
        decimals,
        ..
    } = TestContext::new(vec![])
    .await
    .unwrap();

    let alice_account = Keypair::new();
    let alice_account = token
        .create_auxiliary_token_account(&alice_account, &alice.pubkey())
        .await
        .unwrap();
    let bob_account = Keypair::new();
    let bob_account = token
        .create_auxiliary_token_account(&bob_account, &bob.pubkey())
        .await
        .unwrap();

    // mint a token
    let amount = 10;
    token.mint_to(&alice_account, &mint_authority, amount).await.unwrap();

    // unchecked is ok
    token.transfer_unchecked(
        &alice_account,
        &bob_account,
        &alice,
        1
    ).await.unwrap();

    // checked is ok
    token.transfer_checked(
        &alice_account,
        &bob_account,
        &alice,
        1,
        decimals,
    ).await.unwrap();

    // transfer too much is not ok
    let error = token.transfer_checked(
        &alice_account,
        &bob_account,
        &alice,
        amount,
        decimals,
    ).await.unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::Custom(TokenError::InsufficientFunds as u32))
        )))
    );

    // wrong signer
    let error = token.transfer_checked(
        &alice_account,
        &bob_account,
        &bob,
        1,
        decimals,
    ).await.unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::Custom(TokenError::OwnerMismatch as u32))
        )))
    );
}

#[tokio::test]
async fn self_transfer() {
    let TestContext {
        token,
        mint_authority,
        alice,
        decimals,
        ..
    } = TestContext::new(vec![])
    .await
    .unwrap();

    let alice_account = Keypair::new();
    let alice_account = token
        .create_auxiliary_token_account(&alice_account, &alice.pubkey())
        .await
        .unwrap();

    // mint a token
    let amount = 10;
    token.mint_to(&alice_account, &mint_authority, amount).await.unwrap();

    // self transfer is ok
    token.transfer_checked(
        &alice_account,
        &alice_account,
        &alice,
        1,
        decimals,
    ).await.unwrap();
    token.transfer_unchecked(
        &alice_account,
        &alice_account,
        &alice,
        1,
    ).await.unwrap();

    // too much self transfer is not ok
    let error = token.transfer_checked(
        &alice_account,
        &alice_account,
        &alice,
        amount + 1,
        decimals,
    ).await.unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::Custom(TokenError::InsufficientFunds as u32))
        )))
    );
}

#[tokio::test]
async fn self_owned() {
    let TestContext {
        token,
        mint_authority,
        decimals,
        alice,
        bob,
        ..
    } = TestContext::new(vec![])
    .await
    .unwrap();

    let alice_account = token
        .create_auxiliary_token_account(&alice, &alice.pubkey())
        .await
        .unwrap();
    let bob_account = Keypair::new();
    let bob_account = token
        .create_auxiliary_token_account(&bob_account, &bob.pubkey())
        .await
        .unwrap();

    // mint a token
    let amount = 10;
    token.mint_to(&alice_account, &mint_authority, amount).await.unwrap();

    // unchecked is ok
    token.transfer_unchecked(
        &alice_account,
        &bob_account,
        &alice,
        1
    ).await.unwrap();

    // checked is ok
    token.transfer_checked(
        &alice_account,
        &bob_account,
        &alice,
        1,
        decimals,
    ).await.unwrap();

    // self transfer is ok
    token.transfer_checked(
        &alice_account,
        &alice_account,
        &alice,
        1,
        decimals,
    ).await.unwrap();
}
