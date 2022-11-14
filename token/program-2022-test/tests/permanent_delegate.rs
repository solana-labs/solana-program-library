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
        extension::{permanent_delegate::PermanentDelegate, BaseStateWithExtensions},
        instruction,
    },
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    std::convert::TryInto,
};

async fn setup_accounts(token_context: &TokenContext, amount: u64) -> (Pubkey, Pubkey) {
    let alice_account = Keypair::new();
    token_context
        .token
        .create_auxiliary_token_account(&alice_account, &token_context.alice.pubkey())
        .await
        .unwrap();
    let alice_account = alice_account.pubkey();
    let bob_account = Keypair::new();
    token_context
        .token
        .create_auxiliary_token_account(&bob_account, &token_context.bob.pubkey())
        .await
        .unwrap();
    let bob_account = bob_account.pubkey();

    // mint tokens
    token_context
        .token
        .mint_to(
            &alice_account,
            &token_context.mint_authority.pubkey(),
            amount,
            &[&token_context.mint_authority],
        )
        .await
        .unwrap();
    (alice_account, bob_account)
}

#[tokio::test]
async fn success_init() {
    let delegate = Pubkey::new_unique();
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::PermanentDelegate {
            delegate,
        }])
        .await
        .unwrap();
    let TokenContext { token, .. } = context.token_context.unwrap();

    let state = token.get_mint_info().await.unwrap();
    assert!(state.base.is_initialized);
    let extension = state.get_extension::<PermanentDelegate>().unwrap();
    assert_eq!(extension.delegate, Some(delegate).try_into().unwrap(),);
}

#[tokio::test]
async fn set_authority() {
    let delegate = Keypair::new();
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::PermanentDelegate {
            delegate: delegate.pubkey(),
        }])
        .await
        .unwrap();
    let token_context = context.token_context.unwrap();
    let new_delegate = Keypair::new();

    // fail, wrong signature
    let wrong = Keypair::new();
    let err = token_context
        .token
        .set_authority(
            token_context.token.get_address(),
            &wrong.pubkey(),
            Some(&new_delegate.pubkey()),
            instruction::AuthorityType::PermanentDelegate,
            &[&wrong],
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
    token_context
        .token
        .set_authority(
            token_context.token.get_address(),
            &delegate.pubkey(),
            Some(&new_delegate.pubkey()),
            instruction::AuthorityType::PermanentDelegate,
            &[&delegate],
        )
        .await
        .unwrap();
    let state = token_context.token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<PermanentDelegate>().unwrap();
    assert_eq!(
        extension.delegate,
        Some(new_delegate.pubkey()).try_into().unwrap(),
    );

    // set to none
    token_context
        .token
        .set_authority(
            token_context.token.get_address(),
            &new_delegate.pubkey(),
            None,
            instruction::AuthorityType::PermanentDelegate,
            &[&new_delegate],
        )
        .await
        .unwrap();
    let state = token_context.token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<PermanentDelegate>().unwrap();
    assert_eq!(extension.delegate, None.try_into().unwrap(),);

    // fail set again
    let err = token_context
        .token
        .set_authority(
            token_context.token.get_address(),
            &new_delegate.pubkey(),
            Some(&delegate.pubkey()),
            instruction::AuthorityType::PermanentDelegate,
            &[&new_delegate],
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

    // setup accounts
    let amount = 10;
    let (alice_account, bob_account) = setup_accounts(&token_context, amount).await;

    // fail transfer
    let error = token_context
        .token
        .transfer(
            &alice_account,
            &bob_account,
            &new_delegate.pubkey(),
            amount,
            &[&new_delegate],
        )
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
async fn success_transfer() {
    let delegate = Keypair::new();
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::PermanentDelegate {
            delegate: delegate.pubkey(),
        }])
        .await
        .unwrap();
    let token_context = context.token_context.unwrap();
    let amount = 10;
    let (alice_account, bob_account) = setup_accounts(&token_context, amount).await;

    token_context
        .token
        .transfer(
            &alice_account,
            &bob_account,
            &delegate.pubkey(),
            amount,
            &[&delegate],
        )
        .await
        .unwrap();

    let destination = token_context
        .token
        .get_account_info(&bob_account)
        .await
        .unwrap();
    assert_eq!(destination.base.amount, amount);
}

#[tokio::test]
async fn success_burn() {
    let delegate = Keypair::new();
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::PermanentDelegate {
            delegate: delegate.pubkey(),
        }])
        .await
        .unwrap();
    let token_context = context.token_context.unwrap();
    let amount = 10;
    let (alice_account, _) = setup_accounts(&token_context, amount).await;

    token_context
        .token
        .burn(&alice_account, &delegate.pubkey(), amount, &[&delegate])
        .await
        .unwrap();

    let destination = token_context
        .token
        .get_account_info(&alice_account)
        .await
        .unwrap();
    assert_eq!(destination.base.amount, 0);
}

#[tokio::test]
async fn fail_without_extension() {
    let delegate = Pubkey::new_unique();
    let mut context = TestContext::new().await;
    context.init_token_with_mint(vec![]).await.unwrap();
    let token_context = context.token_context.unwrap();

    // fail set
    let err = token_context
        .token
        .set_authority(
            token_context.token.get_address(),
            &token_context.mint_authority.pubkey(),
            Some(&delegate),
            instruction::AuthorityType::PermanentDelegate,
            &[&token_context.mint_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::InvalidAccountData)
        )))
    );
}
