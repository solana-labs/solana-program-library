use crate::client::{ProgramClient, ProgramClientError, SendTransaction};
use solana_program_test::tokio::time;
use solana_sdk::{
    account::Account as BaseAccount,
    hash::Hash,
    instruction::Instruction,
    program_error::ProgramError,
    pubkey::Pubkey,
    signer::{signers::Signers, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};
use spl_token_2022::{
    extension::{
        default_account_state, memo_transfer, transfer_fee, ExtensionType, StateWithExtensionsOwned,
    },
    instruction, native_mint,
    state::{Account, AccountState, Mint},
};
use std::{
    fmt, io,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use thiserror::Error;

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
}
impl PartialEq for TokenError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // TODO not great, but workable for tests
            (Self::Client(ref a), Self::Client(ref b)) => a.to_string() == b.to_string(),
            (Self::Program(ref a), Self::Program(ref b)) => a == b,
            (Self::AccountNotFound, Self::AccountNotFound) => true,
            (Self::AccountInvalidOwner, Self::AccountInvalidOwner) => true,
            (Self::AccountInvalidMint, Self::AccountInvalidMint) => true,
            _ => false,
        }
    }
}

/// Encapsulates initializing an extension
#[derive(Clone, Debug, PartialEq)]
pub enum ExtensionInitializationParams {
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
}
impl ExtensionInitializationParams {
    /// Get the extension type associated with the init params
    pub fn extension(&self) -> ExtensionType {
        match self {
            Self::DefaultAccountState { .. } => ExtensionType::DefaultAccountState,
            Self::MintCloseAuthority { .. } => ExtensionType::MintCloseAuthority,
            Self::TransferFeeConfig { .. } => ExtensionType::TransferFeeConfig,
        }
    }
    /// Generate an appropriate initialization instruction for the given mint
    pub fn instruction(
        self,
        token_program_id: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Instruction, ProgramError> {
        match self {
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
        }
    }
}

pub type TokenResult<T> = Result<T, TokenError>;

pub struct Token<T, S> {
    client: Arc<dyn ProgramClient<T>>,
    pubkey: Pubkey,
    payer: S,
    program_id: Pubkey,
    memo: Arc<RwLock<Option<String>>>,
}

impl<T, S> fmt::Debug for Token<T, S>
where
    S: Signer,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Token")
            .field("pubkey", &self.pubkey)
            .field("payer", &self.payer.pubkey())
            .field("memo", &self.memo.read().unwrap())
            .finish()
    }
}

