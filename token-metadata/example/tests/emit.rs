#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{setup, setup_metadata, setup_mint},
    solana_program_test::tokio,
    solana_sdk::{
        borsh1::try_from_slice_unchecked, program::MAX_RETURN_DATA, pubkey::Pubkey,
        signature::Signer, signer::keypair::Keypair, transaction::Transaction,
    },
    spl_token_metadata_interface::{borsh, instruction::emit, state::TokenMetadata},
    test_case::test_case,
};

#[test_case(Some(40), Some(40) ; "zero bytes")]
#[test_case(Some(40), Some(41) ; "one byte")]
#[test_case(Some(1_000_000), Some(1_000_001) ; "too far")]
#[test_case(Some(50), Some(49) ; "wrong way")]
#[test_case(Some(50), None ; "truncate start")]
#[test_case(None, Some(50) ; "truncate end")]
#[test_case(None, None ; "full data")]
#[tokio::test]
async fn success(start: Option<u64>, end: Option<u64>) {
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup(&program_id).await;

    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let token_program_id = spl_token_2022::id();
    let decimals = 2;
    let token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;
    let mut context = context.lock().await;

    let update_authority = Pubkey::new_unique();
    let name = "MySuperCoolToken".to_string();
    let symbol = "MINE".to_string();
    let uri = "my.super.cool.token".to_string();
    let token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority).try_into().unwrap(),
        mint: *token.get_address(),
        ..Default::default()
    };

    let metadata_keypair = Keypair::new();
    let metadata_pubkey = metadata_keypair.pubkey();

    setup_metadata(
        &mut context,
        &program_id,
        token.get_address(),
        &token_metadata,
        &metadata_keypair,
        &mint_authority,
    )
    .await;

    let transaction = Transaction::new_signed_with_payer(
        &[emit(&program_id, &metadata_pubkey, start, end)],
        Some(&payer.pubkey()),
        &[payer.as_ref()],
        context.last_blockhash,
    );
    let simulation = context
        .banks_client
        .simulate_transaction(transaction)
        .await
        .unwrap();

    let metadata_buffer = borsh::to_vec(&token_metadata).unwrap();
    if let Some(check_buffer) = TokenMetadata::get_slice(&metadata_buffer, start, end) {
        if !check_buffer.is_empty() {
            // pad the data if necessary
            let mut return_data = vec![0; MAX_RETURN_DATA];
            let simulation_return_data =
                simulation.simulation_details.unwrap().return_data.unwrap();
            assert_eq!(simulation_return_data.program_id, program_id);
            return_data[..simulation_return_data.data.len()]
                .copy_from_slice(&simulation_return_data.data);

            assert_eq!(*check_buffer, return_data[..check_buffer.len()]);
            // we're sure that we're getting the full data, so also compare the deserialized
            // type
            if start.is_none() && end.is_none() {
                let emitted_token_metadata =
                    try_from_slice_unchecked::<TokenMetadata>(&return_data).unwrap();
                assert_eq!(token_metadata, emitted_token_metadata);
            }
        } else {
            assert!(simulation.simulation_details.unwrap().return_data.is_none());
        }
    } else {
        assert!(simulation.simulation_details.unwrap().return_data.is_none());
    }
}
