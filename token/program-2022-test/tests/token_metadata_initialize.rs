#![cfg(feature = "test-sbf")]

mod program_test;
use {
    borsh::BorshDeserialize,
    program_test::TestContext,
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{
        instruction::InstructionError,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
    spl_token_2022::{error::TokenError, extension::BaseStateWithExtensions, processor::Processor},
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    spl_token_metadata_interface::{
        error::TokenMetadataError,
        state::{OptionalNonZeroPubkey, TokenMetadata},
    },
    std::{convert::TryInto, sync::Arc},
};

fn setup_program_test() -> ProgramTest {
    let mut program_test = ProgramTest::default();
    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(Processor::process),
    );
    program_test
}

async fn setup(mint: Keypair, authority: &Pubkey) -> TestContext {
    let program_test = setup_program_test();

    let context = program_test.start_with_context().await;
    let context = Arc::new(tokio::sync::Mutex::new(context));
    let mut context = TestContext {
        context,
        token_context: None,
    };
    let metadata_address = Some(mint.pubkey());
    context
        .init_token_with_mint_keypair_and_freeze_authority(
            mint,
            vec![ExtensionInitializationParams::MetadataPointer {
                authority: Some(*authority),
                metadata_address,
            }],
            None,
        )
        .await
        .unwrap();
    context
}

#[tokio::test]
async fn success_initialize() {
    let authority = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let mut test_context = setup(mint_keypair, &authority).await;
    let payer_pubkey = test_context.context.lock().await.payer.pubkey();
    let token_context = test_context.token_context.take().unwrap();

    let update_authority = Pubkey::new_unique();
    let name = "MyTokenNeedsMetadata".to_string();
    let symbol = "NEEDS".to_string();
    let uri = "my.token.needs.metadata".to_string();
    let token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority).try_into().unwrap(),
        mint: *token_context.token.get_address(),
        ..Default::default()
    };

    // fails without more lamports for new rent-exemption
    let error = token_context
        .token
        .initialize_token_metadata(
            &update_authority,
            &token_context.mint_authority.pubkey(),
            token_metadata.name.clone(),
            token_metadata.symbol.clone(),
            token_metadata.uri.clone(),
            &[&token_context.mint_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InsufficientFundsForRent { account_index: 2 }
        )))
    );

    token_context
        .token
        .initialize_token_metadata_with_rent_transfer(
            &payer_pubkey,
            &update_authority,
            &token_context.mint_authority.pubkey(),
            token_metadata.name.clone(),
            token_metadata.symbol.clone(),
            token_metadata.uri.clone(),
            &[&token_context.mint_authority],
        )
        .await
        .unwrap();

    // check that the data is correct
    let mint_info = token_context.token.get_mint_info().await.unwrap();
    let metadata_bytes = mint_info.get_extension_bytes::<TokenMetadata>().unwrap();
    let fetched_metadata = TokenMetadata::try_from_slice(&metadata_bytes).unwrap();
    assert_eq!(fetched_metadata, token_metadata);
}
