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

    let mint_account = Keypair::new();
    let mint_authority = Keypair::new();

    let token = Token::create_mint(
        Arc::clone(&client),
        &payer,
        &mint_account,
        &mint_authority.pubkey(),
        None,
        6,
    )
    .await
    .expect("failed to create mint");

    let alice = Keypair::new();
    let alice_vault = token
        .create_associated_token_account(&alice.pubkey())
        .await
        .expect("failed to create associated token account");

    token
        .mint_to(&alice_vault, &mint_authority, u64::pow(10, 6))
        .await
        .expect("failed to mint token");

    println!("{:?}", token);
}
