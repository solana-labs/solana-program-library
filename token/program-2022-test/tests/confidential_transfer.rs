#![cfg(feature = "test-bpf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError, signature::Signer, signer::keypair::Keypair,
        transaction::TransactionError, transport::TransportError,
    },
    spl_token_2022::extension::confidential_transfer::ConfidentialTransferMint,
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
};

#[tokio::test]
async fn ct_initialize_and_update_mint() {
    let wrong_keypair = Keypair::new();

    let mut ct_mint = ConfidentialTransferMint::default();
    let ct_mint_authority = Keypair::new();
    ct_mint.authority = ct_mint_authority.pubkey();

    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::ConfidentialTransferMint { ct_mint },
        ])
        .await
        .unwrap();

    let TokenContext { token, .. } = context.token_context.unwrap();

    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<ConfidentialTransferMint>().unwrap();
    assert_eq!(*extension, ct_mint);

    // Change the authority
    let mut new_ct_mint = ConfidentialTransferMint::default();
    let new_ct_mint_authority = Keypair::new();
    new_ct_mint.authority = new_ct_mint_authority.pubkey();

    let err = token
        .confidential_transfer_update_mint(
            &wrong_keypair,
            new_ct_mint,
            Some(&new_ct_mint_authority),
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::MissingRequiredSignature)
        )))
    );
    token
        .confidential_transfer_update_mint(
            &ct_mint_authority,
            new_ct_mint,
            Some(&new_ct_mint_authority),
        )
        .await
        .unwrap();

    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<ConfidentialTransferMint>().unwrap();
    assert_eq!(*extension, new_ct_mint);

    // Clear the authority
    let new_ct_mint = ConfidentialTransferMint::default();
    token
        .confidential_transfer_update_mint(&new_ct_mint_authority, new_ct_mint, None)
        .await
        .unwrap();

    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<ConfidentialTransferMint>().unwrap();
    assert_eq!(*extension, new_ct_mint);
}
