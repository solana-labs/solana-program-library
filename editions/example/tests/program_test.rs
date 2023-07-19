#![cfg(feature = "test-sbf")]

use {
    solana_program_test::{processor, tokio::sync::Mutex, ProgramTest, ProgramTestContext},
    solana_sdk::{
        pubkey::Pubkey, signature::Signer, signer::keypair::Keypair, system_instruction,
        transaction::Transaction,
    },
    spl_token_client::{
        client::{
            ProgramBanksClient, ProgramBanksClientProcessTransaction, ProgramClient,
            SendTransaction,
        },
        token::{ExtensionInitializationParams, Token},
    },
    spl_token_editions_interface::{
        instruction::{create_original, create_reprint},
        state::{Original, Reprint},
    },
    spl_token_metadata_interface::state::TokenMetadata,
    std::sync::Arc,
};

fn keypair_clone(kp: &Keypair) -> Keypair {
    Keypair::from_bytes(&kp.to_bytes()).expect("failed to copy keypair")
}

pub async fn setup(
    program_id: &Pubkey,
) -> (
    Arc<Mutex<ProgramTestContext>>,
    Arc<dyn ProgramClient<ProgramBanksClientProcessTransaction>>,
    Arc<Keypair>,
) {
    let mut program_test = ProgramTest::new(
        "spl_token_editions_example",
        *program_id,
        processor!(spl_token_editions_example::processor::process),
    );

    program_test.prefer_bpf(false); // simplicity in the build
    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(spl_token_2022::processor::Processor::process),
    );

    let context = program_test.start_with_context().await;
    let payer = Arc::new(keypair_clone(&context.payer));
    let context = Arc::new(Mutex::new(context));

    let client: Arc<dyn ProgramClient<ProgramBanksClientProcessTransaction>> =
        Arc::new(ProgramBanksClient::new_from_context(
            Arc::clone(&context),
            ProgramBanksClientProcessTransaction,
        ));
    (context, client, payer)
}

pub async fn setup_mint<T: SendTransaction>(
    program_id: &Pubkey,
    mint_authority: &Pubkey,
    metadata: &Pubkey,
    update_authority: &Pubkey,
    decimals: u8,
    payer: Arc<Keypair>,
    client: Arc<dyn ProgramClient<T>>,
) -> Token<T> {
    let mint_account = Keypair::new();
    let token = Token::new(
        client,
        program_id,
        &mint_account.pubkey(),
        Some(decimals),
        payer,
    );
    token
        .create_mint(
            mint_authority,
            None,
            vec![ExtensionInitializationParams::MetadataPointer {
                authority: Some(*update_authority),
                metadata_address: Some(*metadata),
            }],
            &[&mint_account],
        )
        .await
        .unwrap();
    token
}

pub async fn setup_metadata<T: SendTransaction>(
    token: &Token<T>,
    update_authority: &Pubkey,
    token_metadata: &TokenMetadata,
    _metadata_keypair: &Keypair,
    mint_authority: &Keypair,
    payer: Arc<Keypair>,
) {
    token
        .token_metadata_initialize_with_rent_transfer(
            &payer.pubkey(),
            update_authority,
            &mint_authority.pubkey(),
            token_metadata.name.clone(),
            token_metadata.symbol.clone(),
            token_metadata.uri.clone(),
            &[&payer, mint_authority],
        )
        .await
        .unwrap();
}

pub async fn setup_original_print(
    context: &mut ProgramTestContext,
    editions_program_id: &Pubkey,
    metadata: &Pubkey,
    mint: &Pubkey,
    original_data: &Original,
    original_keypair: &Keypair,
    mint_authority: &Keypair,
) {
    let rent = context.banks_client.get_rent().await.unwrap();
    let space = original_data.tlv_size_of().unwrap();
    let rent_lamports = rent.minimum_balance(space);
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &original_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                editions_program_id,
            ),
            create_original(
                editions_program_id,
                &original_keypair.pubkey(),
                metadata,
                mint,
                &mint_authority.pubkey(),
                Option::<Pubkey>::from(original_data.update_authority.clone()),
                original_data.max_supply,
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, original_keypair, mint_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub async fn setup_reprint(
    context: &mut ProgramTestContext,
    editions_program_id: &Pubkey,
    reprint_metadata: &Pubkey,
    reprint_mint: &Pubkey,
    original_pubkey: &Pubkey,
    original_metadata: &Pubkey,
    original_mint: &Pubkey,
    metadata_program_id: &Pubkey,
    reprint_data: &Reprint,
    token_metadata: &TokenMetadata,
    reprint_keypair: &Keypair,
    update_authority: &Keypair,
    mint_authority: &Keypair,
) {
    let rent = context.banks_client.get_rent().await.unwrap();

    let token_metadata_space = token_metadata.tlv_size_of().unwrap();
    let token_metadata_rent_lamports = rent.minimum_balance(token_metadata_space);

    let reprint_space = reprint_data.tlv_size_of().unwrap();
    let reprint_rent_lamports = rent.minimum_balance(reprint_space);

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &reprint_keypair.pubkey(),
                reprint_rent_lamports,
                reprint_space.try_into().unwrap(),
                editions_program_id,
            ),
            // Fund the mint with extra rent for metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                reprint_mint,
                token_metadata_rent_lamports,
            ),
            create_reprint(
                editions_program_id,
                &reprint_keypair.pubkey(),
                reprint_metadata,
                reprint_mint,
                original_pubkey,
                &update_authority.pubkey(),
                original_metadata,
                original_mint,
                &mint_authority.pubkey(),
                metadata_program_id,
            ),
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            reprint_keypair,
            update_authority,
            mint_authority,
        ],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}
