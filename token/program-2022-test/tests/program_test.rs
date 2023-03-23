#![allow(dead_code)]

use {
    solana_program_test::{processor, tokio::sync::Mutex, ProgramTest, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    spl_token_2022::{id, native_mint, processor::Processor},
    spl_token_client::{
        client::{ProgramBanksClient, ProgramBanksClientProcessTransaction, ProgramClient},
        token::{ExtensionInitializationParams, Token, TokenResult},
    },
    std::sync::Arc,
};

pub struct TokenContext {
    pub decimals: u8,
    pub mint_authority: Keypair,
    pub token: Token<ProgramBanksClientProcessTransaction>,
    pub token_unchecked: Token<ProgramBanksClientProcessTransaction>,
    pub alice: Keypair,
    pub bob: Keypair,
    pub freeze_authority: Option<Keypair>,
}

pub struct TestContext {
    pub context: Arc<Mutex<ProgramTestContext>>,
    pub token_context: Option<TokenContext>,
}

impl TestContext {
    pub async fn new() -> Self {
        let program_test = ProgramTest::new("spl_token_2022", id(), processor!(Processor::process));
        let context = program_test.start_with_context().await;
        let context = Arc::new(Mutex::new(context));

        Self {
            context,
            token_context: None,
        }
    }

    pub async fn init_token_with_mint(
        &mut self,
        extension_init_params: Vec<ExtensionInitializationParams>,
    ) -> TokenResult<()> {
        self._init_token_with_mint(extension_init_params, None)
            .await
    }

    pub async fn init_token_with_freezing_mint(
        &mut self,
        extension_init_params: Vec<ExtensionInitializationParams>,
    ) -> TokenResult<()> {
        let freeze_authority = Keypair::new();
        self._init_token_with_mint(extension_init_params, Some(freeze_authority))
            .await
    }

    pub async fn _init_token_with_mint(
        &mut self,
        extension_init_params: Vec<ExtensionInitializationParams>,
        freeze_authority: Option<Keypair>,
    ) -> TokenResult<()> {
        let payer = keypair_clone(&self.context.lock().await.payer);
        let client: Arc<dyn ProgramClient<ProgramBanksClientProcessTransaction>> =
            Arc::new(ProgramBanksClient::new_from_context(
                Arc::clone(&self.context),
                ProgramBanksClientProcessTransaction,
            ));

        let decimals: u8 = 9;

        let mint_account = Keypair::new();
        let mint_authority = Keypair::new();
        let mint_authority_pubkey = mint_authority.pubkey();
        let freeze_authority_pubkey = freeze_authority
            .as_ref()
            .map(|authority| authority.pubkey());

        let token = Token::new(
            Arc::clone(&client),
            &id(),
            &mint_account.pubkey(),
            Some(decimals),
            Arc::new(keypair_clone(&payer)),
        );

        let token_unchecked = Token::new(
            Arc::clone(&client),
            &id(),
            &mint_account.pubkey(),
            None,
            Arc::new(payer),
        );

        token
            .create_mint(
                &mint_authority_pubkey,
                freeze_authority_pubkey.as_ref(),
                extension_init_params,
                &[&mint_account],
            )
            .await?;

        self.token_context = Some(TokenContext {
            decimals,
            mint_authority,
            token,
            token_unchecked,
            alice: Keypair::new(),
            bob: Keypair::new(),
            freeze_authority,
        });

        Ok(())
    }

    pub async fn init_token_with_native_mint(&mut self) -> TokenResult<()> {
        let payer = keypair_clone(&self.context.lock().await.payer);
        let client: Arc<dyn ProgramClient<ProgramBanksClientProcessTransaction>> =
            Arc::new(ProgramBanksClient::new_from_context(
                Arc::clone(&self.context),
                ProgramBanksClientProcessTransaction,
            ));

        let token =
            Token::create_native_mint(Arc::clone(&client), &id(), Arc::new(keypair_clone(&payer)))
                .await?;
        // unchecked native is never needed because decimals is known statically
        let token_unchecked = Token::new_native(Arc::clone(&client), &id(), Arc::new(payer));
        self.token_context = Some(TokenContext {
            decimals: native_mint::DECIMALS,
            mint_authority: Keypair::new(), /*bogus*/
            token,
            token_unchecked,
            alice: Keypair::new(),
            bob: Keypair::new(),
            freeze_authority: None,
        });
        Ok(())
    }
}

pub(crate) fn keypair_clone(kp: &Keypair) -> Keypair {
    Keypair::from_bytes(&kp.to_bytes()).expect("failed to copy keypair")
}