impl<T, S> Token<T, S>
where
    T: SendTransaction,
    S: Signer,
{
    pub fn new(
        client: Arc<dyn ProgramClient<T>>,
        program_id: &Pubkey,
        address: &Pubkey,
        payer: S,
    ) -> Self {
        Token {
            client,
            pubkey: *address,
            payer,
            program_id: *program_id,
            memo: Arc::new(RwLock::new(None)),
        }
    }

    /// Get token address.
    pub fn get_address(&self) -> &Pubkey {
        &self.pubkey
    }

    pub fn with_payer<S2: Signer>(&self, payer: S2) -> Token<T, S2> {
        Token {
            client: Arc::clone(&self.client),
            pubkey: self.pubkey,
            payer,
            program_id: self.program_id,
            memo: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_memo<M: AsRef<str>>(&self, memo: M) -> &Self {
        let mut w_memo = self.memo.write().unwrap();
        *w_memo = Some(memo.as_ref().to_string());
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

    pub async fn process_ixs<S2: Signers>(
        &self,
        token_instructions: &[Instruction],
        signing_keypairs: &S2,
    ) -> TokenResult<T::Output> {
        let mut instructions = vec![];
        let mut w_memo = self.memo.write().unwrap();
        if let Some(memo) = w_memo.take() {
            instructions.push(spl_memo::build_memo(memo.as_bytes(), &[]));
        }
        instructions.extend_from_slice(token_instructions);
        let latest_blockhash = self
            .client
            .get_latest_blockhash()
            .await
            .map_err(TokenError::Client)?;

        let mut tx = Transaction::new_with_payer(&instructions, Some(&self.payer.pubkey()));
        tx.try_partial_sign(&[&self.payer], latest_blockhash)
            .map_err(|error| TokenError::Client(error.into()))?;
        tx.try_sign(signing_keypairs, latest_blockhash)
            .map_err(|error| TokenError::Client(error.into()))?;

        self.client
            .send_transaction(&tx)
            .await
            .map_err(TokenError::Client)
    }

    /// Create and initialize a token.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_mint<'a, S2: Signer>(
        client: Arc<dyn ProgramClient<T>>,
        program_id: &'a Pubkey,
        payer: S,
        mint_account: &'a S2,
        mint_authority: &'a Pubkey,
        freeze_authority: Option<&'a Pubkey>,
        decimals: u8,
        extension_initialization_params: Vec<ExtensionInitializationParams>,
    ) -> TokenResult<Self> {
        let mint_pubkey = mint_account.pubkey();
        let extension_types = extension_initialization_params
            .iter()
            .map(|e| e.extension())
            .collect::<Vec<_>>();
        let space = ExtensionType::get_account_len::<Mint>(&extension_types);
        let token = Self::new(client, program_id, &mint_account.pubkey(), payer);
        let mut instructions = vec![system_instruction::create_account(
            &token.payer.pubkey(),
            &mint_pubkey,
            token
                .client
                .get_minimum_balance_for_rent_exemption(space)
                .await
                .map_err(TokenError::Client)?,
            space as u64,
            program_id,
        )];
        for params in extension_initialization_params {
            instructions.push(params.instruction(program_id, &mint_pubkey)?);
        }
        instructions.push(instruction::initialize_mint(
            program_id,
            &mint_pubkey,
            mint_authority,
            freeze_authority,
            decimals,
        )?);
        token.process_ixs(&instructions, &[mint_account]).await?;

        Ok(token)
    }

    /// Create native mint
    pub async fn create_native_mint(
        client: Arc<dyn ProgramClient<T>>,
        program_id: &Pubkey,
        payer: S,
    ) -> TokenResult<Self> {
        let token = Self::new(client, program_id, &native_mint::id(), payer);
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

    /// Get the address for the associated token account.
    pub fn get_associated_token_address(&self, owner: &Pubkey) -> Pubkey {
        get_associated_token_address_with_program_id(owner, &self.pubkey, &self.program_id)
    }

    /// Create and initialize the associated account.
    pub async fn create_associated_token_account(&self, owner: &Pubkey) -> TokenResult<Pubkey> {
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
        .map(|_| self.get_associated_token_address(owner))
        .map_err(Into::into)
    }

    /// Create and initialize a new token account.
    pub async fn create_auxiliary_token_account(
        &self,
        account: &S,
        owner: &Pubkey,
    ) -> TokenResult<Pubkey> {
        self.create_auxiliary_token_account_with_extension_space(account, owner, vec![])
            .await
    }

    /// Create and initialize a new token account.
    pub async fn create_auxiliary_token_account_with_extension_space(
        &self,
        account: &S,
        owner: &Pubkey,
        extensions: Vec<ExtensionType>,
    ) -> TokenResult<Pubkey> {
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
        self.process_ixs(
            &[
                system_instruction::create_account(
                    &self.payer.pubkey(),
                    &account.pubkey(),
                    self.client
                        .get_minimum_balance_for_rent_exemption(space)
                        .await
                        .map_err(TokenError::Client)?,
                    space as u64,
                    &self.program_id,
                ),
                instruction::initialize_account(
                    &self.program_id,
                    &account.pubkey(),
                    &self.pubkey,
                    owner,
                )?,
            ],
            &[account],
        )
        .await
        .map(|_| account.pubkey())
        .map_err(Into::into)
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

        StateWithExtensionsOwned::<Mint>::unpack(account.data).map_err(Into::into)
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
    pub async fn set_authority<S2: Signer>(
        &self,
        account: &Pubkey,
        new_authority: Option<&Pubkey>,
        authority_type: instruction::AuthorityType,
        owner: &S2,
    ) -> TokenResult<()> {
        self.process_ixs(
            &[instruction::set_authority(
                &self.program_id,
                account,
                new_authority,
                authority_type,
                &owner.pubkey(),
                &[],
            )?],
            &[owner],
        )
        .await
        .map(|_| ())
    }

    /// Mint new tokens
    pub async fn mint_to<S2: Signer>(
        &self,
        dest: &Pubkey,
        authority: &S2,
        amount: u64,
    ) -> TokenResult<()> {
        self.process_ixs(
            &[instruction::mint_to(
                &self.program_id,
                &self.pubkey,
                dest,
                &authority.pubkey(),
                &[],
                amount,
            )?],
            &[authority],
        )
        .await
        .map(|_| ())
    }

    /// Transfer tokens to another account
    pub async fn transfer_unchecked<S2: Signer>(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &S2,
        amount: u64,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            #[allow(deprecated)]
            &[instruction::transfer(
                &self.program_id,
                source,
                destination,
                &authority.pubkey(),
                &[],
                amount,
            )?],
            &[authority],
        )
        .await
    }

    /// Transfer tokens to another account
    pub async fn transfer_checked<S2: Signer>(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &S2,
        amount: u64,
        decimals: u8,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[instruction::transfer_checked(
                &self.program_id,
                source,
                &self.pubkey,
                destination,
                &authority.pubkey(),
                &[],
                amount,
                decimals,
            )?],
            &[authority],
        )
        .await
    }

    /// Transfer tokens to another account, given an expected fee
    pub async fn transfer_checked_with_fee<S2: Signer>(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &S2,
        amount: u64,
        decimals: u8,
        fee: u64,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[transfer_fee::instruction::transfer_checked_with_fee(
                &self.program_id,
                source,
                &self.pubkey,
                destination,
                &authority.pubkey(),
                &[],
                amount,
                decimals,
                fee,
            )?],
            &[authority],
        )
        .await
    }

    /// Burn tokens from account
    pub async fn burn<S2: Signer>(
        &self,
        source: &Pubkey,
        authority: &S2,
        amount: u64,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[instruction::burn(
                &self.program_id,
                source,
                &self.pubkey,
                &authority.pubkey(),
                &[],
                amount,
            )?],
            &[authority],
        )
        .await
    }

    /// Burn tokens from account
    pub async fn burn_checked<S2: Signer>(
        &self,
        source: &Pubkey,
        authority: &S2,
        amount: u64,
        decimals: u8,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[instruction::burn_checked(
                &self.program_id,
                source,
                &self.pubkey,
                &authority.pubkey(),
                &[],
                amount,
                decimals,
            )?],
            &[authority],
        )
        .await
    }

    /// Approve a delegate to spend tokens
    pub async fn approve<S2: Signer>(
        &self,
        source: &Pubkey,
        delegate: &Pubkey,
        authority: &S2,
        amount: u64,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[instruction::approve(
                &self.program_id,
                source,
                delegate,
                &authority.pubkey(),
                &[],
                amount,
            )?],
            &[authority],
        )
        .await
    }

    /// Approve a delegate to spend tokens, with decimal check
    pub async fn approve_checked<S2: Signer>(
        &self,
        source: &Pubkey,
        delegate: &Pubkey,
        authority: &S2,
        amount: u64,
        decimals: u8,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[instruction::approve_checked(
                &self.program_id,
                source,
                &self.pubkey,
                delegate,
                &authority.pubkey(),
                &[],
                amount,
                decimals,
            )?],
            &[authority],
        )
        .await
    }

    /// Revoke a delegate
    pub async fn revoke<S2: Signer>(
        &self,
        source: &Pubkey,
        authority: &S2,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[instruction::revoke(
                &self.program_id,
                source,
                &authority.pubkey(),
                &[],
            )?],
            &[authority],
        )
        .await
    }

    /// Close account into another
    pub async fn close_account<S2: Signer>(
        &self,
        account: &Pubkey,
        destination: &Pubkey,
        authority: &S2,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[instruction::close_account(
                &self.program_id,
                account,
                destination,
                &authority.pubkey(),
                &[],
            )?],
            &[authority],
        )
        .await
    }

    /// Freeze a token account
    pub async fn freeze_account<S2: Signer>(
        &self,
        account: &Pubkey,
        authority: &S2,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[instruction::freeze_account(
                &self.program_id,
                account,
                &self.pubkey,
                &authority.pubkey(),
                &[],
            )?],
            &[authority],
        )
        .await
    }

    /// Thaw / unfreeze a token account
    pub async fn thaw_account<S2: Signer>(
        &self,
        account: &Pubkey,
        authority: &S2,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[instruction::thaw_account(
                &self.program_id,
                account,
                &self.pubkey,
                &authority.pubkey(),
                &[],
            )?],
            &[authority],
        )
        .await
    }

    /// Set transfer fee
    pub async fn set_transfer_fee<S2: Signer>(
        &self,
        authority: &S2,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[transfer_fee::instruction::set_transfer_fee(
                &self.program_id,
                &self.pubkey,
                &authority.pubkey(),
                &[],
                transfer_fee_basis_points,
                maximum_fee,
            )?],
            &[authority],
        )
        .await
    }

    /// Set default account state on mint
    pub async fn set_default_account_state<S2: Signer>(
        &self,
        authority: &S2,
        state: &AccountState,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[
                default_account_state::instruction::update_default_account_state(
                    &self.program_id,
                    &self.pubkey,
                    &authority.pubkey(),
                    &[],
                    state,
                )?,
            ],
            &[authority],
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
    pub async fn withdraw_withheld_tokens_from_mint<S2: Signer>(
        &self,
        destination: &Pubkey,
        authority: &S2,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[
                transfer_fee::instruction::withdraw_withheld_tokens_from_mint(
                    &self.program_id,
                    &self.pubkey,
                    destination,
                    &authority.pubkey(),
                    &[],
                )?,
            ],
            &[authority],
        )
        .await
    }

    /// Withdraw withheld tokens from accounts
    pub async fn withdraw_withheld_tokens_from_accounts<S2: Signer>(
        &self,
        destination: &Pubkey,
        authority: &S2,
        sources: &[&Pubkey],
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[
                transfer_fee::instruction::withdraw_withheld_tokens_from_accounts(
                    &self.program_id,
                    &self.pubkey,
                    destination,
                    &authority.pubkey(),
                    &[],
                    sources,
                )?,
            ],
            &[authority],
        )
        .await
    }

    /// Reallocate a token account to be large enough for a set of ExtensionTypes
    pub async fn reallocate<S2: Signer>(
        &self,
        account: &Pubkey,
        authority: &S2,
        extension_types: &[ExtensionType],
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[instruction::reallocate(
                &self.program_id,
                account,
                &self.payer.pubkey(),
                &authority.pubkey(),
                &[],
                extension_types,
            )?],
            &[authority],
        )
        .await
    }

    /// Require memos on transfers into this account
    pub async fn enable_required_transfer_memos<S2: Signer>(
        &self,
        account: &Pubkey,
        authority: &S2,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[memo_transfer::instruction::enable_required_transfer_memos(
                &self.program_id,
                account,
                &authority.pubkey(),
                &[],
            )?],
            &[authority],
        )
        .await
    }

    /// Stop requiring memos on transfers into this account
    pub async fn disable_required_transfer_memos<S2: Signer>(
        &self,
        account: &Pubkey,
        authority: &S2,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[memo_transfer::instruction::disable_required_transfer_memos(
                &self.program_id,
                account,
                &authority.pubkey(),
                &[],
            )?],
            &[authority],
        )
        .await
    }
}
