#![allow(dead_code)]

use {
    solana_program_test::{processor, tokio::sync::Mutex, ProgramTest, ProgramTestContext},
    solana_sdk::{
        pubkey::Pubkey,
        signer::{keypair::Keypair, Signer},
    },
    spl_token_2022::{
        extension::{
            confidential_transfer::ConfidentialTransferAccount, BaseStateWithExtensions,
            ExtensionType,
        },
        id, native_mint,
        processor::Processor,
        solana_zk_sdk::encryption::{auth_encryption::*, elgamal::*},
    },
    spl_token_client::{
        client::{
            ProgramBanksClient, ProgramBanksClientProcessTransaction, ProgramClient,
            SendTransaction, SimulateTransaction,
        },
        token::{ComputeUnitLimit, ExtensionInitializationParams, Token, TokenResult},
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
        let mut program_test =
            ProgramTest::new("spl_token_2022", id(), processor!(Processor::process));
        program_test.prefer_bpf(false);
        program_test.add_program(
            "spl_record",
            spl_record::id(),
            processor!(spl_record::processor::process_instruction),
        );
        program_test.add_program(
            "spl_elgamal_registry",
            spl_elgamal_registry::id(),
            processor!(spl_elgamal_registry::processor::process_instruction),
        );
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
        self.init_token_with_mint_and_freeze_authority(extension_init_params, None)
            .await
    }

    pub async fn init_token_with_freezing_mint(
        &mut self,
        extension_init_params: Vec<ExtensionInitializationParams>,
    ) -> TokenResult<()> {
        let freeze_authority = Keypair::new();
        self.init_token_with_mint_and_freeze_authority(
            extension_init_params,
            Some(freeze_authority),
        )
        .await
    }

    pub async fn init_token_with_mint_and_freeze_authority(
        &mut self,
        extension_init_params: Vec<ExtensionInitializationParams>,
        freeze_authority: Option<Keypair>,
    ) -> TokenResult<()> {
        let mint_account = Keypair::new();
        self.init_token_with_mint_keypair_and_freeze_authority(
            mint_account,
            extension_init_params,
            freeze_authority,
        )
        .await
    }

    pub async fn init_token_with_mint_keypair_and_freeze_authority(
        &mut self,
        mint_account: Keypair,
        extension_init_params: Vec<ExtensionInitializationParams>,
        freeze_authority: Option<Keypair>,
    ) -> TokenResult<()> {
        let mint_authority = Keypair::new();
        self.init_token_with_mint_keypair_and_freeze_authority_and_mint_authority(
            mint_account,
            extension_init_params,
            freeze_authority,
            mint_authority,
        )
        .await
    }

    pub async fn init_token_with_mint_keypair_and_freeze_authority_and_mint_authority(
        &mut self,
        mint_account: Keypair,
        extension_init_params: Vec<ExtensionInitializationParams>,
        freeze_authority: Option<Keypair>,
        mint_authority: Keypair,
    ) -> TokenResult<()> {
        let payer = keypair_clone(&self.context.lock().await.payer);
        let client: Arc<dyn ProgramClient<ProgramBanksClientProcessTransaction>> =
            Arc::new(ProgramBanksClient::new_from_context(
                Arc::clone(&self.context),
                ProgramBanksClientProcessTransaction,
            ));

        let decimals: u8 = 9;

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
        )
        .with_compute_unit_limit(ComputeUnitLimit::Simulated);

        let token_unchecked = Token::new(
            Arc::clone(&client),
            &id(),
            &mint_account.pubkey(),
            None,
            Arc::new(payer),
        )
        .with_compute_unit_limit(ComputeUnitLimit::Simulated);

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
        let token_unchecked = Token::new_native(Arc::clone(&client), &id(), Arc::new(payer))
            .with_compute_unit_limit(ComputeUnitLimit::Simulated);
        self.token_context = Some(TokenContext {
            decimals: native_mint::DECIMALS,
            mint_authority: Keypair::new(), /* bogus */
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

pub(crate) struct ConfidentialTokenAccountMeta {
    pub(crate) token_account: Pubkey,
    pub(crate) elgamal_keypair: ElGamalKeypair,
    pub(crate) aes_key: AeKey,
}

impl ConfidentialTokenAccountMeta {
    pub(crate) async fn new<T>(
        token: &Token<T>,
        owner: &Keypair,
        maximum_pending_balance_credit_counter: Option<u64>,
        require_memo: bool,
        require_fee: bool,
    ) -> Self
    where
        T: SendTransaction + SimulateTransaction,
    {
        let token_account_keypair = Keypair::new();

        let mut extensions = vec![ExtensionType::ConfidentialTransferAccount];
        if require_memo {
            extensions.push(ExtensionType::MemoTransfer);
        }
        if require_fee {
            extensions.push(ExtensionType::ConfidentialTransferFeeAmount);
        }

        token
            .create_auxiliary_token_account_with_extension_space(
                &token_account_keypair,
                &owner.pubkey(),
                extensions,
            )
            .await
            .unwrap();
        let token_account = token_account_keypair.pubkey();

        let elgamal_keypair =
            ElGamalKeypair::new_from_signer(owner, &token_account.to_bytes()).unwrap();
        let aes_key = AeKey::new_from_signer(owner, &token_account.to_bytes()).unwrap();

        token
            .confidential_transfer_configure_token_account(
                &token_account,
                &owner.pubkey(),
                None,
                maximum_pending_balance_credit_counter,
                &elgamal_keypair,
                &aes_key,
                &[owner],
            )
            .await
            .unwrap();

        if require_memo {
            token
                .enable_required_transfer_memos(&token_account, &owner.pubkey(), &[owner])
                .await
                .unwrap();
        }

        Self {
            token_account,
            elgamal_keypair,
            aes_key,
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "zk-ops")]
    pub(crate) async fn new_with_tokens<T>(
        token: &Token<T>,
        owner: &Keypair,
        maximum_pending_balance_credit_counter: Option<u64>,
        require_memo: bool,
        require_fee: bool,
        mint_authority: &Keypair,
        amount: u64,
        decimals: u8,
    ) -> Self
    where
        T: SendTransaction + SimulateTransaction,
    {
        let meta = Self::new(
            token,
            owner,
            maximum_pending_balance_credit_counter,
            require_memo,
            require_fee,
        )
        .await;

        token
            .mint_to(
                &meta.token_account,
                &mint_authority.pubkey(),
                amount,
                &[mint_authority],
            )
            .await
            .unwrap();

        token
            .confidential_transfer_deposit(
                &meta.token_account,
                &owner.pubkey(),
                amount,
                decimals,
                &[owner],
            )
            .await
            .unwrap();

        token
            .confidential_transfer_apply_pending_balance(
                &meta.token_account,
                &owner.pubkey(),
                None,
                meta.elgamal_keypair.secret(),
                &meta.aes_key,
                &[owner],
            )
            .await
            .unwrap();
        meta
    }

    #[cfg(feature = "zk-ops")]
    pub(crate) async fn check_balances<T>(
        &self,
        token: &Token<T>,
        expected: ConfidentialTokenAccountBalances,
    ) where
        T: SendTransaction + SimulateTransaction,
    {
        let state = token.get_account_info(&self.token_account).await.unwrap();
        let extension = state
            .get_extension::<ConfidentialTransferAccount>()
            .unwrap();

        assert_eq!(
            self.elgamal_keypair
                .secret()
                .decrypt_u32(&extension.pending_balance_lo.try_into().unwrap())
                .unwrap(),
            expected.pending_balance_lo,
        );
        assert_eq!(
            self.elgamal_keypair
                .secret()
                .decrypt_u32(&extension.pending_balance_hi.try_into().unwrap())
                .unwrap(),
            expected.pending_balance_hi,
        );
        assert_eq!(
            self.elgamal_keypair
                .secret()
                .decrypt_u32(&extension.available_balance.try_into().unwrap())
                .unwrap(),
            expected.available_balance,
        );
        assert_eq!(
            self.aes_key
                .decrypt(&extension.decryptable_available_balance.try_into().unwrap())
                .unwrap(),
            expected.decryptable_available_balance,
        );
    }
}

#[cfg(feature = "zk-ops")]
pub(crate) struct ConfidentialTokenAccountBalances {
    pub(crate) pending_balance_lo: u64,
    pub(crate) pending_balance_hi: u64,
    pub(crate) available_balance: u64,
    pub(crate) decryptable_available_balance: u64,
}

#[derive(Clone, Copy)]
pub enum ConfidentialTransferOption {
    InstructionData,
    RecordAccount,
    ContextStateAccount,
}
