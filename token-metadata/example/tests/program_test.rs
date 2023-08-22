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
            SendTransaction, SimulateTransaction,
        },
        token::Token,
    },
    spl_token_metadata_interface::{
        instruction::{initialize, update_field},
        state::{Field, TokenMetadata},
    },
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
        "spl_token_metadata_example",
        *program_id,
        processor!(spl_token_metadata_example::processor::process),
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

pub async fn setup_mint<T: SendTransaction + SimulateTransaction>(
    program_id: &Pubkey,
    mint_authority: &Pubkey,
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
        .create_mint(mint_authority, None, vec![], &[&mint_account])
        .await
        .unwrap();
    token
}

pub async fn setup_metadata(
    context: &mut ProgramTestContext,
    metadata_program_id: &Pubkey,
    mint: &Pubkey,
    token_metadata: &TokenMetadata,
    metadata_keypair: &Keypair,
    mint_authority: &Keypair,
) {
    let rent = context.banks_client.get_rent().await.unwrap();
    let space = token_metadata.tlv_size_of().unwrap();
    let rent_lamports = rent.minimum_balance(space);
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &metadata_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                metadata_program_id,
            ),
            initialize(
                metadata_program_id,
                &metadata_keypair.pubkey(),
                &Option::<Pubkey>::from(token_metadata.update_authority).unwrap(),
                mint,
                &mint_authority.pubkey(),
                token_metadata.name.clone(),
                token_metadata.symbol.clone(),
                token_metadata.uri.clone(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, metadata_keypair, mint_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

#[allow(dead_code)]
pub async fn setup_update_field(
    context: &mut ProgramTestContext,
    metadata_program_id: &Pubkey,
    token_metadata: &mut TokenMetadata,
    metadata: &Pubkey,
    update_authority: &Keypair,
    field: Field,
    value: String,
) {
    let rent = context.banks_client.get_rent().await.unwrap();
    let old_space = token_metadata.tlv_size_of().unwrap();
    let old_rent_lamports = rent.minimum_balance(old_space);

    token_metadata.update(field.clone(), value.clone());

    let new_space = token_metadata.tlv_size_of().unwrap();
    let new_rent_lamports = rent.minimum_balance(new_space);
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &context.payer.pubkey(),
                metadata,
                new_rent_lamports.saturating_sub(old_rent_lamports),
            ),
            update_field(
                metadata_program_id,
                metadata,
                &update_authority.pubkey(),
                field,
                value,
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, update_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}
