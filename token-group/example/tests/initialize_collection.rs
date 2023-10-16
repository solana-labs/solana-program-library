#![cfg(feature = "test-sbf")]

mod setup;

use {
    setup::{setup_mint, setup_program_test},
    solana_program::{instruction::InstructionError, pubkey::Pubkey, system_instruction},
    solana_program_test::tokio,
    solana_sdk::{
        signature::Keypair,
        signer::Signer,
        transaction::{Transaction, TransactionError},
    },
    spl_token_client::token::Token,
    spl_token_group_interface::{instruction::initialize_group, state::TokenGroup},
    spl_type_length_value::{
        error::TlvError,
        state::{TlvState, TlvStateBorrowed},
    },
};

#[tokio::test]
async fn test_initialize_collection() {
    let program_id = Pubkey::new_unique();
    let collection = Keypair::new();
    let collection_mint = Keypair::new();
    let collection_mint_authority = Keypair::new();

    let collection_group_state = TokenGroup {
        update_authority: None.try_into().unwrap(),
        size: 0.into(),
        max_size: 50.into(),
    };

    let (context, client, payer) = setup_program_test(&program_id).await;

    let token_client = Token::new(
        client,
        &spl_token_2022::id(),
        &collection_mint.pubkey(),
        Some(0),
        payer.clone(),
    );
    setup_mint(&token_client, &collection_mint, &collection_mint_authority).await;

    let mut context = context.lock().await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let space = TlvStateBorrowed::get_base_len() + std::mem::size_of::<TokenGroup>();
    let rent_lamports = rent.minimum_balance(space);

    // Fail: mint authority not signer
    let mut init_group_ix = initialize_group(
        &program_id,
        &collection.pubkey(),
        &collection_mint.pubkey(),
        &collection_mint_authority.pubkey(),
        collection_group_state.update_authority.try_into().unwrap(),
        collection_group_state.max_size.into(),
    );
    init_group_ix.accounts[2].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &collection.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            init_group_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &collection],
        context.last_blockhash,
    );
    assert_eq!(
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(1, InstructionError::MissingRequiredSignature)
    );

    // Success: create the collection
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &collection.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            initialize_group(
                &program_id,
                &collection.pubkey(),
                &collection_mint.pubkey(),
                &collection_mint_authority.pubkey(),
                collection_group_state.update_authority.try_into().unwrap(),
                collection_group_state.max_size.into(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &collection_mint_authority, &collection],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Fetch the collection account and ensure it matches our state
    let fetched_collection_account = context
        .banks_client
        .get_account(collection.pubkey())
        .await
        .unwrap()
        .unwrap();
    let fetched_meta = TlvStateBorrowed::unpack(&fetched_collection_account.data).unwrap();
    let fetched_collection_group_state = fetched_meta.get_first_value::<TokenGroup>().unwrap();
    assert_eq!(fetched_collection_group_state, &collection_group_state);

    // Fail: can't initialize twice
    let transaction = Transaction::new_signed_with_payer(
        &[initialize_group(
            &program_id,
            &collection.pubkey(),
            &collection_mint.pubkey(),
            &collection_mint_authority.pubkey(),
            Pubkey::new_unique().into(), // Intentionally changed
            collection_group_state.max_size.into(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &collection_mint_authority],
        context.last_blockhash,
    );
    assert_eq!(
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(TlvError::TypeAlreadyExists as u32)
        )
    );
}
