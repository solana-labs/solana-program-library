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
        error::TokenError, extension::default_account_state::DefaultAccountState,
        instruction::AuthorityType, state::AccountState,
    },
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    std::convert::TryFrom,
};

#[tokio::test]
async fn success_init_default_acct_state_frozen() {
    let default_account_state = AccountState::Frozen;
    let mut context = TestContext::new().await;
    context
        .init_token_with_freezing_mint(vec![ExtensionInitializationParams::DefaultAccountState {
            state: default_account_state,
        }])
        .await
        .unwrap();
    let TokenContext {
        decimals,
        mint_authority,
        freeze_authority,
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
    assert_eq!(
        state.base.freeze_authority,
        COption::Some(freeze_authority.unwrap().pubkey())
    );
    let extension = state.get_extension::<DefaultAccountState>().unwrap();
    assert_eq!(
        AccountState::try_from(extension.state).unwrap(),
        default_account_state,
    );
}

#[tokio::test]
async fn fail_init_no_authority_default_acct_state_frozen() {
    let default_account_state = AccountState::Frozen;
    let mut context = TestContext::new().await;
    let err = context
        .init_token_with_mint(vec![ExtensionInitializationParams::DefaultAccountState {
            state: default_account_state,
        }])
        .await
        .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                2,
                InstructionError::Custom(TokenError::MintCannotFreeze as u32)
            )
        )))
    );
}

#[tokio::test]
async fn success_init_default_acct_state_initialized() {
    let default_account_state = AccountState::Initialized;
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::DefaultAccountState {
            state: default_account_state,
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
    let extension = state.get_extension::<DefaultAccountState>().unwrap();
    assert_eq!(
        AccountState::try_from(extension.state).unwrap(),
        default_account_state,
    );
}

#[tokio::test]
async fn success_no_authority_init_default_acct_state_initialized() {
    let default_account_state = AccountState::Initialized;
    let mut context = TestContext::new().await;
    context
        .init_token_with_freezing_mint(vec![ExtensionInitializationParams::DefaultAccountState {
            state: default_account_state,
        }])
        .await
        .unwrap();
    let TokenContext {
        decimals,
        mint_authority,
        freeze_authority,
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
    assert_eq!(
        state.base.freeze_authority,
        COption::Some(freeze_authority.unwrap().pubkey())
    );
    let extension = state.get_extension::<DefaultAccountState>().unwrap();
    assert_eq!(
        AccountState::try_from(extension.state).unwrap(),
        default_account_state,
    );
}

#[tokio::test]
async fn fail_invalid_default_acct_state() {
    let default_account_state = AccountState::Uninitialized;
    let mut context = TestContext::new().await;
    let err = context
        .init_token_with_freezing_mint(vec![ExtensionInitializationParams::DefaultAccountState {
            state: default_account_state,
        }])
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(TokenError::InvalidState as u32)
            )
        )))
    );
}

#[tokio::test]
async fn end_to_end_default_account_state() {
    let default_account_state = AccountState::Frozen;
    let mut context = TestContext::new().await;
    context
        .init_token_with_freezing_mint(vec![ExtensionInitializationParams::DefaultAccountState {
            state: default_account_state,
        }])
        .await
        .unwrap();
    let TokenContext {
        mint_authority,
        freeze_authority,
        token,
        ..
    } = context.token_context.unwrap();

    let freeze_authority = freeze_authority.unwrap();

    let owner = Pubkey::new_unique();
    let account = Keypair::new();
    let account = token
        .create_auxiliary_token_account(&account, &owner)
        .await
        .unwrap();
    let account_state = token.get_account_info(&account).await.unwrap();
    assert_eq!(account_state.base.state, default_account_state);

    // Invalid default state
    let err = token
        .set_default_account_state(&mint_authority, &AccountState::Uninitialized)
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::InvalidState as u32)
            )
        )))
    );

    token
        .set_default_account_state(&freeze_authority, &AccountState::Initialized)
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<DefaultAccountState>().unwrap();
    assert_eq!(
        AccountState::try_from(extension.state).unwrap(),
        AccountState::Initialized,
    );

    let owner = Pubkey::new_unique();
    let account = Keypair::new();
    let account = token
        .create_auxiliary_token_account(&account, &owner)
        .await
        .unwrap();
    let account_state = token.get_account_info(&account).await.unwrap();
    assert_eq!(account_state.base.state, AccountState::Initialized);

    // adjusting freeze authority adjusts default state authority
    let new_authority = Keypair::new();
    token
        .set_authority(
            token.get_address(),
            Some(&new_authority.pubkey()),
            AuthorityType::FreezeAccount,
            &freeze_authority,
        )
        .await
        .unwrap();

    let err = token
        .set_default_account_state(&mint_authority, &AccountState::Frozen)
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

    token
        .set_default_account_state(&new_authority, &AccountState::Frozen)
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<DefaultAccountState>().unwrap();
    assert_eq!(
        AccountState::try_from(extension.state).unwrap(),
        AccountState::Frozen,
    );

    token
        .set_authority(
            token.get_address(),
            None,
            AuthorityType::FreezeAccount,
            &new_authority,
        )
        .await
        .unwrap();

    let err = token
        .set_default_account_state(&new_authority, &AccountState::Initialized)
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NoAuthorityExists as u32)
            )
        )))
    );
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<DefaultAccountState>().unwrap();
    assert_eq!(
        AccountState::try_from(extension.state).unwrap(),
        AccountState::Frozen,
    );
}
