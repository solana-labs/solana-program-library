use solana_program_test::{
    tokio::{self, sync::Mutex},
    ProgramTest,
};
use solana_sdk::signer::{keypair::Keypair, Signer};
use spl_token_api::{Token, TokenBanksClient, TokenClient};
use std::sync::Arc;

#[tokio::test]
async fn create_associated_token_account() {
    let program_test = ProgramTest::default();
    let ctx = program_test.start_with_context().await;
    let ctx = Arc::new(Mutex::new(ctx));

    let payer =
        Keypair::from_bytes(&ctx.lock().await.payer.to_bytes()).expect("failed to copy keypair");

    let client = TokenBanksClient::new_from_context(Arc::clone(&ctx));
    let client: Arc<Box<dyn TokenClient>> = Arc::new(Box::new(client));

    let mint_account = Keypair::new();
    let mint_authority = Keypair::new().pubkey();

    let token = Token::create_mint(
        Arc::clone(&client),
        &payer,
        &mint_account,
        &mint_authority,
        None,
        6,
    )
    .await
    .expect("failed to create mint");

    let account_owner = Keypair::new();
    token
        .create_associated_token_account(&account_owner.pubkey())
        .await
        .expect("failed to create associated token account");

    println!("{:?}", token);
}
