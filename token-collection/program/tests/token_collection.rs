#![cfg(feature = "test-sbf")]

mod setup;

use {
    setup::{setup_group, setup_mint_and_metadata, setup_program_test},
    solana_program::{pubkey::Pubkey, system_instruction},
    solana_program_test::tokio,
    solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction},
    spl_token_client::token::Token,
    spl_token_group_interface::{
        instruction::initialize_member,
        state::{TokenGroup, TokenGroupMember},
    },
    spl_token_metadata_interface::state::TokenMetadata,
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

/// All snakes are reptiles, but not all reptiles are snakes.
#[tokio::test]
async fn test_token_collection() {
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup_program_test(&program_id).await;

    // Set up the "Reptiles" collection mint and metadata
    let reptile = Keypair::new();
    let reptile_mint = Keypair::new();
    let reptile_mint_authority = Keypair::new();
    let reptile_update_authority = Keypair::new();
    let reptile_metadata_state = TokenMetadata {
        name: "Reptiles".to_string(),
        symbol: "RPTL".to_string(),
        ..TokenMetadata::default()
    };
    setup_mint_and_metadata(
        &Token::new(
            client.clone(),
            &spl_token_2022::id(),
            &reptile_mint.pubkey(),
            Some(0),
            payer.clone(),
        ),
        &reptile_mint,
        &reptile_mint_authority,
        &reptile_metadata_state,
        payer.clone(),
    )
    .await;

    // Set up the "Snakes" collection mint and metadata
    let snake = Keypair::new();
    let snake_mint = Keypair::new();
    let snake_mint_authority = Keypair::new();
    let snake_update_authority = Keypair::new();
    let snake_metadata_state = TokenMetadata {
        name: "Snakes".to_string(),
        symbol: "SNKE".to_string(),
        ..TokenMetadata::default()
    };
    setup_mint_and_metadata(
        &Token::new(
            client.clone(),
            &spl_token_2022::id(),
            &snake_mint.pubkey(),
            Some(0),
            payer.clone(),
        ),
        &snake_mint,
        &snake_mint_authority,
        &snake_metadata_state,
        payer.clone(),
    )
    .await;

    // Set up the "Python" mint and metadata
    let python = Keypair::new();
    let python_mint = Keypair::new();
    let python_mint_authority = Keypair::new();
    let python_metadata_state = TokenMetadata {
        name: "Python".to_string(),
        symbol: "PYTH".to_string(),
        ..TokenMetadata::default()
    };
    setup_mint_and_metadata(
        &Token::new(
            client.clone(),
            &spl_token_2022::id(),
            &python_mint.pubkey(),
            Some(0),
            payer.clone(),
        ),
        &python_mint,
        &python_mint_authority,
        &python_metadata_state,
        payer.clone(),
    )
    .await;

    // Set up the "Cobra" mint and metadata
    let cobra = Keypair::new();
    let cobra_mint = Keypair::new();
    let cobra_mint_authority = Keypair::new();
    let cobra_metadata_state = TokenMetadata {
        name: "Cobra".to_string(),
        symbol: "CBRA".to_string(),
        ..TokenMetadata::default()
    };
    setup_mint_and_metadata(
        &Token::new(
            client.clone(),
            &spl_token_2022::id(),
            &cobra_mint.pubkey(),
            Some(0),
            payer.clone(),
        ),
        &cobra_mint,
        &cobra_mint_authority,
        &cobra_metadata_state,
        payer.clone(),
    )
    .await;

    // Set up the "Iguana" mint and metadata
    let iguana = Keypair::new();
    let iguana_mint = Keypair::new();
    let iguana_mint_authority = Keypair::new();
    let iguana_metadata_state = TokenMetadata {
        name: "Iguana".to_string(),
        symbol: "IGUA".to_string(),
        ..TokenMetadata::default()
    };
    setup_mint_and_metadata(
        &Token::new(
            client.clone(),
            &spl_token_2022::id(),
            &iguana_mint.pubkey(),
            Some(0),
            payer.clone(),
        ),
        &iguana_mint,
        &iguana_mint_authority,
        &iguana_metadata_state,
        payer.clone(),
    )
    .await;

    let mut context = context.lock().await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let collection_space = TlvStateBorrowed::get_base_len() + std::mem::size_of::<TokenGroup>();
    let collection_rent_lamports = rent.minimum_balance(collection_space);
    let member_space = TlvStateBorrowed::get_base_len() + std::mem::size_of::<TokenGroupMember>();
    let member_rent_lamports = rent.minimum_balance(member_space);

    // Create the collections using the SPL Token Collection program
    setup_group(
        &mut context,
        &program_id,
        &reptile,
        &reptile_mint,
        &reptile_mint_authority,
        Some(reptile_update_authority.pubkey()),
        3,
        collection_rent_lamports,
        collection_space,
    )
    .await;
    setup_group(
        &mut context,
        &program_id,
        &snake,
        &snake_mint,
        &snake_mint_authority,
        Some(snake_update_authority.pubkey()),
        2,
        collection_rent_lamports,
        collection_space,
    )
    .await;

    // Create the member accounts ahead of time
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &python.pubkey(),
                member_rent_lamports.checked_mul(2).unwrap(), // 2 collections
                u64::try_from(member_space).unwrap().checked_mul(2).unwrap(), // 2 collections
                &program_id,
            ),
            system_instruction::create_account(
                &context.payer.pubkey(),
                &cobra.pubkey(),
                member_rent_lamports.checked_mul(2).unwrap(), // 2 collections
                u64::try_from(member_space).unwrap().checked_mul(2).unwrap(), // 2 collections
                &program_id,
            ),
            system_instruction::create_account(
                &context.payer.pubkey(),
                &iguana.pubkey(),
                member_rent_lamports,
                member_space.try_into().unwrap(),
                &program_id,
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &python, &cobra, &iguana],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // A python is both a reptile and a snake!
    let transaction = Transaction::new_signed_with_payer(
        &[
            initialize_member(
                &program_id,
                &python.pubkey(),
                &python_mint.pubkey(),
                &python_mint_authority.pubkey(),
                &reptile.pubkey(),
                &reptile_update_authority.pubkey(),
            ),
            initialize_member(
                &program_id,
                &python.pubkey(),
                &python_mint.pubkey(),
                &python_mint_authority.pubkey(),
                &snake.pubkey(),
                &snake_update_authority.pubkey(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            &python_mint_authority,
            &reptile_update_authority,
            &snake_update_authority,
        ],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // A cobra is both a reptile and a snake!
    let transaction = Transaction::new_signed_with_payer(
        &[
            initialize_member(
                &program_id,
                &cobra.pubkey(),
                &cobra_mint.pubkey(),
                &cobra_mint_authority.pubkey(),
                &reptile.pubkey(),
                &reptile_update_authority.pubkey(),
            ),
            initialize_member(
                &program_id,
                &cobra.pubkey(),
                &cobra_mint.pubkey(),
                &cobra_mint_authority.pubkey(),
                &snake.pubkey(),
                &snake_update_authority.pubkey(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            &cobra_mint_authority,
            &reptile_update_authority,
            &snake_update_authority,
        ],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // An iguana is a reptile but not a snake!
    let mut transaction = Transaction::new_signed_with_payer(
        &[initialize_member(
            &program_id,
            &iguana.pubkey(),
            &iguana_mint.pubkey(),
            &iguana_mint_authority.pubkey(),
            &reptile.pubkey(),
            &reptile_update_authority.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            &iguana_mint_authority,
            &reptile_update_authority,
        ],
        context.last_blockhash,
    );
    transaction.sign(
        &[
            &context.payer,
            &iguana_mint_authority,
            &reptile_update_authority,
        ],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // The "Reptiles" collection should have 3 members
    let buffer = context
        .banks_client
        .get_account(reptile.pubkey())
        .await
        .unwrap()
        .unwrap()
        .data;
    let state = TlvStateBorrowed::unpack(&buffer).unwrap();
    let collection = state.get_first_value::<TokenGroup>().unwrap();
    assert_eq!(u64::from(collection.size), 3);

    // The "Snakes" collection should have 2 members
    let buffer = context
        .banks_client
        .get_account(snake.pubkey())
        .await
        .unwrap()
        .unwrap()
        .data;
    let state = TlvStateBorrowed::unpack(&buffer).unwrap();
    let collection = state.get_first_value::<TokenGroup>().unwrap();
    assert_eq!(u64::from(collection.size), 2);

    // The "Python" should be a member of 2 collections
    let buffer = context
        .banks_client
        .get_account(python.pubkey())
        .await
        .unwrap()
        .unwrap()
        .data;
    let state = TlvStateBorrowed::unpack(&buffer).unwrap();
    let membership = state
        .get_value_with_repetition::<TokenGroupMember>(0)
        .unwrap();
    assert_eq!(membership.group, reptile.pubkey(),);
    let membership = state
        .get_value_with_repetition::<TokenGroupMember>(1)
        .unwrap();
    assert_eq!(membership.group, snake.pubkey(),);

    // The "Cobra" should be a member of 2 collections
    let buffer = context
        .banks_client
        .get_account(cobra.pubkey())
        .await
        .unwrap()
        .unwrap()
        .data;
    let state = TlvStateBorrowed::unpack(&buffer).unwrap();
    let membership = state
        .get_value_with_repetition::<TokenGroupMember>(0)
        .unwrap();
    assert_eq!(membership.group, reptile.pubkey(),);
    let membership = state
        .get_value_with_repetition::<TokenGroupMember>(1)
        .unwrap();
    assert_eq!(membership.group, snake.pubkey(),);

    // The "Iguana" should be a member of 1 collection
    let buffer = context
        .banks_client
        .get_account(iguana.pubkey())
        .await
        .unwrap()
        .unwrap()
        .data;
    let state = TlvStateBorrowed::unpack(&buffer).unwrap();
    let membership = state.get_first_value::<TokenGroupMember>().unwrap();
    assert_eq!(membership.group, reptile.pubkey(),);
}
