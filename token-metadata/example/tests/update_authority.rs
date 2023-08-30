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
        transaction::{Transaction, TransactionError},
    },
    spl_pod::optional_keys::OptionalNonZeroPubkey,
    spl_token_metadata_interface::{
        error::TokenMetadataError, instruction::update_authority, state::TokenMetadata,
    },
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

#[tokio::test]
async fn success_update() {
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

    let authority = Keypair::new();
    let name = "MySuperCoolToken".to_string();
    let symbol = "MINE".to_string();
    let uri = "my.super.cool.token".to_string();
    let mut token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(authority.pubkey()).try_into().unwrap(),
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

    let new_update_authority = Keypair::new();
    let new_update_authority_pubkey =
        OptionalNonZeroPubkey::try_from(Some(new_update_authority.pubkey())).unwrap();
    token_metadata.update_authority = new_update_authority_pubkey;

    let transaction = Transaction::new_signed_with_payer(
        &[update_authority(
            &program_id,
            &metadata_pubkey,
            &authority.pubkey(),
            new_update_authority_pubkey,
        )],
        Some(&payer.pubkey()),
        &[&payer, &authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // check that the data is correct
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

    // unset
    token_metadata.update_authority = None.try_into().unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[update_authority(
            &program_id,
            &metadata_pubkey,
            &new_update_authority.pubkey(),
            None.try_into().unwrap(),
        )],
        Some(&payer.pubkey()),
        &[&payer, &new_update_authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

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

    // fail to update
    let transaction = Transaction::new_signed_with_payer(
        &[update_authority(
            &program_id,
            &metadata_pubkey,
            &new_update_authority.pubkey(),
            Some(new_update_authority.pubkey()).try_into().unwrap(),
        )],
        Some(&payer.pubkey()),
        &[&payer, &new_update_authority],
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
            InstructionError::Custom(TokenMetadataError::ImmutableMetadata as u32)
        )
    );
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

    let authority = Keypair::new();
    let name = "MySuperCoolToken".to_string();
    let symbol = "MINE".to_string();
    let uri = "my.super.cool.token".to_string();
    let token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(authority.pubkey()).try_into().unwrap(),
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
    let mut instruction = update_authority(
        &program_id,
        &metadata_pubkey,
        &authority.pubkey(),
        None.try_into().unwrap(),
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
        &[update_authority(
            &program_id,
            &metadata_pubkey,
            &payer.pubkey(),
            None.try_into().unwrap(),
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
