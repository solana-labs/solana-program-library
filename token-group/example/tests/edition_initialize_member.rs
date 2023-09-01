#![cfg(feature = "test-sbf")]

use solana_program::instruction::AccountMeta;

mod program_test;
use {
    program_test::{
        setup_group, setup_member_with_metadata_rent, setup_mint_with_metadata_pointer,
        setup_program_test, TokenGroupTestContext,
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
    spl_token_group_example::state::{Edition, EditionLine, MembershipLevel},
    spl_token_group_interface::{
        error::TokenGroupError, instruction::initialize_member, state::Member,
    },
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

#[tokio::test]
async fn success_initialize_reprint() {
    let meta = Some(Edition {
        line: EditionLine::Original,
        membership_level: MembershipLevel::Ultimate,
    });

    let TokenGroupTestContext {
        context,
        client,
        payer,
        token_program_id,
        program_id,
        mint_keypair,
        mint_authority_keypair,
        metadata_keypair,
        group_keypair: original_print_keypair,
        group: original_print,
        group_token_metadata: original_print_token_metadata,
        ..
    } = setup_program_test::<Edition>("My Cool Edition", meta).await;

    // In this test (similar to `setup_group_test`):
    // - The metadata is stored in the mint (Token-2022)
    // - The reprint is in a separate account
    // - The reprint's _metadata_ update authority is the mint authority
    // - The mint is an NFT (0 decimals)
    let reprint_keypair = Keypair::new();
    let reprint_mint_keypair = Keypair::new();
    let reprint_mint_authority_keypair = Keypair::new();
    let reprint_metadata_keypair = reprint_mint_keypair.insecure_clone();
    let reprint_metadata_update_authority_keypair = reprint_metadata_keypair.insecure_clone();
    let decimals = 0;
    let reprint = Member {
        group: original_print_keypair.pubkey(),
        member_number: 1,
    };

    // Set up a mint for the reprint
    setup_mint_with_metadata_pointer(
        &Token::new(
            client.clone(),
            &token_program_id,
            &reprint_mint_keypair.pubkey(),
            Some(decimals),
            payer.clone(),
        ),
        &reprint_mint_keypair,
        &reprint_mint_authority_keypair,
        &reprint_metadata_keypair.pubkey(),
        &reprint_metadata_update_authority_keypair.pubkey(),
    )
    .await;

    let mut context = context.lock().await;

    let group_extra_metas = [AccountMeta::new_readonly(metadata_keypair.pubkey(), false)];
    let member_extra_metas = [
        AccountMeta::new(reprint_metadata_keypair.pubkey(), false),
        AccountMeta::new_readonly(reprint_metadata_update_authority_keypair.pubkey(), false),
        AccountMeta::new_readonly(metadata_keypair.pubkey(), false), // Group metadata
        AccountMeta::new_readonly(token_program_id, false),          // Token metadata program
    ];

    setup_group::<Edition>(
        &mut context,
        &program_id,
        &original_print_keypair,
        &mint_keypair.pubkey(),
        &mint_authority_keypair,
        &original_print,
        &group_extra_metas,
    )
    .await;

    setup_member_with_metadata_rent::<Edition>(
        &mut context,
        &program_id,
        &original_print_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &mint_authority_keypair,
        &original_print_token_metadata,
        &reprint_keypair,
        &reprint_mint_keypair.pubkey(),
        &reprint_mint_authority_keypair,
        &reprint,
        &member_extra_metas,
    )
    .await;

    let fetched_reprint_account = context
        .banks_client
        .get_account(reprint_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let fetched_reprint_state = TlvStateBorrowed::unpack(&fetched_reprint_account.data).unwrap();
    let fetched_reprint = fetched_reprint_state
        .get_first_variable_len_value::<Member>()
        .unwrap();
    assert_eq!(fetched_reprint, reprint);
}

#[tokio::test]
async fn fail_without_authority_signature() {
    let meta = Some(Edition {
        line: EditionLine::Original,
        membership_level: MembershipLevel::Ultimate,
    });

    let TokenGroupTestContext {
        context,
        client,
        payer,
        token_program_id,
        program_id,
        mint_keypair,
        mint_authority_keypair,
        metadata_keypair,
        group_keypair: original_print_keypair,
        group: original_print,
        group_token_metadata: original_print_token_metadata,
        ..
    } = setup_program_test::<Edition>("My Cool Edition", meta).await;

    // In this test (similar to `setup_group_test`):
    // - The metadata is stored in the mint (Token-2022)
    // - The reprint is in a separate account
    // - The reprint's _metadata_ update authority is the mint authority
    // - The mint is an NFT (0 decimals)
    let reprint_keypair = Keypair::new();
    let reprint_mint_keypair = Keypair::new();
    let reprint_mint_authority_keypair = Keypair::new();
    let reprint_metadata_keypair = reprint_mint_keypair.insecure_clone();
    let reprint_metadata_update_authority_keypair = reprint_metadata_keypair.insecure_clone();
    let decimals = 0;
    let reprint = Member {
        group: original_print_keypair.pubkey(),
        member_number: 1,
    };

    // Set up a mint for the reprint
    setup_mint_with_metadata_pointer(
        &Token::new(
            client.clone(),
            &token_program_id,
            &reprint_mint_keypair.pubkey(),
            Some(decimals),
            payer.clone(),
        ),
        &reprint_mint_keypair,
        &reprint_mint_authority_keypair,
        &reprint_metadata_keypair.pubkey(),
        &reprint_metadata_update_authority_keypair.pubkey(),
    )
    .await;

    let mut context = context.lock().await;

    let group_extra_metas = [AccountMeta::new_readonly(metadata_keypair.pubkey(), false)];
    let member_extra_metas = [
        AccountMeta::new(reprint_metadata_keypair.pubkey(), false),
        AccountMeta::new_readonly(reprint_metadata_update_authority_keypair.pubkey(), false),
        AccountMeta::new_readonly(metadata_keypair.pubkey(), false), // Group metadata
        AccountMeta::new_readonly(token_program_id, false),          // Token metadata program
    ];

    setup_group::<Edition>(
        &mut context,
        &program_id,
        &original_print_keypair,
        &mint_keypair.pubkey(),
        &mint_authority_keypair,
        &original_print,
        &group_extra_metas,
    )
    .await;

    let rent = context.banks_client.get_rent().await.unwrap();

    let token_metadata_space = original_print_token_metadata.tlv_size_of().unwrap();
    let token_metadata_rent_lamports = rent.minimum_balance(token_metadata_space);

    let space = TlvStateBorrowed::get_base_len() + get_instance_packed_len(&reprint).unwrap();
    let rent_lamports = rent.minimum_balance(space);

    // Fail missing reprint mint authority

    let mut initialize_reprint_ix = initialize_member::<Edition>(
        &program_id,
        &original_print_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &mint_authority_keypair.pubkey(),
        &reprint_keypair.pubkey(),
        &reprint_mint_keypair.pubkey(),
        &reprint_mint_authority_keypair.pubkey(),
        reprint.member_number,
        &member_extra_metas,
    );
    initialize_reprint_ix.accounts[2].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &reprint_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                &reprint_mint_keypair.pubkey(),
                token_metadata_rent_lamports,
            ),
            initialize_reprint_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &reprint_keypair, &mint_authority_keypair], /* Missing reprint mint
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

    // Fail missing original print mint authority

    let mut initialize_reprint_ix = initialize_member::<Edition>(
        &program_id,
        &original_print_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &mint_authority_keypair.pubkey(),
        &reprint_keypair.pubkey(),
        &reprint_mint_keypair.pubkey(),
        &reprint_mint_authority_keypair.pubkey(),
        reprint.member_number,
        &member_extra_metas,
    );
    initialize_reprint_ix.accounts[5].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &reprint_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                &reprint_mint_keypair.pubkey(),
                token_metadata_rent_lamports,
            ),
            initialize_reprint_ix,
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            &reprint_keypair,
            &reprint_mint_authority_keypair,
        ], /* Missing original print mint
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
    let meta = Some(Edition {
        line: EditionLine::Original,
        membership_level: MembershipLevel::Ultimate,
    });

    let TokenGroupTestContext {
        context,
        client,
        payer,
        token_program_id,
        program_id,
        mint_keypair,
        mint_authority_keypair,
        metadata_keypair,
        group_keypair: original_print_keypair,
        group: original_print,
        group_token_metadata: original_print_token_metadata,
        ..
    } = setup_program_test::<Edition>("My Cool Edition", meta).await;

    // In this test (similar to `setup_group_test`):
    // - The metadata is stored in the mint (Token-2022)
    // - The reprint is in a separate account
    // - The reprint's _metadata_ update authority is the mint authority
    // - The mint is an NFT (0 decimals)
    let reprint_keypair = Keypair::new();
    let reprint_mint_keypair = Keypair::new();
    let reprint_mint_authority_keypair = Keypair::new();
    let reprint_metadata_keypair = reprint_mint_keypair.insecure_clone();
    let reprint_metadata_update_authority_keypair = reprint_metadata_keypair.insecure_clone();
    let decimals = 0;
    let reprint = Member {
        group: original_print_keypair.pubkey(),
        member_number: 1,
    };

    // Set up a mint for the reprint
    setup_mint_with_metadata_pointer(
        &Token::new(
            client.clone(),
            &token_program_id,
            &reprint_mint_keypair.pubkey(),
            Some(decimals),
            payer.clone(),
        ),
        &reprint_mint_keypair,
        &reprint_mint_authority_keypair,
        &reprint_metadata_keypair.pubkey(),
        &reprint_metadata_update_authority_keypair.pubkey(),
    )
    .await;

    let mut context = context.lock().await;

    let group_extra_metas = [AccountMeta::new_readonly(metadata_keypair.pubkey(), false)];
    let member_extra_metas = [
        AccountMeta::new(reprint_metadata_keypair.pubkey(), false),
        AccountMeta::new_readonly(reprint_metadata_update_authority_keypair.pubkey(), false),
        AccountMeta::new_readonly(metadata_keypair.pubkey(), false), // Group metadata
        AccountMeta::new_readonly(token_program_id, false),          // Token metadata program
    ];

    setup_group::<Edition>(
        &mut context,
        &program_id,
        &original_print_keypair,
        &mint_keypair.pubkey(),
        &mint_authority_keypair,
        &original_print,
        &group_extra_metas,
    )
    .await;

    let rent = context.banks_client.get_rent().await.unwrap();

    let token_metadata_space = original_print_token_metadata.tlv_size_of().unwrap();
    let token_metadata_rent_lamports = rent.minimum_balance(token_metadata_space);

    let space = TlvStateBorrowed::get_base_len() + get_instance_packed_len(&reprint).unwrap();
    let rent_lamports = rent.minimum_balance(space);

    // Fail incorrect reprint mint authority

    let mut initialize_reprint_ix = initialize_member::<Edition>(
        &program_id,
        &original_print_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &mint_authority_keypair.pubkey(),
        &reprint_keypair.pubkey(),
        &reprint_mint_keypair.pubkey(),
        &mint_authority_keypair.pubkey(), // NOT the reprint mint authority
        reprint.member_number,
        &member_extra_metas,
    );
    initialize_reprint_ix.accounts[5].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &reprint_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                &reprint_mint_keypair.pubkey(),
                token_metadata_rent_lamports,
            ),
            initialize_reprint_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &reprint_keypair, &mint_authority_keypair],
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

    // Fail missing original print mint authority

    let mut initialize_reprint_ix = initialize_member::<Edition>(
        &program_id,
        &original_print_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &reprint_mint_authority_keypair.pubkey(), // NOT the original print mint authority
        &reprint_keypair.pubkey(),
        &reprint_mint_keypair.pubkey(),
        &reprint_mint_authority_keypair.pubkey(),
        reprint.member_number,
        &member_extra_metas,
    );
    initialize_reprint_ix.accounts[2].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &reprint_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                &reprint_mint_keypair.pubkey(),
                token_metadata_rent_lamports,
            ),
            initialize_reprint_ix,
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            &reprint_keypair,
            &reprint_mint_authority_keypair,
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

    // Fail missing extra metas

    let initialize_reprint_ix = initialize_member::<Edition>(
        &program_id,
        &original_print_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &mint_authority_keypair.pubkey(),
        &reprint_keypair.pubkey(),
        &reprint_mint_keypair.pubkey(),
        &reprint_mint_authority_keypair.pubkey(),
        reprint.member_number,
        &[], // Missing extra metas
    );

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &reprint_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                &reprint_mint_keypair.pubkey(),
                token_metadata_rent_lamports,
            ),
            initialize_reprint_ix,
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            &reprint_keypair,
            &mint_authority_keypair,
            &reprint_mint_authority_keypair,
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
        TransactionError::InstructionError(2, InstructionError::NotEnoughAccountKeys)
    );
}
