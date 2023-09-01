#![cfg(feature = "test-sbf")]

use solana_program::instruction::AccountMeta;

mod program_test;
use {
    program_test::{setup_group, setup_program_test, TokenGroupTestContext},
    solana_program_test::tokio,
    solana_sdk::{
        borsh::get_instance_packed_len,
        instruction::InstructionError,
        pubkey::Pubkey,
        signer::Signer,
        system_instruction,
        transaction::{Transaction, TransactionError},
    },
    spl_token_group_example::state::{Edition, EditionLine, MembershipLevel},
    spl_token_group_interface::{
        error::TokenGroupError, instruction::initialize_group, state::Group,
    },
    spl_type_length_value::{
        error::TlvError,
        state::{TlvState, TlvStateBorrowed},
    },
};

#[tokio::test]
async fn success_initialize_original_print() {
    let meta = Some(Edition {
        line: EditionLine::Original,
        membership_level: MembershipLevel::Ultimate,
    });

    // Setup a test for creating a token `Edition`:
    // - Mint:         An NFT representing the `Edition` mint (original print)
    // - Metadata:     A `TokenMetadata` representing the original print's metadata
    // - Edition:      An `Edition` representing the `Edition` group
    let TokenGroupTestContext {
        context,
        payer,
        program_id,
        mint_keypair,
        mint_authority_keypair,
        metadata_keypair,
        group_keypair: original_print_keypair,
        group: original_print,
        ..
    } = setup_program_test::<Edition>("My Cool Edition", meta.clone()).await;

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

    // Fetch the original print account and ensure it matches our state
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
    assert_eq!(fetched_original_print, original_print);

    // Fail doing it again, and change some params to ensure a new tx
    {
        let transaction = Transaction::new_signed_with_payer(
            &[initialize_group::<Edition>(
                &program_id,
                &original_print_keypair.pubkey(),
                &mint_keypair.pubkey(),
                &mint_authority_keypair.pubkey(),
                None, // Intentionally changed params
                Some(500),
                &meta,
                &extra_metas,
            )],
            Some(&payer.pubkey()),
            &[&payer, &mint_authority_keypair],
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
                InstructionError::Custom(TlvError::TypeAlreadyExists as u32)
            )
        );
    }
}

#[tokio::test]
async fn fail_without_authority_signature() {
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
        group: original_print,
        ..
    } = setup_program_test::<Edition>("My Cool Edition", meta.clone()).await;

    let mut context = context.lock().await;

    let extra_metas = [AccountMeta::new_readonly(metadata_keypair.pubkey(), false)];

    let rent = context.banks_client.get_rent().await.unwrap();
    let space =
        TlvStateBorrowed::get_base_len() + get_instance_packed_len(&original_print).unwrap();
    let rent_lamports = rent.minimum_balance(space);
    let mut initialize_group_ix = initialize_group::<Edition>(
        &program_id,
        &original_print_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &mint_authority_keypair.pubkey(),
        Option::<Pubkey>::from(original_print.update_authority),
        original_print.max_size,
        &meta,
        &extra_metas,
    );
    initialize_group_ix.accounts[2].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &original_print_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            initialize_group_ix,
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &original_print_keypair], // Missing mint authority
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
        TransactionError::InstructionError(1, InstructionError::MissingRequiredSignature,)
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
        program_id,
        mint_keypair,
        metadata_keypair,
        group_keypair: original_print_keypair,
        group: original_print,
        ..
    } = setup_program_test::<Edition>("My Cool Edition", meta.clone()).await;

    let mut context = context.lock().await;

    let extra_metas = [AccountMeta::new_readonly(metadata_keypair.pubkey(), false)];

    let rent = context.banks_client.get_rent().await.unwrap();
    let space =
        TlvStateBorrowed::get_base_len() + get_instance_packed_len(&original_print).unwrap();
    let rent_lamports = rent.minimum_balance(space);
    let mut initialize_group_ix = initialize_group::<Edition>(
        &program_id,
        &original_print_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &original_print_keypair.pubkey(), // NOT the mint authority
        Option::<Pubkey>::from(original_print.update_authority),
        original_print.max_size,
        &meta,
        &extra_metas,
    );
    initialize_group_ix.accounts[2].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &original_print_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            initialize_group_ix,
        ],
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
            1,
            InstructionError::Custom(TokenGroupError::IncorrectAuthority as u32)
        )
    );
}

#[tokio::test]
async fn fail_missing_extra_metas() {
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
        group: original_print,
        ..
    } = setup_program_test::<Edition>("My Cool Edition", meta.clone()).await;

    let mut context = context.lock().await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let space =
        TlvStateBorrowed::get_base_len() + get_instance_packed_len(&original_print).unwrap();
    let rent_lamports = rent.minimum_balance(space);
    let initialize_group_ix = initialize_group::<Edition>(
        &program_id,
        &original_print_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &mint_authority_keypair.pubkey(),
        Option::<Pubkey>::from(original_print.update_authority),
        original_print.max_size,
        &meta,
        &[], // Missing extra metas
    );
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &original_print_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            initialize_group_ix,
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            &original_print_keypair,
            &mint_authority_keypair,
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
        TransactionError::InstructionError(1, InstructionError::NotEnoughAccountKeys)
    );
}
