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

#[derive(PartialEq)]
enum TransferMode {
    All,
    CheckedOnly,
}

#[derive(PartialEq)]
enum ApproveMode {
    Unchecked,
    Checked,
}

#[derive(PartialEq)]
enum OwnerMode {
    SelfOwned,
    External,
}

async fn run_basic(
    context: TestContext,
    owner_mode: OwnerMode,
    transfer_mode: TransferMode,
    approve_mode: ApproveMode,
) {
    let TokenContext {
        decimals,
        mint_authority,
        token,
        alice,
        bob,
        ..
    } = context.token_context.unwrap();

    let alice_account = match owner_mode {
        OwnerMode::SelfOwned => token
            .create_auxiliary_token_account(&alice, &alice.pubkey())
            .await
            .unwrap(),
        OwnerMode::External => {
            let alice_account = Keypair::new();
            token
                .create_auxiliary_token_account(&alice_account, &alice.pubkey())
                .await
                .unwrap()
        }
    };
    let bob_account = Keypair::new();
    let bob_account = token
        .create_auxiliary_token_account(&bob_account, &bob.pubkey())
        .await
        .unwrap();

    // mint tokens
    let amount = 100;
    token
        .mint_to(&alice_account, &mint_authority, amount)
        .await
        .unwrap();

    // delegate to bob
    let delegated_amount = 10;
    match approve_mode {
        ApproveMode::Unchecked => token
            .approve(&alice_account, &bob.pubkey(), &alice, delegated_amount)
            .await
            .unwrap(),
        ApproveMode::Checked => token
            .approve_checked(
                &alice_account,
                &bob.pubkey(),
                &alice,
                delegated_amount,
                decimals,
            )
            .await
            .unwrap(),
    }

    // transfer too much is not ok
    let error = token
        .transfer_checked(
            &alice_account,
            &bob_account,
            &bob,
            delegated_amount + 1,
            decimals,
        )
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

    // transfer is ok
    if transfer_mode == TransferMode::All {
        token
            .transfer_unchecked(&alice_account, &bob_account, &bob, 1)
            .await
            .unwrap();
    }

    token
        .transfer_checked(&alice_account, &bob_account, &bob, 1, decimals)
        .await
        .unwrap();

    // burn is ok
    token.burn(&alice_account, &bob, 1).await.unwrap();
    token
        .burn_checked(&alice_account, &bob, 1, decimals)
        .await
        .unwrap();

    // wrong signer
    let error = token
        .transfer_checked(&alice_account, &bob_account, &Keypair::new(), 1, decimals)
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

    // revoke
    token.revoke(&alice_account, &alice).await.unwrap();

    // now fails
    let error = token
        .transfer_checked(&alice_account, &bob_account, &bob, 1, decimals)
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
    run_basic(
        context,
        OwnerMode::External,
        TransferMode::All,
        ApproveMode::Unchecked,
    )
    .await;
}

#[tokio::test]
async fn basic_checked() {
    let mut context = TestContext::new().await;
    context.init_token_with_mint(vec![]).await.unwrap();
    run_basic(
        context,
        OwnerMode::External,
        TransferMode::All,
        ApproveMode::Checked,
    )
    .await;
}

#[tokio::test]
async fn basic_self_owned() {
    let mut context = TestContext::new().await;
    context.init_token_with_mint(vec![]).await.unwrap();
    run_basic(
        context,
        OwnerMode::SelfOwned,
        TransferMode::All,
        ApproveMode::Checked,
    )
    .await;
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
    run_basic(
        context,
        OwnerMode::External,
        TransferMode::CheckedOnly,
        ApproveMode::Unchecked,
    )
    .await;
}

#[tokio::test]
async fn basic_with_extension_checked() {
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
    run_basic(
        context,
        OwnerMode::External,
        TransferMode::CheckedOnly,
        ApproveMode::Checked,
    )
    .await;
}

#[tokio::test]
async fn basic_self_owned_with_extension() {
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
    run_basic(
        context,
        OwnerMode::SelfOwned,
        TransferMode::CheckedOnly,
        ApproveMode::Checked,
    )
    .await;
}
