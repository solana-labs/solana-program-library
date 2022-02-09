#![cfg(feature = "test-bpf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::tokio,
    solana_sdk::{signature::Signer, signer::keypair::Keypair},
    spl_token_2022::state::AccountState,
};

#[tokio::test]
async fn basic() {
    let mut context = TestContext::new().await;
    context.init_token_with_freezing_mint(vec![]).await.unwrap();
    let TokenContext {
        freeze_authority,
        token,
        alice,
        ..
    } = context.token_context.unwrap();
    let freeze_authority = freeze_authority.unwrap();

    let account = Keypair::new();
    let account = token
        .create_auxiliary_token_account(&account, &alice.pubkey())
        .await
        .unwrap();
    let state = token.get_account_info(&account).await.unwrap();
    assert_eq!(state.base.state, AccountState::Initialized);

    token
        .freeze_account(&account, &freeze_authority)
        .await
        .unwrap();
    let state = token.get_account_info(&account).await.unwrap();
    assert_eq!(state.base.state, AccountState::Frozen);

    token
        .thaw_account(&account, &freeze_authority)
        .await
        .unwrap();
    let state = token.get_account_info(&account).await.unwrap();
    assert_eq!(state.base.state, AccountState::Initialized);
}
