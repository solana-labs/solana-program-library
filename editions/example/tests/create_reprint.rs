#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{setup, setup_metadata, setup_mint, setup_original_print, setup_reprint},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        system_instruction,
        transaction::{Transaction, TransactionError},
    },
    spl_token_2022::error::TokenError,
    spl_token_editions_interface::{
        error::TokenEditionsError,
        instruction::create_reprint,
        state::{Original, Reprint},
    },
    spl_token_metadata_interface::state::TokenMetadata,
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

#[tokio::test]
async fn success_create_reprint() {
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup(&program_id).await;

    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let token_program_id = spl_token_2022::id();
    let metadata_program_id = spl_token_2022::id();
    let decimals = 0;

    let update_authority_keypair = Keypair::new();
    let update_authority_pubkey = update_authority_keypair.pubkey();

    let original_metadata_keypair = Keypair::new();
    let original_metadata_pubkey = original_metadata_keypair.pubkey();

    let original_token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &original_metadata_pubkey,
        &update_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    let reprint_metadata_keypair = Keypair::new();
    let reprint_metadata_pubkey = reprint_metadata_keypair.pubkey();

    let reprint_token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &reprint_metadata_pubkey,
        &update_authority_pubkey,
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
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        mint: *original_token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &original_token,
        &update_authority_pubkey,
        &token_metadata,
        &original_metadata_keypair,
        &mint_authority,
        payer.clone(),
    )
    .await;
    let mut context = context.lock().await;

    let original_keypair = Keypair::new();
    let original_pubkey = original_keypair.pubkey();

    let original_print = Original {
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        max_supply: Some(100),
        supply: 0,
    };

    setup_original_print(
        &mut context,
        &program_id,
        &original_metadata_pubkey,
        original_token.get_address(),
        &original_print,
        &original_keypair,
        &mint_authority,
    )
    .await;

    let reprint_keypair = Keypair::new();
    let reprint_pubkey = reprint_keypair.pubkey();

    let reprint = Reprint {
        original: original_pubkey,
        copy: 1,
    };

    setup_reprint(
        &mut context,
        &program_id,
        &reprint_metadata_pubkey,
        reprint_token.get_address(),
        &original_pubkey,
        &original_metadata_pubkey,
        original_token.get_address(),
        &metadata_program_id,
        &reprint,
        &token_metadata,
        &reprint_keypair,
        &update_authority_keypair,
        &mint_authority,
    )
    .await;

    let fetched_reprint_account = context
        .banks_client
        .get_account(reprint_pubkey)
        .await
        .unwrap()
        .unwrap();
    let fetched_reprint_state = TlvStateBorrowed::unpack(&fetched_reprint_account.data).unwrap();
    let fetched_reprint = fetched_reprint_state
        .get_variable_len_value::<Reprint>()
        .unwrap();
    assert_eq!(fetched_reprint, reprint);

    // Fail trying to create a copy in the same account as the original
    {
        let transaction = Transaction::new_signed_with_payer(
            &[create_reprint(
                &program_id,
                &original_keypair.pubkey(),   // Reprint
                &original_metadata_pubkey,    // Reprint
                original_token.get_address(), // Reprint
                &original_keypair.pubkey(),
                &update_authority_pubkey,
                &original_metadata_pubkey,
                original_token.get_address(),
                &mint_authority.pubkey(),
                &metadata_program_id,
            )],
            Some(&payer.pubkey()),
            &[&payer, &update_authority_keypair, &mint_authority],
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
                InstructionError::Custom(TokenError::ExtensionAlreadyInitialized as u32)
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
    let metadata_program_id = spl_token_2022::id();
    let decimals = 0;

    let update_authority_keypair = Keypair::new();
    let update_authority_pubkey = update_authority_keypair.pubkey();

    let original_metadata_keypair = Keypair::new();
    let original_metadata_pubkey = original_metadata_keypair.pubkey();

    let original_token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &original_metadata_pubkey,
        &update_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    let reprint_metadata_keypair = Keypair::new();
    let reprint_metadata_pubkey = reprint_metadata_keypair.pubkey();

    let reprint_token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &reprint_metadata_pubkey,
        &update_authority_pubkey,
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
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        mint: *original_token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &original_token,
        &update_authority_pubkey,
        &token_metadata,
        &original_metadata_keypair,
        &mint_authority,
        payer.clone(),
    )
    .await;
    let mut context = context.lock().await;

    let original_keypair = Keypair::new();
    let original_pubkey = original_keypair.pubkey();

    let original_print = Original {
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        max_supply: Some(100),
        supply: 0,
    };

    setup_original_print(
        &mut context,
        &program_id,
        &original_metadata_pubkey,
        original_token.get_address(),
        &original_print,
        &original_keypair,
        &mint_authority,
    )
    .await;

    let reprint_keypair = Keypair::new();
    let _reprint_pubkey = reprint_keypair.pubkey();

    let reprint_data = Reprint {
        original: original_pubkey,
        copy: 1,
    };

    let rent = context.banks_client.get_rent().await.unwrap();

    let token_metadata_space = token_metadata.tlv_size_of().unwrap();
    let token_metadata_rent_lamports = rent.minimum_balance(token_metadata_space);

    let reprint_space = reprint_data.tlv_size_of().unwrap();
    let reprint_rent_lamports = rent.minimum_balance(reprint_space);

    // Fail missing update authority

    let mut create_reprint_ix = create_reprint(
        &program_id,
        &reprint_keypair.pubkey(),
        &reprint_metadata_pubkey,
        reprint_token.get_address(),
        &original_pubkey,
        &update_authority_pubkey,
        &original_metadata_pubkey,
        original_token.get_address(),
        &mint_authority.pubkey(),
        &metadata_program_id,
    );
    create_reprint_ix.accounts[4].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &reprint_keypair.pubkey(),
                reprint_rent_lamports,
                reprint_space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                reprint_token.get_address(),
                token_metadata_rent_lamports,
            ),
            create_reprint_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &reprint_keypair, &mint_authority], // Missing update authority
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

    // Fail missing mint authority

    let mut create_reprint_ix = create_reprint(
        &program_id,
        &reprint_keypair.pubkey(),
        &reprint_metadata_pubkey,
        reprint_token.get_address(),
        &original_pubkey,
        &update_authority_pubkey,
        &original_metadata_pubkey,
        original_token.get_address(),
        &mint_authority.pubkey(),
        &metadata_program_id,
    );
    create_reprint_ix.accounts[7].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &reprint_keypair.pubkey(),
                reprint_rent_lamports,
                reprint_space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                reprint_token.get_address(),
                token_metadata_rent_lamports,
            ),
            create_reprint_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &reprint_keypair, &update_authority_keypair], // Missing mint authority
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

    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let token_program_id = spl_token_2022::id();
    let metadata_program_id = spl_token_2022::id();
    let decimals = 0;

    let update_authority_keypair = Keypair::new();
    let update_authority_pubkey = update_authority_keypair.pubkey();

    let original_metadata_keypair = Keypair::new();
    let original_metadata_pubkey = original_metadata_keypair.pubkey();

    let original_token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &original_metadata_pubkey,
        &update_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    let reprint_metadata_keypair = Keypair::new();
    let reprint_metadata_pubkey = reprint_metadata_keypair.pubkey();

    let reprint_token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &reprint_metadata_pubkey,
        &update_authority_pubkey,
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
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        mint: *original_token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &original_token,
        &update_authority_pubkey,
        &token_metadata,
        &original_metadata_keypair,
        &mint_authority,
        payer.clone(),
    )
    .await;
    let mut context = context.lock().await;

    let original_keypair = Keypair::new();
    let original_pubkey = original_keypair.pubkey();

    let original_print = Original {
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        max_supply: Some(100),
        supply: 0,
    };

    setup_original_print(
        &mut context,
        &program_id,
        &original_metadata_pubkey,
        original_token.get_address(),
        &original_print,
        &original_keypair,
        &mint_authority,
    )
    .await;

    let reprint_keypair = Keypair::new();
    let reprint_pubkey = reprint_keypair.pubkey();

    let reprint_data = Reprint {
        original: original_pubkey,
        copy: 1,
    };

    let rent = context.banks_client.get_rent().await.unwrap();

    let token_metadata_space = token_metadata.tlv_size_of().unwrap();
    let token_metadata_rent_lamports = rent.minimum_balance(token_metadata_space);

    let reprint_space = reprint_data.tlv_size_of().unwrap();
    let reprint_rent_lamports = rent.minimum_balance(reprint_space);

    // Fail incorrect update authority

    let mut create_reprint_ix = create_reprint(
        &program_id,
        &reprint_keypair.pubkey(),
        &reprint_metadata_pubkey,
        reprint_token.get_address(),
        &original_pubkey,
        &reprint_pubkey, // NOT the update authority
        &original_metadata_pubkey,
        original_token.get_address(),
        &mint_authority.pubkey(),
        &metadata_program_id,
    );
    create_reprint_ix.accounts[4].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &reprint_keypair.pubkey(),
                reprint_rent_lamports,
                reprint_space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                reprint_token.get_address(),
                token_metadata_rent_lamports,
            ),
            create_reprint_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &reprint_keypair, &mint_authority],
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
            InstructionError::Custom(TokenEditionsError::IncorrectUpdateAuthority as u32)
        )
    );

    // Fail missing mint authority

    let mut create_reprint_ix = create_reprint(
        &program_id,
        &reprint_keypair.pubkey(),
        &reprint_metadata_pubkey,
        reprint_token.get_address(),
        &original_pubkey,
        &update_authority_pubkey,
        &original_metadata_pubkey,
        original_token.get_address(),
        &reprint_pubkey, // NOT the mint authority
        &metadata_program_id,
    );
    create_reprint_ix.accounts[7].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &reprint_keypair.pubkey(),
                reprint_rent_lamports,
                reprint_space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                reprint_token.get_address(),
                token_metadata_rent_lamports,
            ),
            create_reprint_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &reprint_keypair, &update_authority_keypair],
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
            InstructionError::Custom(TokenEditionsError::IncorrectMintAuthority as u32)
        )
    );
}
