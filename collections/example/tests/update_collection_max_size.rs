#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{setup, setup_collection, setup_metadata, setup_mint},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        transaction::{Transaction, TransactionError},
    },
    spl_token_collections_interface::{
        error::TokenCollectionsError, instruction::update_collection_max_size, state::Collection,
    },
    spl_token_metadata_interface::state::TokenMetadata,
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

#[tokio::test]
async fn success_update_collection_max_size() {
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup(&program_id).await;

    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let token_program_id = spl_token_2022::id();
    let decimals = 0;

    let update_authority_keypair = Keypair::new();
    let update_authority_pubkey = update_authority_keypair.pubkey();

    let metadata_keypair = Keypair::new();
    let metadata_pubkey = metadata_keypair.pubkey();

    let token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &metadata_pubkey,
        &update_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    let name = "My Cool Collection".to_string();
    let symbol = "COOL".to_string();
    let uri = "cool.collection.com".to_string();
    let token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        mint: *token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &token,
        &update_authority_pubkey,
        &token_metadata,
        &metadata_keypair,
        &mint_authority,
        payer.clone(),
    )
    .await;
    let mut context = context.lock().await;

    let collection_keypair = Keypair::new();
    let collection_pubkey = collection_keypair.pubkey();

    let collection_data = Collection {
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        max_size: Some(100),
        size: 0,
    };

    setup_collection(
        &mut context,
        &program_id,
        token.get_address(),
        &collection_data,
        &collection_keypair,
        &mint_authority,
    )
    .await;

    let new_max_size = Some(200);

    let transaction = Transaction::new_signed_with_payer(
        &[update_collection_max_size(
            &program_id,
            &collection_pubkey,
            &update_authority_pubkey,
            new_max_size,
        )],
        Some(&payer.pubkey()),
        &[&payer, &update_authority_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let fetched_collection_account = context
        .banks_client
        .get_account(collection_pubkey)
        .await
        .unwrap()
        .unwrap();
    let fetched_collection_state =
        TlvStateBorrowed::unpack(&fetched_collection_account.data).unwrap();
    let fetched_collection_data = fetched_collection_state
        .get_variable_len_value::<Collection>()
        .unwrap();
    assert_eq!(fetched_collection_data.max_size, new_max_size);
}

#[tokio::test]
async fn fail_authority_checks() {
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup(&program_id).await;

    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let token_program_id = spl_token_2022::id();
    let decimals = 0;

    let update_authority_keypair = Keypair::new();
    let update_authority_pubkey = update_authority_keypair.pubkey();

    let metadata_keypair = Keypair::new();
    let metadata_pubkey = metadata_keypair.pubkey();

    let token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &metadata_pubkey,
        &update_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    let name = "My Cool Collection".to_string();
    let symbol = "COOL".to_string();
    let uri = "cool.collection.com".to_string();
    let token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        mint: *token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &token,
        &update_authority_pubkey,
        &token_metadata,
        &metadata_keypair,
        &mint_authority,
        payer.clone(),
    )
    .await;
    let mut context = context.lock().await;

    let collection_keypair = Keypair::new();
    let collection_pubkey = collection_keypair.pubkey();

    let collection_data = Collection {
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        max_size: Some(100),
        size: 0,
    };

    setup_collection(
        &mut context,
        &program_id,
        token.get_address(),
        &collection_data,
        &collection_keypair,
        &mint_authority,
    )
    .await;

    let new_max_size = Some(200);

    // No signature
    let mut update_size_ix = update_collection_max_size(
        &program_id,
        &collection_pubkey,
        &update_authority_pubkey,
        new_max_size,
    );
    update_size_ix.accounts[1].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[update_size_ix],
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

    // Wrong authority
    let transaction = Transaction::new_signed_with_payer(
        &[update_collection_max_size(
            &program_id,
            &collection_pubkey,
            &collection_pubkey,
            new_max_size,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &collection_keypair],
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
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(TokenCollectionsError::IncorrectUpdateAuthority as u32),
        )
    );
}
