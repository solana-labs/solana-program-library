#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{setup, setup_metadata, setup_mint, setup_original_print, setup_reprint},
    solana_program_test::tokio,
    solana_sdk::{pubkey::Pubkey, signature::Signer, signer::keypair::Keypair},
    spl_token_editions_interface::state::{Original, Reprint},
    spl_token_metadata_interface::state::TokenMetadata,
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
};

#[tokio::test]
async fn success_create_reprint() {
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup(&program_id).await;

    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let token_program_id = spl_token_2022::id();
    let decimals = 0;

    let update_authority_keypair = Keypair::new();
    let update_authority_pubkey = update_authority_keypair.pubkey();

    let original_metadata_keypair = Keypair::new();
    let original_metadata_pubkey = original_metadata_keypair.pubkey();

    let original_token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &original_metadata_pubkey,
        &update_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    let reprint_metadata_keypair = Keypair::new();
    let reprint_metadata_pubkey = reprint_metadata_keypair.pubkey();

    let reprint_token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &reprint_metadata_pubkey,
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
        mint: *original_token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &original_token,
        &update_authority_pubkey,
        &token_metadata,
        &original_metadata_keypair,
        &mint_authority,
        payer.clone(),
    )
    .await;
    setup_metadata(
        &reprint_token,
        &update_authority_pubkey,
        &token_metadata,
        &reprint_metadata_keypair,
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
        &original_metadata_pubkey,
        original_token.get_address(),
        &original_print,
        &original_keypair,
        &mint_authority,
    )
    .await;

    let reprint_metadata_keypair = Keypair::new();
    let reprint_metadata_pubkey = reprint_metadata_keypair.pubkey();

    let reprint_keypair = Keypair::new();
    let reprint_pubkey = reprint_keypair.pubkey();

    let reprint = Reprint {
        original: original_pubkey,
        copy: 1,
    };

    setup_reprint(
        &mut context,
        &program_id,
        &reprint_metadata_pubkey,
        reprint_token.get_address(),
        &original_pubkey,
        &original_metadata_pubkey,
        original_token.get_address(),
        &reprint,
        &reprint_keypair,
        &update_authority_keypair,
        &mint_authority,
    )
    .await;

    let fetched_reprint_account = context
        .banks_client
        .get_account(reprint_pubkey)
        .await
        .unwrap()
        .unwrap();
    let fetched_reprint_state = TlvStateBorrowed::unpack(&fetched_reprint_account.data).unwrap();
    let fetched_reprint = fetched_reprint_state
        .get_variable_len_value::<Reprint>()
        .unwrap();
    assert_eq!(fetched_reprint, reprint);
}
