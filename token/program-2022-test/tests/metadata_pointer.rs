#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::TestContext,
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{
        instruction::InstructionError, pubkey::Pubkey, signature::Signer, signer::keypair::Keypair,
        transaction::TransactionError, transport::TransportError,
    },
    spl_token_2022::{
        error::TokenError,
        extension::{metadata_pointer::MetadataPointer, BaseStateWithExtensions},
        instruction,
        processor::Processor,
    },
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    std::{convert::TryInto, sync::Arc},
};

fn setup_program_test() -> ProgramTest {
    let mut program_test = ProgramTest::default();
    program_test.prefer_bpf(false);
    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(Processor::process),
    );
    program_test
}

async fn setup(mint: Keypair, metadata_address: &Pubkey, authority: &Pubkey) -> TestContext {
    let program_test = setup_program_test();

    let context = program_test.start_with_context().await;
    let context = Arc::new(tokio::sync::Mutex::new(context));
    let mut context = TestContext {
        context,
        token_context: None,
    };
    context
        .init_token_with_mint_keypair_and_freeze_authority(
            mint,
            vec![ExtensionInitializationParams::MetadataPointer {
                authority: Some(*authority),
                metadata_address: Some(*metadata_address),
            }],
            None,
        )
        .await
        .unwrap();
    context
}

#[tokio::test]
async fn success_init() {
    let authority = Pubkey::new_unique();
    let metadata_address = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let token = setup(mint_keypair, &metadata_address, &authority)
        .await
        .token_context
        .take()
        .unwrap()
        .token;

    let state = token.get_mint_info().await.unwrap();
    assert!(state.base.is_initialized);
    let extension = state.get_extension::<MetadataPointer>().unwrap();
    assert_eq!(extension.authority, Some(authority).try_into().unwrap());
    assert_eq!(
        extension.metadata_address,
        Some(metadata_address).try_into().unwrap()
    );
}

#[tokio::test]
async fn fail_init_all_none() {
    let mut program_test = ProgramTest::default();
    program_test.prefer_bpf(false);
    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(Processor::process),
    );
    let context = program_test.start_with_context().await;
    let context = Arc::new(tokio::sync::Mutex::new(context));
    let mut context = TestContext {
        context,
        token_context: None,
    };
    let err = context
        .init_token_with_mint_keypair_and_freeze_authority(
            Keypair::new(),
            vec![ExtensionInitializationParams::MetadataPointer {
                authority: None,
                metadata_address: None,
            }],
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(TokenError::InvalidInstruction as u32)
            )
        )))
    );
}

#[tokio::test]
async fn set_authority() {
    let authority = Keypair::new();
    let metadata_address = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let token = setup(mint_keypair, &metadata_address, &authority.pubkey())
        .await
        .token_context
        .take()
        .unwrap()
        .token;
    let new_authority = Keypair::new();

    // fail, wrong signature
    let wrong = Keypair::new();
    let err = token
        .set_authority(
            token.get_address(),
            &wrong.pubkey(),
            Some(&new_authority.pubkey()),
            instruction::AuthorityType::MetadataPointer,
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
    token
        .set_authority(
            token.get_address(),
            &authority.pubkey(),
            Some(&new_authority.pubkey()),
            instruction::AuthorityType::MetadataPointer,
            &[&authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<MetadataPointer>().unwrap();
    assert_eq!(
        extension.authority,
        Some(new_authority.pubkey()).try_into().unwrap(),
    );

    // set to none
    token
        .set_authority(
            token.get_address(),
            &new_authority.pubkey(),
            None,
            instruction::AuthorityType::MetadataPointer,
            &[&new_authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<MetadataPointer>().unwrap();
    assert_eq!(extension.authority, None.try_into().unwrap(),);

    // fail set again
    let err = token
        .set_authority(
            token.get_address(),
            &new_authority.pubkey(),
            Some(&authority.pubkey()),
            instruction::AuthorityType::MetadataPointer,
            &[&new_authority],
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
}

#[tokio::test]
async fn update_metadata_address() {
    let authority = Keypair::new();
    let metadata_address = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let token = setup(mint_keypair, &metadata_address, &authority.pubkey())
        .await
        .token_context
        .take()
        .unwrap()
        .token;
    let new_metadata_address = Pubkey::new_unique();

    // fail, wrong signature
    let wrong = Keypair::new();
    let err = token
        .update_metadata_address(&wrong.pubkey(), Some(new_metadata_address), &[&wrong])
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
        .update_metadata_address(
            &authority.pubkey(),
            Some(new_metadata_address),
            &[&authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<MetadataPointer>().unwrap();
    assert_eq!(
        extension.metadata_address,
        Some(new_metadata_address).try_into().unwrap(),
    );

    // set to none
    token
        .update_metadata_address(&authority.pubkey(), None, &[&authority])
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<MetadataPointer>().unwrap();
    assert_eq!(extension.metadata_address, None.try_into().unwrap(),);
}
