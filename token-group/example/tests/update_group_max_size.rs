#![cfg(feature = "test-sbf")]

mod setup;

use {
    setup::{setup_mint, setup_program_test},
    solana_program::{instruction::InstructionError, pubkey::Pubkey, system_instruction},
    solana_program_test::tokio,
    solana_sdk::{
        account::Account as SolanaAccount,
        signature::Keypair,
        signer::Signer,
        transaction::{Transaction, TransactionError},
    },
    spl_token_client::token::Token,
    spl_token_group_interface::{
        error::TokenGroupError,
        instruction::{initialize_group, update_group_max_size},
        state::TokenGroup,
    },
    spl_type_length_value::state::{TlvState, TlvStateBorrowed, TlvStateMut},
};

#[tokio::test]
async fn test_update_group_max_size() {
    let program_id = Pubkey::new_unique();
    let group = Keypair::new();
    let group_mint = Keypair::new();
    let group_mint_authority = Keypair::new();
    let group_update_authority = Keypair::new();

    let group_state = TokenGroup::new(
        &group_mint.pubkey(),
        Some(group_update_authority.pubkey()).try_into().unwrap(),
        50,
    );

    let (context, client, payer) = setup_program_test(&program_id).await;

    let token_client = Token::new(
        client,
        &spl_token_2022::id(),
        &group_mint.pubkey(),
        Some(0),
        payer.clone(),
    );
    setup_mint(&token_client, &group_mint, &group_mint_authority).await;

    let mut context = context.lock().await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let space = TlvStateBorrowed::get_base_len() + std::mem::size_of::<TokenGroup>();
    let rent_lamports = rent.minimum_balance(space);

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &group.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            initialize_group(
                &program_id,
                &group.pubkey(),
                &group_mint.pubkey(),
                &group_mint_authority.pubkey(),
                group_state.update_authority.into(),
                group_state.max_size.into(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &group_mint_authority, &group],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Fail: update authority not signer
    let mut update_ix = update_group_max_size(&program_id, &group.pubkey(), &group.pubkey(), 100);
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
        &[update_group_max_size(
            &program_id,
            &group.pubkey(),
            &group.pubkey(),
            100,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &group],
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

    // Fail: size exceeds new max size
    let fetched_group_account = context
        .banks_client
        .get_account(group.pubkey())
        .await
        .unwrap()
        .unwrap();
    let mut data = fetched_group_account.data;
    let mut state = TlvStateMut::unpack(&mut data).unwrap();
    let group_data = state.get_first_value_mut::<TokenGroup>().unwrap();
    group_data.size = 30.into();
    context.set_account(
        &group.pubkey(),
        &SolanaAccount {
            data,
            ..fetched_group_account
        }
        .into(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &[update_group_max_size(
            &program_id,
            &group.pubkey(),
            &group_update_authority.pubkey(),
            20,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &group_update_authority],
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
            InstructionError::Custom(TokenGroupError::SizeExceedsNewMaxSize as u32)
        )
    );

    // Success: update max size
    let transaction = Transaction::new_signed_with_payer(
        &[update_group_max_size(
            &program_id,
            &group.pubkey(),
            &group_update_authority.pubkey(),
            100,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &group_update_authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Fetch the account and assert the new max size
    let fetched_group_account = context
        .banks_client
        .get_account(group.pubkey())
        .await
        .unwrap()
        .unwrap();
    let fetched_meta = TlvStateBorrowed::unpack(&fetched_group_account.data).unwrap();
    let fetched_group_state = fetched_meta.get_first_value::<TokenGroup>().unwrap();
    assert_eq!(fetched_group_state.max_size, 100.into());
}

// Fail: immutable group
#[tokio::test]
async fn test_update_group_max_size_fail_immutable_group() {
    let program_id = Pubkey::new_unique();
    let group = Keypair::new();
    let group_mint = Keypair::new();
    let group_mint_authority = Keypair::new();
    let group_update_authority = Keypair::new();

    let group_state = TokenGroup::new(
        &group_mint.pubkey(),
        Some(group_update_authority.pubkey()).try_into().unwrap(),
        50,
    );

    let (context, client, payer) = setup_program_test(&program_id).await;

    let token_client = Token::new(
        client,
        &spl_token_2022::id(),
        &group_mint.pubkey(),
        Some(0),
        payer.clone(),
    );
    setup_mint(&token_client, &group_mint, &group_mint_authority).await;

    let mut context = context.lock().await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let space = TlvStateBorrowed::get_base_len() + std::mem::size_of::<TokenGroup>();
    let rent_lamports = rent.minimum_balance(space);

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &group.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                &program_id,
            ),
            initialize_group(
                &program_id,
                &group.pubkey(),
                &group_mint.pubkey(),
                &group_mint_authority.pubkey(),
                None,
                group_state.max_size.into(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &group_mint_authority, &group],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[update_group_max_size(
            &program_id,
            &group.pubkey(),
            &group_update_authority.pubkey(),
            100,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &group_update_authority],
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
