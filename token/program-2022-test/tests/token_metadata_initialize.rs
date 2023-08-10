#![cfg(feature = "test-sbf")]

mod program_test;
use {
    borsh::BorshDeserialize,
    program_test::TestContext,
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{
        instruction::InstructionError, pubkey::Pubkey, signature::Signer, signer::keypair::Keypair,
        transaction::TransactionError, transport::TransportError,
    },
    spl_token_2022::{error::TokenError, extension::BaseStateWithExtensions, processor::Processor},
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    spl_token_metadata_interface::{error::TokenMetadataError, state::TokenMetadata},
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
        .token_metadata_initialize(
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

    // fail wrong signer
    let not_mint_authority = Keypair::new();
    let error = token_context
        .token
        .token_metadata_initialize_with_rent_transfer(
            &payer_pubkey,
            &update_authority,
            &not_mint_authority.pubkey(),
            token_metadata.name.clone(),
            token_metadata.symbol.clone(),
            token_metadata.uri.clone(),
            &[&not_mint_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(TokenMetadataError::IncorrectMintAuthority as u32)
            )
        )))
    );

    token_context
        .token
        .token_metadata_initialize_with_rent_transfer(
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
    let fetched_metadata = TokenMetadata::try_from_slice(metadata_bytes).unwrap();
    assert_eq!(fetched_metadata, token_metadata);

    // fail double-init
    let error = token_context
        .token
        .token_metadata_initialize_with_rent_transfer(
            &payer_pubkey,
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
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::ExtensionAlreadyInitialized as u32)
            )
        )))
    );
}

#[tokio::test]
async fn fail_without_metadata_pointer() {
    let mut test_context = {
        let mint_keypair = Keypair::new();
        let program_test = setup_program_test();
        let context = program_test.start_with_context().await;
        let context = Arc::new(tokio::sync::Mutex::new(context));
        let mut context = TestContext {
            context,
            token_context: None,
        };
        context
            .init_token_with_mint_keypair_and_freeze_authority(mint_keypair, vec![], None)
            .await
            .unwrap();
        context
    };

    let payer_pubkey = test_context.context.lock().await.payer.pubkey();
    let token_context = test_context.token_context.take().unwrap();

    let error = token_context
        .token
        .token_metadata_initialize_with_rent_transfer(
            &payer_pubkey,
            &Pubkey::new_unique(),
            &token_context.mint_authority.pubkey(),
            "Name".to_string(),
            "Symbol".to_string(),
            "URI".to_string(),
            &[&token_context.mint_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(TokenError::InvalidExtensionCombination as u32)
            )
        )))
    );
}

#[tokio::test]
async fn fail_init_in_another_mint() {
    let authority = Pubkey::new_unique();
    let first_mint_keypair = Keypair::new();
    let first_mint = first_mint_keypair.pubkey();
    let mut test_context = setup(first_mint_keypair, &authority).await;
    let second_mint_keypair = Keypair::new();
    let second_mint = second_mint_keypair.pubkey();
    test_context
        .init_token_with_mint_keypair_and_freeze_authority(
            second_mint_keypair,
            vec![ExtensionInitializationParams::MetadataPointer {
                authority: Some(authority),
                metadata_address: Some(second_mint),
            }],
            None,
        )
        .await
        .unwrap();

    let token_context = test_context.token_context.take().unwrap();

    let error = token_context
        .token
        .process_ixs(
            &[spl_token_metadata_interface::instruction::initialize(
                &spl_token_2022::id(),
                &first_mint,
                &Pubkey::new_unique(),
                token_context.token.get_address(),
                &token_context.mint_authority.pubkey(),
                "Name".to_string(),
                "Symbol".to_string(),
                "URI".to_string(),
            )],
            &[&token_context.mint_authority],
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintMismatch as u32)
            )
        )))
    );
}

#[tokio::test]
async fn fail_without_signature() {
    let authority = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let mut test_context = setup(mint_keypair, &authority).await;

    let token_context = test_context.token_context.take().unwrap();

    let mut instruction = spl_token_metadata_interface::instruction::initialize(
        &spl_token_2022::id(),
        token_context.token.get_address(),
        &Pubkey::new_unique(),
        token_context.token.get_address(),
        &token_context.mint_authority.pubkey(),
        "Name".to_string(),
        "Symbol".to_string(),
        "URI".to_string(),
    );
    instruction.accounts[3].is_signer = false;
    let error = token_context
        .token
        .process_ixs(&[instruction], &[] as &[&dyn Signer; 0]) // yuck, but the compiler needs it
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::MissingRequiredSignature)
        )))
    );
}
