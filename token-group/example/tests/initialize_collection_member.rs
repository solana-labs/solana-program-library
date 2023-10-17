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
        instruction::{initialize_group, initialize_member},
        state::{TokenGroup, TokenGroupMember},
    },
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

#[tokio::test]
async fn test_initialize_collection_member() {
    let program_id = Pubkey::new_unique();
    let collection = Keypair::new();
    let collection_mint = Keypair::new();
    let collection_mint_authority = Keypair::new();
    let collection_update_authority = Keypair::new();
    let member = Keypair::new();
    let member_mint = Keypair::new();
    let member_mint_authority = Keypair::new();

    let collection_group_state = TokenGroup {
        update_authority: Some(collection_update_authority.pubkey())
            .try_into()
            .unwrap(),
        size: 30.into(),
        max_size: 50.into(),
    };

    let (context, client, payer) = setup_program_test(&program_id).await;

    setup_mint(
        &Token::new(
            client.clone(),
            &spl_token_2022::id(),
            &collection_mint.pubkey(),
            Some(0),
            payer.clone(),
        ),
        &collection_mint,
        &collection_mint_authority,
        vec![],
    )
    .await;
    setup_mint(
        &Token::new(
            client.clone(),
            &spl_token_2022::id(),
            &member_mint.pubkey(),
            Some(0),
            payer.clone(),
        ),
        &member_mint,
        &member_mint_authority,
        vec![],
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

    let member_space = TlvStateBorrowed::get_base_len() + std::mem::size_of::<TokenGroupMember>();
    let member_rent_lamports = rent.minimum_balance(member_space);

    // Fail: member mint authority not signer
    let mut init_member_ix = initialize_member(
        &program_id,
        &member.pubkey(),
        &member_mint.pubkey(),
        &member_mint_authority.pubkey(),
        &collection.pubkey(),
        &collection_update_authority.pubkey(),
    );
    init_member_ix.accounts[2].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &member.pubkey(),
                member_rent_lamports,
                member_space.try_into().unwrap(),
                &program_id,
            ),
            init_member_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &member, &collection_update_authority],
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

    // Fail: group update authority not signer
    let mut init_member_ix = initialize_member(
        &program_id,
        &member.pubkey(),
        &member_mint.pubkey(),
        &member_mint_authority.pubkey(),
        &collection.pubkey(),
        &collection_update_authority.pubkey(),
    );
    init_member_ix.accounts[4].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &member.pubkey(),
                member_rent_lamports,
                member_space.try_into().unwrap(),
                &program_id,
            ),
            init_member_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &member, &member_mint_authority],
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

    // Success: initialize member
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &member.pubkey(),
                member_rent_lamports,
                member_space.try_into().unwrap(),
                &program_id,
            ),
            initialize_member(
                &program_id,
                &member.pubkey(),
                &member_mint.pubkey(),
                &member_mint_authority.pubkey(),
                &collection.pubkey(),
                &collection_update_authority.pubkey(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            &member,
            &member_mint_authority,
            &collection_update_authority,
        ],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Fetch the member account and ensure it matches our state
    let member_account = context
        .banks_client
        .get_account(member.pubkey())
        .await
        .unwrap()
        .unwrap();
    let fetched_meta = TlvStateBorrowed::unpack(&member_account.data).unwrap();
    let fetched_collection_member_state =
        fetched_meta.get_first_value::<TokenGroupMember>().unwrap();
    assert_eq!(fetched_collection_member_state.group, collection.pubkey());
    assert_eq!(u32::from(fetched_collection_member_state.member_number), 1);
}
