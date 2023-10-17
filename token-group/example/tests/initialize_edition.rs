#![cfg(feature = "test-sbf")]

mod setup;

use {
    setup::{setup_mint, setup_mint_and_metadata, setup_program_test},
    solana_program::{
        borsh0_10::get_instance_packed_len,
        instruction::{AccountMeta, Instruction, InstructionError},
        pubkey::Pubkey,
        system_instruction,
    },
    solana_program_test::tokio,
    solana_sdk::{
        signature::Keypair,
        signer::Signer,
        transaction::{Transaction, TransactionError},
    },
    spl_token_2022::{
        extension::{BaseStateWithExtensions, StateWithExtensions},
        state::Mint,
    },
    spl_token_client::token::{ExtensionInitializationParams, Token},
    spl_token_group_interface::{
        instruction::{initialize_group, initialize_member},
        state::{TokenGroup, TokenGroupMember},
    },
    spl_token_metadata_interface::state::TokenMetadata,
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

fn initialize_edition_reprint(
    program_id: &Pubkey,
    reprint: &Pubkey,
    reprint_mint: &Pubkey,
    reprint_mint_authority: &Pubkey,
    original: &Pubkey,
    original_update_authority: &Pubkey,
    original_mint: &Pubkey,
) -> Instruction {
    let mut ix = initialize_member(
        program_id,
        reprint,
        reprint_mint,
        reprint_mint_authority,
        original,
        original_update_authority,
    );
    // Our program requires the reprint mint to be writable
    ix.accounts[1].is_writable = true;
    ix.accounts.extend_from_slice(&[
        AccountMeta::new_readonly(*original_mint, false),
        AccountMeta::new_readonly(spl_token_2022::id(), false),
    ]);
    ix
}

#[tokio::test]
async fn test_initialize_edition_reprint() {
    let program_id = Pubkey::new_unique();
    let original = Keypair::new();
    let original_mint = Keypair::new();
    let original_mint_authority = Keypair::new();
    let original_update_authority = Keypair::new();
    let reprint = Keypair::new();
    let reprint_mint = Keypair::new();
    let reprint_mint_authority = Keypair::new();

    let original_metadata_state = TokenMetadata {
        update_authority: None.try_into().unwrap(),
        mint: original_mint.pubkey(),
        name: "The Coolest Collection".to_string(),
        symbol: "COOL".to_string(),
        uri: "https://cool.com".to_string(),
        additional_metadata: vec![],
    };
    let original_group_state = TokenGroup {
        update_authority: Some(original_update_authority.pubkey()).try_into().unwrap(),
        size: 30.into(),
        max_size: 50.into(),
    };

    let (context, client, payer) = setup_program_test(&program_id).await;

    setup_mint_and_metadata(
        &Token::new(
            client.clone(),
            &spl_token_2022::id(),
            &original_mint.pubkey(),
            Some(0),
            payer.clone(),
        ),
        &original_mint,
        &original_mint_authority,
        &original_metadata_state,
        payer.clone(),
    )
    .await;
    // Add the metadata pointer extension ahead of time
    setup_mint(
        &Token::new(
            client.clone(),
            &spl_token_2022::id(),
            &reprint_mint.pubkey(),
            Some(0),
            payer.clone(),
        ),
        &reprint_mint,
        &reprint_mint_authority,
        vec![ExtensionInitializationParams::MetadataPointer {
            authority: Some(reprint_mint_authority.pubkey()),
            metadata_address: Some(reprint_mint.pubkey()),
        }],
    )
    .await;

    let mut context = context.lock().await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let space = TlvStateBorrowed::get_base_len() + std::mem::size_of::<TokenGroup>();
    let rent_lamports = rent.minimum_balance(space);

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &original.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            initialize_group(
                &program_id,
                &original.pubkey(),
                &original_mint.pubkey(),
                &original_mint_authority.pubkey(),
                original_group_state.update_authority.try_into().unwrap(),
                original_group_state.max_size.into(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &original_mint_authority, &original],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let metadata_space = TlvStateBorrowed::get_base_len()
        + get_instance_packed_len(&original_metadata_state).unwrap();
    let metadata_rent_lamports = rent.minimum_balance(metadata_space);

    let reprint_space = TlvStateBorrowed::get_base_len() + std::mem::size_of::<TokenGroupMember>();
    let reprint_rent_lamports = rent.minimum_balance(reprint_space);

    // Fail: reprint mint authority not signer
    let mut init_reprint_ix = initialize_edition_reprint(
        &program_id,
        &reprint.pubkey(),
        &reprint_mint.pubkey(),
        &reprint_mint_authority.pubkey(),
        &original.pubkey(),
        &original_update_authority.pubkey(),
        &original_mint.pubkey(),
    );
    init_reprint_ix.accounts[2].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &reprint.pubkey(),
                reprint_rent_lamports,
                reprint_space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                &reprint_mint.pubkey(),
                metadata_rent_lamports,
            ),
            init_reprint_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &reprint, &original_update_authority],
        context.last_blockhash,
    );
    assert_eq!(
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(2, InstructionError::MissingRequiredSignature)
    );

    // Fail: group update authority not signer
    let mut init_reprint_ix = initialize_edition_reprint(
        &program_id,
        &reprint.pubkey(),
        &reprint_mint.pubkey(),
        &reprint_mint_authority.pubkey(),
        &original.pubkey(),
        &original_update_authority.pubkey(),
        &original_mint.pubkey(),
    );
    init_reprint_ix.accounts[4].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &reprint.pubkey(),
                reprint_rent_lamports,
                reprint_space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                &reprint_mint.pubkey(),
                metadata_rent_lamports,
            ),
            init_reprint_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &reprint, &reprint_mint_authority],
        context.last_blockhash,
    );
    assert_eq!(
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(2, InstructionError::MissingRequiredSignature)
    );

    // Success: initialize edition reprint
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &reprint.pubkey(),
                reprint_rent_lamports,
                reprint_space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                &reprint_mint.pubkey(),
                metadata_rent_lamports,
            ),
            initialize_edition_reprint(
                &program_id,
                &reprint.pubkey(),
                &reprint_mint.pubkey(),
                &reprint_mint_authority.pubkey(),
                &original.pubkey(),
                &original_update_authority.pubkey(),
                &original_mint.pubkey(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            &reprint,
            &reprint_mint_authority,
            &original_update_authority,
        ],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Fetch the reprint account and ensure it matches our state
    let reprint_account = context
        .banks_client
        .get_account(reprint.pubkey())
        .await
        .unwrap()
        .unwrap();
    let fetched_meta = TlvStateBorrowed::unpack(&reprint_account.data).unwrap();
    let fetched_original_reprint_state =
        fetched_meta.get_first_value::<TokenGroupMember>().unwrap();
    assert_eq!(fetched_original_reprint_state.group, original.pubkey());
    assert_eq!(u32::from(fetched_original_reprint_state.member_number), 1);

    // Fetch the reprint's metadata and ensure it matches our original
    let reprint_mint_account = context
        .banks_client
        .get_account(reprint_mint.pubkey())
        .await
        .unwrap()
        .unwrap();
    let fetched_reprint_meta =
        StateWithExtensions::<Mint>::unpack(&reprint_mint_account.data).unwrap();
    let fetched_reprint_metadata = fetched_reprint_meta
        .get_variable_len_extension::<TokenMetadata>()
        .unwrap();
    assert_eq!(fetched_reprint_metadata.name, original_metadata_state.name);
    assert_eq!(
        fetched_reprint_metadata.symbol,
        original_metadata_state.symbol
    );
    assert_eq!(fetched_reprint_metadata.uri, original_metadata_state.uri);
}
