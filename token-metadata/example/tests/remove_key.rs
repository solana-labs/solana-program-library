#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{setup, setup_metadata, setup_mint, setup_update_field},
    solana_program_test::{tokio, ProgramTestBanksClientExt},
    solana_sdk::{
        instruction::InstructionError,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        transaction::{Transaction, TransactionError},
    },
    spl_token_metadata_interface::{
        error::TokenMetadataError,
        instruction::remove_key,
        state::{Field, TokenMetadata},
    },
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

#[tokio::test]
async fn success_remove() {
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

    let update_authority = Keypair::new();
    let name = "MySuperCoolToken".to_string();
    let symbol = "MINE".to_string();
    let uri = "my.super.cool.token".to_string();
    let mut token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority.pubkey()).try_into().unwrap(),
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

    let key = "key".to_string();
    let value = "value".to_string();
    let field = Field::Key(key.clone());
    setup_update_field(
        &mut context,
        &program_id,
        &mut token_metadata,
        &metadata_pubkey,
        &update_authority,
        field,
        value,
    )
    .await;

    let transaction = Transaction::new_signed_with_payer(
        &[remove_key(
            &program_id,
            &metadata_pubkey,
            &update_authority.pubkey(),
            key.clone(),
            false, // idempotent
        )],
        Some(&payer.pubkey()),
        &[&payer, &update_authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // check that the data is correct
    token_metadata.remove_key(&key);
    let fetched_metadata_account = context
        .banks_client
        .get_account(metadata_pubkey)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        fetched_metadata_account.data.len(),
        token_metadata.tlv_size_of().unwrap()
    );
    let fetched_metadata_state = TlvStateBorrowed::unpack(&fetched_metadata_account.data).unwrap();
    let fetched_metadata = fetched_metadata_state
        .get_first_variable_len_value::<TokenMetadata>()
        .unwrap();
    assert_eq!(fetched_metadata, token_metadata);

    // refresh blockhash before trying again
    let last_blockhash = context.last_blockhash;
    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();

    // fail doing it again without idempotent flag
    let transaction = Transaction::new_signed_with_payer(
        &[remove_key(
            &program_id,
            &metadata_pubkey,
            &update_authority.pubkey(),
            key.clone(),
            false, // idempotent
        )],
        Some(&payer.pubkey()),
        &[&payer, &update_authority],
        last_blockhash,
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
            InstructionError::Custom(TokenMetadataError::KeyNotFound as u32)
        )
    );

    // succeed with idempotent flag
    let transaction = Transaction::new_signed_with_payer(
        &[remove_key(
            &program_id,
            &metadata_pubkey,
            &update_authority.pubkey(),
            key,
            true, // idempotent
        )],
        Some(&payer.pubkey()),
        &[&payer, &update_authority],
        last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

#[tokio::test]
async fn fail_authority_checks() {
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

    let update_authority = Keypair::new();
    let name = "MySuperCoolToken".to_string();
    let symbol = "MINE".to_string();
    let uri = "my.super.cool.token".to_string();
    let token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority.pubkey()).try_into().unwrap(),
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

    // no signature
    let mut instruction = remove_key(
        &program_id,
        &metadata_pubkey,
        &update_authority.pubkey(),
        "new_name".to_string(),
        true, // idempotent
    );
    instruction.accounts[1].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer.as_ref()],
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

    // wrong authority
    let transaction = Transaction::new_signed_with_payer(
        &[remove_key(
            &program_id,
            &metadata_pubkey,
            &payer.pubkey(),
            "new_name".to_string(),
            true, // idempotent
        )],
        Some(&payer.pubkey()),
        &[payer.as_ref()],
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
            InstructionError::Custom(TokenMetadataError::IncorrectUpdateAuthority as u32),
        )
    );
}
