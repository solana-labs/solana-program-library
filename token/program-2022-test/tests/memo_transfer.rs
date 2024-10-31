#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::{
        tokio::{self, sync::Mutex},
        ProgramTestContext,
    },
    solana_sdk::{
        instruction::InstructionError,
        pubkey::Pubkey,
        signature::Signer,
        system_instruction,
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
    spl_token_2022::{
        error::TokenError,
        extension::{memo_transfer::MemoTransfer, BaseStateWithExtensions, ExtensionType},
    },
    spl_token_client::token::TokenError as TokenClientError,
    std::sync::Arc,
};

async fn test_memo_transfers(
    context: Arc<Mutex<ProgramTestContext>>,
    token_context: TokenContext,
    alice_account: Pubkey,
    bob_account: Pubkey,
) {
    let TokenContext {
        mint_authority,
        token,
        alice,
        bob,
        ..
    } = token_context;

    // mint tokens
    token
        .mint_to(
            &alice_account,
            &mint_authority.pubkey(),
            4242,
            &[&mint_authority],
        )
        .await
        .unwrap();

    // require memo transfers into bob_account
    token
        .enable_required_transfer_memos(&bob_account, &bob.pubkey(), &[&bob])
        .await
        .unwrap();

    let bob_state = token.get_account_info(&bob_account).await.unwrap();
    let extension = bob_state.get_extension::<MemoTransfer>().unwrap();
    assert!(bool::from(extension.require_incoming_transfer_memos));

    // attempt to transfer from alice to bob without memo
    let err = token
        .transfer(&alice_account, &bob_account, &alice.pubkey(), 10, &[&alice])
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NoMemo as u32)
            )
        )))
    );
    let bob_state = token.get_account_info(&bob_account).await.unwrap();
    assert_eq!(bob_state.base.amount, 0);

    // attempt to transfer from bob to bob without memo
    let err = token
        .transfer(&bob_account, &bob_account, &bob.pubkey(), 0, &[&bob])
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NoMemo as u32)
            )
        )))
    );
    let bob_state = token.get_account_info(&bob_account).await.unwrap();
    assert_eq!(bob_state.base.amount, 0);

    // attempt to transfer from alice to bob with misplaced memo, v1 and current
    let mut memo_ix = spl_memo::build_memo(&[240, 159, 166, 150], &[]);
    for program_id in [spl_memo::id(), spl_memo::v1::id()] {
        let ctx = context.lock().await;
        memo_ix.program_id = program_id;
        #[allow(deprecated)]
        let instructions = vec![
            memo_ix.clone(),
            system_instruction::transfer(&ctx.payer.pubkey(), &alice.pubkey(), 42),
            spl_token_2022::instruction::transfer(
                &spl_token_2022::id(),
                &alice_account,
                &bob_account,
                &alice.pubkey(),
                &[],
                10,
            )
            .unwrap(),
        ];
        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &alice],
            ctx.last_blockhash,
        );
        let err = ctx
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();
        drop(ctx);
        assert_eq!(
            err,
            TransactionError::InstructionError(
                2,
                InstructionError::Custom(TokenError::NoMemo as u32)
            )
        );
        let bob_state = token.get_account_info(&bob_account).await.unwrap();
        assert_eq!(bob_state.base.amount, 0);
    }

    // transfer with memo
    token
        .with_memo("🦖", vec![alice.pubkey()])
        .transfer(&alice_account, &bob_account, &alice.pubkey(), 10, &[&alice])
        .await
        .unwrap();
    let bob_state = token.get_account_info(&bob_account).await.unwrap();
    assert_eq!(bob_state.base.amount, 10);

    // transfer with memo v1
    let ctx = context.lock().await;
    memo_ix.program_id = spl_memo::v1::id();
    #[allow(deprecated)]
    let instructions = vec![
        memo_ix,
        spl_token_2022::instruction::transfer(
            &spl_token_2022::id(),
            &alice_account,
            &bob_account,
            &alice.pubkey(),
            &[],
            11,
        )
        .unwrap(),
    ];
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &alice],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();
    drop(ctx);
    let bob_state = token.get_account_info(&bob_account).await.unwrap();
    assert_eq!(bob_state.base.amount, 21);

    // stop requiring memo transfers into bob_account
    token
        .disable_required_transfer_memos(&bob_account, &bob.pubkey(), &[&bob])
        .await
        .unwrap();

    // transfer from alice to bob without memo
    token
        .transfer(&alice_account, &bob_account, &alice.pubkey(), 12, &[&alice])
        .await
        .unwrap();
    let bob_state = token.get_account_info(&bob_account).await.unwrap();
    assert_eq!(bob_state.base.amount, 33);
}

#[tokio::test]
async fn require_memo_transfers_without_realloc() {
    let mut context = TestContext::new().await;
    context.init_token_with_mint(vec![]).await.unwrap();
    let token_context = context.token_context.unwrap();

    // create token accounts
    token_context
        .token
        .create_auxiliary_token_account(&token_context.alice, &token_context.alice.pubkey())
        .await
        .unwrap();
    let alice_account = token_context.alice.pubkey();
    token_context
        .token
        .create_auxiliary_token_account_with_extension_space(
            &token_context.bob,
            &token_context.bob.pubkey(),
            vec![ExtensionType::MemoTransfer],
        )
        .await
        .unwrap();
    let bob_account = token_context.bob.pubkey();

    test_memo_transfers(context.context, token_context, alice_account, bob_account).await;
}

#[tokio::test]
async fn require_memo_transfers_with_realloc() {
    let mut context = TestContext::new().await;
    context.init_token_with_mint(vec![]).await.unwrap();
    let token_context = context.token_context.unwrap();

    // create token accounts
    token_context
        .token
        .create_auxiliary_token_account(&token_context.alice, &token_context.alice.pubkey())
        .await
        .unwrap();
    let alice_account = token_context.alice.pubkey();
    token_context
        .token
        .create_auxiliary_token_account(&token_context.bob, &token_context.bob.pubkey())
        .await
        .unwrap();
    let bob_account = token_context.bob.pubkey();
    token_context
        .token
        .reallocate(
            &token_context.bob.pubkey(),
            &token_context.bob.pubkey(),
            &[ExtensionType::MemoTransfer],
            &[&token_context.bob],
        )
        .await
        .unwrap();

    test_memo_transfers(context.context, token_context, alice_account, bob_account).await;
}
