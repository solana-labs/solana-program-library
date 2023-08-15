#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{
        setup_group, setup_member, setup_mint_and_metadata, setup_program_test,
        TokenGroupTestContext,
    },
    solana_program_test::tokio,
    solana_sdk::{
        borsh::get_instance_packed_len,
        instruction::InstructionError,
        signature::Signer,
        signer::keypair::Keypair,
        system_instruction,
        transaction::{Transaction, TransactionError},
    },
    spl_token_client::token::Token,
    spl_token_group_example::state::Collection,
    spl_token_group_interface::{
        error::TokenGroupError, instruction::initialize_member, state::Member,
    },
    spl_token_metadata_interface::state::TokenMetadata,
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

#[tokio::test]
async fn success_initialize_member() {
    let meta = Some(Collection {
        creation_date: "August 15".to_string(),
    });

    let TokenGroupTestContext {
        context,
        client,
        payer,
        token_program_id,
        program_id,
        mint_keypair,
        mint_authority_keypair,
        group_keypair: collection_keypair,
        group: collection,
        ..
    } = setup_program_test::<Collection>("My Cool Collection", meta).await;

    // In this test (similar to `setup_group_test`):
    // - The metadata is stored in the mint (Token-2022)
    // - The member is in a separate account
    // - The member's _metadata_ update authority is the mint authority
    // - The _member_ update authority is also the mint authority
    // - The mint is an NFT (0 decimals)
    let member_keypair = Keypair::new();
    let member_mint_keypair = Keypair::new();
    let member_mint_authority_keypair = Keypair::new();
    let member_metadata_keypair = member_mint_keypair.insecure_clone();
    let member_metadata_update_authority_keypair = member_metadata_keypair.insecure_clone();
    let member_update_authority_keypair = member_metadata_keypair.insecure_clone();
    let decimals = 0;
    let member = Member {
        group: collection_keypair.pubkey(),
        member_number: 1,
    };

    // Set up a mint and metadata for the member
    setup_mint_and_metadata(
        &Token::new(
            client.clone(),
            &token_program_id,
            &member_mint_keypair.pubkey(),
            Some(decimals),
            payer.clone(),
        ),
        &member_mint_keypair,
        &member_mint_authority_keypair,
        &member_metadata_keypair.pubkey(),
        &member_metadata_update_authority_keypair.pubkey(),
        &TokenMetadata {
            name: "I'm a Member!".to_string(),
            symbol: "MEM".to_string(),
            uri: "member.com".to_string(),
            update_authority: Some(member_update_authority_keypair.pubkey())
                .try_into()
                .unwrap(),
            mint: member_mint_keypair.pubkey(),
            ..Default::default()
        },
        payer,
    )
    .await;

    let mut context = context.lock().await;

    setup_group::<Collection>(
        &mut context,
        &program_id,
        &collection_keypair,
        &mint_keypair.pubkey(),
        &mint_authority_keypair,
        &collection,
    )
    .await;

    setup_member::<Collection>(
        &mut context,
        &program_id,
        &collection_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &mint_authority_keypair,
        &member_keypair,
        &member_mint_keypair.pubkey(),
        &member_mint_authority_keypair,
        &member,
    )
    .await;

    let fetched_member_account = context
        .banks_client
        .get_account(member_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let fetched_member_state = TlvStateBorrowed::unpack(&fetched_member_account.data).unwrap();
    let fetched_member = fetched_member_state
        .get_first_variable_len_value::<Member>()
        .unwrap();
    assert_eq!(fetched_member, member);
}

#[tokio::test]
async fn fail_without_authority_signature() {
    let meta = Some(Collection {
        creation_date: "August 15".to_string(),
    });

    let TokenGroupTestContext {
        context,
        client,
        payer,
        token_program_id,
        program_id,
        mint_keypair,
        mint_authority_keypair,
        group_keypair: collection_keypair,
        group: collection,
        ..
    } = setup_program_test::<Collection>("My Cool Collection", meta).await;

    // In this test (similar to `setup_group_test`):
    // - The metadata is stored in the mint (Token-2022)
    // - The member is in a separate account
    // - The member's _metadata_ update authority is the mint authority
    // - The _member_ update authority is also the mint authority
    // - The mint is an NFT (0 decimals)
    let member_keypair = Keypair::new();
    let member_mint_keypair = Keypair::new();
    let member_mint_authority_keypair = Keypair::new();
    let member_metadata_keypair = member_mint_keypair.insecure_clone();
    let member_metadata_update_authority_keypair = member_metadata_keypair.insecure_clone();
    let member_update_authority_keypair = member_metadata_keypair.insecure_clone();
    let decimals = 0;
    let member = Member {
        group: collection_keypair.pubkey(),
        member_number: 1,
    };

    // Set up a mint and metadata for the member
    setup_mint_and_metadata(
        &Token::new(
            client.clone(),
            &token_program_id,
            &member_mint_keypair.pubkey(),
            Some(decimals),
            payer.clone(),
        ),
        &member_mint_keypair,
        &member_mint_authority_keypair,
        &member_metadata_keypair.pubkey(),
        &member_metadata_update_authority_keypair.pubkey(),
        &TokenMetadata {
            name: "I'm a Member!".to_string(),
            symbol: "MEM".to_string(),
            uri: "member.com".to_string(),
            update_authority: Some(member_update_authority_keypair.pubkey())
                .try_into()
                .unwrap(),
            mint: member_mint_keypair.pubkey(),
            ..Default::default()
        },
        payer,
    )
    .await;

    let mut context = context.lock().await;

    setup_group::<Collection>(
        &mut context,
        &program_id,
        &collection_keypair,
        &mint_keypair.pubkey(),
        &mint_authority_keypair,
        &collection,
    )
    .await;

    let rent = context.banks_client.get_rent().await.unwrap();

    let token_metadata_space = TokenMetadata::default().tlv_size_of().unwrap();
    let token_metadata_rent_lamports = rent.minimum_balance(token_metadata_space);

    let space = TlvStateBorrowed::get_base_len() + get_instance_packed_len(&member).unwrap();
    let rent_lamports = rent.minimum_balance(space);

    // Fail missing member mint authority

    let mut initialize_member_ix = initialize_member::<Collection>(
        &program_id,
        &collection_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &mint_authority_keypair.pubkey(),
        &member_keypair.pubkey(),
        &member_mint_keypair.pubkey(),
        &member_mint_authority_keypair.pubkey(),
        member.member_number,
    );
    initialize_member_ix.accounts[2].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &member_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                &member_mint_keypair.pubkey(),
                token_metadata_rent_lamports,
            ),
            initialize_member_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &member_keypair, &mint_authority_keypair], /* Missing member mint
                                                                      * authority */
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

    // Fail missing collection mint authority

    let mut initialize_member_ix = initialize_member::<Collection>(
        &program_id,
        &collection_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &mint_authority_keypair.pubkey(),
        &member_keypair.pubkey(),
        &member_mint_keypair.pubkey(),
        &member_mint_authority_keypair.pubkey(),
        member.member_number,
    );
    initialize_member_ix.accounts[5].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &member_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                &member_mint_keypair.pubkey(),
                token_metadata_rent_lamports,
            ),
            initialize_member_ix,
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            &member_keypair,
            &member_mint_authority_keypair,
        ], /* Missing collection mint
            * authority */
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
    let meta = Some(Collection {
        creation_date: "August 15".to_string(),
    });

    let TokenGroupTestContext {
        context,
        client,
        payer,
        token_program_id,
        program_id,
        mint_keypair,
        mint_authority_keypair,
        group_keypair: collection_keypair,
        group: collection,
        ..
    } = setup_program_test::<Collection>("My Cool Collection", meta).await;

    // In this test (similar to `setup_group_test`):
    // - The metadata is stored in the mint (Token-2022)
    // - The member is in a separate account
    // - The member's _metadata_ update authority is the mint authority
    // - The _member_ update authority is also the mint authority
    // - The mint is an NFT (0 decimals)
    let member_keypair = Keypair::new();
    let member_mint_keypair = Keypair::new();
    let member_mint_authority_keypair = Keypair::new();
    let member_metadata_keypair = member_mint_keypair.insecure_clone();
    let member_metadata_update_authority_keypair = member_metadata_keypair.insecure_clone();
    let member_update_authority_keypair = member_metadata_keypair.insecure_clone();
    let decimals = 0;
    let member = Member {
        group: collection_keypair.pubkey(),
        member_number: 1,
    };

    // Set up a mint and metadata for the member
    setup_mint_and_metadata(
        &Token::new(
            client.clone(),
            &token_program_id,
            &member_mint_keypair.pubkey(),
            Some(decimals),
            payer.clone(),
        ),
        &member_mint_keypair,
        &member_mint_authority_keypair,
        &member_metadata_keypair.pubkey(),
        &member_metadata_update_authority_keypair.pubkey(),
        &TokenMetadata {
            name: "I'm a Member!".to_string(),
            symbol: "MEM".to_string(),
            uri: "member.com".to_string(),
            update_authority: Some(member_update_authority_keypair.pubkey())
                .try_into()
                .unwrap(),
            mint: member_mint_keypair.pubkey(),
            ..Default::default()
        },
        payer,
    )
    .await;

    let mut context = context.lock().await;

    setup_group::<Collection>(
        &mut context,
        &program_id,
        &collection_keypair,
        &mint_keypair.pubkey(),
        &mint_authority_keypair,
        &collection,
    )
    .await;

    let rent = context.banks_client.get_rent().await.unwrap();

    let token_metadata_space = TokenMetadata::default().tlv_size_of().unwrap();
    let token_metadata_rent_lamports = rent.minimum_balance(token_metadata_space);

    let space = TlvStateBorrowed::get_base_len() + get_instance_packed_len(&member).unwrap();
    let rent_lamports = rent.minimum_balance(space);

    // Fail incorrect member mint authority

    let mut initialize_member_ix = initialize_member::<Collection>(
        &program_id,
        &collection_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &mint_authority_keypair.pubkey(),
        &member_keypair.pubkey(),
        &member_mint_keypair.pubkey(),
        &mint_authority_keypair.pubkey(), // NOT the member mint authority
        member.member_number,
    );
    initialize_member_ix.accounts[5].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &member_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                &member_mint_keypair.pubkey(),
                token_metadata_rent_lamports,
            ),
            initialize_member_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &member_keypair, &mint_authority_keypair],
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
            InstructionError::Custom(TokenGroupError::IncorrectAuthority as u32)
        )
    );

    // Fail missing collection mint authority

    let mut initialize_member_ix = initialize_member::<Collection>(
        &program_id,
        &collection_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &member_mint_authority_keypair.pubkey(), // NOT the collection mint authority
        &member_keypair.pubkey(),
        &member_mint_keypair.pubkey(),
        &member_mint_authority_keypair.pubkey(),
        member.member_number,
    );
    initialize_member_ix.accounts[2].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &member_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                &member_mint_keypair.pubkey(),
                token_metadata_rent_lamports,
            ),
            initialize_member_ix,
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            &member_keypair,
            &member_mint_authority_keypair,
        ],
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
            InstructionError::Custom(TokenGroupError::IncorrectAuthority as u32)
        )
    );
}
