#![cfg(feature = "test-bpf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError, program_option::COption, pubkey::Pubkey, signature::Signer,
        signer::keypair::Keypair, transaction::TransactionError, transport::TransportError,
    },
    spl_token_2022::{
        error::TokenError, extension::mint_close_authority::MintCloseAuthority, instruction,
    },
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    std::convert::TryInto,
};

#[tokio::test]
async fn success_init() {
    let close_authority = Some(Pubkey::new_unique());
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::MintCloseAuthority {
            close_authority,
        }])
        .await
        .unwrap();
    let TokenContext {
        decimals,
        mint_authority,
        token,
        ..
    } = context.token_context.unwrap();

    let state = token.get_mint_info().await.unwrap();
    assert_eq!(state.base.decimals, decimals);
    assert_eq!(
        state.base.mint_authority,
        COption::Some(mint_authority.pubkey())
    );
    assert_eq!(state.base.supply, 0);
    assert!(state.base.is_initialized);
    assert_eq!(state.base.freeze_authority, COption::None);
    let extension = state.get_extension::<MintCloseAuthority>().unwrap();
    assert_eq!(
        extension.close_authority,
        close_authority.try_into().unwrap(),
    );
}

#[tokio::test]
async fn set_authority() {
    let close_authority = Keypair::new();
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::MintCloseAuthority {
            close_authority: Some(close_authority.pubkey()),
        }])
        .await
        .unwrap();
    let token = context.token_context.unwrap().token;
    let new_authority = Keypair::new();

    // fail, wrong signature
    let wrong = Keypair::new();
    let err = token
        .set_authority(
            token.get_address(),
            Some(&new_authority.pubkey()),
            instruction::AuthorityType::CloseAccount,
            &wrong,
        )
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

    // success
    token
        .set_authority(
            token.get_address(),
            Some(&new_authority.pubkey()),
            instruction::AuthorityType::CloseAccount,
            &close_authority,
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<MintCloseAuthority>().unwrap();
    assert_eq!(
        extension.close_authority,
        Some(new_authority.pubkey()).try_into().unwrap(),
    );

    // set to none
    token
        .set_authority(
            token.get_address(),
            None,
            instruction::AuthorityType::CloseAccount,
            &new_authority,
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<MintCloseAuthority>().unwrap();
    assert_eq!(extension.close_authority, None.try_into().unwrap(),);

    // fail set again
    let err = token
        .set_authority(
            token.get_address(),
            Some(&close_authority.pubkey()),
            instruction::AuthorityType::CloseAccount,
            &new_authority,
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::AuthorityTypeNotSupported as u32)
            )
        )))
    );

    // fail close
    let destination = Pubkey::new_unique();
    let err = token
        .close_account(token.get_address(), &destination, &new_authority)
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::AuthorityTypeNotSupported as u32)
            )
        )))
    );
}

#[tokio::test]
async fn success_close() {
    let close_authority = Keypair::new();
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::MintCloseAuthority {
            close_authority: Some(close_authority.pubkey()),
        }])
        .await
        .unwrap();
    let token = context.token_context.unwrap().token;

    let destination = Pubkey::new_unique();
    token
        .close_account(token.get_address(), &destination, &close_authority)
        .await
        .unwrap();
    let destination = token.get_account(&destination).await.unwrap();
    assert!(destination.lamports > 0);
}

#[tokio::test]
async fn fail_without_extension() {
    let close_authority = Pubkey::new_unique();
    let mut context = TestContext::new().await;
    context.init_token_with_mint(vec![]).await.unwrap();
    let TokenContext {
        mint_authority,
        token,
        ..
    } = context.token_context.unwrap();

    // fail set
    let err = token
        .set_authority(
            token.get_address(),
            Some(&close_authority),
            instruction::AuthorityType::CloseAccount,
            &mint_authority,
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::InvalidAccountData)
        )))
    );

    // fail close
    let destination = Pubkey::new_unique();
    let err = token
        .close_account(token.get_address(), &destination, &mint_authority)
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::InvalidAccountData)
        )))
    );
}

#[tokio::test]
async fn fail_close_with_supply() {
    let close_authority = Keypair::new();
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::MintCloseAuthority {
            close_authority: Some(close_authority.pubkey()),
        }])
        .await
        .unwrap();
    let TokenContext {
        mint_authority,
        token,
        ..
    } = context.token_context.unwrap();

    // mint a token
    let owner = Pubkey::new_unique();
    let account = Keypair::new();
    let account = token
        .create_auxiliary_token_account(&account, &owner)
        .await
        .unwrap();
    token.mint_to(&account, &mint_authority, 1).await.unwrap();

    // fail close
    let destination = Pubkey::new_unique();
    let err = token
        .close_account(token.get_address(), &destination, &close_authority)
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintHasSupply as u32)
            )
        )))
    );
}
