use {
    solana_program_test::{processor, tokio::sync::Mutex, ProgramTest, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    spl_token_2022::{id, processor::Processor},
    spl_token_client::{
        client::{ProgramBanksClient, ProgramBanksClientProcessTransaction, ProgramClient},
        token::{ExtensionInitializationParams, Token, TokenResult},
    },
    std::sync::Arc,
};

pub struct TestContext {
    pub decimals: u8,
    pub mint_authority: Keypair,
    pub token: Token<ProgramBanksClientProcessTransaction, Keypair>,
    pub alice: Keypair,
    pub bob: Keypair,
    pub context: Arc<Mutex<ProgramTestContext>>, // ProgramTestContext needs to #[derive(Debug)]
}

impl TestContext {
    pub async fn new(
        extension_init_params: Vec<ExtensionInitializationParams>,
    ) -> TokenResult<Self> {
        let program_test = ProgramTest::new("spl_token_2022", id(), processor!(Processor::process));
        let context = program_test.start_with_context().await;
        let context = Arc::new(Mutex::new(context));

        let payer = keypair_clone(&context.lock().await.payer);

        let client: Arc<dyn ProgramClient<ProgramBanksClientProcessTransaction>> =
            Arc::new(ProgramBanksClient::new_from_context(
                Arc::clone(&context),
                ProgramBanksClientProcessTransaction,
            ));

        let decimals: u8 = 9;

        let mint_account = Keypair::new();
        let mint_authority = Keypair::new();
        let mint_authority_pubkey = mint_authority.pubkey();

        let token = Token::create_mint(
            Arc::clone(&client),
            &id(),
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
            context,
        })
    }
}

fn keypair_clone(kp: &Keypair) -> Keypair {
    Keypair::from_bytes(&kp.to_bytes()).expect("failed to copy keypair")
}
