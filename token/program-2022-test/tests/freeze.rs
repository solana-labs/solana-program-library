#![cfg(feature = "test-sbf")]

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
    token
        .create_auxiliary_token_account(&account, &alice.pubkey())
        .await
        .unwrap();
    let account = account.pubkey();
    let state = token.get_account_info(&account).await.unwrap();
    assert_eq!(state.base.state, AccountState::Initialized);

    token
        .freeze(&account, &freeze_authority.pubkey(), &[&freeze_authority])
        .await
        .unwrap();
    let state = token.get_account_info(&account).await.unwrap();
    assert_eq!(state.base.state, AccountState::Frozen);

    token
        .thaw(&account, &freeze_authority.pubkey(), &[&freeze_authority])
        .await
        .unwrap();
    let state = token.get_account_info(&account).await.unwrap();
    assert_eq!(state.base.state, AccountState::Initialized);
}
