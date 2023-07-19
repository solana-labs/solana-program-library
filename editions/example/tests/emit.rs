#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{setup, setup_metadata, setup_mint, setup_original_print, setup_reprint},
    solana_program_test::{tokio, ProgramTestContext},
    solana_sdk::{
        borsh::try_from_slice_unchecked, program::MAX_RETURN_DATA, pubkey::Pubkey,
        signature::Signer, signer::keypair::Keypair, transaction::Transaction,
    },
    spl_token_editions_interface::{
        borsh::{BorshDeserialize, BorshSerialize},
        instruction::{emit, PrintType},
        state::{get_emit_slice, Original, Reprint},
    },
    spl_token_metadata_interface::state::TokenMetadata,
    test_case::test_case,
};

#[allow(clippy::too_many_arguments)]
async fn check_emit<V: BorshDeserialize + BorshSerialize + std::fmt::Debug + PartialEq>(
    context: &mut ProgramTestContext,
    print_type: PrintType,
    print_buffer: Vec<u8>,
    print_pubkey: &Pubkey,
    start: Option<u64>,
    end: Option<u64>,
    program_id: &Pubkey,
    payer: &Keypair,
    check_print_data: V,
) {
    let transaction = Transaction::new_signed_with_payer(
        &[emit(program_id, print_pubkey, print_type, start, end)],
        Some(&payer.pubkey()),
        &[payer],
        context.last_blockhash,
    );
    let simulation = context
        .banks_client
        .simulate_transaction(transaction)
        .await
        .unwrap();

    if let Some(check_buffer) = get_emit_slice(&print_buffer, start, end) {
        if !check_buffer.is_empty() {
            // pad the data if necessary
            let mut return_data = vec![0; MAX_RETURN_DATA];
            let simulation_return_data =
                simulation.simulation_details.unwrap().return_data.unwrap();
            assert_eq!(simulation_return_data.program_id, *program_id);
            return_data[..simulation_return_data.data.len()]
                .copy_from_slice(&simulation_return_data.data);

            assert_eq!(*check_buffer, return_data[..check_buffer.len()]);
            // we're sure that we're getting the full data, so also compare the deserialized
            // type
            if start.is_none() && end.is_none() {
                let emitted_token_edition = try_from_slice_unchecked::<V>(&return_data).unwrap();
                assert_eq!(check_print_data, emitted_token_edition);
            }
        } else {
            assert!(simulation.simulation_details.unwrap().return_data.is_none());
        }
    } else {
        assert!(simulation.simulation_details.unwrap().return_data.is_none());
    }
}

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
    let metadata_program_id = spl_token_2022::id();
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
    let mut context = context.lock().await;

    let original_keypair = Keypair::new();
    let original_pubkey = original_keypair.pubkey();

    let original_print = Original {
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        max_supply: Some(100),
        supply: 1, // Supply will be 1 after printing a reprint
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
        &metadata_program_id,
        &reprint,
        &token_metadata,
        &reprint_keypair,
        &update_authority_keypair,
        &mint_authority,
    )
    .await;

    // Original
    let original_buffer = original_print.try_to_vec().unwrap();
    check_emit::<Original>(
        &mut context,
        PrintType::Original,
        original_buffer,
        &original_pubkey,
        start,
        end,
        &program_id,
        &payer,
        original_print,
    )
    .await;

    // Reprint
    let reprint_buffer = reprint.try_to_vec().unwrap();
    check_emit::<Reprint>(
        &mut context,
        PrintType::Reprint,
        reprint_buffer,
        &reprint_pubkey,
        start,
        end,
        &program_id,
        &payer,
        reprint,
    )
    .await;
}
