use super::client::{SendTransaction, TokenClient, TokenClientError};
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
    Client(TokenClientError),
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

pub struct Token<ST, TS> {
    client: Arc<dyn TokenClient<ST>>,
    pubkey: Pubkey,
    payer: TS,
}

impl<ST, TS> fmt::Debug for Token<ST, TS>
where
    TS: Signer,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Token")
            .field("pubkey", &self.pubkey)
            .field("payer", &self.payer.pubkey())
            .finish()
    }
}

impl<ST, TS> Token<ST, TS>
where
    ST: SendTransaction,
    TS: Signer,
{
    async fn process_ixs<T: Signers>(
        client: &Arc<dyn TokenClient<ST>>,
        payer: &TS,
        instructions: &[Instruction],
        signing_keypairs: &T,
    ) -> TokenResult<ST::Output> {
        let recent_blockhash = client
            .get_recent_blockhash()
            .await
            .map_err(TokenError::Client)?;

        let mut tx = Transaction::new_with_payer(instructions, Some(&payer.pubkey()));
        tx.try_partial_sign(&[payer], recent_blockhash)
            .map_err(|error| TokenError::Client(error.into()))?;
        tx.try_sign(signing_keypairs, recent_blockhash)
            .map_err(|error| TokenError::Client(error.into()))?;

        client
            .send_transaction(tx)
            .await
            .map_err(TokenError::Client)
    }

    pub fn new(client: Arc<dyn TokenClient<ST>>, address: Pubkey, payer: TS) -> Self {
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

    /// Create and initialize a token.
    pub async fn create_mint<'a, S: Signer>(
        client: Arc<dyn TokenClient<ST>>,
        payer: TS,
        mint_account: &'a S,
        mint_authority: &'a Pubkey,
        freeze_authority: Option<&'a Pubkey>,
        decimals: u8,
    ) -> TokenResult<Self> {
        Self::process_ixs(
            &client,
            &payer,
            &[
                system_instruction::create_account(
                    &payer.pubkey(),
                    &mint_account.pubkey(),
                    client
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

        Ok(Self::new(client, mint_account.pubkey(), payer))
    }

    /// Get the address for the associated token account.
    pub fn get_associated_token_address(&self, owner: &Pubkey) -> Pubkey {
        get_associated_token_address(owner, &self.pubkey)
    }

    /// Create and initialize the associated account.
    pub async fn create_associated_token_account(&self, owner: &Pubkey) -> TokenResult<Pubkey> {
        Self::process_ixs(
            &self.client,
            &self.payer,
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
    pub async fn set_authority<S: Signer>(
        &self,
        account: &Pubkey,
        new_authority: Option<&Pubkey>,
        authority_type: instruction::AuthorityType,
        owner: &S,
    ) -> TokenResult<()> {
        Self::process_ixs(
            &self.client,
            &self.payer,
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
    pub async fn mint_to<S: Signer>(
        &self,
        dest: &Pubkey,
        authority: &S,
        amount: u64,
    ) -> TokenResult<()> {
        Self::process_ixs(
            &self.client,
            &self.payer,
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
    pub async fn transfer<S: Signer>(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &S,
        amount: u64,
    ) -> TokenResult<ST::Output> {
        Self::process_ixs(
            &self.client,
            &self.payer,
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
