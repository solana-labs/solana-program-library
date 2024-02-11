#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError, signature::Signer, signer::keypair::Keypair,
        transaction::TransactionError, transport::TransportError,
    },
    spl_token_2022::{
        error::TokenError,
        extension::{
            immutable_owner::ImmutableOwner, transfer_fee::TransferFee, BaseStateWithExtensions,
            ExtensionType,
        },
    },
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
};

#[tokio::test]
async fn transfer() {
    let test_transfer_amount = 100;
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::NonTransferable])
        .await
        .unwrap();

    let TokenContext {
        mint_authority,
        token,
        token_unchecked,
        alice,
        bob,
        ..
    } = context.token_context.unwrap();

    // create token accounts
    token
        .create_auxiliary_token_account(&alice, &alice.pubkey())
        .await
        .unwrap();
    let alice_account = alice.pubkey();

    // immutable ownership is added to alice's account during initialization
    token
        .get_account_info(&alice_account)
        .await
        .unwrap()
        .get_extension::<ImmutableOwner>()
        .unwrap();

    token
        .create_auxiliary_token_account_with_extension_space(
            &bob,
            &bob.pubkey(),
            vec![ExtensionType::ImmutableOwner],
        )
        .await
        .unwrap();
    let bob_account = bob.pubkey();

    // mint to alice should be successful
    token
        .mint_to(
            &alice_account,
            &mint_authority.pubkey(),
            test_transfer_amount,
            &[&mint_authority],
        )
        .await
        .unwrap();

    token
        .mint_to(
            &bob_account,
            &mint_authority.pubkey(),
            test_transfer_amount,
            &[&mint_authority],
        )
        .await
        .unwrap();

    // self-transfer fails
    let error = token
        .transfer(
            &bob_account,
            &bob_account,
            &bob.pubkey(),
            test_transfer_amount,
            &[&bob],
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NonTransferable as u32)
            )
        )))
    );

    // regular transfer fails
    let error = token
        .transfer(
            &bob_account,
            &alice_account,
            &bob.pubkey(),
            test_transfer_amount,
            &[&bob],
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NonTransferable as u32)
            )
        )))
    );

    // regular unchecked transfer fails
    let error = token_unchecked
        .transfer(
            &bob_account,
            &alice_account,
            &bob.pubkey(),
            test_transfer_amount,
            &[&bob],
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NonTransferable as u32)
            )
        )))
    );
}

#[tokio::test]
async fn transfer_checked_with_fee() {
    let test_transfer_amount = 100;
    let maximum_fee = 10;
    let transfer_fee_basis_points = 100;

    let transfer_fee_config_authority = Keypair::new();
    let withdraw_withheld_authority = Keypair::new();

    let transfer_fee = TransferFee {
        epoch: 0.into(),
        transfer_fee_basis_points: transfer_fee_basis_points.into(),
        maximum_fee: maximum_fee.into(),
    };

    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::TransferFeeConfig {
                transfer_fee_config_authority: transfer_fee_config_authority.pubkey().into(),
                withdraw_withheld_authority: withdraw_withheld_authority.pubkey().into(),
                transfer_fee_basis_points,
                maximum_fee,
            },
            ExtensionInitializationParams::NonTransferable,
        ])
        .await
        .unwrap();

    let TokenContext {
        mint_authority,
        token,
        token_unchecked,
        alice,
        bob,
        ..
    } = context.token_context.unwrap();

    // create token accounts
    token
        .create_auxiliary_token_account_with_extension_space(
            &alice,
            &alice.pubkey(),
            vec![ExtensionType::ImmutableOwner],
        )
        .await
        .unwrap();
    let alice_account = alice.pubkey();

    token
        .create_auxiliary_token_account_with_extension_space(
            &bob,
            &bob.pubkey(),
            vec![ExtensionType::ImmutableOwner],
        )
        .await
        .unwrap();
    let bob_account = bob.pubkey();

    token
        .mint_to(
            &alice_account,
            &mint_authority.pubkey(),
            test_transfer_amount,
            &[&mint_authority],
        )
        .await
        .unwrap();

    // self-transfer fails
    let error = token
        .transfer(
            &alice_account,
            &alice_account,
            &alice.pubkey(),
            test_transfer_amount,
            &[&alice],
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NonTransferable as u32)
            )
        )))
    );

    // regular transfer fails
    let error = token
        .transfer(
            &alice_account,
            &bob_account,
            &alice.pubkey(),
            test_transfer_amount,
            &[&alice],
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NonTransferable as u32)
            )
        )))
    );

    // unchecked transfer fails
    let error = token_unchecked
        .transfer(
            &alice_account,
            &bob_account,
            &alice.pubkey(),
            test_transfer_amount,
            &[&alice],
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NonTransferable as u32)
            )
        )))
    );

    // self-transfer checked with fee fails
    let fee = transfer_fee.calculate_fee(test_transfer_amount).unwrap();
    let error = token
        .transfer_with_fee(
            &alice_account,
            &alice_account,
            &alice.pubkey(),
            test_transfer_amount,
            fee,
            &[&alice],
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NonTransferable as u32)
            )
        )))
    );

    // transfer checked with fee fails
    let fee = transfer_fee.calculate_fee(test_transfer_amount).unwrap();
    let error = token
        .transfer_with_fee(
            &alice_account,
            &bob_account,
            &alice.pubkey(),
            test_transfer_amount,
            fee,
            &[&alice],
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NonTransferable as u32)
            )
        )))
    );
}
