#![cfg(feature = "test-sbf")]

use solana_program::instruction::AccountMeta;

mod program_test;
use {
    program_test::{setup_group, setup_program_test, TokenGroupTestContext},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError,
        signer::Signer,
        transaction::{Transaction, TransactionError},
    },
    spl_token_group_example::state::{Edition, EditionLine, MembershipLevel},
    spl_token_group_interface::{
        error::TokenGroupError, instruction::update_group_max_size, state::Group,
    },
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

#[tokio::test]
async fn success_update_edition_max_size() {
    let meta = Some(Edition {
        line: EditionLine::Original,
        membership_level: MembershipLevel::Ultimate,
    });

    let TokenGroupTestContext {
        context,
        payer,
        program_id,
        mint_keypair,
        mint_authority_keypair,
        metadata_keypair,
        group_keypair: original_print_keypair,
        group_update_authority_keypair: original_print_update_authority_keypair,
        group: original_print,
        ..
    } = setup_program_test::<Edition>("My Cool Edition", meta).await;

    let mut context = context.lock().await;

    let extra_metas = [AccountMeta::new_readonly(metadata_keypair.pubkey(), false)];

    // Hit our program to initialize the original print
    setup_group::<Edition>(
        &mut context,
        &program_id,
        &original_print_keypair,
        &mint_keypair.pubkey(),
        &mint_authority_keypair,
        &original_print,
        &extra_metas,
    )
    .await;

    let new_max_size = Some(200);

    let transaction = Transaction::new_signed_with_payer(
        &[update_group_max_size::<Edition>(
            &program_id,
            &original_print_keypair.pubkey(),
            &original_print_update_authority_keypair.pubkey(),
            new_max_size,
        )],
        Some(&payer.pubkey()),
        &[&payer, &original_print_update_authority_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let fetched_original_print_account = context
        .banks_client
        .get_account(original_print_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let fetched_meta = TlvStateBorrowed::unpack(&fetched_original_print_account.data).unwrap();
    let fetched_original_print_data = fetched_meta
        .get_first_variable_len_value::<Group<Edition>>()
        .unwrap();
    assert_eq!(fetched_original_print_data.max_size, new_max_size);
}

#[tokio::test]
async fn fail_authority_checks() {
    let meta = Some(Edition {
        line: EditionLine::Original,
        membership_level: MembershipLevel::Ultimate,
    });

    let TokenGroupTestContext {
        context,
        program_id,
        mint_keypair,
        mint_authority_keypair,
        metadata_keypair,
        group_keypair: original_print_keypair,
        group_update_authority_keypair: original_print_update_authority_keypair,
        group: original_print,
        ..
    } = setup_program_test::<Edition>("My Cool Edition", meta).await;

    let mut context = context.lock().await;

    let extra_metas = [AccountMeta::new_readonly(metadata_keypair.pubkey(), false)];

    // Hit our program to initialize the original print
    setup_group::<Edition>(
        &mut context,
        &program_id,
        &original_print_keypair,
        &mint_keypair.pubkey(),
        &mint_authority_keypair,
        &original_print,
        &extra_metas,
    )
    .await;

    let new_max_size = Some(200);

    // No signature
    let mut update_size_ix = update_group_max_size::<Edition>(
        &program_id,
        &original_print_keypair.pubkey(),
        &original_print_update_authority_keypair.pubkey(),
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
        &[update_group_max_size::<Edition>(
            &program_id,
            &original_print_keypair.pubkey(),
            &original_print_keypair.pubkey(),
            new_max_size,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &original_print_keypair],
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
