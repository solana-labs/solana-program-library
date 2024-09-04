#![cfg(feature = "test-sbf")]
#![allow(clippy::items_after_test_module)]

mod program_test;
use {
    program_test::TestContext,
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{
        account::Account as SolanaAccount, instruction::InstructionError, pubkey::Pubkey,
        signature::Signer, signer::keypair::Keypair, transaction::TransactionError,
        transport::TransportError,
    },
    spl_token_2022::{extension::BaseStateWithExtensions, processor::Processor},
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    spl_token_group_interface::{
        error::TokenGroupError, instruction::update_group_max_size, state::TokenGroup,
    },
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

// Successful attempts to set higher than size
#[test_case(0, 0, 10)]
#[test_case(5, 0, 10)]
#[test_case(50, 0, 200_000)]
#[test_case(100_000, 100_000, 200_000)]
#[test_case(50, 0, 300_000_000)]
#[test_case(100_000, 100_000, 300_000_000)]
#[test_case(100_000_000, 100_000_000, 300_000_000)]
#[test_case(0, 0, u64::MAX)]
#[test_case(200_000, 200_000, u64::MAX)]
#[test_case(300_000_000, 300_000_000, u64::MAX)]
// Attempts to set lower than size
#[test_case(5, 5, 4)]
#[test_case(200_000, 200_000, 50)]
#[test_case(200_000, 200_000, 100_000)]
#[test_case(300_000_000, 300_000_000, 50)]
#[test_case(u64::MAX, u64::MAX, 0)]
#[tokio::test]
async fn test_update_group_max_size(max_size: u64, size: u64, new_max_size: u64) {
    let authority = Keypair::new();
    let mint_keypair = Keypair::new();
    let mut test_context = setup(mint_keypair.insecure_clone(), &authority.pubkey()).await;
    let payer_pubkey = test_context.context.lock().await.payer.pubkey();
    let token_context = test_context.token_context.take().unwrap();

    let update_authority = Keypair::new();
    let mut token_group = TokenGroup::new(
        &mint_keypair.pubkey(),
        Some(update_authority.pubkey()).try_into().unwrap(),
        max_size,
    );

    token_context
        .token
        .token_group_initialize_with_rent_transfer(
            &payer_pubkey,
            &token_context.mint_authority.pubkey(),
            &update_authority.pubkey(),
            max_size,
            &[&token_context.mint_authority],
        )
        .await
        .unwrap();

    {
        // Update the group's size manually
        let mut context = test_context.context.lock().await;

        let group_mint_account = context
            .banks_client
            .get_account(mint_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();

        let old_data = context
            .banks_client
            .get_account(mint_keypair.pubkey())
            .await
            .unwrap()
            .unwrap()
            .data;

        let data = {
            // 0....81:     mint
            // 82...164:    padding
            // 165..166:    account type
            // 167..170:    extension discriminator (GroupPointer)
            // 171..202:    authority
            // 203..234:    group pointer
            // 235..238:    extension discriminator (TokenGroup)
            // 239..270:    mint
            // 271..302:    update_authority
            // 303..306:    size
            // 307..310:    max_size
            let (front, back) = old_data.split_at(302);
            let (_, back) = back.split_at(4);
            let size_bytes = size.to_le_bytes();
            let mut bytes = vec![];
            bytes.extend_from_slice(front);
            bytes.extend_from_slice(&size_bytes);
            bytes.extend_from_slice(back);
            bytes
        };

        context.set_account(
            &mint_keypair.pubkey(),
            &SolanaAccount {
                data,
                ..group_mint_account
            }
            .into(),
        );

        token_group.size = size.into();
    }

    token_group.max_size = new_max_size.into();

    if new_max_size < size {
        let error = token_context
            .token
            .token_group_update_max_size(
                &update_authority.pubkey(),
                new_max_size,
                &[&update_authority],
            )
            .await
            .unwrap_err();
        assert_eq!(
            error,
            TokenClientError::Client(Box::new(TransportError::TransactionError(
                TransactionError::InstructionError(
                    0,
                    InstructionError::Custom(TokenGroupError::SizeExceedsNewMaxSize as u32)
                )
            ))),
        );
    } else {
        token_context
            .token
            .token_group_update_max_size(
                &update_authority.pubkey(),
                new_max_size,
                &[&update_authority],
            )
            .await
            .unwrap();

        let mint_info = token_context.token.get_mint_info().await.unwrap();
        let fetched_group = mint_info.get_extension::<TokenGroup>().unwrap();
        assert_eq!(fetched_group, &token_group);
    }
}

#[tokio::test]
async fn fail_authority_checks() {
    let authority = Keypair::new();
    let mint_keypair = Keypair::new();
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

    // no signature
    let mut instruction = update_group_max_size(
        &spl_token_2022::id(),
        token_context.token.get_address(),
        &update_authority.pubkey(),
        20,
    );
    instruction.accounts[1].is_signer = false;

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

    // wrong authority
    let wrong_authority = Keypair::new();
    let error = token_context
        .token
        .token_group_update_max_size(&wrong_authority.pubkey(), 20, &[&wrong_authority])
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenGroupError::IncorrectUpdateAuthority as u32)
            )
        )))
    );
}
