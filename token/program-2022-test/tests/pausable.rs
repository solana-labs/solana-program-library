#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError, pubkey::Pubkey, signature::Signer, signer::keypair::Keypair,
        transaction::TransactionError, transport::TransportError,
    },
    spl_token_2022::{
        error::TokenError,
        extension::{
            pausable::{PausableAccount, PausableConfig},
            BaseStateWithExtensions,
        },
        instruction::AuthorityType,
    },
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    std::convert::TryInto,
};

#[tokio::test]
async fn success_initialize() {
    let authority = Pubkey::new_unique();
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::PausableConfig {
            authority,
        }])
        .await
        .unwrap();
    let TokenContext { token, alice, .. } = context.token_context.unwrap();

    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<PausableConfig>().unwrap();
    assert_eq!(Option::<Pubkey>::from(extension.authority), Some(authority));
    assert!(!bool::from(extension.paused));

    let account = Keypair::new();
    token
        .create_auxiliary_token_account(&account, &alice.pubkey())
        .await
        .unwrap();
    let state = token.get_account_info(&account.pubkey()).await.unwrap();
    let _ = state.get_extension::<PausableAccount>().unwrap();
}

#[tokio::test]
async fn set_authority() {
    let authority = Keypair::new();
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::PausableConfig {
            authority: authority.pubkey(),
        }])
        .await
        .unwrap();
    let TokenContext { token, .. } = context.token_context.take().unwrap();

    // success
    let new_authority = Keypair::new();
    token
        .set_authority(
            token.get_address(),
            &authority.pubkey(),
            Some(&new_authority.pubkey()),
            AuthorityType::Pause,
            &[&authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<PausableConfig>().unwrap();
    assert_eq!(
        extension.authority,
        Some(new_authority.pubkey()).try_into().unwrap(),
    );
    token
        .pause(&new_authority.pubkey(), &[&new_authority])
        .await
        .unwrap();
    let err = token
        .pause(&authority.pubkey(), &[&authority])
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::OwnerMismatch as u32)
            )
        )))
    );

    // set to none
    token
        .set_authority(
            token.get_address(),
            &new_authority.pubkey(),
            None,
            AuthorityType::Pause,
            &[&new_authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<PausableConfig>().unwrap();
    assert_eq!(extension.authority, None.try_into().unwrap(),);
}

#[tokio::test]
async fn pause_mint() {
    let authority = Keypair::new();
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::PausableConfig {
            authority: authority.pubkey(),
        }])
        .await
        .unwrap();
    let TokenContext {
        mint_authority,
        token,
        token_unchecked,
        alice,
        ..
    } = context.token_context.take().unwrap();

    let alice_account = Keypair::new();
    token
        .create_auxiliary_token_account(&alice_account, &alice.pubkey())
        .await
        .unwrap();
    let alice_account = alice_account.pubkey();

    token
        .pause(&authority.pubkey(), &[&authority])
        .await
        .unwrap();

    let amount = 10;
    let error = token
        .mint_to(
            &alice_account,
            &mint_authority.pubkey(),
            amount,
            &[&mint_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintPaused as u32)
            )
        )))
    );

    let error = token_unchecked
        .mint_to(
            &alice_account,
            &mint_authority.pubkey(),
            amount,
            &[&mint_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintPaused as u32)
            )
        )))
    );
}

#[tokio::test]
async fn pause_burn() {
    let authority = Keypair::new();
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::PausableConfig {
            authority: authority.pubkey(),
        }])
        .await
        .unwrap();
    let TokenContext {
        mint_authority,
        token,
        token_unchecked,
        alice,
        ..
    } = context.token_context.take().unwrap();

    let alice_account = Keypair::new();
    token
        .create_auxiliary_token_account(&alice_account, &alice.pubkey())
        .await
        .unwrap();
    let alice_account = alice_account.pubkey();

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

    token
        .pause(&authority.pubkey(), &[&authority])
        .await
        .unwrap();

    let error = token_unchecked
        .burn(&alice_account, &alice.pubkey(), 1, &[&alice])
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintPaused as u32)
            )
        )))
    );

    let error = token
        .burn(&alice_account, &alice.pubkey(), 1, &[&alice])
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintPaused as u32)
            )
        )))
    );
}

#[tokio::test]
async fn pause_transfer() {
    let authority = Keypair::new();
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::PausableConfig {
            authority: authority.pubkey(),
        }])
        .await
        .unwrap();
    let TokenContext {
        mint_authority,
        token,
        token_unchecked,
        alice,
        bob,
        ..
    } = context.token_context.take().unwrap();

    let alice_account = Keypair::new();
    token
        .create_auxiliary_token_account(&alice_account, &alice.pubkey())
        .await
        .unwrap();
    let alice_account = alice_account.pubkey();

    let bob_account = Keypair::new();
    token
        .create_auxiliary_token_account(&bob_account, &bob.pubkey())
        .await
        .unwrap();
    let bob_account = bob_account.pubkey();

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

    token
        .pause(&authority.pubkey(), &[&authority])
        .await
        .unwrap();

    let error = token_unchecked
        .transfer(&alice_account, &bob_account, &alice.pubkey(), 1, &[&alice])
        .await
        .unwrap_err();

    // need to use checked transfer
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintRequiredForTransfer as u32)
            )
        )))
    );

    let error = token
        .transfer(&alice_account, &bob_account, &alice.pubkey(), 1, &[&alice])
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintPaused as u32)
            )
        )))
    );

    let error = token
        .transfer_with_fee(
            &alice_account,
            &bob_account,
            &alice.pubkey(),
            1,
            0,
            &[&alice],
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintPaused as u32)
            )
        )))
    );
}
