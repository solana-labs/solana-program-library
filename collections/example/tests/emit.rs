#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{setup, setup_collection, setup_member, setup_metadata, setup_mint},
    solana_program_test::{tokio, ProgramTestContext},
    solana_sdk::{
        borsh::try_from_slice_unchecked, program::MAX_RETURN_DATA, pubkey::Pubkey,
        signature::Signer, signer::keypair::Keypair, transaction::Transaction,
    },
    spl_token_collections_interface::{
        borsh::{BorshDeserialize, BorshSerialize},
        instruction::{emit, ItemType},
        state::{get_emit_slice, Collection, Member},
    },
    spl_token_metadata_interface::state::TokenMetadata,
    test_case::test_case,
};

#[allow(clippy::too_many_arguments)]
async fn check_emit<V: BorshDeserialize + BorshSerialize + std::fmt::Debug + PartialEq>(
    context: &mut ProgramTestContext,
    item_type: ItemType,
    print_buffer: Vec<u8>,
    print_pubkey: &Pubkey,
    start: Option<u64>,
    end: Option<u64>,
    program_id: &Pubkey,
    payer: &Keypair,
    check_print_data: V,
) {
    let transaction = Transaction::new_signed_with_payer(
        &[emit(program_id, print_pubkey, item_type, start, end)],
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
                let emitted_token_collection = try_from_slice_unchecked::<V>(&return_data).unwrap();
                assert_eq!(check_print_data, emitted_token_collection);
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
    let decimals = 0;

    let update_authority_keypair = Keypair::new();
    let update_authority_pubkey = update_authority_keypair.pubkey();

    let collection_metadata_keypair = Keypair::new();
    let collection_metadata_pubkey = collection_metadata_keypair.pubkey();

    let collection_token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &collection_metadata_pubkey,
        &update_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    let member_metadata_keypair = Keypair::new();
    let member_metadata_pubkey = member_metadata_keypair.pubkey();

    let member_token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        &member_metadata_pubkey,
        &update_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    let name = "My Cool Collection".to_string();
    let symbol = "COOL".to_string();
    let uri = "cool.collection.com".to_string();
    let token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        mint: *collection_token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &collection_token,
        &update_authority_pubkey,
        &token_metadata,
        &collection_metadata_keypair,
        &mint_authority,
        payer.clone(),
    )
    .await;

    // For demonstration purposes, we'll set up _different_ metadata for
    // the collection member
    let name = "I'm a member of the Cool Collection!".to_string();
    let symbol = "YAY".to_string();
    let uri = "i.am.a.member".to_string();
    let token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        mint: *member_token.get_address(),
        ..Default::default()
    };

    setup_metadata(
        &member_token,
        &update_authority_pubkey,
        &token_metadata,
        &member_metadata_keypair,
        &mint_authority,
        payer.clone(),
    )
    .await;
    let mut context = context.lock().await;

    let collection_keypair = Keypair::new();
    let collection_pubkey = collection_keypair.pubkey();

    let collection = Collection {
        update_authority: Some(update_authority_pubkey).try_into().unwrap(),
        max_size: Some(100),
        size: 1, // Supply will be 1 after printing a member
    };

    setup_collection(
        &mut context,
        &program_id,
        collection_token.get_address(),
        &collection,
        &collection_keypair,
        &mint_authority,
    )
    .await;

    let member_keypair = Keypair::new();
    let member_pubkey = member_keypair.pubkey();

    let member = Member {
        collection: collection_pubkey,
    };

    setup_member(
        &mut context,
        &program_id,
        member_token.get_address(),
        &collection_pubkey,
        collection_token.get_address(),
        &member_keypair,
        &mint_authority,
        &mint_authority,
    )
    .await;

    // Collection
    let collection_buffer = collection.try_to_vec().unwrap();
    check_emit::<Collection>(
        &mut context,
        ItemType::Collection,
        collection_buffer,
        &collection_pubkey,
        start,
        end,
        &program_id,
        &payer,
        collection,
    )
    .await;

    // Member
    let member_buffer = member.try_to_vec().unwrap();
    check_emit::<Member>(
        &mut context,
        ItemType::Member,
        member_buffer,
        &member_pubkey,
        start,
        end,
        &program_id,
        &payer,
        member,
    )
    .await;
}
