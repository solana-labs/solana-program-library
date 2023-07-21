#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{setup, setup_metadata, setup_mint},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        system_instruction,
        transaction::{Transaction, TransactionError},
    },
    spl_token_metadata_interface::{
        error::TokenMetadataError, instruction::initialize, state::TokenMetadata,
    },
    spl_type_length_value::{
        error::TlvError,
        state::{TlvState, TlvStateBorrowed},
    },
};

#[tokio::test]
async fn success_initialize() {
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup(&program_id).await;

    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let token_program_id = spl_token_2022::id();
    let decimals = 2;
    let token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;
    let mut context = context.lock().await;

    let update_authority = Pubkey::new_unique();
    let name = "MySuperCoolToken".to_string();
    let symbol = "MINE".to_string();
    let uri = "my.super.cool.token".to_string();
    let token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority).try_into().unwrap(),
        mint: *token.get_address(),
        ..Default::default()
    };

    let metadata_keypair = Keypair::new();
    let metadata_pubkey = metadata_keypair.pubkey();

    setup_metadata(
        &mut context,
        &program_id,
        token.get_address(),
        &token_metadata,
        &metadata_keypair,
        &mint_authority,
    )
    .await;

    // check that the data is correct
    let fetched_metadata_account = context
        .banks_client
        .get_account(metadata_pubkey)
        .await
        .unwrap()
        .unwrap();
    let fetched_metadata_state = TlvStateBorrowed::unpack(&fetched_metadata_account.data).unwrap();
    let fetched_metadata = fetched_metadata_state
        .get_first_variable_len_value::<TokenMetadata>()
        .unwrap();
    assert_eq!(fetched_metadata, token_metadata);

    // fail doing it again, and reverse some params to ensure a new tx
    {
        let transaction = Transaction::new_signed_with_payer(
            &[initialize(
                &program_id,
                &metadata_pubkey,
                &update_authority,
                token.get_address(),
                &mint_authority_pubkey,
                token_metadata.symbol.clone(), // intentionally reversed!
                token_metadata.name.clone(),
                token_metadata.uri.clone(),
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
    let decimals = 2;
    let token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;
    let mut context = context.lock().await;

    let update_authority = Pubkey::new_unique();
    let name = "MySuperCoolToken".to_string();
    let symbol = "MINE".to_string();
    let uri = "my.super.cool.token".to_string();
    let token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority).try_into().unwrap(),
        mint: *token.get_address(),
        ..Default::default()
    };

    let metadata_keypair = Keypair::new();
    let metadata_pubkey = metadata_keypair.pubkey();
    let rent = context.banks_client.get_rent().await.unwrap();
    let space = token_metadata.tlv_size_of().unwrap();
    let rent_lamports = rent.minimum_balance(space);
    let mut initialize_ix = initialize(
        &program_id,
        &metadata_pubkey,
        &update_authority,
        token.get_address(),
        &mint_authority_pubkey,
        token_metadata.name.clone(),
        token_metadata.symbol.clone(),
        token_metadata.uri.clone(),
    );
    initialize_ix.accounts[3].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &metadata_pubkey,
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            initialize_ix,
        ],
        Some(&payer.pubkey()),
        &[&payer, &metadata_keypair],
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
    let decimals = 2;
    let token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;
    let mut context = context.lock().await;

    let update_authority = Pubkey::new_unique();
    let name = "MySuperCoolToken".to_string();
    let symbol = "MINE".to_string();
    let uri = "my.super.cool.token".to_string();
    let token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority).try_into().unwrap(),
        mint: *token.get_address(),
        ..Default::default()
    };

    let metadata_keypair = Keypair::new();
    let metadata_pubkey = metadata_keypair.pubkey();
    let rent = context.banks_client.get_rent().await.unwrap();
    let space = token_metadata.tlv_size_of().unwrap();
    let rent_lamports = rent.minimum_balance(space);
    let mut initialize_ix = initialize(
        &program_id,
        &metadata_pubkey,
        &update_authority,
        token.get_address(),
        &metadata_pubkey,
        token_metadata.name.clone(),
        token_metadata.symbol.clone(),
        token_metadata.uri.clone(),
    );
    initialize_ix.accounts[3].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &metadata_pubkey,
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            initialize_ix,
        ],
        Some(&payer.pubkey()),
        &[&payer, &metadata_keypair],
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
            InstructionError::Custom(TokenMetadataError::IncorrectMintAuthority as u32)
        )
    );
}
