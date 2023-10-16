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
    spl_token_group_interface::{
        error::TokenGroupError,
        instruction::{initialize_group, update_group_authority},
        state::TokenGroup,
    },
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

#[tokio::test]
async fn test_update_collection_authority() {
    let program_id = Pubkey::new_unique();
    let collection = Keypair::new();
    let collection_mint = Keypair::new();
    let collection_mint_authority = Keypair::new();
    let collection_update_authority = Keypair::new();

    let collection_group_state = TokenGroup {
        update_authority: Some(collection_update_authority.pubkey())
            .try_into()
            .unwrap(),
        mint: collection_mint.pubkey(),
        size: 30.into(),
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

    // Fail: update authority not signer
    let mut update_ix = update_group_authority(
        &program_id,
        &collection.pubkey(),
        &collection_update_authority.pubkey(),
        None,
    );
    update_ix.accounts[1].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[update_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    assert_eq!(
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::MissingRequiredSignature)
    );

    // Fail: incorrect update authority
    let transaction = Transaction::new_signed_with_payer(
        &[update_group_authority(
            &program_id,
            &collection.pubkey(),
            &collection.pubkey(),
            None,
        )],
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
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(TokenGroupError::IncorrectUpdateAuthority as u32)
        )
    );

    // Success: update authority
    let transaction = Transaction::new_signed_with_payer(
        &[update_group_authority(
            &program_id,
            &collection.pubkey(),
            &collection_update_authority.pubkey(),
            None,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &collection_update_authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Fetch the account and assert the new authority
    let fetched_collection_account = context
        .banks_client
        .get_account(collection.pubkey())
        .await
        .unwrap()
        .unwrap();
    let fetched_meta = TlvStateBorrowed::unpack(&fetched_collection_account.data).unwrap();
    let fetched_collection_group_state = fetched_meta.get_first_value::<TokenGroup>().unwrap();
    assert_eq!(
        fetched_collection_group_state.update_authority,
        None.try_into().unwrap(),
    );

    // Fail: immutable group
    let transaction = Transaction::new_signed_with_payer(
        &[update_group_authority(
            &program_id,
            &collection.pubkey(),
            &collection_update_authority.pubkey(),
            Some(collection_update_authority.pubkey()),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &collection_update_authority],
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
            InstructionError::Custom(TokenGroupError::ImmutableGroup as u32)
        )
    );
}
