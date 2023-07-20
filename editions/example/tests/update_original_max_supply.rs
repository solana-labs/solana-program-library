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
        transaction::{Transaction, TransactionError},
    },
    spl_token_editions_interface::{
        error::TokenEditionsError, instruction::update_original_max_supply, state::Original,
    },
    spl_token_metadata_interface::state::TokenMetadata,
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

#[tokio::test]
async fn success_update_original_max_supply() {
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

    let name = "My Cool Original Print".to_string();
    let symbol = "COOL".to_string();
    let uri = "cool.original.print".to_string();
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
        &metadata_pubkey,
        token.get_address(),
        &original_print,
        &original_keypair,
        &mint_authority,
    )
    .await;

    let new_max_supply = Some(200);

    let transaction = Transaction::new_signed_with_payer(
        &[update_original_max_supply(
            &program_id,
            &original_pubkey,
            &update_authority_pubkey,
            new_max_supply,
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
    assert_eq!(fetched_original_print.max_supply, new_max_supply);
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

    let name = "My Cool Original Print".to_string();
    let symbol = "COOL".to_string();
    let uri = "cool.original.print".to_string();
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
        &metadata_pubkey,
        token.get_address(),
        &original_print,
        &original_keypair,
        &mint_authority,
    )
    .await;

    let new_max_supply = Some(200);

    // No signature
    let mut update_supply_ix = update_original_max_supply(
        &program_id,
        &original_pubkey,
        &update_authority_pubkey,
        new_max_supply,
    );
    update_supply_ix.accounts[1].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[update_supply_ix],
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
        &[update_original_max_supply(
            &program_id,
            &original_pubkey,
            &original_pubkey,
            new_max_supply,
        )],
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
            0,
            InstructionError::Custom(TokenEditionsError::IncorrectUpdateAuthority as u32),
        )
    );
}
