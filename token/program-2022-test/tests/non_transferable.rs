#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError, program_option::COption, pubkey::Pubkey, signature::Signer,
        signer::keypair::Keypair, transaction::TransactionError, transport::TransportError,
    },
    spl_token_2022::{
        error::TokenError,
        extension::{
            transfer_fee::{
                TransferFee, TransferFeeAmount, TransferFeeConfig, MAX_FEE_BASIS_POINTS,
            },
            ExtensionType,
        },
        instruction,
    },
    spl_token_client::{
        client::ProgramBanksClientProcessTransaction,
        token::{ExtensionInitializationParams, Token, TokenError as TokenClientError},
    },
    std::convert::TryInto,
};

#[tokio::test]
async fn transfer_checked() {
    let test_amount = 100;
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::NonTransferable])
        .await
        .unwrap();

    let TokenContext {
        decimals,
        mint_authority,
        token,
        alice,
        bob,
        ..
    } = context.token_context.unwrap();

    // create token accounts
    let alice_account = token
        .create_auxiliary_token_account(&alice, &alice.pubkey())
        .await
        .unwrap();

    let bob_account = token
        .create_auxiliary_token_account_with_extension_space(
            &bob,
            &bob.pubkey(),
            vec![ExtensionType::ImmutableOwner],
        )
        .await
        .unwrap();

    // mint fails because the account does not have immutable ownership
    let error = token
        .mint_to(
            &alice_account,
            &mint_authority.pubkey(),
            test_amount,
            Some(decimals),
            &vec![&mint_authority],
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NonTransferableNeedsImmutableOwnership as u32)
            )
        )))
    );

    // mint succeeds if the account has immutable ownership
    token
        .mint_to(
            &bob_account,
            &mint_authority.pubkey(),
            test_amount,
            Some(decimals),
            &vec![&mint_authority],
        )
        .await
        .unwrap();

    // self-transfer fails
    let error = token
        .transfer(
            &bob_account,
            &bob_account,
            &bob.pubkey(),
            test_amount,
            Some(decimals),
            &vec![&bob],
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
            test_amount,
            Some(decimals),
            &vec![&bob],
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
