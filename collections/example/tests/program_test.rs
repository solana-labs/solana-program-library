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
    spl_token_collections_interface::{
        instruction::{create_collection, create_member},
        state::{Collection, Member},
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
        "spl_token_collections_example",
        *program_id,
        processor!(spl_token_collections_example::processor::process),
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

pub async fn setup_collection(
    context: &mut ProgramTestContext,
    collections_program_id: &Pubkey,
    mint: &Pubkey,
    collection_data: &Collection,
    collection_keypair: &Keypair,
    mint_authority: &Keypair,
) {
    let rent = context.banks_client.get_rent().await.unwrap();
    let space = collection_data.tlv_size_of().unwrap();
    let rent_lamports = rent.minimum_balance(space);
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &collection_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                collections_program_id,
            ),
            create_collection(
                collections_program_id,
                &collection_keypair.pubkey(),
                mint,
                &mint_authority.pubkey(),
                Option::<Pubkey>::from(collection_data.update_authority.clone()),
                collection_data.max_size,
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, collection_keypair, mint_authority],
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
pub async fn setup_member(
    context: &mut ProgramTestContext,
    collections_program_id: &Pubkey,
    member_mint: &Pubkey,
    collection_pubkey: &Pubkey,
    collection_mint: &Pubkey,
    member_keypair: &Keypair,
    member_mint_authority: &Keypair,
    collection_mint_authority: &Keypair,
) {
    let rent = context.banks_client.get_rent().await.unwrap();
    let member_space = Member::default().tlv_size_of().unwrap();
    let member_rent_lamports = rent.minimum_balance(member_space);

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &member_keypair.pubkey(),
                member_rent_lamports,
                member_space.try_into().unwrap(),
                collections_program_id,
            ),
            create_member(
                collections_program_id,
                &member_keypair.pubkey(),
                member_mint,
                &member_mint_authority.pubkey(),
                collection_pubkey,
                collection_mint,
                &collection_mint_authority.pubkey(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            member_keypair,
            member_mint_authority,
            collection_mint_authority,
        ],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}
