#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{setup, setup_metadata, setup_mint, setup_original_print},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        system_instruction,
        transaction::{Transaction, TransactionError},
    },
    spl_token_editions_interface::{
        error::TokenEditionsError, instruction::create_original, state::Original,
    },
    spl_token_metadata_interface::state::TokenMetadata,
    spl_type_length_value::{
        error::TlvError,
        state::{TlvState, TlvStateBorrowed},
    },
};

#[tokio::test]
async fn success_create_original() {
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

    let name = "My Cool Original Print".to_string();
    let symbol = "COOL".to_string();
    let uri = "cool.original.print".to_string();
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

    let original_keypair = Keypair::new();
    let original_pubkey = original_keypair.pubkey();

    let original_print = Original {
        update_authority: Some(update_authority).try_into().unwrap(),
        max_supply: Some(100),
        supply: 0,
    };

    setup_original_print(
        &mut context,
        &program_id,
        &metadata_pubkey,
        token.get_address(),
        &original_print,
        &original_keypair,
        &mint_authority,
    )
    .await;

    let fetched_original_account = context
        .banks_client
        .get_account(original_pubkey)
        .await
        .unwrap()
        .unwrap();
    let fetched_original_state = TlvStateBorrowed::unpack(&fetched_original_account.data).unwrap();
    let fetched_original_print = fetched_original_state
        .get_variable_len_value::<Original>()
        .unwrap();
    assert_eq!(fetched_original_print, original_print);

    // Fail doing it again, and change some params to ensure a new tx
    {
        let transaction = Transaction::new_signed_with_payer(
            &[create_original(
                &program_id,
                &original_keypair.pubkey(),
                &metadata_pubkey,
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

    let name = "My Cool Original Print".to_string();
    let symbol = "COOL".to_string();
    let uri = "cool.original.print".to_string();
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

    let original_keypair = Keypair::new();
    let original_pubkey = original_keypair.pubkey();

    let original_print = Original {
        update_authority: Some(update_authority).try_into().unwrap(),
        max_supply: Some(100),
        supply: 0,
    };

    let rent = context.banks_client.get_rent().await.unwrap();
    let space = original_print.tlv_size_of().unwrap();
    let rent_lamports = rent.minimum_balance(space);
    let mut create_original_ix = create_original(
        &program_id,
        &original_keypair.pubkey(),
        &metadata_pubkey,
        token.get_address(),
        &mint_authority.pubkey(),
        Option::<Pubkey>::from(original_print.update_authority.clone()),
        original_print.max_supply,
    );
    create_original_ix.accounts[3].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &original_pubkey,
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            create_original_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &original_keypair], // Missing mint authority
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

    let name = "My Cool Original Print".to_string();
    let symbol = "COOL".to_string();
    let uri = "cool.original.print".to_string();
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

    let original_keypair = Keypair::new();
    let original_pubkey = original_keypair.pubkey();

    let original_print = Original {
        update_authority: Some(update_authority).try_into().unwrap(),
        max_supply: Some(100),
        supply: 0,
    };

    let rent = context.banks_client.get_rent().await.unwrap();
    let space = original_print.tlv_size_of().unwrap();
    let rent_lamports = rent.minimum_balance(space);
    let mut create_original_ix = create_original(
        &program_id,
        &original_keypair.pubkey(),
        &metadata_pubkey,
        token.get_address(),
        &original_keypair.pubkey(), // NOT the mint authority
        Option::<Pubkey>::from(original_print.update_authority.clone()),
        original_print.max_supply,
    );
    create_original_ix.accounts[3].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &original_pubkey,
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            create_original_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &original_keypair],
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
            InstructionError::Custom(TokenEditionsError::IncorrectMintAuthority as u32)
        )
    );
}
