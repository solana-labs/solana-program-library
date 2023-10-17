#![cfg(feature = "test-sbf")]
#![allow(clippy::items_after_test_module)]

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
        error::TokenMetadataError,
        instruction::update_field,
        state::{Field, TokenMetadata},
    },
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
    test_case::test_case,
};

#[test_case(Field::Name, "This is my larger name".to_string() ; "larger name")]
#[test_case(Field::Name, "Smaller".to_string() ; "smaller name")]
#[test_case(Field::Key("my new field".to_string()), "Some data for the new field!".to_string() ; "new field")]
#[tokio::test]
async fn success_update(field: Field, value: String) {
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

    let rent = context.banks_client.get_rent().await.unwrap();
    let old_space = token_metadata.tlv_size_of().unwrap();
    let old_rent_lamports = rent.minimum_balance(old_space);

    token_metadata.update(field.clone(), value.clone());

    let new_space = token_metadata.tlv_size_of().unwrap();

    if new_space > old_space {
        // fails without more lamports
        let transaction = Transaction::new_signed_with_payer(
            &[update_field(
                &program_id,
                &metadata_pubkey,
                &update_authority.pubkey(),
                field.clone(),
                value.clone(),
            )],
            Some(&payer.pubkey()),
            &[&payer, &update_authority],
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
            TransactionError::InsufficientFundsForRent { account_index: 2 }
        );
    }

    // transfer required lamports
    let new_rent_lamports = rent.minimum_balance(new_space);
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &payer.pubkey(),
                &metadata_pubkey,
                new_rent_lamports.saturating_sub(old_rent_lamports),
            ),
            update_field(
                &program_id,
                &metadata_pubkey,
                &update_authority.pubkey(),
                field.clone(),
                value.clone(),
            ),
        ],
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
    let mut instruction = update_field(
        &program_id,
        &metadata_pubkey,
        &update_authority.pubkey(),
        Field::Name,
        "new_name".to_string(),
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
        &[update_field(
            &program_id,
            &metadata_pubkey,
            &payer.pubkey(),
            Field::Name,
            "new_name".to_string(),
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
