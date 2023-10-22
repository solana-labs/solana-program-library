#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::{
        processor,
        tokio::{self, sync::Mutex},
        ProgramTest, ProgramTestContext,
    },
    solana_sdk::{
        account::Account as SolanaAccount,
        instruction::InstructionError,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
    spl_token_2022::{
        error::TokenError,
        extension::{
            group_member_pointer::{instruction::update, GroupMemberPointer},
            BaseStateWithExtensions,
        },
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

async fn setup_group_mint(
    context: Arc<Mutex<ProgramTestContext>>,
    mint: Keypair,
    authority: &Pubkey,
) -> TestContext {
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

async fn setup_token_group(
    token_context: &TokenContext,
    mint_authority: &Keypair,
    update_authority: &Pubkey,
    payer: &Keypair,
) {
    token_context
        .token
        .token_group_initialize_with_rent_transfer(
            &payer.pubkey(),
            &mint_authority.pubkey(),
            update_authority,
            3,
            &[&payer, &mint_authority],
        )
        .await
        .unwrap();
}

async fn setup_member_mint(
    context: Arc<Mutex<ProgramTestContext>>,
    mint: Keypair,
    authority: &Pubkey,
) -> TestContext {
    let mut context = TestContext {
        context,
        token_context: None,
    };
    let member_address = Some(mint.pubkey());
    context
        .init_token_with_mint_keypair_and_freeze_authority(
            mint,
            vec![ExtensionInitializationParams::GroupMemberPointer {
                authority: Some(*authority),
                member_address,
            }],
            None,
        )
        .await
        .unwrap();
    context
}

#[tokio::test]
async fn success_init() {
    let payer = Keypair::new();
    let group_mint = Keypair::new();
    let group_update_authority = Keypair::new();
    let member_mint = Keypair::new();
    let member_authority = Keypair::new();

    let program_test = setup_program_test();
    let mut context = program_test.start_with_context().await;
    context.set_account(
        &payer.pubkey(),
        &SolanaAccount {
            lamports: 500_000_000,
            ..SolanaAccount::default()
        }
        .into(),
    );
    let context = Arc::new(tokio::sync::Mutex::new(context));

    let group_token = setup_group_mint(
        context.clone(),
        group_mint.insecure_clone(),
        &group_update_authority.pubkey(),
    )
    .await
    .token_context
    .take()
    .unwrap();

    setup_token_group(
        &group_token,
        &group_token.mint_authority,
        &group_update_authority.pubkey(),
        &payer,
    )
    .await;

    let member_token = setup_member_mint(
        context,
        member_mint.insecure_clone(),
        &member_authority.pubkey(),
    )
    .await
    .token_context
    .take()
    .unwrap()
    .token;

    let state = member_token.get_mint_info().await.unwrap();
    assert!(state.base.is_initialized);
    let extension = state.get_extension::<GroupMemberPointer>().unwrap();
    assert_eq!(
        extension.authority,
        Some(member_authority.pubkey()).try_into().unwrap()
    );
    assert_eq!(
        extension.member_address,
        Some(member_mint.pubkey()).try_into().unwrap()
    );
}

#[tokio::test]
async fn fail_init() {
    let payer = Keypair::new();
    let group_mint = Keypair::new();
    let group_update_authority = Keypair::new();

    let program_test = setup_program_test();
    let mut context = program_test.start_with_context().await;
    context.set_account(
        &payer.pubkey(),
        &SolanaAccount {
            lamports: 500_000_000,
            ..SolanaAccount::default()
        }
        .into(),
    );
    let context = Arc::new(tokio::sync::Mutex::new(context));

    let group_token = setup_group_mint(
        context.clone(),
        group_mint.insecure_clone(),
        &group_update_authority.pubkey(),
    )
    .await
    .token_context
    .take()
    .unwrap();

    setup_token_group(
        &group_token,
        &group_token.mint_authority,
        &group_update_authority.pubkey(),
        &payer,
    )
    .await;

    // fail with all none
    let mut context = TestContext {
        context,
        token_context: None,
    };
    let err = context
        .init_token_with_mint_keypair_and_freeze_authority(
            Keypair::new(),
            vec![ExtensionInitializationParams::GroupMemberPointer {
                authority: None,
                member_address: None,
            }],
            None,
        )
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
async fn success_update() {
    let payer = Keypair::new();
    let group_mint = Keypair::new();
    let group_update_authority = Keypair::new();
    let member_mint = Keypair::new();
    let member_authority = Keypair::new();

    let program_test = setup_program_test();
    let mut context = program_test.start_with_context().await;
    context.set_account(
        &payer.pubkey(),
        &SolanaAccount {
            lamports: 500_000_000,
            ..SolanaAccount::default()
        }
        .into(),
    );
    let context = Arc::new(tokio::sync::Mutex::new(context));

    let group_token = setup_group_mint(
        context.clone(),
        group_mint.insecure_clone(),
        &group_update_authority.pubkey(),
    )
    .await
    .token_context
    .take()
    .unwrap();

    setup_token_group(
        &group_token,
        &group_token.mint_authority,
        &group_update_authority.pubkey(),
        &payer,
    )
    .await;

    let member_token = setup_member_mint(
        context.clone(),
        member_mint.insecure_clone(),
        &member_authority.pubkey(),
    )
    .await
    .token_context
    .take()
    .unwrap()
    .token;

    let new_member_address = Pubkey::new_unique();

    // success
    member_token
        .update_group_member_address(
            &member_authority.pubkey(),
            Some(new_member_address),
            &[&group_update_authority, &member_authority],
        )
        .await
        .unwrap();
    let state = member_token.get_mint_info().await.unwrap();
    assert!(state.base.is_initialized);
    let extension = state.get_extension::<GroupMemberPointer>().unwrap();
    assert_eq!(
        extension.authority,
        Some(member_authority.pubkey()).try_into().unwrap()
    );
    assert_eq!(
        extension.member_address,
        Some(new_member_address).try_into().unwrap()
    );

    // set to none
    member_token
        .update_group_member_address(
            &member_authority.pubkey(),
            None,
            &[&group_update_authority, &member_authority],
        )
        .await
        .unwrap();
    let state = member_token.get_mint_info().await.unwrap();
    assert!(state.base.is_initialized);
    let extension = state.get_extension::<GroupMemberPointer>().unwrap();
    assert_eq!(
        extension.authority,
        Some(member_authority.pubkey()).try_into().unwrap()
    );
    assert_eq!(extension.member_address, None.try_into().unwrap());
}

#[tokio::test]
async fn fail_update() {
    let payer = Keypair::new();
    let group_mint = Keypair::new();
    let group_update_authority = Keypair::new();
    let member_mint = Keypair::new();
    let member_authority = Keypair::new();

    let program_test = setup_program_test();
    let mut context = program_test.start_with_context().await;
    context.set_account(
        &payer.pubkey(),
        &SolanaAccount {
            lamports: 500_000_000,
            ..SolanaAccount::default()
        }
        .into(),
    );
    let context = Arc::new(tokio::sync::Mutex::new(context));

    let group_token = setup_group_mint(
        context.clone(),
        group_mint.insecure_clone(),
        &group_update_authority.pubkey(),
    )
    .await
    .token_context
    .take()
    .unwrap();

    setup_token_group(
        &group_token,
        &group_token.mint_authority,
        &group_update_authority.pubkey(),
        &payer,
    )
    .await;

    let member_token = setup_member_mint(
        context.clone(),
        member_mint.insecure_clone(),
        &member_authority.pubkey(),
    )
    .await
    .token_context
    .take()
    .unwrap()
    .token;

    let wrong = Keypair::new();
    let new_member_address = Pubkey::new_unique();

    // fail, wrong signature for authority
    let err = member_token
        .update_group_member_address(
            &wrong.pubkey(),
            Some(new_member_address),
            &[&group_update_authority, &wrong],
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

    let mut context = context.lock().await;

    // fail, missing authority signature
    let mut instruction = update(
        &spl_token_2022::id(),
        &member_mint.pubkey(),
        &member_authority.pubkey(),
        &[],
        Some(member_mint.pubkey()),
    )
    .unwrap();
    instruction.accounts[1].is_signer = false;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
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
