#![cfg(feature = "test-sbf")]
#![allow(clippy::integer_arithmetic)]

use {
    solana_program_test::{processor, tokio::sync::Mutex, ProgramTest, ProgramTestContext},
    solana_sdk::{
        borsh::get_instance_packed_len, instruction::AccountMeta, pubkey::Pubkey,
        signature::Signer, signer::keypair::Keypair, system_instruction, transaction::Transaction,
    },
    spl_token_client::{
        client::{
            ProgramBanksClient, ProgramBanksClientProcessTransaction, ProgramClient,
            SendTransaction, SimulateTransaction,
        },
        token::{ExtensionInitializationParams, Token},
    },
    spl_token_group_interface::{
        instruction::{initialize_group, initialize_member},
        state::{Group, Member, SplTokenGroup},
    },
    spl_token_metadata_interface::state::TokenMetadata,
    spl_type_length_value::state::{TlvState, TlvStateBorrowed},
    std::sync::Arc,
};

pub struct TokenGroupTestContext<G>
where
    G: SplTokenGroup,
{
    pub context: Arc<Mutex<ProgramTestContext>>,
    pub client: Arc<dyn ProgramClient<ProgramBanksClientProcessTransaction>>,
    pub payer: Arc<Keypair>,
    pub token_program_id: Pubkey,
    pub program_id: Pubkey,
    pub mint_keypair: Keypair,
    pub mint_authority_keypair: Keypair,
    pub metadata_keypair: Keypair,
    pub metadata_update_authority_keypair: Keypair,
    pub group_keypair: Keypair,
    pub group_update_authority_keypair: Keypair,
    pub group: Group<G>,
    pub group_token_metadata: TokenMetadata,
}

/// Set up a program test
pub async fn setup(
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

/// Set up a Token-2022 mint and metadata
pub async fn setup_mint_with_metadata_pointer<T: SendTransaction + SimulateTransaction>(
    token_client: &Token<T>,
    mint_keypair: &Keypair,
    mint_authority_keypair: &Keypair,
    metadata_pubkey: &Pubkey,
    metadata_update_authority_pubkey: &Pubkey,
) {
    token_client
        .create_mint(
            &mint_authority_keypair.pubkey(),
            None,
            vec![ExtensionInitializationParams::MetadataPointer {
                authority: Some(*metadata_update_authority_pubkey),
                metadata_address: Some(*metadata_pubkey),
            }],
            &[mint_keypair],
        )
        .await
        .unwrap();
}

/// Set up a Token-2022 mint and metadata
pub async fn setup_mint_and_metadata<T: SendTransaction + SimulateTransaction>(
    token_client: &Token<T>,
    mint_keypair: &Keypair,
    mint_authority_keypair: &Keypair,
    metadata_pubkey: &Pubkey,
    metadata_update_authority_pubkey: &Pubkey,
    token_metadata: &TokenMetadata,
    payer: Arc<Keypair>,
) {
    setup_mint_with_metadata_pointer(
        token_client,
        mint_keypair,
        mint_authority_keypair,
        metadata_pubkey,
        metadata_update_authority_pubkey,
    )
    .await;
    token_client
        .token_metadata_initialize_with_rent_transfer(
            &payer.pubkey(),
            metadata_update_authority_pubkey,
            &mint_authority_keypair.pubkey(),
            token_metadata.name.clone(),
            token_metadata.symbol.clone(),
            token_metadata.uri.clone(),
            &[&payer, mint_authority_keypair],
        )
        .await
        .unwrap();
}

pub async fn setup_group<G: SplTokenGroup>(
    context: &mut ProgramTestContext,
    program_id: &Pubkey,
    group_keypair: &Keypair,
    mint: &Pubkey,
    mint_authority_keypair: &Keypair,
    group: &Group<G>,
    extra_account_metas: &[AccountMeta],
) {
    let rent = context.banks_client.get_rent().await.unwrap();
    let space = TlvStateBorrowed::get_base_len() + get_instance_packed_len(group).unwrap();
    let rent_lamports = rent.minimum_balance(space);
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &group_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                program_id,
            ),
            initialize_group::<G>(
                program_id,
                &group_keypair.pubkey(),
                mint,
                &mint_authority_keypair.pubkey(),
                Option::<Pubkey>::from(group.update_authority),
                group.max_size,
                &group.meta,
                extra_account_metas,
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, group_keypair, mint_authority_keypair],
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
pub async fn setup_member<G: SplTokenGroup>(
    context: &mut ProgramTestContext,
    group_program_id: &Pubkey,
    group_pubkey: &Pubkey,
    group_mint_pubkey: &Pubkey,
    group_mint_authority_keypair: &Keypair,
    member_keypair: &Keypair,
    member_mint_pubkey: &Pubkey,
    member_mint_authority_keypair: &Keypair,
    member_data: &Member,
    extra_account_metas: &[AccountMeta],
) {
    let rent = context.banks_client.get_rent().await.unwrap();
    let space = TlvStateBorrowed::get_base_len() + get_instance_packed_len(member_data).unwrap();
    let rent_lamports = rent.minimum_balance(space);

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &member_keypair.pubkey(),
                rent_lamports,
                space.try_into().unwrap(),
                group_program_id,
            ),
            initialize_member::<G>(
                group_program_id,
                group_pubkey,
                group_mint_pubkey,
                &group_mint_authority_keypair.pubkey(),
                &member_keypair.pubkey(),
                member_mint_pubkey,
                &member_mint_authority_keypair.pubkey(),
                member_data.member_number,
                extra_account_metas,
            ),
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            member_keypair,
            member_mint_authority_keypair,
            group_mint_authority_keypair,
        ],
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
pub async fn setup_member_with_metadata_rent<G: SplTokenGroup>(
    context: &mut ProgramTestContext,
    group_program_id: &Pubkey,
    group_pubkey: &Pubkey,
    group_mint_pubkey: &Pubkey,
    group_mint_authority_keypair: &Keypair,
    group_token_metadata: &TokenMetadata,
    member_keypair: &Keypair,
    member_mint_pubkey: &Pubkey,
    member_mint_authority_keypair: &Keypair,
    member_data: &Member,
    extra_account_metas: &[AccountMeta],
) {
    let rent = context.banks_client.get_rent().await.unwrap();
    let member_space =
        TlvStateBorrowed::get_base_len() + get_instance_packed_len(member_data).unwrap();
    let member_rent_lamports = rent.minimum_balance(member_space);

    let metadata_space = group_token_metadata.tlv_size_of().unwrap();
    let metadata_rent_lamports = rent.minimum_balance(metadata_space);

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &member_keypair.pubkey(),
                member_rent_lamports,
                member_space.try_into().unwrap(),
                group_program_id,
            ),
            // Fund the mint for the metadata
            system_instruction::transfer(
                &context.payer.pubkey(),
                member_mint_pubkey,
                metadata_rent_lamports,
            ),
            initialize_member::<G>(
                group_program_id,
                group_pubkey,
                group_mint_pubkey,
                &group_mint_authority_keypair.pubkey(),
                &member_keypair.pubkey(),
                member_mint_pubkey,
                &member_mint_authority_keypair.pubkey(),
                member_data.member_number,
                extra_account_metas,
            ),
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            member_keypair,
            member_mint_authority_keypair,
            group_mint_authority_keypair,
        ],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

