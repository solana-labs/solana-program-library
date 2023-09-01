#![cfg(feature = "test-sbf")]

mod program_test;
use {
    borsh::{BorshDeserialize, BorshSerialize},
    program_test::{
        setup_group, setup_member, setup_member_with_metadata_rent, setup_mint_and_metadata,
        setup_mint_with_metadata_pointer, setup_program_test, TokenGroupTestContext,
    },
    solana_program_test::{tokio, ProgramTestContext},
    solana_sdk::{
        borsh::try_from_slice_unchecked, instruction::AccountMeta, program::MAX_RETURN_DATA,
        pubkey::Pubkey, signature::Signer, signer::keypair::Keypair, transaction::Transaction,
    },
    spl_token_client::token::Token,
    spl_token_group_example::state::{Collection, Edition, EditionLine, MembershipLevel},
    spl_token_group_interface::{
        instruction::{emit, get_emit_slice, ItemType},
        state::{Group, Member},
    },
    spl_token_metadata_interface::state::TokenMetadata,
    test_case::test_case,
};

#[allow(clippy::too_many_arguments)]
async fn check_emit<V: BorshDeserialize + BorshSerialize + std::fmt::Debug + PartialEq>(
    context: &mut ProgramTestContext,
    print_buffer: Vec<u8>,
    print_pubkey: &Pubkey,
    start: Option<u64>,
    end: Option<u64>,
    item_type: ItemType,
    program_id: &Pubkey,
    payer: &Keypair,
    check_print_data: V,
) {
    let transaction = Transaction::new_signed_with_payer(
        &[emit::<Collection>(
            program_id,
            print_pubkey,
            start,
            end,
            item_type,
        )],
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
async fn success_emit_collection(start: Option<u64>, end: Option<u64>) {
    let collection_state = Collection {
        creation_date: "August 15".to_string(),
    };

    let TokenGroupTestContext {
        context,
        client,
        payer,
        token_program_id,
        program_id,
        mint_keypair,
        mint_authority_keypair,
        group_keypair: collection_keypair,
        group: mut collection,
        ..
    } = setup_program_test::<Collection>("My Cool Collection", Some(collection_state)).await;

    // In this test (similar to `setup_collection_test`):
    // - The metadata is stored in the mint (Token-2022)
    // - The member is in a separate account
    // - The member's _metadata_ update authority is the mint authority
    // - The _member_ update authority is also the mint authority
    // - The mint is an NFT (0 decimals)
    let member_keypair = Keypair::new();
    let member_mint_keypair = Keypair::new();
    let member_mint_authority_keypair = Keypair::new();
    let member_metadata_keypair = member_mint_keypair.insecure_clone();
    let member_metadata_update_authority_keypair = member_metadata_keypair.insecure_clone();
    let member_update_authority_keypair = member_metadata_keypair.insecure_clone();
    let decimals = 0;
    let member = Member {
        group: collection_keypair.pubkey(),
        member_number: 1,
    };

    // Size will be 1 after creation of a member
    collection.size = 1;

    // Set up a mint and metadata for the member
    setup_mint_and_metadata(
        &Token::new(
            client.clone(),
            &token_program_id,
            &member_mint_keypair.pubkey(),
            Some(decimals),
            payer.clone(),
        ),
        &member_mint_keypair,
        &member_mint_authority_keypair,
        &member_metadata_keypair.pubkey(),
        &member_metadata_update_authority_keypair.pubkey(),
        &TokenMetadata {
            name: "I'm a Member!".to_string(),
            symbol: "MEM".to_string(),
            uri: "member.com".to_string(),
            update_authority: Some(member_update_authority_keypair.pubkey())
                .try_into()
                .unwrap(),
            mint: member_mint_keypair.pubkey(),
            ..Default::default()
        },
        payer.clone(),
    )
    .await;

    let mut context = context.lock().await;

    setup_group::<Collection>(
        &mut context,
        &program_id,
        &collection_keypair,
        &mint_keypair.pubkey(),
        &mint_authority_keypair,
        &collection,
        &[], // No extra account metas
    )
    .await;

    setup_member::<Collection>(
        &mut context,
        &program_id,
        &collection_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &mint_authority_keypair,
        &member_keypair,
        &member_mint_keypair.pubkey(),
        &member_mint_authority_keypair,
        &member,
        &[], // No extra account metas
    )
    .await;

    // Group
    let collection_buffer = collection.try_to_vec().unwrap();
    check_emit::<Group<Collection>>(
        &mut context,
        collection_buffer,
        &collection_keypair.pubkey(),
        start,
        end,
        ItemType::Group,
        &program_id,
        &payer,
        collection,
    )
    .await;

    // Member
    let member_buffer = member.try_to_vec().unwrap();
    check_emit::<Member>(
        &mut context,
        member_buffer,
        &member_keypair.pubkey(),
        start,
        end,
        ItemType::Member,
        &program_id,
        &payer,
        member,
    )
    .await;
}

#[test_case(Some(20), Some(20) ; "zero bytes")]
#[test_case(Some(20), Some(21) ; "one byte")]
#[test_case(Some(1_000_000), Some(1_000_001) ; "too far")]
#[test_case(Some(30), Some(29) ; "wrong way")]
#[test_case(Some(30), None ; "truncate start")]
#[test_case(None, Some(30) ; "truncate end")]
#[test_case(None, None ; "full data")]
#[tokio::test]
async fn success_emit_edition(start: Option<u64>, end: Option<u64>) {
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
        group: mut original_print,
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

    // Size will be 1 after creation of a reprint
    original_print.size = 1;

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
        AccountMeta::new_readonly(reprint_metadata_keypair.pubkey(), false),
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

    // Group
    let original_print_buffer = original_print.try_to_vec().unwrap();
    check_emit::<Group<Edition>>(
        &mut context,
        original_print_buffer,
        &original_print_keypair.pubkey(),
        start,
        end,
        ItemType::Group,
        &program_id,
        &payer,
        original_print,
    )
    .await;

    // Member
    let reprint_buffer = reprint.try_to_vec().unwrap();
    check_emit::<Member>(
        &mut context,
        reprint_buffer,
        &reprint_keypair.pubkey(),
        start,
        end,
        ItemType::Member,
        &program_id,
        &payer,
        reprint,
    )
    .await;
}
