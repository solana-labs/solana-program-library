#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{setup_group, setup_program_test, TokenGroupTestContext},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        transaction::{Transaction, TransactionError},
    },
    spl_token_group_example::state::{Edition, EditionLine, MembershipLevel},
    spl_token_group_interface::{
        error::TokenGroupError, instruction::update_group_authority, state::Group,
    },
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

#[tokio::test]
async fn success_update_edition_authority() {
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
        group_keypair: original_print_keypair,
        group_update_authority_keypair: original_print_update_authority_keypair,
        group: original_print,
        ..
    } = setup_program_test::<Edition>("My Cool Edition", meta).await;

    let mut context = context.lock().await;

    // Hit our program to initialize the original print
    setup_group::<Edition>(
        &mut context,
        &program_id,
        &original_print_keypair,
        &mint_keypair.pubkey(),
        &mint_authority_keypair,
        &original_print,
    )
    .await;

    let new_authority_keypair = Keypair::new();
    let new_authority_pubkey = new_authority_keypair.pubkey();

    let transaction = Transaction::new_signed_with_payer(
        &[update_group_authority::<Edition>(
            &program_id,
            &original_print_keypair.pubkey(),
            &original_print_update_authority_keypair.pubkey(),
            Some(new_authority_pubkey),
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
    let fetched_original_print = fetched_meta
        .get_first_variable_len_value::<Group<Edition>>()
        .unwrap();
    assert_eq!(
        Option::<Pubkey>::from(fetched_original_print.update_authority),
        Some(new_authority_pubkey),
    );

    // Can change to `None`

    let second_new_authority = None;

    let transaction = Transaction::new_signed_with_payer(
        &[update_group_authority::<Edition>(
            &program_id,
            &original_print_keypair.pubkey(),
            &new_authority_pubkey,
            second_new_authority,
        )],
        Some(&payer.pubkey()),
        &[&payer, &new_authority_keypair],
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
    let fetched_original_print = fetched_meta
        .get_first_variable_len_value::<Group<Edition>>()
        .unwrap();
    assert_eq!(
        Option::<Pubkey>::from(fetched_original_print.update_authority),
        second_new_authority
    );
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
        group_keypair: original_print_keypair,
        group_update_authority_keypair: original_print_update_authority_keypair,
        group: original_print,
        ..
    } = setup_program_test::<Edition>("My Cool Edition", meta).await;

    let mut context = context.lock().await;

    // Hit our program to initialize the original print
    setup_group::<Edition>(
        &mut context,
        &program_id,
        &original_print_keypair,
        &mint_keypair.pubkey(),
        &mint_authority_keypair,
        &original_print,
    )
    .await;

    let new_authority_keypair = Keypair::new();
    let new_authority_pubkey = new_authority_keypair.pubkey();

    // No signature
    let mut update_authority_ix = update_group_authority::<Edition>(
        &program_id,
        &original_print_keypair.pubkey(),
        &original_print_update_authority_keypair.pubkey(),
        Some(new_authority_pubkey),
    );
    update_authority_ix.accounts[1].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[update_authority_ix],
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
        &[update_group_authority::<Edition>(
            &program_id,
            &original_print_keypair.pubkey(),
            &original_print_keypair.pubkey(),
            Some(new_authority_pubkey),
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
