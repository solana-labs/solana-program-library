#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::TestContext,
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{
        borsh1::try_from_slice_unchecked, program::MAX_RETURN_DATA, pubkey::Pubkey,
        signature::Signer, signer::keypair::Keypair, transaction::Transaction,
    },
    spl_token_2022::processor::Processor,
    spl_token_client::token::ExtensionInitializationParams,
    spl_token_metadata_interface::{instruction::emit, state::TokenMetadata},
    std::{convert::TryInto, sync::Arc},
    test_case::test_case,
};

fn setup_program_test() -> ProgramTest {
    let mut program_test = ProgramTest::default();
    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(Processor::process),
    );
    program_test
}

async fn setup(mint: Keypair, authority: &Pubkey) -> TestContext {
    let program_test = setup_program_test();

    let context = program_test.start_with_context().await;
    let context = Arc::new(tokio::sync::Mutex::new(context));
    let mut context = TestContext {
        context,
        token_context: None,
    };
    let metadata_address = Some(mint.pubkey());
    context
        .init_token_with_mint_keypair_and_freeze_authority(
            mint,
            vec![ExtensionInitializationParams::MetadataPointer {
                authority: Some(*authority),
                metadata_address,
            }],
            None,
        )
        .await
        .unwrap();
    context
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
    let program_id = spl_token_2022::id();
    let authority = Keypair::new();
    let mint_keypair = Keypair::new();
    let mut test_context = setup(mint_keypair, &authority.pubkey()).await;
    let payer_pubkey = test_context.context.lock().await.payer.pubkey();
    let token_context = test_context.token_context.take().unwrap();

    let update_authority = Keypair::new();
    let name = "MySuperCoolToken".to_string();
    let symbol = "MINE".to_string();
    let uri = "my.super.cool.token".to_string();
    let token_metadata = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(update_authority.pubkey()).try_into().unwrap(),
        mint: *token_context.token.get_address(),
        ..Default::default()
    };

    token_context
        .token
        .token_metadata_initialize_with_rent_transfer(
            &payer_pubkey,
            &update_authority.pubkey(),
            &token_context.mint_authority.pubkey(),
            token_metadata.name.clone(),
            token_metadata.symbol.clone(),
            token_metadata.uri.clone(),
            &[&token_context.mint_authority],
        )
        .await
        .unwrap();

    let context = test_context.context.lock().await;

    let transaction = Transaction::new_signed_with_payer(
        &[emit(
            &program_id,
            token_context.token.get_address(),
            start,
            end,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
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
            if let Some(simulation_details) = simulation.simulation_details {
                if let Some(simulation_return_data) = simulation_details.return_data {
                    assert_eq!(simulation_return_data.program_id, program_id);
                    return_data[..simulation_return_data.data.len()]
                        .copy_from_slice(&simulation_return_data.data);
                }
            }

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
