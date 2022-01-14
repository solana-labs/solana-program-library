#![cfg(feature = "test-bpf")]

mod program_test;
use {
    program_test::TestContext,
    solana_program_test::tokio,
    solana_sdk::{signature::Signer, signer::keypair::Keypair},
};

#[tokio::test]
async fn transfer() {
    let TestContext {
        decimals,
        mint_authority,
        token,
        alice,
        bob,
        ..
    } = TestContext::new(&[], &[]).await;

    let alice_vault = Keypair::new();
    let alice_vault = token
        .create_auxiliary_token_account(&alice_vault, &alice.pubkey())
        .await
        .expect("failed to create associated token account");
    let bob_vault = Keypair::new();
    let bob_vault = token
        .create_auxiliary_token_account(&bob_vault, &bob.pubkey())
        .await
        .expect("failed to create associated token account");

    let mint_amount = 10 * u64::pow(10, decimals as u32);
    token
        .mint_to(&alice_vault, &mint_authority, mint_amount)
        .await
        .expect("failed to mint token");

    let transfer_amount = mint_amount.overflowing_div(3).0;
    token
        .transfer_checked(&alice_vault, &bob_vault, &alice, transfer_amount, decimals)
        .await
        .expect("failed to transfer");

    assert_eq!(
        token
            .get_account_info(alice_vault)
            .await
            .expect("failed to get account")
            .amount,
        mint_amount - transfer_amount
    );
    assert_eq!(
        token
            .get_account_info(bob_vault)
            .await
            .expect("failed to get account")
            .amount,
        transfer_amount
    );
}
