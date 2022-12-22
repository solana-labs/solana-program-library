use {
    crate::client::{ProgramClient, ProgramClientError, SendTransaction},
    solana_program_test::tokio::time,
    solana_sdk::{
        account::Account as BaseAccount,
        epoch_info::EpochInfo,
        hash::Hash,
        instruction::Instruction,
        message::Message,
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        signer::{signers::Signers, Signer, SignerError},
        system_instruction,
        transaction::Transaction,
    },
    spl_associated_token_account::{
        get_associated_token_address_with_program_id, instruction::create_associated_token_account,
        instruction::create_associated_token_account_idempotent,
    },
    spl_token_2022::{
        extension::{
            confidential_transfer, cpi_guard, default_account_state, interest_bearing_mint,
            memo_transfer, transfer_fee, BaseStateWithExtensions, ExtensionType,
            StateWithExtensionsOwned,
        },
        instruction,
        pod::EncryptionPubkey,
        solana_zk_token_sdk::{
            encryption::{auth_encryption::*, elgamal::*},
            errors::ProofError,
            instruction::transfer_with_fee::FeeParameters,
        },
        state::{Account, AccountState, Mint, Multisig},
    },
    std::{
        convert::TryInto,
        fmt, io,
        sync::{Arc, RwLock},
        time::{Duration, Instant},
    },
    thiserror::Error,
};

#[derive(Error, Debug)]
pub enum TokenError {
    #[error("client error: {0}")]
    Client(ProgramClientError),
    #[error("program error: {0}")]
    Program(#[from] ProgramError),
    #[error("account not found")]
    AccountNotFound,
    #[error("invalid account owner")]
    AccountInvalidOwner,
    #[error("invalid account mint")]
    AccountInvalidMint,
    #[error("invalid associated account address")]
    AccountInvalidAssociatedAddress,
    #[error("invalid auxiliary account address")]
    AccountInvalidAuxiliaryAddress,
    #[error("proof error: {0}")]
    Proof(ProofError),
    #[error("maximum deposit transfer amount exceeded")]
    MaximumDepositTransferAmountExceeded,
    #[error("encryption key error")]
    Key(SignerError),
    #[error("account decryption failed")]
    AccountDecryption,
    #[error("not enough funds in account")]
    NotEnoughFunds,
    #[error("missing memo signer")]
    MissingMemoSigner,
    #[error("decimals required, but missing")]
    MissingDecimals,
    #[error("decimals specified, but incorrect")]
    InvalidDecimals,
}
impl PartialEq for TokenError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // TODO not great, but workable for tests
            // currently missing: proof error, signer error
            (Self::Client(ref a), Self::Client(ref b)) => a.to_string() == b.to_string(),
            (Self::Program(ref a), Self::Program(ref b)) => a == b,
            (Self::AccountNotFound, Self::AccountNotFound) => true,
            (Self::AccountInvalidOwner, Self::AccountInvalidOwner) => true,
            (Self::AccountInvalidMint, Self::AccountInvalidMint) => true,
            (Self::AccountInvalidAssociatedAddress, Self::AccountInvalidAssociatedAddress) => true,
            (Self::AccountInvalidAuxiliaryAddress, Self::AccountInvalidAuxiliaryAddress) => true,
            (
                Self::MaximumDepositTransferAmountExceeded,
                Self::MaximumDepositTransferAmountExceeded,
            ) => true,
            (Self::AccountDecryption, Self::AccountDecryption) => true,
            (Self::NotEnoughFunds, Self::NotEnoughFunds) => true,
            (Self::MissingMemoSigner, Self::MissingMemoSigner) => true,
            (Self::MissingDecimals, Self::MissingDecimals) => true,
            (Self::InvalidDecimals, Self::InvalidDecimals) => true,
            _ => false,
        }
    }
}

/// Encapsulates initializing an extension
#[derive(Clone, Debug, PartialEq)]
pub enum ExtensionInitializationParams {
    ConfidentialTransferMint {
        authority: Option<Pubkey>,
        auto_approve_new_accounts: bool,
        auditor_encryption_pubkey: Option<EncryptionPubkey>,
        withdraw_withheld_authority_encryption_pubkey: Option<EncryptionPubkey>,
    },
    DefaultAccountState {
        state: AccountState,
    },
    MintCloseAuthority {
        close_authority: Option<Pubkey>,
    },
    TransferFeeConfig {
        transfer_fee_config_authority: Option<Pubkey>,
        withdraw_withheld_authority: Option<Pubkey>,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
    },
    InterestBearingConfig {
        rate_authority: Option<Pubkey>,
        rate: i16,
    },
    NonTransferable,
    PermanentDelegate {
        delegate: Pubkey,
    },
}
impl ExtensionInitializationParams {
    /// Get the extension type associated with the init params
    pub fn extension(&self) -> ExtensionType {
        match self {
            Self::ConfidentialTransferMint { .. } => ExtensionType::ConfidentialTransferMint,
            Self::DefaultAccountState { .. } => ExtensionType::DefaultAccountState,
            Self::MintCloseAuthority { .. } => ExtensionType::MintCloseAuthority,
            Self::TransferFeeConfig { .. } => ExtensionType::TransferFeeConfig,
            Self::InterestBearingConfig { .. } => ExtensionType::InterestBearingConfig,
            Self::NonTransferable => ExtensionType::NonTransferable,
            Self::PermanentDelegate { .. } => ExtensionType::PermanentDelegate,
        }
    }
    /// Generate an appropriate initialization instruction for the given mint
    pub fn instruction(
        self,
        token_program_id: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Instruction, ProgramError> {
        match self {
            Self::ConfidentialTransferMint {
                authority,
                auto_approve_new_accounts,
                auditor_encryption_pubkey,
                withdraw_withheld_authority_encryption_pubkey,
            } => confidential_transfer::instruction::initialize_mint(
                token_program_id,
                mint,
                authority,
                auto_approve_new_accounts,
                auditor_encryption_pubkey,
                withdraw_withheld_authority_encryption_pubkey,
            ),
            Self::DefaultAccountState { state } => {
                default_account_state::instruction::initialize_default_account_state(
                    token_program_id,
                    mint,
                    &state,
                )
            }
            Self::MintCloseAuthority { close_authority } => {
                instruction::initialize_mint_close_authority(
                    token_program_id,
                    mint,
                    close_authority.as_ref(),
                )
            }
            Self::TransferFeeConfig {
                transfer_fee_config_authority,
                withdraw_withheld_authority,
                transfer_fee_basis_points,
                maximum_fee,
            } => transfer_fee::instruction::initialize_transfer_fee_config(
                token_program_id,
                mint,
                transfer_fee_config_authority.as_ref(),
                withdraw_withheld_authority.as_ref(),
                transfer_fee_basis_points,
                maximum_fee,
            ),
            Self::InterestBearingConfig {
                rate_authority,
                rate,
            } => interest_bearing_mint::instruction::initialize(
                token_program_id,
                mint,
                rate_authority,
                rate,
            ),
            Self::NonTransferable => {
                instruction::initialize_non_transferable_mint(token_program_id, mint)
            }
            Self::PermanentDelegate { delegate } => {
                instruction::initialize_permanent_delegate(token_program_id, mint, &delegate)
            }
        }
    }
}

pub type TokenResult<T> = Result<T, TokenError>;

#[derive(Debug)]
struct TokenMemo {
    text: String,
    signers: Vec<Pubkey>,
}
impl TokenMemo {
    pub fn to_instruction(&self) -> Instruction {
        spl_memo::build_memo(
            self.text.as_bytes(),
            &self.signers.iter().collect::<Vec<_>>(),
        )
    }
}

pub struct Token<T> {
    client: Arc<dyn ProgramClient<T>>,
    pubkey: Pubkey, /*token mint*/
    decimals: Option<u8>,
    payer: Arc<dyn Signer>,
    program_id: Pubkey,
    nonce_account: Option<Pubkey>,
    nonce_authority: Option<Pubkey>,
    memo: Arc<RwLock<Option<TokenMemo>>>,
}

impl<T> fmt::Debug for Token<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Token")
            .field("pubkey", &self.pubkey)
            .field("decimals", &self.decimals)
            .field("payer", &self.payer.pubkey())
            .field("program_id", &self.program_id)
            .field("nonce_account", &self.nonce_account)
            .field("nonce_authority", &self.nonce_authority)
            .field("memo", &self.memo.read().unwrap())
            .finish()
    }
}

