use solana_program_test::{
    tokio::{self, sync::Mutex},
    ProgramTest,
};
use solana_sdk::signer::{keypair::Keypair, Signer};
use spl_token_client::{
    client::{TokenBanksClient, TokenBanksClientProcessTransaction, TokenClient},
    token::Token,
};
use std::sync::Arc;

#[tokio::test]
async fn create_associated_token_account() {
    let program_test = ProgramTest::default();
    let ctx = program_test.start_with_context().await;
    let ctx = Arc::new(Mutex::new(ctx));

    let payer =
        Keypair::from_bytes(&ctx.lock().await.payer.to_bytes()).expect("failed to copy keypair");

    let client: Arc<dyn TokenClient<TokenBanksClientProcessTransaction>> = Arc::new(
        TokenBanksClient::new_from_context(Arc::clone(&ctx), TokenBanksClientProcessTransaction),
    );

    let decimals: u8 = 6;

    let mint_account = Keypair::new();
    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    // Create token
    let token = Token::create_mint(
        Arc::clone(&client),
        &payer,
        &mint_account,
        &mint_authority_pubkey,
        None,
        decimals,
    )
    .await
    .expect("failed to create mint");

    // Create associated address
    let alice = Keypair::new();
    let alice_vault = token
        .create_associated_token_account(&alice.pubkey())
        .await
        .expect("failed to create associated token account");

    let bob = Keypair::new();
    let bob_vault = token
        .create_associated_token_account(&bob.pubkey())
        .await
        .expect("failed to create associated token account");

    // Get associated address
    assert_eq!(
        token.get_associated_token_address(&alice.pubkey()),
        alice_vault
    );

    // Mint
    let mint_amount = 10 * u64::pow(10, decimals as u32);
    token
        .mint_to(&alice_vault, &mint_authority, mint_amount)
        .await
        .expect("failed to mint token");
    assert_eq!(
        token
            .get_account_info(alice_vault)
            .await
            .expect("failed to get account")
            .amount,
        mint_amount
    );

    // Transfer
    let transfer_amount = mint_amount.overflowing_div(3).0;
    token
        .transfer(&alice_vault, &bob_vault, &alice, transfer_amount)
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
