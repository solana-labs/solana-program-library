#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::TestContext,
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{
        instruction::InstructionError, pubkey::Pubkey, signature::Signer, signer::keypair::Keypair,
        transaction::TransactionError, transport::TransportError,
    },
    spl_token_2022::{
        error::TokenError,
        extension::{
            group_member_pointer::GroupMemberPointer, group_pointer::GroupPointer,
            BaseStateWithExtensions,
        },
        instruction,
        processor::Processor,
    },
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    std::{convert::TryInto, sync::Arc},
};

fn setup_program_test() -> ProgramTest {
    let mut program_test = ProgramTest::default();
    program_test.prefer_bpf(false);
    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(Processor::process),
    );
    program_test
}

async fn setup(
    mint: Keypair,
    member_address: &Pubkey,
    authority: &Pubkey,
    maybe_group_address: Option<Pubkey>,
) -> TestContext {
    let program_test = setup_program_test();

    let context = program_test.start_with_context().await;
    let context = Arc::new(tokio::sync::Mutex::new(context));
    let mut context = TestContext {
        context,
        token_context: None,
    };
    let mut extension_init_params = vec![ExtensionInitializationParams::GroupMemberPointer {
        authority: Some(*authority),
        member_address: Some(*member_address),
    }];
    if let Some(group_address) = maybe_group_address {
        extension_init_params.push(ExtensionInitializationParams::GroupPointer {
            authority: Some(*authority),
            group_address: Some(group_address),
        });
    }
    context
        .init_token_with_mint_keypair_and_freeze_authority(mint, extension_init_params, None)
        .await
        .unwrap();
    context
}

#[tokio::test]
async fn success_init() {
    let authority = Pubkey::new_unique();
    let member_address = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let token = setup(mint_keypair, &member_address, &authority, None)
        .await
        .token_context
        .take()
        .unwrap()
        .token;

    let state = token.get_mint_info().await.unwrap();
    assert!(state.base.is_initialized);
    let extension = state.get_extension::<GroupMemberPointer>().unwrap();
    assert_eq!(extension.authority, Some(authority).try_into().unwrap());
    assert_eq!(
        extension.member_address,
        Some(member_address).try_into().unwrap()
    );
}

#[tokio::test]
async fn success_init_with_group() {
    let authority = Pubkey::new_unique();
    let group_address = Pubkey::new_unique();
    let member_address = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let token = setup(
        mint_keypair,
        &member_address,
        &authority,
        Some(group_address),
    )
    .await
    .token_context
    .take()
    .unwrap()
    .token;

    let state = token.get_mint_info().await.unwrap();
    assert!(state.base.is_initialized);
    let extension = state.get_extension::<GroupMemberPointer>().unwrap();
    assert_eq!(extension.authority, Some(authority).try_into().unwrap());
    assert_eq!(
        extension.member_address,
        Some(member_address).try_into().unwrap()
    );
    let extension = state.get_extension::<GroupPointer>().unwrap();
    assert_eq!(extension.authority, Some(authority).try_into().unwrap());
    assert_eq!(
        extension.group_address,
        Some(group_address).try_into().unwrap()
    );
}

#[tokio::test]
async fn fail_init_all_none() {
    let mut program_test = ProgramTest::default();
    program_test.prefer_bpf(false);
    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(Processor::process),
    );
    let context = program_test.start_with_context().await;
    let context = Arc::new(tokio::sync::Mutex::new(context));
    let mut context = TestContext {
        context,
        token_context: None,
    };
    let err = context
        .init_token_with_mint(vec![ExtensionInitializationParams::GroupMemberPointer {
            authority: None,
            member_address: None,
        }])
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(TokenError::InvalidInstruction as u32)
            )
        )))
    );
}

#[tokio::test]
async fn set_authority() {
    let authority = Keypair::new();
    let member_address = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let token = setup(mint_keypair, &member_address, &authority.pubkey(), None)
        .await
        .token_context
        .take()
        .unwrap()
        .token;
    let new_authority = Keypair::new();

    // fail, wrong signature
    let wrong = Keypair::new();
    let err = token
        .set_authority(
            token.get_address(),
            &wrong.pubkey(),
            Some(&new_authority.pubkey()),
            instruction::AuthorityType::GroupMemberPointer,
            &[&wrong],
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::OwnerMismatch as u32)
            )
        )))
    );

    // success
    token
        .set_authority(
            token.get_address(),
            &authority.pubkey(),
            Some(&new_authority.pubkey()),
            instruction::AuthorityType::GroupMemberPointer,
            &[&authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<GroupMemberPointer>().unwrap();
    assert_eq!(
        extension.authority,
        Some(new_authority.pubkey()).try_into().unwrap(),
    );

    // set to none
    token
        .set_authority(
            token.get_address(),
            &new_authority.pubkey(),
            None,
            instruction::AuthorityType::GroupMemberPointer,
            &[&new_authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<GroupMemberPointer>().unwrap();
    assert_eq!(extension.authority, None.try_into().unwrap(),);

    // fail set again
    let err = token
        .set_authority(
            token.get_address(),
            &new_authority.pubkey(),
            Some(&authority.pubkey()),
            instruction::AuthorityType::GroupMemberPointer,
            &[&new_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::AuthorityTypeNotSupported as u32)
            )
        )))
    );
}

#[tokio::test]
async fn update_member_address() {
    let authority = Keypair::new();
    let member_address = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let token = setup(mint_keypair, &member_address, &authority.pubkey(), None)
        .await
        .token_context
        .take()
        .unwrap()
        .token;
    let new_member_address = Pubkey::new_unique();

    // fail, wrong signature
    let wrong = Keypair::new();
    let err = token
        .update_group_member_address(&wrong.pubkey(), Some(new_member_address), &[&wrong])
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::OwnerMismatch as u32)
            )
        )))
    );

    // success
    token
        .update_group_member_address(&authority.pubkey(), Some(new_member_address), &[&authority])
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<GroupMemberPointer>().unwrap();
    assert_eq!(
        extension.member_address,
        Some(new_member_address).try_into().unwrap(),
    );

    // set to none
    token
        .update_group_member_address(&authority.pubkey(), None, &[&authority])
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<GroupMemberPointer>().unwrap();
    assert_eq!(extension.member_address, None.try_into().unwrap(),);
}
