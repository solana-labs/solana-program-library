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
    spl_token_group_interface::{error::TokenGroupError, state::TokenGroupMember},
    std::sync::Arc,
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

type SetupConfig = (Keypair, Pubkey); // Mint, Authority

async fn setup(group: SetupConfig, members: Vec<SetupConfig>) -> (TestContext, Vec<TestContext>) {
    let program_test = setup_program_test();

    let context = program_test.start_with_context().await;
    let context = Arc::new(tokio::sync::Mutex::new(context));
    let mut group_context = TestContext {
        context: context.clone(),
        token_context: None,
    };

    let (group_mint, group_authority) = group;
    let group_address = Some(group_mint.pubkey());
    group_context
        .init_token_with_mint_keypair_and_freeze_authority(
            group_mint,
            vec![ExtensionInitializationParams::GroupPointer {
                authority: Some(group_authority),
                group_address,
            }],
            None,
        )
        .await
        .unwrap();

    let mut member_contexts = vec![];
    for member in members.into_iter() {
        let (member_mint, member_authority) = member;
        let member_address = Some(member_mint.pubkey());
        let mut member_context = TestContext {
            context: context.clone(),
            token_context: None,
        };
        member_context
            .init_token_with_mint_keypair_and_freeze_authority(
                member_mint,
                vec![ExtensionInitializationParams::GroupMemberPointer {
                    authority: Some(member_authority),
                    member_address,
                }],
                None,
            )
            .await
            .unwrap();
        member_contexts.push(member_context);
    }

    let payer_pubkey = group_context.context.lock().await.payer.pubkey();
    let group_token_context = group_context.token_context.as_ref().unwrap();
    group_token_context
        .token
        .token_group_initialize_with_rent_transfer(
            &payer_pubkey,
            &group_token_context.mint_authority.pubkey(),
            &group_authority,
            2,
            &[&group_token_context.mint_authority],
        )
        .await
        .unwrap();

    (group_context, member_contexts)
}