// pub async fn setup_extra_account_metas<I:
// SplDiscriminate>(extra_account_metas: &[ExtraAccountMeta]) { todo!()
// TODO: We want to use the instruction for creating extra metas, but in
// our program we actually need to create this account once (if it doesn't)
// exist, and write the validation data in. The key here is that the
// validation data is going to be for more than one instruction.
// }

/// Setup a test for creating a token `Collection`:
/// - Mint:         An NFT representing the `Collection` mint
/// - Metadata:     A `TokenMetadata` representing the `Collection` metadata
/// - Collection:   A `Collection` representing the `Collection` group
pub async fn setup_program_test<G>(group_name: &str, meta: Option<G>) -> TokenGroupTestContext<G>
where
    G: SplTokenGroup,
{
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup(&program_id).await;

    // We'll use Token-2022 for the mint and the metadata
    let token_program_id = spl_token_2022::id();
    let mint_authority_keypair = Keypair::new();
    let mint_keypair = Keypair::new();

    // In this test:
    // - The metadata is stored in the mint (Token-2022)
    // - The group is in a separate account
    // - The _metadata_ update authority is the mint authority
    // - The _group_ update authority is also the mint authority
    // - The mint is an NFT (0 decimals)
    let metadata_keypair = mint_keypair.insecure_clone();
    let group_keypair = Keypair::new();
    let metadata_update_authority_keypair = mint_authority_keypair.insecure_clone();
    let group_update_authority_keypair = mint_authority_keypair.insecure_clone();
    let decimals = 0;

    let token_client = Token::new(
        client.clone(),
        &token_program_id,
        &mint_keypair.pubkey(),
        Some(decimals),
        payer.clone(),
    );

    let group_token_metadata = TokenMetadata {
        name: group_name.to_string(),
        symbol: "GRP".to_string(),
        uri: "cool.token.group.com".to_string(),
        update_authority: Some(metadata_update_authority_keypair.pubkey())
            .try_into()
            .unwrap(),
        mint: mint_keypair.pubkey(),
        ..Default::default()
    };

    setup_mint_and_metadata(
        &token_client,
        &mint_keypair,
        &mint_authority_keypair,
        &metadata_keypair.pubkey(),
        &metadata_update_authority_keypair.pubkey(),
        &group_token_metadata,
        payer.clone(),
    )
    .await;

    let group = Group {
        update_authority: Some(group_update_authority_keypair.pubkey())
            .try_into()
            .unwrap(),
        max_size: Some(100),
        size: 0,
        meta,
    };

    TokenGroupTestContext {
        context,
        client,
        payer,
        token_program_id,
        program_id,
        mint_keypair,
        mint_authority_keypair,
        metadata_keypair,
        metadata_update_authority_keypair,
        group_keypair,
        group_update_authority_keypair,
        group,
        group_token_metadata,
    }
}