fn native_mint(program_id: &Pubkey) -> Pubkey {
    if program_id == &spl_token_2022::id() {
        spl_token_2022::native_mint::id()
    } else if program_id == &spl_token::id() {
        spl_token::native_mint::id()
    } else {
        panic!("Unrecognized token program id: {}", program_id);
    }
}

fn native_mint_decimals(program_id: &Pubkey) -> u8 {
    if program_id == &spl_token_2022::id() {
        spl_token_2022::native_mint::DECIMALS
    } else if program_id == &spl_token::id() {
        spl_token::native_mint::DECIMALS
    } else {
        panic!("Unrecognized token program id: {}", program_id);
    }
}

impl<T> Token<T>
where
    T: SendTransaction,
{
    pub fn new(
        client: Arc<dyn ProgramClient<T>>,
        program_id: &Pubkey,
        address: &Pubkey,
        decimals: Option<u8>,
        payer: Arc<dyn Signer>,
    ) -> Self {
        Token {
            client,
            pubkey: *address,
            decimals,
            payer,
            program_id: *program_id,
            nonce_account: None,
            nonce_authority: None,
            memo: Arc::new(RwLock::new(None)),
        }
    }

    pub fn new_native(
        client: Arc<dyn ProgramClient<T>>,
        program_id: &Pubkey,
        payer: Arc<dyn Signer>,
    ) -> Self {
        Self::new(
            client,
            program_id,
            &native_mint(program_id),
            Some(native_mint_decimals(program_id)),
            payer,
        )
    }

    pub fn is_native(&self) -> bool {
        self.pubkey == native_mint(&self.program_id)
    }

    /// Get token address.
    pub fn get_address(&self) -> &Pubkey {
        &self.pubkey
    }

    pub fn with_payer(&self, payer: Arc<dyn Signer>) -> Token<T> {
        Token {
            client: Arc::clone(&self.client),
            pubkey: self.pubkey,
            decimals: self.decimals,
            payer,
            program_id: self.program_id,
            nonce_account: self.nonce_account,
            nonce_authority: self.nonce_authority,
            memo: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_nonce(&self, nonce_account: &Pubkey, nonce_authority: &Pubkey) -> Token<T> {
        Token {
            client: Arc::clone(&self.client),
            pubkey: self.pubkey,
            decimals: self.decimals,
            payer: self.payer.clone(),
            program_id: self.program_id,
            nonce_account: Some(*nonce_account),
            nonce_authority: Some(*nonce_authority),
            memo: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_memo<M: AsRef<str>>(&self, memo: M, signers: Vec<Pubkey>) -> &Self {
        let mut w_memo = self.memo.write().unwrap();
        *w_memo = Some(TokenMemo {
            text: memo.as_ref().to_string(),
            signers,
        });
        self
    }

    pub async fn get_new_latest_blockhash(&self) -> TokenResult<Hash> {
        let blockhash = self
            .client
            .get_latest_blockhash()
            .await
            .map_err(TokenError::Client)?;
        let start = Instant::now();
        let mut num_retries = 0;
        while start.elapsed().as_secs() < 5 {
            let new_blockhash = self
                .client
                .get_latest_blockhash()
                .await
                .map_err(TokenError::Client)?;
            if new_blockhash != blockhash {
                return Ok(new_blockhash);
            }

            time::sleep(Duration::from_millis(200)).await;
            num_retries += 1;
        }

        Err(TokenError::Client(Box::new(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Unable to get new blockhash after {}ms (retried {} times), stuck at {}",
                start.elapsed().as_millis(),
                num_retries,
                blockhash
            ),
        ))))
    }

    fn get_multisig_signers<'a, 'b>(
        &self,
        authority: &'b Pubkey,
        signing_pubkeys: &'a Vec<Pubkey>,
    ) -> Vec<&'a Pubkey> {
        if signing_pubkeys.as_ref() == [*authority] {
            vec![]
        } else {
            signing_pubkeys.iter().collect::<Vec<_>>()
        }
    }

    async fn construct_tx<S: Signers>(
        &self,
        token_instructions: &[Instruction],
        signing_keypairs: &S,
    ) -> TokenResult<Transaction> {
        let mut instructions = vec![];
        let payer_key = self.payer.pubkey();
        let fee_payer = Some(&payer_key);

        {
            let mut w_memo = self.memo.write().unwrap();
            if let Some(memo) = w_memo.take() {
                let signing_pubkeys = signing_keypairs.pubkeys();
                if !memo
                    .signers
                    .iter()
                    .all(|signer| signing_pubkeys.contains(signer))
                {
                    return Err(TokenError::MissingMemoSigner);
                }

                instructions.push(memo.to_instruction());
            }
        }

        instructions.extend_from_slice(token_instructions);

        let latest_blockhash = self
            .client
            .get_latest_blockhash()
            .await
            .map_err(TokenError::Client)?;

        let message = if let (Some(nonce_account), Some(nonce_authority)) =
            (self.nonce_account, self.nonce_authority)
        {
            let mut message = Message::new_with_nonce(
                token_instructions.to_vec(),
                fee_payer,
                &nonce_account,
                &nonce_authority,
            );
            message.recent_blockhash = latest_blockhash;
            message
        } else {
            Message::new_with_blockhash(&instructions, fee_payer, &latest_blockhash)
        };

        let mut transaction = Transaction::new_unsigned(message);

        transaction
            .try_partial_sign(&vec![self.payer.clone()], latest_blockhash)
            .map_err(|error| TokenError::Client(error.into()))?;
        transaction
            .try_partial_sign(signing_keypairs, latest_blockhash)
            .map_err(|error| TokenError::Client(error.into()))?;

        Ok(transaction)
    }

    pub async fn process_ixs<S: Signers>(
        &self,
        token_instructions: &[Instruction],
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let transaction = self
            .construct_tx(token_instructions, signing_keypairs)
            .await?;

        self.client
            .send_transaction(&transaction)
            .await
            .map_err(TokenError::Client)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_mint<'a, S: Signers>(
        &self,
        mint_authority: &'a Pubkey,
        freeze_authority: Option<&'a Pubkey>,
        extension_initialization_params: Vec<ExtensionInitializationParams>,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let decimals = self.decimals.ok_or(TokenError::MissingDecimals)?;

        let extension_types = extension_initialization_params
            .iter()
            .map(|e| e.extension())
            .collect::<Vec<_>>();
        let space = ExtensionType::get_account_len::<Mint>(&extension_types);

        let mut instructions = vec![system_instruction::create_account(
            &self.payer.pubkey(),
            &self.pubkey,
            self.client
                .get_minimum_balance_for_rent_exemption(space)
                .await
                .map_err(TokenError::Client)?,
            space as u64,
            &self.program_id,
        )];

        for params in extension_initialization_params {
            instructions.push(params.instruction(&self.program_id, &self.pubkey)?);
        }

        instructions.push(instruction::initialize_mint(
            &self.program_id,
            &self.pubkey,
            mint_authority,
            freeze_authority,
            decimals,
        )?);

        self.process_ixs(&instructions, signing_keypairs).await
    }

    /// Create native mint
    pub async fn create_native_mint(
        client: Arc<dyn ProgramClient<T>>,
        program_id: &Pubkey,
        payer: Arc<dyn Signer>,
    ) -> TokenResult<Self> {
        let token = Self::new_native(client, program_id, payer);
        token
            .process_ixs::<[&dyn Signer; 0]>(
                &[instruction::create_native_mint(
                    program_id,
                    &token.payer.pubkey(),
                )?],
                &[],
            )
            .await?;

        Ok(token)
    }

    /// Create multisig
    pub async fn create_multisig(
        &self,
        account: &dyn Signer,
        multisig_members: &[&Pubkey],
        minimum_signers: u8,
    ) -> TokenResult<T::Output> {
        let instructions = vec![
            system_instruction::create_account(
                &self.payer.pubkey(),
                &account.pubkey(),
                self.client
                    .get_minimum_balance_for_rent_exemption(Multisig::LEN)
                    .await
                    .map_err(TokenError::Client)?,
                Multisig::LEN as u64,
                &self.program_id,
            ),
            instruction::initialize_multisig(
                &self.program_id,
                &account.pubkey(),
                multisig_members,
                minimum_signers,
            )?,
        ];

        self.process_ixs(&instructions, &[account]).await
    }

    /// Get the address for the associated token account.
    pub fn get_associated_token_address(&self, owner: &Pubkey) -> Pubkey {
        get_associated_token_address_with_program_id(owner, &self.pubkey, &self.program_id)
    }

    /// Create and initialize the associated account.
    pub async fn create_associated_token_account(&self, owner: &Pubkey) -> TokenResult<T::Output> {
        self.process_ixs::<[&dyn Signer; 0]>(
            &[create_associated_token_account(
                &self.payer.pubkey(),
                owner,
                &self.pubkey,
                &self.program_id,
            )],
            &[],
        )
        .await
    }

    /// Create and initialize a new token account.
    pub async fn create_auxiliary_token_account(
        &self,
        account: &dyn Signer,
        owner: &Pubkey,
    ) -> TokenResult<T::Output> {
        self.create_auxiliary_token_account_with_extension_space(account, owner, vec![])
            .await
    }

    /// Create and initialize a new token account.
    pub async fn create_auxiliary_token_account_with_extension_space(
        &self,
        account: &dyn Signer,
        owner: &Pubkey,
        extensions: Vec<ExtensionType>,
    ) -> TokenResult<T::Output> {
        let state = self.get_mint_info().await?;
        let mint_extensions: Vec<ExtensionType> = state.get_extension_types()?;
        let mut required_extensions =
            ExtensionType::get_required_init_account_extensions(&mint_extensions);
        for extension_type in extensions.into_iter() {
            if !required_extensions.contains(&extension_type) {
                required_extensions.push(extension_type);
            }
        }
        let space = ExtensionType::get_account_len::<Account>(&required_extensions);
        let mut instructions = vec![system_instruction::create_account(
            &self.payer.pubkey(),
            &account.pubkey(),
            self.client
                .get_minimum_balance_for_rent_exemption(space)
                .await
                .map_err(TokenError::Client)?,
            space as u64,
            &self.program_id,
        )];

        if required_extensions.contains(&ExtensionType::ImmutableOwner) {
            instructions.push(instruction::initialize_immutable_owner(
                &self.program_id,
                &account.pubkey(),
            )?)
        }

        instructions.push(instruction::initialize_account(
            &self.program_id,
            &account.pubkey(),
            &self.pubkey,
            owner,
        )?);

        self.process_ixs(&instructions, &[account]).await
    }

    /// Retrieve a raw account
    pub async fn get_account(&self, account: &Pubkey) -> TokenResult<BaseAccount> {
        self.client
            .get_account(*account)
            .await
            .map_err(TokenError::Client)?
            .ok_or(TokenError::AccountNotFound)
    }

    /// Retrive mint information.
    pub async fn get_mint_info(&self) -> TokenResult<StateWithExtensionsOwned<Mint>> {
        let account = self.get_account(&self.pubkey).await?;
        if account.owner != self.program_id {
            return Err(TokenError::AccountInvalidOwner);
        }

        let mint_result =
            StateWithExtensionsOwned::<Mint>::unpack(account.data).map_err(Into::into);

        if let (Ok(mint), Some(decimals)) = (&mint_result, self.decimals) {
            if decimals != mint.base.decimals {
                return Err(TokenError::InvalidDecimals);
            }
        }

        mint_result
    }

    /// Retrieve account information.
    pub async fn get_account_info(
        &self,
        account: &Pubkey,
    ) -> TokenResult<StateWithExtensionsOwned<Account>> {
        let account = self.get_account(account).await?;
        if account.owner != self.program_id {
            return Err(TokenError::AccountInvalidOwner);
        }
        let account = StateWithExtensionsOwned::<Account>::unpack(account.data)?;
        if account.base.mint != *self.get_address() {
            return Err(TokenError::AccountInvalidMint);
        }

        Ok(account)
    }

    /// Retrieve the associated account or create one if not found.
    pub async fn get_or_create_associated_account_info(
        &self,
        owner: &Pubkey,
    ) -> TokenResult<StateWithExtensionsOwned<Account>> {
        let account = self.get_associated_token_address(owner);
        match self.get_account_info(&account).await {
            Ok(account) => Ok(account),
            // AccountInvalidOwner is possible if account already received some lamports.
            Err(TokenError::AccountNotFound) | Err(TokenError::AccountInvalidOwner) => {
                self.create_associated_token_account(owner).await?;
                self.get_account_info(&account).await
            }
            Err(error) => Err(error),
        }
    }

    /// Assign a new authority to the account.
    pub async fn set_authority<S: Signers>(
        &self,
        account: &Pubkey,
        authority: &Pubkey,
        new_authority: Option<&Pubkey>,
        authority_type: instruction::AuthorityType,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        self.process_ixs(
            &[instruction::set_authority(
                &self.program_id,
                account,
                new_authority,
                authority_type,
                authority,
                &multisig_signers,
            )?],
            signing_keypairs,
        )
        .await
    }

    /// Mint new tokens
    pub async fn mint_to<S: Signers>(
        &self,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        let instructions = if let Some(decimals) = self.decimals {
            [instruction::mint_to_checked(
                &self.program_id,
                &self.pubkey,
                destination,
                authority,
                &multisig_signers,
                amount,
                decimals,
            )?]
        } else {
            [instruction::mint_to(
                &self.program_id,
                &self.pubkey,
                destination,
                authority,
                &multisig_signers,
                amount,
            )?]
        };

        self.process_ixs(&instructions, signing_keypairs).await
    }

    /// Transfer tokens to another account
    #[allow(clippy::too_many_arguments)]
    pub async fn transfer<S: Signers>(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        let instructions = if let Some(decimals) = self.decimals {
            [instruction::transfer_checked(
                &self.program_id,
                source,
                &self.pubkey,
                destination,
                authority,
                &multisig_signers,
                amount,
                decimals,
            )?]
        } else {
            #[allow(deprecated)]
            [instruction::transfer(
                &self.program_id,
                source,
                destination,
                authority,
                &multisig_signers,
                amount,
            )?]
        };

        self.process_ixs(&instructions, signing_keypairs).await
    }

    /// Transfer tokens to an associated account, creating it if it does not exist
    #[allow(clippy::too_many_arguments)]
    pub async fn create_recipient_associated_account_and_transfer<S: Signers>(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        destination_owner: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        fee: Option<u64>,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        if *destination != self.get_associated_token_address(destination_owner) {
            return Err(TokenError::AccountInvalidAssociatedAddress);
        }

        let mut instructions = vec![
            (create_associated_token_account_idempotent(
                &self.payer.pubkey(),
                destination_owner,
                &self.pubkey,
                &self.program_id,
            )),
        ];

        if let Some(fee) = fee {
            let decimals = self.decimals.ok_or(TokenError::MissingDecimals)?;
            instructions.push(transfer_fee::instruction::transfer_checked_with_fee(
                &self.program_id,
                source,
                &self.pubkey,
                destination,
                authority,
                &multisig_signers,
                amount,
                decimals,
                fee,
            )?);
        } else if let Some(decimals) = self.decimals {
            instructions.push(instruction::transfer_checked(
                &self.program_id,
                source,
                &self.pubkey,
                destination,
                authority,
                &multisig_signers,
                amount,
                decimals,
            )?);
        } else {
            #[allow(deprecated)]
            instructions.push(instruction::transfer(
                &self.program_id,
                source,
                destination,
                authority,
                &multisig_signers,
                amount,
            )?);
        }

        self.process_ixs(&instructions, signing_keypairs).await
    }

    /// Transfer tokens to another account, given an expected fee
    #[allow(clippy::too_many_arguments)]
    pub async fn transfer_with_fee<S: Signers>(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        fee: u64,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);
        let decimals = self.decimals.ok_or(TokenError::MissingDecimals)?;

        self.process_ixs(
            &[transfer_fee::instruction::transfer_checked_with_fee(
                &self.program_id,
                source,
                &self.pubkey,
                destination,
                authority,
                &multisig_signers,
                amount,
                decimals,
                fee,
            )?],
            signing_keypairs,
        )
        .await
    }

    /// Burn tokens from account
    pub async fn burn<S: Signers>(
        &self,
        source: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        let instructions = if let Some(decimals) = self.decimals {
            [instruction::burn_checked(
                &self.program_id,
                source,
                &self.pubkey,
                authority,
                &multisig_signers,
                amount,
                decimals,
            )?]
        } else {
            [instruction::burn(
                &self.program_id,
                source,
                &self.pubkey,
                authority,
                &multisig_signers,
                amount,
            )?]
        };

        self.process_ixs(&instructions, signing_keypairs).await
    }

    /// Approve a delegate to spend tokens
    pub async fn approve<S: Signers>(
        &self,
        source: &Pubkey,
        delegate: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        let instructions = if let Some(decimals) = self.decimals {
            [instruction::approve_checked(
                &self.program_id,
                source,
                &self.pubkey,
                delegate,
                authority,
                &multisig_signers,
                amount,
                decimals,
            )?]
        } else {
            [instruction::approve(
                &self.program_id,
                source,
                delegate,
                authority,
                &multisig_signers,
                amount,
            )?]
        };

        self.process_ixs(&instructions, signing_keypairs).await
    }

    /// Revoke a delegate
    pub async fn revoke<S: Signers>(
        &self,
        source: &Pubkey,
        authority: &Pubkey,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        self.process_ixs(
            &[instruction::revoke(
                &self.program_id,
                source,
                authority,
                &multisig_signers,
            )?],
            signing_keypairs,
        )
        .await
    }

    /// Close an empty account and reclaim its lamports
    pub async fn close_account<S: Signers>(
        &self,
        account: &Pubkey,
        lamports_destination: &Pubkey,
        authority: &Pubkey,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        let mut instructions = vec![instruction::close_account(
            &self.program_id,
            account,
            lamports_destination,
            authority,
            &multisig_signers,
        )?];

        if let Ok(Some(destination_account)) = self.client.get_account(*lamports_destination).await
        {
            if let Ok(destination_obj) =
                StateWithExtensionsOwned::<Account>::unpack(destination_account.data)
            {
                if destination_obj.base.is_native() {
                    instructions.push(instruction::sync_native(
                        &self.program_id,
                        lamports_destination,
                    )?);
                }
            }
        }

        self.process_ixs(&instructions, signing_keypairs).await
    }

    /// Close an account, reclaiming its lamports and tokens
    pub async fn empty_and_close_account<S: Signers>(
        &self,
        account_to_close: &Pubkey,
        lamports_destination: &Pubkey,
        tokens_destination: &Pubkey,
        authority: &Pubkey,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        // this implicitly validates that the mint on self is correct
        let account_state = self.get_account_info(account_to_close).await?;

        let mut instructions = vec![];

        if !self.is_native() && account_state.base.amount > 0 {
            // if a separate close authority is being used, it must be a delegate also
            if let Some(decimals) = self.decimals {
                instructions.push(instruction::transfer_checked(
                    &self.program_id,
                    account_to_close,
                    &self.pubkey,
                    tokens_destination,
                    authority,
                    &multisig_signers,
                    account_state.base.amount,
                    decimals,
                )?);
            } else {
                #[allow(deprecated)]
                instructions.push(instruction::transfer(
                    &self.program_id,
                    account_to_close,
                    tokens_destination,
                    authority,
                    &multisig_signers,
                    account_state.base.amount,
                )?);
            }
        }

        instructions.push(instruction::close_account(
            &self.program_id,
            account_to_close,
            lamports_destination,
            authority,
            &multisig_signers,
        )?);

        if let Ok(Some(destination_account)) = self.client.get_account(*lamports_destination).await
        {
            if let Ok(destination_obj) =
                StateWithExtensionsOwned::<Account>::unpack(destination_account.data)
            {
                if destination_obj.base.is_native() {
                    instructions.push(instruction::sync_native(
                        &self.program_id,
                        lamports_destination,
                    )?);
                }
            }
        }

        self.process_ixs(&instructions, signing_keypairs).await
    }

    /// Freeze a token account
    pub async fn freeze<S: Signers>(
        &self,
        account: &Pubkey,
        authority: &Pubkey,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        self.process_ixs(
            &[instruction::freeze_account(
                &self.program_id,
                account,
                &self.pubkey,
                authority,
                &multisig_signers,
            )?],
            signing_keypairs,
        )
        .await
    }

    /// Thaw / unfreeze a token account
    pub async fn thaw<S: Signers>(
        &self,
        account: &Pubkey,
        authority: &Pubkey,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        self.process_ixs(
            &[instruction::thaw_account(
                &self.program_id,
                account,
                &self.pubkey,
                authority,
                &multisig_signers,
            )?],
            signing_keypairs,
        )
        .await
    }

    /// Wrap lamports into native account
    pub async fn wrap<S: Signers>(
        &self,
        account: &Pubkey,
        owner: &Pubkey,
        lamports: u64,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        // mutable owner for Tokenkeg, immutable otherwise
        let immutable_owner = self.program_id != spl_token::id();
        let instructions = self.wrap_ixs(account, owner, lamports, immutable_owner)?;

        self.process_ixs(&instructions, signing_keypairs).await
    }

    /// Wrap lamports into a native account that can always have its ownership changed
    pub async fn wrap_with_mutable_ownership<S: Signers>(
        &self,
        account: &Pubkey,
        owner: &Pubkey,
        lamports: u64,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let instructions = self.wrap_ixs(account, owner, lamports, false)?;

        self.process_ixs(&instructions, signing_keypairs).await
    }

    fn wrap_ixs(
        &self,
        account: &Pubkey,
        owner: &Pubkey,
        lamports: u64,
        immutable_owner: bool,
    ) -> TokenResult<Vec<Instruction>> {
        if !self.is_native() {
            return Err(TokenError::AccountInvalidMint);
        }

        let mut instructions = vec![];
        if *account == self.get_associated_token_address(owner) {
            instructions.push(system_instruction::transfer(owner, account, lamports));
            instructions.push(create_associated_token_account(
                &self.payer.pubkey(),
                owner,
                &self.pubkey,
                &self.program_id,
            ));
        } else {
            let extensions = if immutable_owner {
                vec![ExtensionType::ImmutableOwner]
            } else {
                vec![]
            };
            let space = ExtensionType::get_account_len::<Account>(&extensions);

            instructions.push(system_instruction::create_account(
                &self.payer.pubkey(),
                account,
                lamports,
                space as u64,
                &self.program_id,
            ));

            if immutable_owner {
                instructions.push(instruction::initialize_immutable_owner(
                    &self.program_id,
                    account,
                )?)
            }

            instructions.push(instruction::initialize_account(
                &self.program_id,
                account,
                &self.pubkey,
                owner,
            )?);
        };

        Ok(instructions)
    }

    /// Sync native account lamports
    pub async fn sync_native(&self, account: &Pubkey) -> TokenResult<T::Output> {
        self.process_ixs::<[&dyn Signer; 0]>(
            &[instruction::sync_native(&self.program_id, account)?],
            &[],
        )
        .await
    }

    /// Set transfer fee
    pub async fn set_transfer_fee<S: Signers>(
        &self,
        authority: &Pubkey,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        self.process_ixs(
            &[transfer_fee::instruction::set_transfer_fee(
                &self.program_id,
                &self.pubkey,
                authority,
                &multisig_signers,
                transfer_fee_basis_points,
                maximum_fee,
            )?],
            signing_keypairs,
        )
        .await
    }

    /// Set default account state on mint
    pub async fn set_default_account_state<S: Signers>(
        &self,
        authority: &Pubkey,
        state: &AccountState,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        self.process_ixs(
            &[
                default_account_state::instruction::update_default_account_state(
                    &self.program_id,
                    &self.pubkey,
                    authority,
                    &multisig_signers,
                    state,
                )?,
            ],
            signing_keypairs,
        )
        .await
    }

    /// Harvest withheld tokens to mint
    pub async fn harvest_withheld_tokens_to_mint(
        &self,
        sources: &[&Pubkey],
    ) -> TokenResult<T::Output> {
        self.process_ixs::<[&dyn Signer; 0]>(
            &[transfer_fee::instruction::harvest_withheld_tokens_to_mint(
                &self.program_id,
                &self.pubkey,
                sources,
            )?],
            &[],
        )
        .await
    }

    /// Withdraw withheld tokens from mint
    pub async fn withdraw_withheld_tokens_from_mint<S: Signers>(
        &self,
        destination: &Pubkey,
        authority: &Pubkey,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        self.process_ixs(
            &[
                transfer_fee::instruction::withdraw_withheld_tokens_from_mint(
                    &self.program_id,
                    &self.pubkey,
                    destination,
                    authority,
                    &multisig_signers,
                )?,
            ],
            signing_keypairs,
        )
        .await
    }

    /// Withdraw withheld tokens from accounts
    pub async fn withdraw_withheld_tokens_from_accounts<S: Signers>(
        &self,
        destination: &Pubkey,
        authority: &Pubkey,
        sources: &[&Pubkey],
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        self.process_ixs(
            &[
                transfer_fee::instruction::withdraw_withheld_tokens_from_accounts(
                    &self.program_id,
                    &self.pubkey,
                    destination,
                    authority,
                    &multisig_signers,
                    sources,
                )?,
            ],
            signing_keypairs,
        )
        .await
    }

    /// Reallocate a token account to be large enough for a set of ExtensionTypes
    pub async fn reallocate<S: Signers>(
        &self,
        account: &Pubkey,
        authority: &Pubkey,
        extension_types: &[ExtensionType],
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        self.process_ixs(
            &[instruction::reallocate(
                &self.program_id,
                account,
                &self.payer.pubkey(),
                authority,
                &multisig_signers,
                extension_types,
            )?],
            signing_keypairs,
        )
        .await
    }

    /// Require memos on transfers into this account
    pub async fn enable_required_transfer_memos<S: Signers>(
        &self,
        account: &Pubkey,
        authority: &Pubkey,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        self.process_ixs(
            &[memo_transfer::instruction::enable_required_transfer_memos(
                &self.program_id,
                account,
                authority,
                &multisig_signers,
            )?],
            signing_keypairs,
        )
        .await
    }

    /// Stop requiring memos on transfers into this account
    pub async fn disable_required_transfer_memos<S: Signers>(
        &self,
        account: &Pubkey,
        authority: &Pubkey,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        self.process_ixs(
            &[memo_transfer::instruction::disable_required_transfer_memos(
                &self.program_id,
                account,
                authority,
                &multisig_signers,
            )?],
            signing_keypairs,
        )
        .await
    }

    /// Prevent unsafe usage of token account through CPI
    pub async fn enable_cpi_guard<S: Signers>(
        &self,
        account: &Pubkey,
        authority: &Pubkey,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        self.process_ixs(
            &[cpi_guard::instruction::enable_cpi_guard(
                &self.program_id,
                account,
                authority,
                &multisig_signers,
            )?],
            signing_keypairs,
        )
        .await
    }

    /// Stop preventing unsafe usage of token account through CPI
    pub async fn disable_cpi_guard<S: Signers>(
        &self,
        account: &Pubkey,
        authority: &Pubkey,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        self.process_ixs(
            &[cpi_guard::instruction::disable_cpi_guard(
                &self.program_id,
                account,
                authority,
                &multisig_signers,
            )?],
            signing_keypairs,
        )
        .await
    }

    /// Update interest rate
    pub async fn update_interest_rate<S: Signers>(
        &self,
        authority: &Pubkey,
        new_rate: i16,
        signing_keypairs: &S,
    ) -> TokenResult<T::Output> {
        let signing_pubkeys = signing_keypairs.pubkeys();
        let multisig_signers = self.get_multisig_signers(authority, &signing_pubkeys);

        self.process_ixs(
            &[interest_bearing_mint::instruction::update_rate(
                &self.program_id,
                self.get_address(),
                authority,
                &multisig_signers,
                new_rate,
            )?],
            signing_keypairs,
        )
        .await
    }

    /// Update confidential transfer mint
    pub async fn confidential_transfer_update_mint<S: Signer>(
        &self,
        authority: &S,
        new_authority: Option<&S>,
        auto_approve_new_account: bool,
        auditor_encryption_pubkey: Option<EncryptionPubkey>,
    ) -> TokenResult<T::Output> {
        let mut signers = vec![authority];
        let new_authority_pubkey = if let Some(new_authority) = new_authority {
            signers.push(new_authority);
            Some(new_authority.pubkey())
        } else {
            None
        };

        self.process_ixs(
            &[confidential_transfer::instruction::update_mint(
                &self.program_id,
                &self.pubkey,
                &authority.pubkey(),
                new_authority_pubkey.as_ref(),
                auto_approve_new_account,
                auditor_encryption_pubkey,
            )?],
            &signers,
        )
        .await
    }

    /// Configures confidential transfers for a token account
    pub async fn confidential_transfer_configure_token_account<S: Signer>(
        &self,
        token_account: &Pubkey,
        authority: &S,
    ) -> TokenResult<T::Output> {
        let maximum_pending_balance_credit_counter =
            2 << confidential_transfer::MAXIMUM_DEPOSIT_TRANSFER_AMOUNT_BIT_LENGTH;

        self.confidential_transfer_configure_token_account_with_pending_counter(
            token_account,
            authority,
            maximum_pending_balance_credit_counter,
        )
        .await
    }

    pub async fn confidential_transfer_configure_token_account_with_pending_counter<S: Signer>(
        &self,
        token_account: &Pubkey,
        authority: &S,
        maximum_pending_balance_credit_counter: u64,
    ) -> TokenResult<T::Output> {
        let elgamal_keypair =
            ElGamalKeypair::new(authority, token_account).map_err(TokenError::Key)?;
        let decryptable_zero_balance = AeKey::new(authority, token_account)
            .map_err(TokenError::Key)?
            .encrypt(0);

        self.confidential_transfer_configure_token_account_with_pending_counter_and_keypair(
            token_account,
            authority,
            maximum_pending_balance_credit_counter,
            &elgamal_keypair,
            decryptable_zero_balance,
        )
        .await
    }

    pub async fn confidential_transfer_configure_token_account_with_pending_counter_and_keypair<
        S: Signer,
    >(
        &self,
        token_account: &Pubkey,
        authority: &S,
        maximum_pending_balance_credit_counter: u64,
        elgamal_keypair: &ElGamalKeypair,
        decryptable_zero_balance: AeCiphertext,
    ) -> TokenResult<T::Output> {
        let proof_data =
            confidential_transfer::instruction::PubkeyValidityData::new(elgamal_keypair)
                .map_err(TokenError::Proof)?;

        self.process_ixs(
            &confidential_transfer::instruction::configure_account(
                &self.program_id,
                token_account,
                &self.pubkey,
                decryptable_zero_balance,
                maximum_pending_balance_credit_counter,
                &authority.pubkey(),
                &[],
                &proof_data,
            )?,
            &[authority],
        )
        .await
    }

    /// Approves a token account for confidential transfers
    pub async fn confidential_transfer_approve_account<S: Signer>(
        &self,
        token_account: &Pubkey,
        authority: &S,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[confidential_transfer::instruction::approve_account(
                &self.program_id,
                token_account,
                &self.pubkey,
                &authority.pubkey(),
            )?],
            &[authority],
        )
        .await
    }

    /// Prepare a token account with the confidential transfer extension for closing
    pub async fn confidential_transfer_empty_account<S: Signer>(
        &self,
        token_account: &Pubkey,
        authority: &S,
    ) -> TokenResult<T::Output> {
        let elgamal_keypair =
            ElGamalKeypair::new(authority, token_account).map_err(TokenError::Key)?;
        self.confidential_transfer_empty_account_with_keypair(
            token_account,
            authority,
            &elgamal_keypair,
        )
        .await
    }

    pub async fn confidential_transfer_empty_account_with_keypair<S: Signer>(
        &self,
        token_account: &Pubkey,
        authority: &S,
        elgamal_keypair: &ElGamalKeypair,
    ) -> TokenResult<T::Output> {
        let state = self.get_account_info(token_account).await.unwrap();
        let extension =
            state.get_extension::<confidential_transfer::ConfidentialTransferAccount>()?;

        let proof_data = confidential_transfer::instruction::CloseAccountData::new(
            elgamal_keypair,
            &extension.available_balance.try_into().unwrap(),
        )
        .map_err(TokenError::Proof)?;

        self.process_ixs(
            &confidential_transfer::instruction::empty_account(
                &self.program_id,
                token_account,
                &authority.pubkey(),
                &[],
                &proof_data,
            )?,
            &[authority],
        )
        .await
    }

    /// Fetch and decrypt the available balance of a confidential token account using the uniquely
    /// derived decryption key from a signer
    pub async fn confidential_transfer_get_available_balance<S: Signer>(
        &self,
        token_account: &Pubkey,
        authority: &S,
    ) -> TokenResult<u64> {
        let authenticated_encryption_key =
            AeKey::new(authority, token_account).map_err(TokenError::Key)?;

        self.confidential_transfer_get_available_balance_with_key(
            token_account,
            &authenticated_encryption_key,
        )
        .await
    }

    /// Fetch and decrypt the available balance of a confidential token account using a custom
    /// decryption key
    pub async fn confidential_transfer_get_available_balance_with_key(
        &self,
        token_account: &Pubkey,
        authenticated_encryption_key: &AeKey,
    ) -> TokenResult<u64> {
        let state = self.get_account_info(token_account).await.unwrap();
        let extension =
            state.get_extension::<confidential_transfer::ConfidentialTransferAccount>()?;

        let decryptable_balance_ciphertext: AeCiphertext = extension
            .decryptable_available_balance
            .try_into()
            .map_err(TokenError::Proof)?;
        let decryptable_balance = decryptable_balance_ciphertext
            .decrypt(authenticated_encryption_key)
            .ok_or(TokenError::AccountDecryption)?;

        Ok(decryptable_balance)
    }

    /// Fetch and decrypt the pending balance of a confidential token account using the uniquely
    /// derived decryption key from a signer
    pub async fn confidential_transfer_get_pending_balance<S: Signer>(
        &self,
        token_account: &Pubkey,
        authority: &S,
    ) -> TokenResult<u64> {
        let elgamal_keypair =
            ElGamalKeypair::new(authority, token_account).map_err(TokenError::Key)?;

        self.confidential_transfer_get_pending_balance_with_key(token_account, &elgamal_keypair)
            .await
    }

    /// Fetch and decrypt the pending balance of a confidential token account using a custom
    /// decryption key
    pub async fn confidential_transfer_get_pending_balance_with_key(
        &self,
        token_account: &Pubkey,
        elgamal_keypair: &ElGamalKeypair,
    ) -> TokenResult<u64> {
        let state = self.get_account_info(token_account).await.unwrap();
        let extension =
            state.get_extension::<confidential_transfer::ConfidentialTransferAccount>()?;

        // decrypt pending balance
        let pending_balance_lo = extension
            .pending_balance_lo
            .decrypt(&elgamal_keypair.secret)
            .ok_or(TokenError::AccountDecryption)?;
        let pending_balance_hi = extension
            .pending_balance_hi
            .decrypt(&elgamal_keypair.secret)
            .ok_or(TokenError::AccountDecryption)?;

        let pending_balance = pending_balance_lo
            .checked_add(pending_balance_hi << confidential_transfer::PENDING_BALANCE_HI_BIT_LENGTH)
            .ok_or(TokenError::AccountDecryption)?;

        Ok(pending_balance)
    }

    pub async fn confidential_transfer_get_withheld_amount<S: Signer>(
        &self,
        withdraw_withheld_authority: &S,
        sources: &[&Pubkey],
    ) -> TokenResult<u64> {
        let withdraw_withheld_authority_elgamal_keypair =
            ElGamalKeypair::new(withdraw_withheld_authority, &self.pubkey)
                .map_err(TokenError::Key)?;

        self.confidential_transfer_get_withheld_amount_with_key(
            &withdraw_withheld_authority_elgamal_keypair,
            sources,
        )
        .await
    }

    pub async fn confidential_transfer_get_withheld_amount_with_key(
        &self,
        withdraw_withheld_authority_elgamal_keypair: &ElGamalKeypair,
        sources: &[&Pubkey],
    ) -> TokenResult<u64> {
        let mut aggregate_withheld_amount_ciphertext = ElGamalCiphertext::default();
        for &source in sources {
            let state = self.get_account_info(source).await.unwrap();
            let extension =
                state.get_extension::<confidential_transfer::ConfidentialTransferAccount>()?;

            let withheld_amount_ciphertext: ElGamalCiphertext =
                extension.withheld_amount.try_into().unwrap();

            aggregate_withheld_amount_ciphertext =
                aggregate_withheld_amount_ciphertext + withheld_amount_ciphertext;
        }

        let aggregate_withheld_amount = aggregate_withheld_amount_ciphertext
            .decrypt_u32(&withdraw_withheld_authority_elgamal_keypair.secret)
            .ok_or(TokenError::AccountDecryption)?;

        Ok(aggregate_withheld_amount)
    }

    /// Fetch the ElGamal public key associated with a confidential token account
    pub async fn confidential_transfer_get_encryption_pubkey<S: Signer>(
        &self,
        token_account: &Pubkey,
    ) -> TokenResult<ElGamalPubkey> {
        let state = self.get_account_info(token_account).await.unwrap();
        let extension =
            state.get_extension::<confidential_transfer::ConfidentialTransferAccount>()?;
        let encryption_pubkey = extension
            .encryption_pubkey
            .try_into()
            .map_err(TokenError::Proof)?;

        Ok(encryption_pubkey)
    }

    /// Fetch the ElGamal pubkey key of the auditor associated with a confidential token mint
    pub async fn confidential_transfer_get_auditor_encryption_pubkey<S: Signer>(
        &self,
    ) -> TokenResult<Option<ElGamalPubkey>> {
        let mint_state = self.get_mint_info().await.unwrap();
        let ct_mint =
            mint_state.get_extension::<confidential_transfer::ConfidentialTransferMint>()?;
        let auditor_encryption_pubkey: Option<EncryptionPubkey> =
            ct_mint.auditor_encryption_pubkey.into();

        if let Some(encryption_pubkey) = auditor_encryption_pubkey {
            let encryption_pubkey: ElGamalPubkey =
                encryption_pubkey.try_into().map_err(TokenError::Proof)?;
            Ok(Some(encryption_pubkey))
        } else {
            Ok(None)
        }
    }

    /// Fetch the ElGamal pubkey key of the withdraw withheld authority associated with a
    /// confidential token mint
    pub async fn confidential_transfer_get_withdraw_withheld_authority_encryption_pubkey<
        S: Signer,
    >(
        &self,
    ) -> TokenResult<Option<ElGamalPubkey>> {
        let mint_state = self.get_mint_info().await.unwrap();
        let ct_mint =
            mint_state.get_extension::<confidential_transfer::ConfidentialTransferMint>()?;
        let withdraw_withheld_authority_encryption_pubkey: Option<EncryptionPubkey> =
            ct_mint.withdraw_withheld_authority_encryption_pubkey.into();

        if let Some(encryption_pubkey) = withdraw_withheld_authority_encryption_pubkey {
            let encryption_pubkey: ElGamalPubkey =
                encryption_pubkey.try_into().map_err(TokenError::Proof)?;
            Ok(Some(encryption_pubkey))
        } else {
            Ok(None)
        }
    }

    /// Deposit SPL Tokens into the pending balance of a confidential token account
    pub async fn confidential_transfer_deposit<S: Signer>(
        &self,
        token_account: &Pubkey,
        token_authority: &S,
        amount: u64,
        decimals: u8,
    ) -> TokenResult<T::Output> {
        if amount >> confidential_transfer::MAXIMUM_DEPOSIT_TRANSFER_AMOUNT_BIT_LENGTH != 0 {
            return Err(TokenError::MaximumDepositTransferAmountExceeded);
        }

        self.process_ixs(
            &[confidential_transfer::instruction::deposit(
                &self.program_id,
                token_account,
                &self.pubkey,
                amount,
                decimals,
                &token_authority.pubkey(),
                &[],
            )?],
            &[token_authority],
        )
        .await
    }

    /// Withdraw SPL Tokens from the available balance of a confidential token account using the
    /// uniquely derived decryption key from a signer
    #[allow(clippy::too_many_arguments)]
    pub async fn confidential_transfer_withdraw<S: Signer>(
        &self,
        token_account: &Pubkey,
        token_authority: &S,
        amount: u64,
        available_balance: u64,
        available_balance_ciphertext: &ElGamalCiphertext,
        decimals: u8,
    ) -> TokenResult<T::Output> {
        let elgamal_keypair =
            ElGamalKeypair::new(token_authority, token_account).map_err(TokenError::Key)?;
        let authenticated_encryption_key =
            AeKey::new(token_authority, token_account).map_err(TokenError::Key)?;

        self.confidential_transfer_withdraw_with_key(
            token_account,
            token_authority,
            amount,
            decimals,
            available_balance,
            available_balance_ciphertext,
            &elgamal_keypair,
            &authenticated_encryption_key,
        )
        .await
    }

    /// Withdraw SPL Tokens from the available balance of a confidential token account using custom
    /// keys
    #[allow(clippy::too_many_arguments)]
    pub async fn confidential_transfer_withdraw_with_key<S: Signer>(
        &self,
        token_account: &Pubkey,
        token_authority: &S,
        amount: u64,
        decimals: u8,
        available_balance: u64,
        available_balance_ciphertext: &ElGamalCiphertext,
        elgamal_keypair: &ElGamalKeypair,
        authenticated_encryption_key: &AeKey,
    ) -> TokenResult<T::Output> {
        let proof_data = confidential_transfer::instruction::WithdrawData::new(
            amount,
            elgamal_keypair,
            available_balance,
            available_balance_ciphertext,
        )
        .map_err(TokenError::Proof)?;

        let remaining_balance = available_balance
            .checked_sub(amount)
            .ok_or(TokenError::NotEnoughFunds)?;
        let new_decryptable_available_balance =
            authenticated_encryption_key.encrypt(remaining_balance);

        self.process_ixs(
            &confidential_transfer::instruction::withdraw(
                &self.program_id,
                token_account,
                &self.pubkey,
                amount,
                decimals,
                new_decryptable_available_balance,
                &token_authority.pubkey(),
                &[],
                &proof_data,
            )?,
            &[token_authority],
        )
        .await
    }

    /// Transfer tokens confidentially using the uniquely derived decryption keys from a signer
    #[allow(clippy::too_many_arguments)]
    pub async fn confidential_transfer_transfer<S: Signer>(
        &self,
        source_token_account: &Pubkey,
        destination_token_account: &Pubkey,
        source_token_authority: &S,
        amount: u64,
        source_available_balance: u64,
        source_available_balance_ciphertext: &ElGamalCiphertext,
        destination_elgamal_pubkey: &ElGamalPubkey,
        auditor_elgamal_pubkey: Option<ElGamalPubkey>,
    ) -> TokenResult<T::Output> {
        let source_elgamal_keypair =
            ElGamalKeypair::new(source_token_authority, source_token_account)
                .map_err(TokenError::Key)?;
        let source_authenticated_encryption_key =
            AeKey::new(source_token_authority, source_token_account).map_err(TokenError::Key)?;

        self.confidential_transfer_transfer_with_key(
            source_token_account,
            destination_token_account,
            source_token_authority,
            amount,
            source_available_balance,
            source_available_balance_ciphertext,
            destination_elgamal_pubkey,
            auditor_elgamal_pubkey,
            &source_elgamal_keypair,
            &source_authenticated_encryption_key,
        )
        .await
    }

    /// Transfer tokens confidentially using custom decryption keys
    #[allow(clippy::too_many_arguments)]
    pub async fn confidential_transfer_transfer_with_key<S: Signer>(
        &self,
        source_token_account: &Pubkey,
        destination_token_account: &Pubkey,
        source_token_authority: &S,
        amount: u64,
        source_available_balance: u64,
        source_available_balance_ciphertext: &ElGamalCiphertext,
        destination_elgamal_pubkey: &ElGamalPubkey,
        auditor_elgamal_pubkey: Option<ElGamalPubkey>,
        source_elgamal_keypair: &ElGamalKeypair,
        source_authenticated_encryption_key: &AeKey,
    ) -> TokenResult<T::Output> {
        if amount >> confidential_transfer::MAXIMUM_DEPOSIT_TRANSFER_AMOUNT_BIT_LENGTH != 0 {
            return Err(TokenError::MaximumDepositTransferAmountExceeded);
        }

        let auditor_elgamal_pubkey = auditor_elgamal_pubkey.unwrap_or_default();

        let proof_data = confidential_transfer::instruction::TransferData::new(
            amount,
            (
                source_available_balance,
                source_available_balance_ciphertext,
            ),
            source_elgamal_keypair,
            (destination_elgamal_pubkey, &auditor_elgamal_pubkey),
        )
        .map_err(TokenError::Proof)?;

        let source_remaining_balance = source_available_balance
            .checked_sub(amount)
            .ok_or(TokenError::NotEnoughFunds)?;
        let new_source_available_balance =
            source_authenticated_encryption_key.encrypt(source_remaining_balance);

        self.process_ixs(
            &confidential_transfer::instruction::transfer(
                &self.program_id,
                source_token_account,
                destination_token_account,
                &self.pubkey,
                new_source_available_balance,
                &source_token_authority.pubkey(),
                &[],
                &proof_data,
            )?,
            &[source_token_authority],
        )
        .await
    }

    /// Transfer tokens confidentially with fee using the uniquely derived decryption keys from a
    /// signer
    #[allow(clippy::too_many_arguments)]
    pub async fn confidential_transfer_transfer_with_fee<S: Signer>(
        &self,
        source_token_account: &Pubkey,
        destination_token_account: &Pubkey,
        source_token_authority: &S,
        amount: u64,
        source_available_balance: u64,
        source_available_balance_ciphertext: &ElGamalCiphertext,
        destination_elgamal_pubkey: &ElGamalPubkey,
        auditor_elgamal_pubkey: Option<ElGamalPubkey>,
        withdraw_withheld_authority_elgamal_pubkey: &ElGamalPubkey,
        epoch_info: &EpochInfo,
    ) -> TokenResult<T::Output> {
        let source_elgamal_keypair =
            ElGamalKeypair::new(source_token_authority, source_token_account)
                .map_err(TokenError::Key)?;
        let source_authenticated_encryption_key =
            AeKey::new(source_token_authority, source_token_account).map_err(TokenError::Key)?;

        self.confidential_transfer_transfer_with_fee_with_key(
            source_token_account,
            destination_token_account,
            source_token_authority,
            amount,
            source_available_balance,
            source_available_balance_ciphertext,
            destination_elgamal_pubkey,
            auditor_elgamal_pubkey,
            withdraw_withheld_authority_elgamal_pubkey,
            &source_elgamal_keypair,
            &source_authenticated_encryption_key,
            epoch_info,
        )
        .await
    }

    /// Transfer tokens confidential with fee using custom decryption keys
    #[allow(clippy::too_many_arguments)]
    pub async fn confidential_transfer_transfer_with_fee_with_key<S: Signer>(
        &self,
        source_token_account: &Pubkey,
        destination_token_account: &Pubkey,
        source_token_authority: &S,
        amount: u64,
        source_available_balance: u64,
        source_available_balance_ciphertext: &ElGamalCiphertext,
        destination_elgamal_pubkey: &ElGamalPubkey,
        auditor_elgamal_pubkey: Option<ElGamalPubkey>,
        withdraw_withheld_authority_elgamal_pubkey: &ElGamalPubkey,
        source_elgamal_keypair: &ElGamalKeypair,
        source_authenticated_encryption_key: &AeKey,
        epoch_info: &EpochInfo,
    ) -> TokenResult<T::Output> {
        if amount >> confidential_transfer::MAXIMUM_DEPOSIT_TRANSFER_AMOUNT_BIT_LENGTH != 0 {
            return Err(TokenError::MaximumDepositTransferAmountExceeded);
        }

        let auditor_elgamal_pubkey = auditor_elgamal_pubkey.unwrap_or_default();

        let mint_state = self.get_mint_info().await.unwrap();
        let transfer_fee_config = mint_state
            .get_extension::<transfer_fee::TransferFeeConfig>()
            .unwrap();
        let fee_parameters = transfer_fee_config.get_epoch_fee(epoch_info.epoch);

        let proof_data = confidential_transfer::instruction::TransferWithFeeData::new(
            amount,
            (
                source_available_balance,
                source_available_balance_ciphertext,
            ),
            source_elgamal_keypair,
            (destination_elgamal_pubkey, &auditor_elgamal_pubkey),
            FeeParameters {
                fee_rate_basis_points: u16::from(fee_parameters.transfer_fee_basis_points),
                maximum_fee: u64::from(fee_parameters.maximum_fee),
            },
            withdraw_withheld_authority_elgamal_pubkey,
        )
        .map_err(TokenError::Proof)?;

        let source_remaining_balance = source_available_balance
            .checked_sub(amount)
            .ok_or(TokenError::NotEnoughFunds)?;
        let new_source_decryptable_balance =
            source_authenticated_encryption_key.encrypt(source_remaining_balance);

        self.process_ixs(
            &confidential_transfer::instruction::transfer_with_fee(
                &self.program_id,
                source_token_account,
                destination_token_account,
                &self.pubkey,
                new_source_decryptable_balance,
                &source_token_authority.pubkey(),
                &[],
                &proof_data,
            )?,
            &[source_token_authority],
        )
        .await
    }

    /// Applies the confidential transfer pending balance to the available balance using the
    /// uniquely derived decryption key
    pub async fn confidential_transfer_apply_pending_balance<S: Signer>(
        &self,
        token_account: &Pubkey,
        authority: &S,
        available_balance: u64,
        pending_balance: u64,
        expected_pending_balance_credit_counter: u64,
    ) -> TokenResult<T::Output> {
        let authenticated_encryption_key =
            AeKey::new(authority, token_account).map_err(TokenError::Key)?;

        self.confidential_transfer_apply_pending_balance_with_key(
            token_account,
            authority,
            available_balance,
            pending_balance,
            expected_pending_balance_credit_counter,
            &authenticated_encryption_key,
        )
        .await
    }

    /// Applies the confidential transfer pending balance to the available balance using a custom
    /// decryption key
    pub async fn confidential_transfer_apply_pending_balance_with_key<S: Signer>(
        &self,
        token_account: &Pubkey,
        authority: &S,
        available_balance: u64,
        pending_balance: u64,
        expected_pending_balance_credit_counter: u64,
        authenticated_encryption_key: &AeKey,
    ) -> TokenResult<T::Output> {
        let new_decryptable_balance = available_balance.checked_add(pending_balance).unwrap();
        let new_decryptable_balance_ciphertext =
            authenticated_encryption_key.encrypt(new_decryptable_balance);

        self.process_ixs(
            &[confidential_transfer::instruction::apply_pending_balance(
                &self.program_id,
                token_account,
                expected_pending_balance_credit_counter,
                new_decryptable_balance_ciphertext,
                &authority.pubkey(),
                &[],
            )?],
            &[authority],
        )
        .await
    }

    /// Enable confidential transfer `Deposit` and `Transfer` instructions for a token account
    pub async fn confidential_transfer_enable_confidential_credits<S: Signer>(
        &self,
        token_account: &Pubkey,
        authority: &S,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[
                confidential_transfer::instruction::enable_confidential_credits(
                    &self.program_id,
                    token_account,
                    &authority.pubkey(),
                    &[],
                )?,
            ],
            &[authority],
        )
        .await
    }

    /// Disable confidential transfer `Deposit` and `Transfer` instructions for a token account
    pub async fn confidential_transfer_disable_confidential_credits<S: Signer>(
        &self,
        token_account: &Pubkey,
        authority: &S,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[
                confidential_transfer::instruction::disable_confidential_credits(
                    &self.program_id,
                    token_account,
                    &authority.pubkey(),
                    &[],
                )?,
            ],
            &[authority],
        )
        .await
    }

    /// Enable a confidential extension token account to receive non-confidential payments
    pub async fn confidential_transfer_enable_non_confidential_credits<S: Signer>(
        &self,
        token_account: &Pubkey,
        authority: &S,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[
                confidential_transfer::instruction::enable_non_confidential_credits(
                    &self.program_id,
                    token_account,
                    &authority.pubkey(),
                    &[],
                )?,
            ],
            &[authority],
        )
        .await
    }

    /// Disable non-confidential payments for a confidential extension token account
    pub async fn confidential_transfer_disable_non_confidential_credits<S: Signer>(
        &self,
        token_account: &Pubkey,
        authority: &S,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[
                confidential_transfer::instruction::disable_non_confidential_credits(
                    &self.program_id,
                    token_account,
                    &authority.pubkey(),
                    &[],
                )?,
            ],
            &[authority],
        )
        .await
    }

    /// Withdraw withheld confidential tokens from mint using the uniquely derived decryption key
    pub async fn confidential_transfer_withdraw_withheld_tokens_from_mint<S: Signer>(
        &self,
        withdraw_withheld_authority: &S,
        destination_token_account: &Pubkey,
        destination_elgamal_pubkey: &ElGamalPubkey,
        withheld_amount: u64,
        withheld_amount_ciphertext: &ElGamalCiphertext,
    ) -> TokenResult<T::Output> {
        // derive withheld authority elgamal key
        let withdraw_withheld_authority_elgamal_keypair =
            ElGamalKeypair::new(withdraw_withheld_authority, &self.pubkey)
                .map_err(TokenError::Key)?;

        self.confidential_transfer_withdraw_withheld_tokens_from_mint_with_key(
            withdraw_withheld_authority,
            destination_token_account,
            destination_elgamal_pubkey,
            withheld_amount,
            withheld_amount_ciphertext,
            &withdraw_withheld_authority_elgamal_keypair,
        )
        .await
    }

    /// Withdraw withheld confidential tokens from mint using a custom decryption key
    pub async fn confidential_transfer_withdraw_withheld_tokens_from_mint_with_key<S: Signer>(
        &self,
        withdraw_withheld_authority: &S,
        destination_token_account: &Pubkey,
        destination_elgamal_pubkey: &ElGamalPubkey,
        withheld_amount: u64,
        withheld_amount_ciphertext: &ElGamalCiphertext,
        withdraw_withheld_authority_elgamal_keypair: &ElGamalKeypair,
    ) -> TokenResult<T::Output> {
        let proof_data = confidential_transfer::instruction::WithdrawWithheldTokensData::new(
            withdraw_withheld_authority_elgamal_keypair,
            destination_elgamal_pubkey,
            withheld_amount_ciphertext,
            withheld_amount,
        )
        .map_err(TokenError::Proof)?;

        self.process_ixs(
            &confidential_transfer::instruction::withdraw_withheld_tokens_from_mint(
                &self.program_id,
                &self.pubkey,
                destination_token_account,
                &withdraw_withheld_authority.pubkey(),
                &[],
                &proof_data,
            )?,
            &[withdraw_withheld_authority],
        )
        .await
    }

    /// Withdraw withheld confidential tokens from accounts using the uniquely derived decryption
    /// key
    pub async fn confidential_transfer_withdraw_withheld_tokens_from_accounts<S: Signer>(
        &self,
        withdraw_withheld_authority: &S,
        destination_token_account: &Pubkey,
        destination_elgamal_pubkey: &ElGamalPubkey,
        aggregate_withheld_amount: u64,
        aggregate_withheld_amount_ciphertext: &ElGamalCiphertext,
        sources: &[&Pubkey],
    ) -> TokenResult<T::Output> {
        let withdraw_withheld_authority_elgamal_keypair =
            ElGamalKeypair::new(withdraw_withheld_authority, &self.pubkey)
                .map_err(TokenError::Key)?;

        self.confidential_transfer_withdraw_withheld_tokens_from_accounts_with_key(
            withdraw_withheld_authority,
            destination_token_account,
            destination_elgamal_pubkey,
            aggregate_withheld_amount,
            aggregate_withheld_amount_ciphertext,
            &withdraw_withheld_authority_elgamal_keypair,
            sources,
        )
        .await
    }

    /// Withdraw withheld confidential tokens from accounts using a custom decryption key
    #[allow(clippy::too_many_arguments)]
    pub async fn confidential_transfer_withdraw_withheld_tokens_from_accounts_with_key<
        S: Signer,
    >(
        &self,
        withdraw_withheld_authority: &S,
        destination_token_account: &Pubkey,
        destination_elgamal_pubkey: &ElGamalPubkey,
        aggregate_withheld_amount: u64,
        aggregate_withheld_amount_ciphertext: &ElGamalCiphertext,
        withdraw_withheld_authority_elgamal_keypair: &ElGamalKeypair,
        sources: &[&Pubkey],
    ) -> TokenResult<T::Output> {
        let proof_data = confidential_transfer::instruction::WithdrawWithheldTokensData::new(
            withdraw_withheld_authority_elgamal_keypair,
            destination_elgamal_pubkey,
            aggregate_withheld_amount_ciphertext,
            aggregate_withheld_amount,
        )
        .map_err(TokenError::Proof)?;

        self.process_ixs(
            &confidential_transfer::instruction::withdraw_withheld_tokens_from_accounts(
                &self.program_id,
                &self.pubkey,
                destination_token_account,
                &withdraw_withheld_authority.pubkey(),
                &[],
                sources,
                &proof_data,
            )?,
            &[withdraw_withheld_authority],
        )
        .await
    }

    /// Harvest withheld confidential tokens to mint
    pub async fn confidential_transfer_harvest_withheld_tokens_to_mint(
        &self,
        sources: &[&Pubkey],
    ) -> TokenResult<T::Output> {
        self.process_ixs::<[&dyn Signer; 0]>(
            &[
                confidential_transfer::instruction::harvest_withheld_tokens_to_mint(
                    &self.program_id,
                    &self.pubkey,
                    sources,
                )?,
            ],
            &[],
        )
        .await
    }
}
