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
        system_instruction,
        transaction::{Transaction, TransactionError},
    },
    spl_token_collections_interface::{
        error::TokenCollectionsError, instruction::create_collection, state::Collection,
    },
    spl_token_metadata_interface::state::TokenMetadata,
    spl_type_length_value::{
        error::TlvError,
        state::{TlvState, TlvStateBorrowed},
    },
};

#[tokio::test]
async fn success_create_collection() {
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup(&program_id).await;

    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let token_program_id = spl_token_2022::id();
    let decimals = 0;

    let update_authority = Pubkey::new_unique();

    let metadata_keypair = Keypair::new();
    let metadata_pubkey = metadata_keypair.pubkey();

    let token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &metadata_pubkey,
        &update_authority,
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
        update_authority: Some(update_authority).try_into().unwrap(),
        mint: *token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &token,
        &update_authority,
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
        update_authority: Some(update_authority).try_into().unwrap(),
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
    assert_eq!(fetched_collection_data, collection_data);

    // Fail doing it again, and change some params to ensure a new tx
    {
        let transaction = Transaction::new_signed_with_payer(
            &[create_collection(
                &program_id,
                &collection_keypair.pubkey(),
                token.get_address(),
                &mint_authority.pubkey(),
                None, // Intentionally changed params
                Some(500),
            )],
            Some(&payer.pubkey()),
            &[&payer, &mint_authority],
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
                InstructionError::Custom(TlvError::TypeAlreadyExists as u32)
            )
        );
    }
}

#[tokio::test]
async fn fail_without_authority_signature() {
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup(&program_id).await;

    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let token_program_id = spl_token_2022::id();
    let decimals = 0;

    let update_authority = Pubkey::new_unique();

    let metadata_keypair = Keypair::new();
    let metadata_pubkey = metadata_keypair.pubkey();

    let token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &metadata_pubkey,
        &update_authority,
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
        update_authority: Some(update_authority).try_into().unwrap(),
        mint: *token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &token,
        &update_authority,
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
        update_authority: Some(update_authority).try_into().unwrap(),
        max_size: Some(100),
        size: 0,
    };

    let rent = context.banks_client.get_rent().await.unwrap();
    let space = collection_data.tlv_size_of().unwrap();
    let rent_lamports = rent.minimum_balance(space);
    let mut create_collection_ix = create_collection(
        &program_id,
        &collection_keypair.pubkey(),
        token.get_address(),
        &mint_authority.pubkey(),
        Option::<Pubkey>::from(collection_data.update_authority.clone()),
        collection_data.max_size,
    );
    create_collection_ix.accounts[2].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &collection_pubkey,
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            create_collection_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &collection_keypair], // Missing mint authority
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
        TransactionError::InstructionError(1, InstructionError::MissingRequiredSignature,)
    );
}

#[tokio::test]
async fn fail_incorrect_authority() {
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup(&program_id).await;

    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let token_program_id = spl_token_2022::id();
    let decimals = 0;

    let update_authority = Pubkey::new_unique();

    let metadata_keypair = Keypair::new();
    let metadata_pubkey = metadata_keypair.pubkey();

    let token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &metadata_pubkey,
        &update_authority,
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
        update_authority: Some(update_authority).try_into().unwrap(),
        mint: *token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &token,
        &update_authority,
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
        update_authority: Some(update_authority).try_into().unwrap(),
        max_size: Some(100),
        size: 0,
    };

    let rent = context.banks_client.get_rent().await.unwrap();
    let space = collection_data.tlv_size_of().unwrap();
    let rent_lamports = rent.minimum_balance(space);
    let mut create_collection_ix = create_collection(
        &program_id,
        &collection_keypair.pubkey(),
        token.get_address(),
        &collection_keypair.pubkey(), // NOT the mint authority
        Option::<Pubkey>::from(collection_data.update_authority.clone()),
        collection_data.max_size,
    );
    create_collection_ix.accounts[2].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &collection_pubkey,
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            create_collection_ix,
        ],
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
            1,
            InstructionError::Custom(TokenCollectionsError::IncorrectMintAuthority as u32)
        )
    );
}
