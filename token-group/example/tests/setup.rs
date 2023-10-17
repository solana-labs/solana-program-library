#![cfg(feature = "test-sbf")]

use {
    solana_program_test::{processor, tokio::sync::Mutex, ProgramTest, ProgramTestContext},
    solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer},
    spl_token_client::{
        client::{
            ProgramBanksClient, ProgramBanksClientProcessTransaction, ProgramClient,
            SendTransaction, SimulateTransaction,
        },
        token::{ExtensionInitializationParams, Token},
    },
    spl_token_metadata_interface::state::TokenMetadata,
    std::sync::Arc,
};

/// Set up a program test
pub async fn setup_program_test(
    program_id: &Pubkey,
) -> (
    Arc<Mutex<ProgramTestContext>>,
    Arc<dyn ProgramClient<ProgramBanksClientProcessTransaction>>,
    Arc<Keypair>,
) {
    let mut program_test = ProgramTest::new(
        "spl_token_group_example",
        *program_id,
        processor!(spl_token_group_example::processor::process),
    );
    program_test.prefer_bpf(false);
    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(spl_token_2022::processor::Processor::process),
    );
    let context = program_test.start_with_context().await;
    let payer = Arc::new(context.payer.insecure_clone());
    let context = Arc::new(Mutex::new(context));
    let client: Arc<dyn ProgramClient<ProgramBanksClientProcessTransaction>> =
        Arc::new(ProgramBanksClient::new_from_context(
            Arc::clone(&context),
            ProgramBanksClientProcessTransaction,
        ));
    (context, client, payer)
}

/// Set up a Token-2022 mint
pub async fn setup_mint<T: SendTransaction + SimulateTransaction>(
    token_client: &Token<T>,
    mint_keypair: &Keypair,
    mint_authority_keypair: &Keypair,
    extensions: Vec<ExtensionInitializationParams>,
) {
    token_client
        .create_mint(
            &mint_authority_keypair.pubkey(),
            None,
            extensions,
            &[mint_keypair],
        )
        .await
        .unwrap();
}

/// Set up a Token-2022 mint and metadata
///
/// Note: Not every test uses this function, so we need to ignore the
/// lint warning.
#[allow(dead_code)]
pub async fn setup_mint_and_metadata<T: SendTransaction + SimulateTransaction>(
    token_client: &Token<T>,
    mint_keypair: &Keypair,
    mint_authority_keypair: &Keypair,
    token_metadata: &TokenMetadata,
    payer: Arc<Keypair>,
) {
    token_client
        .create_mint(
            &mint_authority_keypair.pubkey(),
            None,
            vec![ExtensionInitializationParams::MetadataPointer {
                authority: Some(mint_authority_keypair.pubkey()),
                metadata_address: Some(mint_keypair.pubkey()),
            }],
            &[mint_keypair],
        )
        .await
        .unwrap();
    token_client
        .token_metadata_initialize_with_rent_transfer(
            &payer.pubkey(),
            &mint_authority_keypair.pubkey(), // Also the metadata update authority
            &mint_authority_keypair.pubkey(),
            token_metadata.name.clone(),
            token_metadata.symbol.clone(),
            token_metadata.uri.clone(),
            &[&payer, mint_authority_keypair],
        )
        .await
        .unwrap();
}
