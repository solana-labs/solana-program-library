use {
    solana_program_test::{processor, tokio::sync::Mutex, ProgramTest},
    solana_sdk::signer::{keypair::Keypair, Signer},
    spl_token_2022::{id, processor::Processor},
    spl_token_client::{
        client::{ProgramBanksClient, ProgramBanksClientProcessTransaction, ProgramClient},
        token::{ExtensionInitializationParams, Token, TokenResult},
    },
    std::sync::Arc,
};

#[derive(Debug)]
pub struct TestContext {
    pub decimals: u8,
    pub mint_authority: Keypair,
    pub token: Token<ProgramBanksClientProcessTransaction, Keypair>,
    pub alice: Keypair,
    pub bob: Keypair,
}

impl TestContext {
    pub async fn new(
        extension_init_params: Vec<ExtensionInitializationParams>,
    ) -> TokenResult<Self> {
        let program_test = ProgramTest::new("spl_token_2022", id(), processor!(Processor::process));
        let ctx = program_test.start_with_context().await;
        let ctx = Arc::new(Mutex::new(ctx));

        let payer = keypair_clone(&ctx.lock().await.payer);

        let client: Arc<dyn ProgramClient<ProgramBanksClientProcessTransaction>> =
            Arc::new(ProgramBanksClient::new_from_context(
                Arc::clone(&ctx),
                ProgramBanksClientProcessTransaction,
            ));

        let decimals: u8 = 9;

        let mint_account = Keypair::new();
        let mint_authority = Keypair::new();
        let mint_authority_pubkey = mint_authority.pubkey();

        let token = Token::create_mint(
            Arc::clone(&client),
            payer,
            &mint_account,
            &mint_authority_pubkey,
            None,
            decimals,
            extension_init_params,
        )
        .await?;

        Ok(Self {
            decimals,
            mint_authority,
            token,
            alice: Keypair::new(),
            bob: Keypair::new(),
        })
    }
}

fn keypair_clone(kp: &Keypair) -> Keypair {
    Keypair::from_bytes(&kp.to_bytes()).expect("failed to copy keypair")
}
