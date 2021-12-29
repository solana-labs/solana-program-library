use super::client::{ProgramClient, ProgramClientError, SendTransaction};
use solana_sdk::{
    instruction::Instruction,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    signer::{signers::Signers, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_associated_token_account::{create_associated_token_account, get_associated_token_address};
use spl_token::{instruction, state};
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

pub type TokenResult<T> = Result<T, TokenError>;

pub struct Token<T, S> {
    client: Arc<dyn ProgramClient<T>>,
    pubkey: Pubkey,
    payer: S,
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
    pub fn new(client: Arc<dyn ProgramClient<T>>, address: Pubkey, payer: S) -> Self {
        Token {
            client,
            pubkey: address,
            payer,
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
        }
    }

    async fn process_ixs<S2: Signers>(
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
    pub async fn create_mint<'a, S2: Signer>(
        client: Arc<dyn ProgramClient<T>>,
        payer: S,
        mint_account: &'a S2,
        mint_authority: &'a Pubkey,
        freeze_authority: Option<&'a Pubkey>,
        decimals: u8,
    ) -> TokenResult<Self> {
        let token = Self::new(client, mint_account.pubkey(), payer);
        token
            .process_ixs(
                &[
                    system_instruction::create_account(
                        &token.payer.pubkey(),
                        &mint_account.pubkey(),
                        token
                            .client
                            .get_minimum_balance_for_rent_exemption(state::Mint::LEN)
                            .await
                            .map_err(TokenError::Client)?,
                        state::Mint::LEN as u64,
                        &spl_token::id(),
                    ),
                    instruction::initialize_mint(
                        &spl_token::id(),
                        &mint_account.pubkey(),
                        mint_authority,
                        freeze_authority,
                        decimals,
                    )?,
                ],
                &[mint_account],
            )
            .await?;

        Ok(token)
    }

    /// Get the address for the associated token account.
    pub fn get_associated_token_address(&self, owner: &Pubkey) -> Pubkey {
        get_associated_token_address(owner, &self.pubkey)
    }

    /// Create and initialize the associated account.
    pub async fn create_associated_token_account(&self, owner: &Pubkey) -> TokenResult<Pubkey> {
        self.process_ixs(
            &[create_associated_token_account(
                &self.payer.pubkey(),
                owner,
                &self.pubkey,
            )],
            &[&self.payer],
        )
        .await
        .map(|_| self.get_associated_token_address(owner))
        .map_err(Into::into)
    }

    /// Retrive mint information.
    pub async fn get_mint_info(&self) -> TokenResult<state::Mint> {
        let account = self
            .client
            .get_account(self.pubkey)
            .await
            .map_err(TokenError::Client)?
            .ok_or(TokenError::AccountNotFound)?;
        if account.owner != spl_token::id() {
            return Err(TokenError::AccountInvalidOwner);
        }

        state::Mint::unpack_from_slice(&account.data).map_err(Into::into)
    }

    /// Retrieve account information.
    pub async fn get_account_info(&self, account: Pubkey) -> TokenResult<state::Account> {
        let account = self
            .client
            .get_account(account)
            .await
            .map_err(TokenError::Client)?
            .ok_or(TokenError::AccountNotFound)?;
        if account.owner != spl_token::id() {
            return Err(TokenError::AccountInvalidOwner);
        }

        let account = state::Account::unpack_from_slice(&account.data)?;
        if account.mint != *self.get_address() {
            return Err(TokenError::AccountInvalidMint);
        }

        Ok(account)
    }

    /// Retrieve the associated account or create one if not found.
    pub async fn get_or_create_associated_account_info(
        &self,
        owner: &Pubkey,
    ) -> TokenResult<state::Account> {
        let account = self.get_associated_token_address(owner);
        match self.get_account_info(account).await {
            Ok(account) => Ok(account),
            // AccountInvalidOwner is possible if account already received some lamports.
            Err(TokenError::AccountNotFound) | Err(TokenError::AccountInvalidOwner) => {
                self.create_associated_token_account(owner).await?;
                self.get_account_info(account).await
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
                &spl_token::id(),
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
                &spl_token::id(),
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
    pub async fn transfer<S2: Signer>(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &S2,
        amount: u64,
    ) -> TokenResult<T::Output> {
        self.process_ixs(
            &[instruction::transfer(
                &spl_token::id(),
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
}
