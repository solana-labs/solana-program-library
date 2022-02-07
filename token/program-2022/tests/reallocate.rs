#![cfg(feature = "test-bpf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError, program_option::COption, pubkey::Pubkey, signature::Signer,
        signer::keypair::Keypair, transaction::TransactionError, transport::TransportError,
    },
    spl_token_2022::{error::TokenError, extension::ExtensionType, state::Account},
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    std::convert::TryInto,
};

#[tokio::test]
async fn reallocate() {
    let mut context = TestContext::new().await;
    context.init_token_with_mint(vec![]).await.unwrap();
    let TokenContext {
        token,
        alice,
        mint_authority,
        ..
    } = context.token_context.unwrap();

    // reallocate fails on wrong account type
    let error = token
        .reallocate(
            token.get_address(),
            &mint_authority,
            &[ExtensionType::ImmutableOwner],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::InvalidAccountData)
        )))
    );

    // create account just large enough for base
    let alice_account = Keypair::new();
    let alice_account = token
        .create_auxiliary_token_account(&alice_account, &alice.pubkey())
        .await
        .unwrap();

    // reallocate fails on invalid extension type
    let error = token
        .reallocate(&alice_account, &alice, &[ExtensionType::MintCloseAuthority])
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::InvalidState as u32)
            )
        )))
    );

    // reallocate fails on invalid authority
    let error = token
        .reallocate(
            &alice_account,
            &mint_authority,
            &[ExtensionType::ImmutableOwner],
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

    // reallocate succeeds
    token
        .reallocate(&alice_account, &alice, &[ExtensionType::ImmutableOwner])
        .await
        .unwrap();
    let account = token.get_account(&alice_account).await.unwrap();
    assert_eq!(
        account.data.len(),
        ExtensionType::get_account_len::<Account>(&[ExtensionType::ImmutableOwner])
    );

    // reallocate succeeds with noop if account is already large enough
    token.get_new_latest_blockhash().await.unwrap();
    token
        .reallocate(&alice_account, &alice, &[ExtensionType::ImmutableOwner])
        .await
        .unwrap();
    let account = token.get_account(&alice_account).await.unwrap();
    assert_eq!(
        account.data.len(),
        ExtensionType::get_account_len::<Account>(&[ExtensionType::ImmutableOwner])
    );

    // reallocate only reallocates enough for new extension, and dedupes extensions
    token
        .reallocate(
            &alice_account,
            &alice,
            &[
                ExtensionType::ImmutableOwner,
                ExtensionType::ImmutableOwner,
                ExtensionType::TransferFeeAmount,
                ExtensionType::TransferFeeAmount,
            ],
        )
        .await
        .unwrap();
    let account = token.get_account(&alice_account).await.unwrap();
    assert_eq!(
        account.data.len(),
        ExtensionType::get_account_len::<Account>(&[
            ExtensionType::ImmutableOwner,
            ExtensionType::TransferFeeAmount
        ])
    );
}

#[tokio::test]
async fn reallocate_without_current_extension_knowledge() {
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: COption::Some(Pubkey::new_unique()).try_into().unwrap(),
            withdraw_withheld_authority: COption::Some(Pubkey::new_unique()).try_into().unwrap(),
            transfer_fee_basis_points: 250,
            maximum_fee: 10_000_000,
        }])
        .await
        .unwrap();
    let TokenContext { token, alice, .. } = context.token_context.unwrap();

    // create account just large enough for TransferFeeAmount extension
    let alice_account = Keypair::new();
    let alice_account = token
        .create_auxiliary_token_account(&alice_account, &alice.pubkey())
        .await
        .unwrap();

    // reallocate resizes account to accommodate new and existing extensions
    token
        .reallocate(&alice_account, &alice, &[ExtensionType::ImmutableOwner])
        .await
        .unwrap();
    let account = token.get_account(&alice_account).await.unwrap();
    assert_eq!(
        account.data.len(),
        ExtensionType::get_account_len::<Account>(&[
            ExtensionType::TransferFeeAmount,
            ExtensionType::ImmutableOwner
        ])
    );
}
