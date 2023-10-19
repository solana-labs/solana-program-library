#![cfg(feature = "test-sbf")]
#![allow(clippy::items_after_test_module)]

mod program_test;
use {
    program_test::TestContext,
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{
        instruction::InstructionError, pubkey::Pubkey, signature::Signer, signer::keypair::Keypair,
        transaction::TransactionError, transport::TransportError,
    },
    spl_token_2022::{extension::BaseStateWithExtensions, processor::Processor},
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    spl_token_metadata_interface::{
        error::TokenMetadataError,
        instruction::update_field,
        state::{Field, TokenMetadata},
    },
    std::{convert::TryInto, sync::Arc},
    test_case::test_case,
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

#[test_case(Field::Name, "This is my larger name".to_string() ; "larger name")]
#[test_case(Field::Name, "Smaller".to_string() ; "smaller name")]
#[test_case(Field::Key("my new field".to_string()), "Some data for the new field!".to_string() ; "new field")]
#[tokio::test]
async fn success_update(field: Field, value: String) {
    let authority = Keypair::new();
    let mint_keypair = Keypair::new();
    let mut test_context = setup(mint_keypair, &authority.pubkey()).await;
    let payer_pubkey = test_context.context.lock().await.payer.pubkey();
    let token_context = test_context.token_context.take().unwrap();

    let update_authority = Keypair::new();
    let name = "MySuperCoolToken".to_string();
    let symbol = "MINE".to_string();
    let uri = "my.super.cool.token".to_string();
    let mut token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority.pubkey()).try_into().unwrap(),
        mint: *token_context.token.get_address(),
        ..Default::default()
    };

    token_context
        .token
        .token_metadata_initialize_with_rent_transfer(
            &payer_pubkey,
            &update_authority.pubkey(),
            &token_context.mint_authority.pubkey(),
            token_metadata.name.clone(),
            token_metadata.symbol.clone(),
            token_metadata.uri.clone(),
            &[&token_context.mint_authority],
        )
        .await
        .unwrap();

    let old_space = token_metadata.tlv_size_of().unwrap();
    token_metadata.update(field.clone(), value.clone());
    let new_space = token_metadata.tlv_size_of().unwrap();

    if new_space > old_space {
        let error = token_context
            .token
            .token_metadata_update_field(
                &update_authority.pubkey(),
                field.clone(),
                value.clone(),
                &[&update_authority],
            )
            .await
            .unwrap_err();
        assert_eq!(
            error,
            TokenClientError::Client(Box::new(TransportError::TransactionError(
                TransactionError::InsufficientFundsForRent { account_index: 2 }
            )))
        );
    }

    // transfer required lamports
    token_context
        .token
        .token_metadata_update_field_with_rent_transfer(
            &payer_pubkey,
            &update_authority.pubkey(),
            field,
            value,
            None,
            &[&update_authority],
        )
        .await
        .unwrap();

    // check that the account looks good
    let mint_info = token_context.token.get_mint_info().await.unwrap();
    let fetched_metadata = mint_info
        .get_variable_len_extension::<TokenMetadata>()
        .unwrap();
    assert_eq!(fetched_metadata, token_metadata);
}

#[tokio::test]
async fn fail_authority_checks() {
    let authority = Keypair::new();
    let mint_keypair = Keypair::new();
    let mut test_context = setup(mint_keypair, &authority.pubkey()).await;
    let payer_pubkey = test_context.context.lock().await.payer.pubkey();
    let token_context = test_context.token_context.take().unwrap();

    let update_authority = Keypair::new();
    token_context
        .token
        .token_metadata_initialize_with_rent_transfer(
            &payer_pubkey,
            &update_authority.pubkey(),
            &token_context.mint_authority.pubkey(),
            "MySuperCoolToken".to_string(),
            "MINE".to_string(),
            "my.super.cool.token".to_string(),
            &[&token_context.mint_authority],
        )
        .await
        .unwrap();

    // no signature
    let mut instruction = update_field(
        &spl_token_2022::id(),
        token_context.token.get_address(),
        &update_authority.pubkey(),
        Field::Name,
        "new_name".to_string(),
    );
    instruction.accounts[1].is_signer = false;

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

    // wrong authority
    let wrong_authority = Keypair::new();
    let error = token_context
        .token
        .token_metadata_update_field(
            &wrong_authority.pubkey(),
            Field::Name,
            "new_name".to_string(),
            &[&wrong_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenMetadataError::IncorrectUpdateAuthority as u32)
            )
        )))
    );
}