#[tokio::test]
async fn success_initialize() {
    let group_authority = Keypair::new();
    let group_mint_keypair = Keypair::new();
    let member1_authority = Keypair::new();
    let member1_mint_keypair = Keypair::new();
    let member2_authority = Keypair::new();
    let member2_mint_keypair = Keypair::new();
    let member3_authority = Keypair::new();
    let member3_mint_keypair = Keypair::new();

    let (_, mut member_contexts) = setup(
        (
            group_mint_keypair.insecure_clone(),
            group_authority.pubkey(),
        ),
        vec![
            (
                member1_mint_keypair.insecure_clone(),
                member1_authority.pubkey(),
            ),
            (
                member2_mint_keypair.insecure_clone(),
                member2_authority.pubkey(),
            ),
            (
                member3_mint_keypair.insecure_clone(),
                member3_authority.pubkey(),
            ),
        ],
    )
    .await;

    let member1_token_context = member_contexts[0].token_context.take().unwrap();

    // fails without more lamports for new rent-exemption
    let error = member1_token_context
        .token
        .token_group_initialize_member(
            &member1_token_context.mint_authority.pubkey(),
            &group_mint_keypair.pubkey(),
            &group_authority.pubkey(),
            &[&member1_token_context.mint_authority, &group_authority],
        )
        .await
        .unwrap_err();
    let member_index = if group_mint_keypair
        .pubkey()
        .cmp(&member1_mint_keypair.pubkey())
        .is_le()
    {
        4
    } else {
        3
    };
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InsufficientFundsForRent {
                account_index: member_index
            }
        )))
    );

    // fail wrong mint authority signer
    let payer_pubkey = member_contexts[0].context.lock().await.payer.pubkey();
    let not_mint_authority = Keypair::new();
    let error = member1_token_context
        .token
        .token_group_initialize_member_with_rent_transfer(
            &payer_pubkey,
            &not_mint_authority.pubkey(),
            &group_mint_keypair.pubkey(),
            &group_authority.pubkey(),
            &[&not_mint_authority, &group_authority],
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

    // fail wrong group update authority signer
    let not_group_update_authority = Keypair::new();
    let error = member1_token_context
        .token
        .token_group_initialize_member_with_rent_transfer(
            &payer_pubkey,
            &member1_token_context.mint_authority.pubkey(),
            &group_mint_keypair.pubkey(),
            &not_group_update_authority.pubkey(),
            &[
                &member1_token_context.mint_authority,
                &not_group_update_authority,
            ],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(TokenGroupError::IncorrectUpdateAuthority as u32)
            )
        )))
    );

    // fail group and member same mint
    let error = member1_token_context
        .token
        .token_group_initialize_member_with_rent_transfer(
            &payer_pubkey,
            &member1_token_context.mint_authority.pubkey(),
            member1_token_context.token.get_address(),
            &group_authority.pubkey(),
            &[&member1_token_context.mint_authority, &group_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(TokenGroupError::MemberAccountIsGroupAccount as u32)
            )
        )))
    );

    member1_token_context
        .token
        .token_group_initialize_member_with_rent_transfer(
            &payer_pubkey,
            &member1_token_context.mint_authority.pubkey(),
            &group_mint_keypair.pubkey(),
            &group_authority.pubkey(),
            &[&member1_token_context.mint_authority, &group_authority],
        )
        .await
        .unwrap();

    // check that the data is correct
    let mint_info = member1_token_context.token.get_mint_info().await.unwrap();
    let member_bytes = mint_info.get_extension_bytes::<TokenGroupMember>().unwrap();
    let fetched_member = pod_from_bytes::<TokenGroupMember>(member_bytes).unwrap();
    assert_eq!(
        fetched_member,
        &TokenGroupMember {
            mint: member1_mint_keypair.pubkey(),
            group: group_mint_keypair.pubkey(),
            member_number: 1.into(),
        }
    );

    // fail double-init
    {
        let mut context = member_contexts[0].context.lock().await;
        context.get_new_latest_blockhash().await.unwrap();
        context.get_new_latest_blockhash().await.unwrap();
    }
    let error = member1_token_context
        .token
        .token_group_initialize_member(
            &member1_token_context.mint_authority.pubkey(),
            &group_mint_keypair.pubkey(),
            &group_authority.pubkey(),
            &[&member1_token_context.mint_authority, &group_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::ExtensionAlreadyInitialized as u32)
            )
        )))
    );

    // Now the second
    let member2_token_context = member_contexts[1].token_context.take().unwrap();
    member2_token_context
        .token
        .token_group_initialize_member_with_rent_transfer(
            &payer_pubkey,
            &member2_token_context.mint_authority.pubkey(),
            &group_mint_keypair.pubkey(),
            &group_authority.pubkey(),
            &[&member2_token_context.mint_authority, &group_authority],
        )
        .await
        .unwrap();
    let mint_info = member2_token_context.token.get_mint_info().await.unwrap();
    let member_bytes = mint_info.get_extension_bytes::<TokenGroupMember>().unwrap();
    let fetched_member = pod_from_bytes::<TokenGroupMember>(member_bytes).unwrap();
    assert_eq!(
        fetched_member,
        &TokenGroupMember {
            mint: member2_mint_keypair.pubkey(),
            group: group_mint_keypair.pubkey(),
            member_number: 2.into(),
        }
    );

    // Third should fail on max size
    let member3_token_context = member_contexts[2].token_context.take().unwrap();
    let error = member3_token_context
        .token
        .token_group_initialize_member_with_rent_transfer(
            &payer_pubkey,
            &member3_token_context.mint_authority.pubkey(),
            &group_mint_keypair.pubkey(),
            &group_authority.pubkey(),
            &[&member3_token_context.mint_authority, &group_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(TokenGroupError::SizeExceedsMaxSize as u32)
            )
        )))
    );
}

