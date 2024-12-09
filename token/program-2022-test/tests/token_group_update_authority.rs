#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::TestContext,
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{
        instruction::InstructionError,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
    spl_pod::optional_keys::OptionalNonZeroPubkey,
    spl_token_2022::{extension::BaseStateWithExtensions, processor::Processor},
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    spl_token_group_interface::{
        error::TokenGroupError, instruction::update_group_authority, state::TokenGroup,
    },
    std::{convert::TryInto, sync::Arc},
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
    let group_address = Some(mint.pubkey());
    context
        .init_token_with_mint_keypair_and_freeze_authority(
            mint,
            vec![ExtensionInitializationParams::GroupPointer {
                authority: Some(*authority),
                group_address,
            }],
            None,
        )
        .await
        .unwrap();
    context
}

#[tokio::test]
async fn success_update() {
    let authority = Keypair::new();
    let mint_keypair = Keypair::new();
    let mut test_context = setup(mint_keypair.insecure_clone(), &authority.pubkey()).await;
    let payer_pubkey = test_context.context.lock().await.payer.pubkey();
    let token_context = test_context.token_context.take().unwrap();

    let update_authority = Keypair::new();
    let mut token_group = TokenGroup::new(
        &mint_keypair.pubkey(),
        Some(update_authority.pubkey()).try_into().unwrap(),
        10,
    );

    token_context
        .token
        .token_group_initialize_with_rent_transfer(
            &payer_pubkey,
            &token_context.mint_authority.pubkey(),
            &update_authority.pubkey(),
            10,
            &[&token_context.mint_authority],
        )
        .await
        .unwrap();

    let new_update_authority = Keypair::new();
    let new_update_authority_pubkey =
        OptionalNonZeroPubkey::try_from(Some(new_update_authority.pubkey())).unwrap();
    token_group.update_authority = new_update_authority_pubkey;

    token_context
        .token
        .token_group_update_authority(
            &update_authority.pubkey(),
            Some(new_update_authority.pubkey()),
            &[&update_authority],
        )
        .await
        .unwrap();

    // check that the data is correct
    let mint = token_context.token.get_mint_info().await.unwrap();
    let fetched_group = mint.get_extension::<TokenGroup>().unwrap();
    assert_eq!(fetched_group, &token_group);

    // unset
    token_group.update_authority = None.try_into().unwrap();
    token_context
        .token
        .token_group_update_authority(
            &new_update_authority.pubkey(),
            None,
            &[&new_update_authority],
        )
        .await
        .unwrap();

    let mint = token_context.token.get_mint_info().await.unwrap();
    let fetched_group = mint.get_extension::<TokenGroup>().unwrap();
    assert_eq!(fetched_group, &token_group);

    // fail to update
    let error = token_context
        .token
        .token_group_update_authority(
            &new_update_authority.pubkey(),
            Some(new_update_authority.pubkey()),
            &[&new_update_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenGroupError::ImmutableGroup as u32)
            )
        )))
    );
}

#[tokio::test]
async fn fail_authority_checks() {
    let program_id = spl_token_2022::id();
    let authority = Keypair::new();
    let mint_keypair = Keypair::new();
    let mint_pubkey = mint_keypair.pubkey();
    let mut test_context = setup(mint_keypair, &authority.pubkey()).await;
    let payer_pubkey = test_context.context.lock().await.payer.pubkey();
    let token_context = test_context.token_context.take().unwrap();

    let update_authority = Keypair::new();
    token_context
        .token
        .token_group_initialize_with_rent_transfer(
            &payer_pubkey,
            &token_context.mint_authority.pubkey(),
            &update_authority.pubkey(),
            10,
            &[&token_context.mint_authority],
        )
        .await
        .unwrap();

    // wrong authority
    let error = token_context
        .token
        .token_group_update_authority(&payer_pubkey, None, &[] as &[&dyn Signer; 0])
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenGroupError::IncorrectUpdateAuthority as u32),
            )
        )))
    );

    // no signature
    let context = test_context.context.lock().await;
    let mut instruction =
        update_group_authority(&program_id, &mint_pubkey, &authority.pubkey(), None);
    instruction.accounts[1].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(0, InstructionError::MissingRequiredSignature,)
    );
}
