#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{setup, setup_collection, setup_member, setup_metadata, setup_mint},
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
        error::TokenCollectionsError,
        instruction::create_member,
        state::{Collection, Member},
    },
    spl_token_metadata_interface::state::TokenMetadata,
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

#[tokio::test]
async fn success_create_member() {
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup(&program_id).await;

    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let token_program_id = spl_token_2022::id();
    let decimals = 0;

    let update_authority_keypair = Keypair::new();
    let update_authority_pubkey = update_authority_keypair.pubkey();

    let collection_metadata_keypair = Keypair::new();
    let collection_metadata_pubkey = collection_metadata_keypair.pubkey();

    let collection_token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &collection_metadata_pubkey,
        &update_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    let member_metadata_keypair = Keypair::new();
    let member_metadata_pubkey = member_metadata_keypair.pubkey();

    let member_token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &member_metadata_pubkey,
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
        mint: *collection_token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &collection_token,
        &update_authority_pubkey,
        &token_metadata,
        &collection_metadata_keypair,
        &mint_authority,
        payer.clone(),
    )
    .await;

    // For demonstration purposes, we'll set up _different_ metadata for
    // the collection member
    let name = "I'm a member of the Cool Collection!".to_string();
    let symbol = "YAY".to_string();
    let uri = "i.am.a.member".to_string();
    let token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        mint: *member_token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &member_token,
        &update_authority_pubkey,
        &token_metadata,
        &member_metadata_keypair,
        &mint_authority,
        payer.clone(),
    )
    .await;
    let mut context = context.lock().await;

    let collection_keypair = Keypair::new();
    let collection_pubkey = collection_keypair.pubkey();

    let collection = Collection {
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        max_size: Some(100),
        size: 0,
    };

    setup_collection(
        &mut context,
        &program_id,
        collection_token.get_address(),
        &collection,
        &collection_keypair,
        &mint_authority,
    )
    .await;

    let member_keypair = Keypair::new();
    let member_pubkey = member_keypair.pubkey();

    let member = Member {
        collection: collection_pubkey,
    };

    setup_member(
        &mut context,
        &program_id,
        member_token.get_address(),
        &collection_pubkey,
        collection_token.get_address(),
        &member_keypair,
        &mint_authority,
        &mint_authority,
    )
    .await;

    let fetched_member_account = context
        .banks_client
        .get_account(member_pubkey)
        .await
        .unwrap()
        .unwrap();
    let fetched_member_state = TlvStateBorrowed::unpack(&fetched_member_account.data).unwrap();
    let fetched_member = fetched_member_state
        .get_variable_len_value::<Member>()
        .unwrap();
    assert_eq!(fetched_member, member);
}

#[tokio::test]
async fn fail_without_authority_signature() {
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup(&program_id).await;

    let collection_mint_authority = Keypair::new();
    let collection_mint_authority_pubkey = collection_mint_authority.pubkey();

    let member_mint_authority = Keypair::new();
    let member_mint_authority_pubkey = member_mint_authority.pubkey();

    let token_program_id = spl_token_2022::id();
    let decimals = 0;

    let update_authority_keypair = Keypair::new();
    let update_authority_pubkey = update_authority_keypair.pubkey();

    let collection_metadata_keypair = Keypair::new();
    let collection_metadata_pubkey = collection_metadata_keypair.pubkey();

    let collection_token = setup_mint(
        &token_program_id,
        &collection_mint_authority_pubkey,
        &collection_metadata_pubkey,
        &update_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    let member_metadata_keypair = Keypair::new();
    let member_metadata_pubkey = member_metadata_keypair.pubkey();

    let member_token = setup_mint(
        &token_program_id,
        &member_mint_authority_pubkey,
        &member_metadata_pubkey,
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
        mint: *collection_token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &collection_token,
        &update_authority_pubkey,
        &token_metadata,
        &collection_metadata_keypair,
        &collection_mint_authority,
        payer.clone(),
    )
    .await;

    // For demonstration purposes, we'll set up _different_ metadata for
    // the collection member
    let name = "I'm a member of the Cool Collection!".to_string();
    let symbol = "YAY".to_string();
    let uri = "i.am.a.member".to_string();
    let token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        mint: *member_token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &member_token,
        &update_authority_pubkey,
        &token_metadata,
        &member_metadata_keypair,
        &member_mint_authority,
        payer.clone(),
    )
    .await;
    let mut context = context.lock().await;

    let collection_keypair = Keypair::new();
    let collection_pubkey = collection_keypair.pubkey();

    let collection = Collection {
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        max_size: Some(100),
        size: 0,
    };

    setup_collection(
        &mut context,
        &program_id,
        collection_token.get_address(),
        &collection,
        &collection_keypair,
        &collection_mint_authority,
    )
    .await;

    let member_keypair = Keypair::new();
    let _member_pubkey = member_keypair.pubkey();

    let member_data = Member {
        collection: collection_pubkey,
    };

    let rent = context.banks_client.get_rent().await.unwrap();

    let token_metadata_space = token_metadata.tlv_size_of().unwrap();
    let token_metadata_rent_lamports = rent.minimum_balance(token_metadata_space);

    let member_space = member_data.tlv_size_of().unwrap();
    let member_rent_lamports = rent.minimum_balance(member_space);

    // Fail missing member mint authority

    let mut create_member_ix = create_member(
        &program_id,
        &member_keypair.pubkey(),
        member_token.get_address(),
        &member_mint_authority.pubkey(),
        &collection_keypair.pubkey(),
        collection_token.get_address(),
        &collection_mint_authority.pubkey(),
    );
    create_member_ix.accounts[2].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &member_keypair.pubkey(),
                member_rent_lamports,
                member_space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                member_token.get_address(),
                token_metadata_rent_lamports,
            ),
            create_member_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &member_keypair, &collection_mint_authority], /* Missing member mint
                                                                         * authority */
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
        TransactionError::InstructionError(2, InstructionError::MissingRequiredSignature,)
    );

    // Fail missing collection mint authority

    let mut create_member_ix = create_member(
        &program_id,
        &member_keypair.pubkey(),
        member_token.get_address(),
        &member_mint_authority.pubkey(),
        &collection_keypair.pubkey(),
        collection_token.get_address(),
        &collection_mint_authority.pubkey(),
    );
    create_member_ix.accounts[5].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &member_keypair.pubkey(),
                member_rent_lamports,
                member_space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                member_token.get_address(),
                token_metadata_rent_lamports,
            ),
            create_member_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &member_keypair, &member_mint_authority], /* Missing collection mint
                                                                     * authority */
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
        TransactionError::InstructionError(2, InstructionError::MissingRequiredSignature,)
    );
}