#[tokio::test]
async fn fail_without_member_pointer() {
    let group_authority = Keypair::new();
    let group_mint_keypair = Keypair::new();
    let member_mint_keypair = Keypair::new();

    let (group_context, _) = setup(
        (
            group_mint_keypair.insecure_clone(),
            group_authority.pubkey(),
        ),
        vec![],
    )
    .await;

    let mut member_test_context = TestContext {
        context: group_context.context.clone(),
        token_context: None,
    };
    member_test_context
        .init_token_with_mint_keypair_and_freeze_authority(member_mint_keypair, vec![], None)
        .await
        .unwrap();

    let payer_pubkey = member_test_context.context.lock().await.payer.pubkey();
    let member_token_context = member_test_context.token_context.take().unwrap();

    let error = member_token_context
        .token
        .token_group_initialize_member_with_rent_transfer(
            &payer_pubkey,
            &member_token_context.mint_authority.pubkey(),
            &group_mint_keypair.pubkey(),
            &group_authority.pubkey(),
            &[&member_token_context.mint_authority, &group_authority],
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
    let group_authority = Keypair::new();
    let group_mint_keypair = Keypair::new();
    let member_authority = Keypair::new();
    let first_member_mint_keypair = Keypair::new();
    let second_member_mint_keypair = Keypair::new();

    let (_, mut member_contexts) = setup(
        (
            group_mint_keypair.insecure_clone(),
            group_authority.pubkey(),
        ),
        vec![(
            second_member_mint_keypair.insecure_clone(),
            member_authority.pubkey(),
        )],
    )
    .await;

    let member_token_context = member_contexts[0].token_context.take().unwrap();
    let error = member_token_context
        .token
        .process_ixs(
            &[spl_token_group_interface::instruction::initialize_member(
                &spl_token_2022::id(),
                &first_member_mint_keypair.pubkey(),
                member_token_context.token.get_address(),
                &member_token_context.mint_authority.pubkey(),
                &group_mint_keypair.pubkey(),
                &group_authority.pubkey(),
            )],
            &[&member_token_context.mint_authority, &group_authority],
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
async fn fail_without_signatures() {
    let group_authority = Keypair::new();
    let group_mint_keypair = Keypair::new();
    let member_authority = Keypair::new();
    let member_mint_keypair = Keypair::new();

    let (_, mut member_contexts) = setup(
        (
            group_mint_keypair.insecure_clone(),
            group_authority.pubkey(),
        ),
        vec![(
            member_mint_keypair.insecure_clone(),
            member_authority.pubkey(),
        )],
    )
    .await;

    let member_token_context = member_contexts[0].token_context.take().unwrap();

    // Missing mint authority
    let mut instruction = spl_token_group_interface::instruction::initialize_member(
        &spl_token_2022::id(),
        &member_mint_keypair.pubkey(),
        member_token_context.token.get_address(),
        &member_token_context.mint_authority.pubkey(),
        &group_mint_keypair.pubkey(),
        &group_authority.pubkey(),
    );
    instruction.accounts[2].is_signer = false;
    let error = member_token_context
        .token
        .process_ixs(&[instruction], &[&group_authority])
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::MissingRequiredSignature)
        )))
    );

    // Missing group update authority
    let mut instruction = spl_token_group_interface::instruction::initialize_member(
        &spl_token_2022::id(),
        &member_mint_keypair.pubkey(),
        member_token_context.token.get_address(),
        &member_token_context.mint_authority.pubkey(),
        &group_mint_keypair.pubkey(),
        &group_authority.pubkey(),
    );
    instruction.accounts[4].is_signer = false;
    let error = member_token_context
        .token
        .process_ixs(&[instruction], &[&member_token_context.mint_authority])
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::MissingRequiredSignature)
        )))
    );
}
