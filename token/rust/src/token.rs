use crate::client::{ProgramClient, ProgramClientError, SendTransaction};
use solana_sdk::{
    account::Account as BaseAccount,
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
    extension::{default_account_state, transfer_fee, ExtensionType, StateWithExtensionsOwned},
    instruction,
    state::{Account, AccountState, Mint},
};
use std::{fmt, sync::Arc};
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
}

impl<T, S> fmt::Debug for Token<T, S>
where
    S: Signer,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Token")
            .field("pubkey", &self.pubkey)
            .field("payer", &self.payer.pubkey())
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
        }
    }

    pub async fn process_ixs<S2: Signers>(
        &self,
        instructions: &[Instruction],
        signing_keypairs: &S2,
    ) -> TokenResult<T::Output> {
        let recent_blockhash = self
            .client
            .get_latest_blockhash()
            .await
            .map_err(TokenError::Client)?;

        let mut tx = Transaction::new_with_payer(instructions, Some(&self.payer.pubkey()));
        tx.try_partial_sign(&[&self.payer], recent_blockhash)
            .map_err(|error| TokenError::Client(error.into()))?;
        tx.try_sign(signing_keypairs, recent_blockhash)
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

    /// Get the address for the associated token account.
    pub fn get_associated_token_address(&self, owner: &Pubkey) -> Pubkey {
        get_associated_token_address_with_program_id(owner, &self.pubkey, &self.program_id)
    }

    /// Create and initialize the associated account.
    pub async fn create_associated_token_account(&self, owner: &Pubkey) -> TokenResult<Pubkey> {
        self.process_ixs(
            &[create_associated_token_account(
                &self.payer.pubkey(),
                owner,
                &self.pubkey,
                &self.program_id,
            )],
            &[&self.payer],
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
        let state = self.get_mint_info().await?;
        let mint_extensions: Vec<ExtensionType> = state.get_extension_types()?;
        let extensions = ExtensionType::get_required_init_account_extensions(&mint_extensions);
        let space = ExtensionType::get_account_len::<Account>(&extensions);
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
            &[&self.payer, account],
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
}
