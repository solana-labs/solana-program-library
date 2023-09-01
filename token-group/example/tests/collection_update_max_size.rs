#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{setup_group, setup_program_test, TokenGroupTestContext},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError,
        signer::Signer,
        transaction::{Transaction, TransactionError},
    },
    spl_token_group_example::state::Collection,
    spl_token_group_interface::{
        error::TokenGroupError, instruction::update_group_max_size, state::Group,
    },
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

#[tokio::test]
async fn success_update_collection_max_size() {
    let meta = Some(Collection {
        creation_date: "August 15".to_string(),
    });

    let TokenGroupTestContext {
        context,
        payer,
        program_id,
        mint_keypair,
        mint_authority_keypair,
        group_keypair: collection_keypair,
        group_update_authority_keypair: collection_update_authority_keypair,
        group: collection,
        ..
    } = setup_program_test::<Collection>("My Cool Collection", meta).await;

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

    let new_max_size = Some(200);

    let transaction = Transaction::new_signed_with_payer(
        &[update_group_max_size::<Collection>(
            &program_id,
            &collection_keypair.pubkey(),
            &collection_update_authority_keypair.pubkey(),
            new_max_size,
        )],
        Some(&payer.pubkey()),
        &[&payer, &collection_update_authority_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let fetched_collection_account = context
        .banks_client
        .get_account(collection_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let fetched_meta = TlvStateBorrowed::unpack(&fetched_collection_account.data).unwrap();
    let fetched_collection_data = fetched_meta
        .get_first_variable_len_value::<Group<Collection>>()
        .unwrap();
    assert_eq!(fetched_collection_data.max_size, new_max_size);
}

#[tokio::test]
async fn fail_authority_checks() {
    let meta = Some(Collection {
        creation_date: "August 15".to_string(),
    });

    let TokenGroupTestContext {
        context,
        program_id,
        mint_keypair,
        mint_authority_keypair,
        group_keypair: collection_keypair,
        group_update_authority_keypair: collection_update_authority_keypair,
        group: collection,
        ..
    } = setup_program_test::<Collection>("My Cool Collection", meta).await;

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

    let new_max_size = Some(200);

    // No signature
    let mut update_size_ix = update_group_max_size::<Collection>(
        &program_id,
        &collection_keypair.pubkey(),
        &collection_update_authority_keypair.pubkey(),
        new_max_size,
    );
    update_size_ix.accounts[1].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[update_size_ix],
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
        &[update_group_max_size::<Collection>(
            &program_id,
            &collection_keypair.pubkey(),
            &collection_keypair.pubkey(),
            new_max_size,
        )],
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
            0,
            InstructionError::Custom(TokenGroupError::IncorrectAuthority as u32),
        )
    );
}
