#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{setup, setup_metadata, setup_mint, setup_original_print},
    solana_program_test::tokio,
    solana_sdk::{
        pubkey::Pubkey, signature::Signer, signer::keypair::Keypair, transaction::Transaction,
    },
    spl_token_editions_interface::{instruction::update_original_authority, state::Original},
    spl_token_metadata_interface::state::TokenMetadata,
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

#[tokio::test]
async fn success_update_original_max_supply() {
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup(&program_id).await;

    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let token_program_id = spl_token_2022::id();
    let decimals = 0;

    let update_authority_keypair = Keypair::new();
    let update_authority_pubkey = update_authority_keypair.pubkey();

    let metadata_keypair = Keypair::new();
    let metadata_pubkey = metadata_keypair.pubkey();

    let token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &metadata_pubkey,
        &update_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    let name = "My Cool Original Print".to_string();
    let symbol = "COOL".to_string();
    let uri = "cool.original.print".to_string();
    let token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        mint: *token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &token,
        &update_authority_pubkey,
        &token_metadata,
        &metadata_keypair,
        &mint_authority,
        payer.clone(),
    )
    .await;
    let mut context = context.lock().await;

    let original_keypair = Keypair::new();
    let original_pubkey = original_keypair.pubkey();

    let original_print = Original {
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        max_supply: Some(100),
        supply: 0,
    };

    setup_original_print(
        &mut context,
        &program_id,
        &metadata_pubkey,
        token.get_address(),
        &original_print,
        &original_keypair,
        &mint_authority,
    )
    .await;

    // Can change to new pubkey

    let new_authority_keypair = Keypair::new();
    let new_authority_pubkey = new_authority_keypair.pubkey();

    let transaction = Transaction::new_signed_with_payer(
        &[update_original_authority(
            &program_id,
            &original_pubkey,
            &update_authority_pubkey,
            Some(new_authority_pubkey),
        )],
        Some(&payer.pubkey()),
        &[&payer, &update_authority_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let fetched_original_account = context
        .banks_client
        .get_account(original_pubkey)
        .await
        .unwrap()
        .unwrap();
    let fetched_original_state = TlvStateBorrowed::unpack(&fetched_original_account.data).unwrap();
    let fetched_original_print = fetched_original_state
        .get_variable_len_value::<Original>()
        .unwrap();
    assert_eq!(
        Option::<Pubkey>::from(fetched_original_print.update_authority),
        Some(new_authority_pubkey),
    );

    // Can change to `None`

    let second_new_authority = None;

    let transaction = Transaction::new_signed_with_payer(
        &[update_original_authority(
            &program_id,
            &original_pubkey,
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

    let fetched_original_account = context
        .banks_client
        .get_account(original_pubkey)
        .await
        .unwrap()
        .unwrap();
    let fetched_original_state = TlvStateBorrowed::unpack(&fetched_original_account.data).unwrap();
    let fetched_original_print = fetched_original_state
        .get_variable_len_value::<Original>()
        .unwrap();
    assert_eq!(
        Option::<Pubkey>::from(fetched_original_print.update_authority),
        second_new_authority
    );
}
