#![cfg(feature = "test-bpf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::tokio,
    solana_sdk::{pubkey::Pubkey, signature::Signer},
    spl_token_2022::extension::{memo_transfer::MemoTransfer, ExtensionType},
};

async fn test_memo_transfers(
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
        .mint_to(&alice_account, &mint_authority, 4242)
        .await
        .unwrap();

    // require memo transfers into bob_account
    token
        .enable_required_transfer_memos(&bob_account, &bob)
        .await
        .unwrap();

    let bob_state = token.get_account_info(&bob_account).await.unwrap();
    let extension = bob_state.get_extension::<MemoTransfer>().unwrap();
    assert!(bool::from(extension.require_incoming_transfer_memos));

    // attempt to transfer from alice to bob without memo
    // TODO: should fail when token/program-2022/src/processor.rs#L376 is completed
    token
        .transfer_unchecked(&alice_account, &bob_account, &alice, 10)
        .await
        .unwrap();
    let bob_state = token.get_account_info(&bob_account).await.unwrap();
    assert_eq!(bob_state.base.amount, 10);

    // stop requiring memo transfers into bob_account
    token
        .disable_required_transfer_memos(&bob_account, &bob)
        .await
        .unwrap();

    // transfer from alice to bob without memo
    token
        .transfer_unchecked(&alice_account, &bob_account, &alice, 11)
        .await
        .unwrap();
    let bob_state = token.get_account_info(&bob_account).await.unwrap();
    assert_eq!(bob_state.base.amount, 21);
}

#[tokio::test]
async fn require_memo_transfers_without_realloc() {
    let mut context = TestContext::new().await;
    context.init_token_with_mint(vec![]).await.unwrap();
    let token_context = context.token_context.unwrap();

    // create token accounts
    let alice_account = token_context
        .token
        .create_auxiliary_token_account(&token_context.alice, &token_context.alice.pubkey())
        .await
        .unwrap();
    let bob_account = token_context
        .token
        .create_auxiliary_token_account_with_extension_space(
            &token_context.bob,
            &token_context.bob.pubkey(),
            vec![ExtensionType::MemoTransfer],
        )
        .await
        .unwrap();

    test_memo_transfers(token_context, alice_account, bob_account).await;
}

#[tokio::test]
async fn require_memo_transfers_with_realloc() {
    let mut context = TestContext::new().await;
    context.init_token_with_mint(vec![]).await.unwrap();
    let token_context = context.token_context.unwrap();

    // create token accounts
    let alice_account = token_context
        .token
        .create_auxiliary_token_account(&token_context.alice, &token_context.alice.pubkey())
        .await
        .unwrap();
    let bob_account = token_context
        .token
        .create_auxiliary_token_account(&token_context.bob, &token_context.bob.pubkey())
        .await
        .unwrap();
    token_context
        .token
        .reallocate(
            &token_context.bob.pubkey(),
            &token_context.bob,
            &[ExtensionType::MemoTransfer],
        )
        .await
        .unwrap();

    test_memo_transfers(token_context, alice_account, bob_account).await;
}
