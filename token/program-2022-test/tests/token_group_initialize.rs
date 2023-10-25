#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::TestContext,
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{
        instruction::InstructionError, pubkey::Pubkey, signature::Signer, signer::keypair::Keypair,
        transaction::TransactionError, transport::TransportError,
    },
    spl_pod::bytemuck::pod_from_bytes,
    spl_token_2022::{error::TokenError, extension::BaseStateWithExtensions, processor::Processor},
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    spl_token_group_interface::{error::TokenGroupError, state::TokenGroup},
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
async fn success_initialize() {
    let authority = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let mut test_context = setup(mint_keypair, &authority).await;
    let payer_pubkey = test_context.context.lock().await.payer.pubkey();
    let token_context = test_context.token_context.take().unwrap();

    let update_authority = Pubkey::new_unique();
    let max_size = 10;
    let token_group = TokenGroup::new(
        token_context.token.get_address(),
        Some(update_authority).try_into().unwrap(),
        max_size,
    );

    // fails without more lamports for new rent-exemption
    let error = token_context
        .token
        .token_group_initialize(
            &token_context.mint_authority.pubkey(),
            &update_authority,
            max_size,
            &[&token_context.mint_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InsufficientFundsForRent { account_index: 2 }
        )))
    );

    // fail wrong signer
    let not_mint_authority = Keypair::new();
    let error = token_context
        .token
        .token_group_initialize_with_rent_transfer(
            &payer_pubkey,
            &not_mint_authority.pubkey(),
            &update_authority,
            max_size,
            &[&not_mint_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(TokenGroupError::IncorrectMintAuthority as u32)
            )
        )))
    );

    token_context
        .token
        .token_group_initialize_with_rent_transfer(
            &payer_pubkey,
            &token_context.mint_authority.pubkey(),
            &update_authority,
            max_size,
            &[&token_context.mint_authority],
        )
        .await
        .unwrap();

    // check that the data is correct
    let mint_info = token_context.token.get_mint_info().await.unwrap();
    let group_bytes = mint_info.get_extension_bytes::<TokenGroup>().unwrap();
    let fetched_group = pod_from_bytes::<TokenGroup>(group_bytes).unwrap();
    assert_eq!(fetched_group, &token_group);

    // fail double-init
    let error = token_context
        .token
        .token_group_initialize_with_rent_transfer(
            &payer_pubkey,
            &token_context.mint_authority.pubkey(),
            &update_authority,
            12, // Change so we get a different transaction
            &[&token_context.mint_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0, // No additional rent
                InstructionError::Custom(TokenError::ExtensionAlreadyInitialized as u32)
            )
        )))
    );
}

#[tokio::test]
async fn fail_without_group_pointer() {
    let mut test_context = {
        let mint_keypair = Keypair::new();
        let program_test = setup_program_test();
        let context = program_test.start_with_context().await;
        let context = Arc::new(tokio::sync::Mutex::new(context));
        let mut context = TestContext {
            context,
            token_context: None,
        };
        context
            .init_token_with_mint_keypair_and_freeze_authority(mint_keypair, vec![], None)
            .await
            .unwrap();
        context
    };

    let payer_pubkey = test_context.context.lock().await.payer.pubkey();
    let token_context = test_context.token_context.take().unwrap();

    let error = token_context
        .token
        .token_group_initialize_with_rent_transfer(
            &payer_pubkey,
            &token_context.mint_authority.pubkey(),
            &Pubkey::new_unique(),
            5,
            &[&token_context.mint_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(TokenError::InvalidExtensionCombination as u32)
            )
        )))
    );
}

#[tokio::test]
async fn fail_init_in_another_mint() {
    let authority = Pubkey::new_unique();
    let first_mint_keypair = Keypair::new();
    let first_mint = first_mint_keypair.pubkey();
    let mut test_context = setup(first_mint_keypair, &authority).await;
    let second_mint_keypair = Keypair::new();
    let second_mint = second_mint_keypair.pubkey();
    test_context
        .init_token_with_mint_keypair_and_freeze_authority(
            second_mint_keypair,
            vec![ExtensionInitializationParams::GroupPointer {
                authority: Some(authority),
                group_address: Some(second_mint),
            }],
            None,
        )
        .await
        .unwrap();

    let token_context = test_context.token_context.take().unwrap();

    let error = token_context
        .token
        .process_ixs(
            &[spl_token_group_interface::instruction::initialize_group(
                &spl_token_2022::id(),
                &first_mint,
                token_context.token.get_address(),
                &token_context.mint_authority.pubkey(),
                Some(Pubkey::new_unique()),
                5,
            )],
            &[&token_context.mint_authority],
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintMismatch as u32)
            )
        )))
    );
}

#[tokio::test]
async fn fail_without_signature() {
    let authority = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let mut test_context = setup(mint_keypair, &authority).await;

    let token_context = test_context.token_context.take().unwrap();

    let mut instruction = spl_token_group_interface::instruction::initialize_group(
        &spl_token_2022::id(),
        token_context.token.get_address(),
        token_context.token.get_address(),
        &token_context.mint_authority.pubkey(),
        Some(Pubkey::new_unique()),
        5,
    );
    instruction.accounts[2].is_signer = false;
    let error = token_context
        .token
        .process_ixs(&[instruction], &[] as &[&dyn Signer; 0]) // yuck, but the compiler needs it
        .await
        .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::MissingRequiredSignature)
        )))
    );
}
