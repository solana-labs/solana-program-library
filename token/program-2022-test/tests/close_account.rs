#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::TestContext,
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError, program_pack::Pack, pubkey::Pubkey, signature::Signer,
        signer::keypair::Keypair, system_instruction, transaction::TransactionError,
        transport::TransportError,
    },
    spl_token_2022::{instruction, state::Account},
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
};

#[tokio::test]
async fn success_init_after_close_account() {
    let mut context = TestContext::new().await;
    let payer = Keypair::from_bytes(&context.context.lock().await.payer.to_bytes()).unwrap();
    context.init_token_with_mint(vec![]).await.unwrap();
    let token = context.token_context.take().unwrap().token;
    let token_program_id = spl_token_2022::id();
    let owner = Keypair::new();
    let token_account_keypair = Keypair::new();
    token
        .create_auxiliary_token_account(&token_account_keypair, &owner.pubkey())
        .await
        .unwrap();
    let token_account = token_account_keypair.pubkey();

    let destination = Pubkey::new_unique();
    token
        .process_ixs(
            &[
                instruction::close_account(
                    &token_program_id,
                    &token_account,
                    &destination,
                    &owner.pubkey(),
                    &[],
                )
                .unwrap(),
                system_instruction::create_account(
                    &payer.pubkey(),
                    &token_account,
                    1_000_000_000,
                    Account::LEN as u64,
                    &token_program_id,
                ),
                instruction::initialize_account(
                    &token_program_id,
                    &token_account,
                    token.get_address(),
                    &owner.pubkey(),
                )
                .unwrap(),
            ],
            &[&owner, &payer, &token_account_keypair],
        )
        .await
        .unwrap();
    let destination = token.get_account(destination).await.unwrap();
    assert!(destination.lamports > 0);
}

#[tokio::test]
async fn fail_init_after_close_account() {
    let mut context = TestContext::new().await;
    let payer = Keypair::from_bytes(&context.context.lock().await.payer.to_bytes()).unwrap();
    context.init_token_with_mint(vec![]).await.unwrap();
    let token = context.token_context.take().unwrap().token;
    let token_program_id = spl_token_2022::id();
    let owner = Keypair::new();
    let token_account_keypair = Keypair::new();
    token
        .create_auxiliary_token_account(&token_account_keypair, &owner.pubkey())
        .await
        .unwrap();
    let token_account = token_account_keypair.pubkey();

    let destination = Pubkey::new_unique();
    let error = token
        .process_ixs(
            &[
                instruction::close_account(
                    &token_program_id,
                    &token_account,
                    &destination,
                    &owner.pubkey(),
                    &[],
                )
                .unwrap(),
                system_instruction::transfer(&payer.pubkey(), &token_account, 1_000_000_000),
                instruction::initialize_account(
                    &token_program_id,
                    &token_account,
                    token.get_address(),
                    &owner.pubkey(),
                )
                .unwrap(),
            ],
            &[&owner, &payer],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(2, InstructionError::InvalidAccountData,)
        )))
    );
    let error = token.get_account(destination).await.unwrap_err();
    assert_eq!(error, TokenClientError::AccountNotFound);
}

#[tokio::test]
async fn fail_init_after_close_mint() {
    let close_authority = Keypair::new();
    let mut context = TestContext::new().await;
    let payer = Keypair::from_bytes(&context.context.lock().await.payer.to_bytes()).unwrap();
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::MintCloseAuthority {
            close_authority: Some(close_authority.pubkey()),
        }])
        .await
        .unwrap();
    let token = context.token_context.take().unwrap().token;
    let token_program_id = spl_token_2022::id();

    let destination = Pubkey::new_unique();
    let error = token
        .process_ixs(
            &[
                instruction::close_account(
                    &token_program_id,
                    token.get_address(),
                    &destination,
                    &close_authority.pubkey(),
                    &[],
                )
                .unwrap(),
                system_instruction::transfer(&payer.pubkey(), token.get_address(), 1_000_000_000),
                instruction::initialize_mint_close_authority(
                    &token_program_id,
                    token.get_address(),
                    None,
                )
                .unwrap(),
                instruction::initialize_mint(
                    &token_program_id,
                    token.get_address(),
                    &close_authority.pubkey(),
                    None,
                    0,
                )
                .unwrap(),
            ],
            &[&close_authority, &payer],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(2, InstructionError::InvalidAccountData,)
        )))
    );
    let error = token.get_account(destination).await.unwrap_err();
    assert_eq!(error, TokenClientError::AccountNotFound);
}
