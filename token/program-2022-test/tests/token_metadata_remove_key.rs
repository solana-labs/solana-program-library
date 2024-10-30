#![cfg(feature = "test-sbf")]

mod program_test;
use {
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
    spl_token_2022::{extension::BaseStateWithExtensions, processor::Processor},
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    spl_token_metadata_interface::{
        error::TokenMetadataError,
        instruction::remove_key,
        state::{Field, TokenMetadata},
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
async fn success_remove() {
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

    let key = "new_field, wow!".to_string();
    let field = Field::Key(key.clone());
    let value = "so impressed with the new field, don't know what to put here".to_string();
    token_metadata.update(field.clone(), value.clone());

    // add the field
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

    // now remove it
    token_context
        .token
        .token_metadata_remove_key(
            &update_authority.pubkey(),
            key.clone(),
            false, // idempotent
            &[&update_authority],
        )
        .await
        .unwrap();

    // check that the data is correct
    token_metadata.remove_key(&key);
    let mint = token_context.token.get_mint_info().await.unwrap();
    let fetched_metadata = mint.get_variable_len_extension::<TokenMetadata>().unwrap();
    assert_eq!(fetched_metadata, token_metadata);

    // succeed again with idempotent flag
    token_context
        .token
        .token_metadata_remove_key(
            &update_authority.pubkey(),
            key.clone(),
            true, // idempotent
            &[&update_authority],
        )
        .await
        .unwrap();

    // fail doing it again without idempotent flag
    {
        // Be really sure to have a new latest blockhash since this keeps failing in CI
        let mut context = test_context.context.lock().await;
        context.get_new_latest_blockhash().await.unwrap();
        context.get_new_latest_blockhash().await.unwrap();
    }
    let error = token_context
        .token
        .token_metadata_remove_key(
            &update_authority.pubkey(),
            key.clone(),
            false, // idempotent
            &[&update_authority],
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenMetadataError::KeyNotFound as u32)
            )
        )))
    );
}

#[tokio::test]
async fn fail_authority_checks() {
    let program_id = spl_token_2022::id();
    let authority = Keypair::new();
    let mint_keypair = Keypair::new();
    let mint_pubkey = mint_keypair.pubkey();
    let mut test_context = setup(mint_keypair, &authority.pubkey()).await;
    let payer_pubkey = test_context.context.lock().await.payer.pubkey();
    let token_context = test_context.token_context.take().unwrap();

    let update_authority = Keypair::new();
    let name = "MySuperCoolToken".to_string();
    let symbol = "MINE".to_string();
    let uri = "my.super.cool.token".to_string();
    let token_metadata = TokenMetadata {
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

    let key = "new_field, wow!".to_string();

    // wrong authority
    let error = token_context
        .token
        .token_metadata_remove_key(
            &payer_pubkey,
            key,
            true, // idempotent
            &[] as &[&dyn Signer; 0],
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

    // no signature
    let context = test_context.context.lock().await;
    let mut instruction = remove_key(
        &program_id,
        &mint_pubkey,
        &update_authority.pubkey(),
        "new_name".to_string(),
        true, // idempotent
    );
    instruction.accounts[1].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(0, InstructionError::MissingRequiredSignature,)
    );
}
