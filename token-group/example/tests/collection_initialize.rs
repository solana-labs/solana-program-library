#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{setup_group, setup_program_test, TokenGroupTestContext},
    solana_program_test::tokio,
    solana_sdk::{
        borsh::get_instance_packed_len,
        instruction::InstructionError,
        pubkey::Pubkey,
        signer::Signer,
        system_instruction,
        transaction::{Transaction, TransactionError},
    },
    spl_token_group_example::state::Collection,
    spl_token_group_interface::{
        error::TokenGroupError, instruction::initialize_group, state::Group,
    },
    spl_type_length_value::{
        error::TlvError,
        state::{TlvState, TlvStateBorrowed},
    },
};

#[tokio::test]
async fn success_initialize_collection() {
    let meta = Some(Collection {
        creation_date: "August 15".to_string(),
    });

    // Setup a test for creating a token `Collection`:
    // - Mint:         An NFT representing the `Collection` mint
    // - Metadata:     A `TokenMetadata` representing the `Collection` metadata
    // - Collection:   A `Collection` representing the `Collection` group
    let TokenGroupTestContext {
        context,
        payer,
        program_id,
        mint_keypair,
        mint_authority_keypair,
        group_keypair: collection_keypair,
        group: collection,
        ..
    } = setup_program_test::<Collection>("My Cool Collection", meta.clone()).await;

    let mut context = context.lock().await;

    // Hit our program to initialize the collection
    setup_group::<Collection>(
        &mut context,
        &program_id,
        &collection_keypair,
        &mint_keypair.pubkey(),
        &mint_authority_keypair,
        &collection,
        &[], // No extra account metas
    )
    .await;

    // Fetch the collection account and ensure it matches our state
    let fetched_collection_account = context
        .banks_client
        .get_account(collection_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let fetched_meta = TlvStateBorrowed::unpack(&fetched_collection_account.data).unwrap();
    let fetched_collection = fetched_meta
        .get_first_variable_len_value::<Group<Collection>>()
        .unwrap();
    assert_eq!(fetched_collection, collection);

    // Fail doing it again, and change some params to ensure a new tx
    {
        let transaction = Transaction::new_signed_with_payer(
            &[initialize_group::<Collection>(
                &program_id,
                &collection_keypair.pubkey(),
                &mint_keypair.pubkey(),
                &mint_authority_keypair.pubkey(),
                None, // Intentionally changed params
                Some(500),
                &meta,
                &[], // No extra account metas
            )],
            Some(&payer.pubkey()),
            &[&payer, &mint_authority_keypair],
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
    let meta = Some(Collection {
        creation_date: "August 15".to_string(),
    });

    let TokenGroupTestContext {
        context,
        program_id,
        mint_keypair,
        mint_authority_keypair,
        group_keypair: collection_keypair,
        group: collection,
        ..
    } = setup_program_test::<Collection>("My Cool Collection", meta.clone()).await;

    let mut context = context.lock().await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let space = TlvStateBorrowed::get_base_len() + get_instance_packed_len(&collection).unwrap();
    let rent_lamports = rent.minimum_balance(space);
    let mut initialize_group_ix = initialize_group::<Collection>(
        &program_id,
        &collection_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &mint_authority_keypair.pubkey(),
        Option::<Pubkey>::from(collection.update_authority),
        collection.max_size,
        &meta,
        &[], // No extra account metas
    );
    initialize_group_ix.accounts[2].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &collection_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            initialize_group_ix,
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
    let meta = Some(Collection {
        creation_date: "August 15".to_string(),
    });

    let TokenGroupTestContext {
        context,
        program_id,
        mint_keypair,
        group_keypair: collection_keypair,
        group: collection,
        ..
    } = setup_program_test::<Collection>("My Cool Collection", meta.clone()).await;

    let mut context = context.lock().await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let space = TlvStateBorrowed::get_base_len() + get_instance_packed_len(&collection).unwrap();
    let rent_lamports = rent.minimum_balance(space);
    let mut initialize_group_ix = initialize_group::<Collection>(
        &program_id,
        &collection_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &collection_keypair.pubkey(), // NOT the mint authority
        Option::<Pubkey>::from(collection.update_authority),
        collection.max_size,
        &meta,
        &[], // No extra account metas
    );
    initialize_group_ix.accounts[2].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &collection_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            initialize_group_ix,
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
            InstructionError::Custom(TokenGroupError::IncorrectAuthority as u32)
        )
    );
}