#[tokio::test]
async fn fail_incorrect_authority() {
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup(&program_id).await;

    let collection_mint_authority = Keypair::new();
    let collection_mint_authority_pubkey = collection_mint_authority.pubkey();

    let member_mint_authority = Keypair::new();
    let member_mint_authority_pubkey = member_mint_authority.pubkey();

    let token_program_id = spl_token_2022::id();
    let decimals = 0;

    let update_authority_keypair = Keypair::new();
    let update_authority_pubkey = update_authority_keypair.pubkey();

    let collection_metadata_keypair = Keypair::new();
    let collection_metadata_pubkey = collection_metadata_keypair.pubkey();

    let collection_token = setup_mint(
        &token_program_id,
        &collection_mint_authority_pubkey,
        &collection_metadata_pubkey,
        &update_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    let member_metadata_keypair = Keypair::new();
    let member_metadata_pubkey = member_metadata_keypair.pubkey();

    let member_token = setup_mint(
        &token_program_id,
        &member_mint_authority_pubkey,
        &member_metadata_pubkey,
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
        mint: *collection_token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &collection_token,
        &update_authority_pubkey,
        &token_metadata,
        &collection_metadata_keypair,
        &collection_mint_authority,
        payer.clone(),
    )
    .await;

    // For demonstration purposes, we'll set up _different_ metadata for
    // the collection member
    let name = "I'm a member of the Cool Collection!".to_string();
    let symbol = "YAY".to_string();
    let uri = "i.am.a.member".to_string();
    let token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        mint: *member_token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &member_token,
        &update_authority_pubkey,
        &token_metadata,
        &member_metadata_keypair,
        &member_mint_authority,
        payer.clone(),
    )
    .await;
    let mut context = context.lock().await;

    let collection_keypair = Keypair::new();
    let collection_pubkey = collection_keypair.pubkey();

    let collection = Collection {
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        max_size: Some(100),
        size: 0,
    };

    setup_collection(
        &mut context,
        &program_id,
        collection_token.get_address(),
        &collection,
        &collection_keypair,
        &collection_mint_authority,
    )
    .await;

    let member_keypair = Keypair::new();
    let _member_pubkey = member_keypair.pubkey();

    let member_data = Member {
        collection: collection_pubkey,
    };

    let rent = context.banks_client.get_rent().await.unwrap();

    let token_metadata_space = token_metadata.tlv_size_of().unwrap();
    let token_metadata_rent_lamports = rent.minimum_balance(token_metadata_space);

    let member_space = member_data.tlv_size_of().unwrap();
    let member_rent_lamports = rent.minimum_balance(member_space);

    // Fail incorrect member mint authority

    let mut create_member_ix = create_member(
        &program_id,
        &member_keypair.pubkey(),
        member_token.get_address(),
        &collection_mint_authority.pubkey(), // NOT the member mint authority
        &collection_keypair.pubkey(),
        collection_token.get_address(),
        &collection_mint_authority.pubkey(),
    );
    create_member_ix.accounts[2].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &member_keypair.pubkey(),
                member_rent_lamports,
                member_space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                member_token.get_address(),
                token_metadata_rent_lamports,
            ),
            create_member_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &member_keypair, &collection_mint_authority],
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
            2,
            InstructionError::Custom(TokenCollectionsError::IncorrectMintAuthority as u32)
        )
    );

    // Fail missing collection mint authority

    let mut create_member_ix = create_member(
        &program_id,
        &member_keypair.pubkey(),
        member_token.get_address(),
        &member_mint_authority.pubkey(),
        &collection_keypair.pubkey(),
        collection_token.get_address(),
        &member_mint_authority.pubkey(), // NOT the collection mint authority
    );
    create_member_ix.accounts[5].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &member_keypair.pubkey(),
                member_rent_lamports,
                member_space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                member_token.get_address(),
                token_metadata_rent_lamports,
            ),
            create_member_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &member_keypair, &member_mint_authority],
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
            2,
            InstructionError::Custom(TokenCollectionsError::IncorrectMintAuthority as u32)
        )
    );
}
