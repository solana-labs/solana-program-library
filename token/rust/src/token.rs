use super::client::{TokenClient, TokenClientError};
use solana_sdk::{
    instruction::Instruction,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    signer::{keypair::Keypair, signers::Signers, Signer},
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
}

pub type TokenResult<T> = Result<T, TokenError>;

pub struct Token<'a> {
    client: Arc<Box<dyn TokenClient>>,
    pubkey: Pubkey,
    payer: &'a Keypair,
}

impl fmt::Debug for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Token")
            .field("pubkey", &self.pubkey)
            .field("payer", &self.payer.pubkey())
            .finish()
    }
}

impl<'a> Token<'a> {
    async fn process_ixs<T: Signers>(
        client: &Arc<Box<dyn TokenClient>>,
        payer: &Keypair,
        instructions: &[Instruction],
        signing_keypairs: &T,
    ) -> TokenResult<()> {
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
            .send_transaction(&tx)
            .await
            .map_err(TokenError::Client)
    }

    /// Get token address.
    pub fn get_address(&self) -> &Pubkey {
        &self.pubkey
    }

    /// Create and initialize a token.
    pub async fn create_mint(
        client: Arc<Box<dyn TokenClient>>,
        payer: &'a Keypair,
        mint_account: &'a Keypair,
        mint_authority: &'a Pubkey,
        freeze_authority: Option<&'a Pubkey>,
        decimals: u8,
    ) -> TokenResult<Token<'a>> {
        Self::process_ixs(
            &client,
            payer,
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

        Ok(Token {
            client,
            pubkey: mint_account.pubkey(),
            payer,
        })
    }

    /// Get the address for the associated token account.
    pub fn get_associated_token_address(&self, owner: &Pubkey) -> Pubkey {
        get_associated_token_address(owner, &self.pubkey)
    }

    /// Create and initialize the associated account.
    pub async fn create_associated_token_account(&self, owner: &Pubkey) -> TokenResult<()> {
        Self::process_ixs(
            &self.client,
            self.payer,
            &[create_associated_token_account(
                &self.payer.pubkey(),
                owner,
                &self.pubkey,
            )],
            &[self.payer],
        )
        .await
        .map_err(Into::into)
    }

    /// Mint new tokens
    pub async fn mint_to(
        &self,
        mint: &Pubkey,
        account: &Pubkey,
        owner: &Pubkey,
        signer_pubkeys: &[&Pubkey],
        amount: u64
    ) -> TokenResult<()> {
        Self::process_ixs(
            &self.client,
            self.payer,
            &[instruction::mint_to(
                &spl_token::id(),
                mint,
                account,
                owner,
                signer_pubkeys,
                amount,
            )?],
            &([] as [&Keypair; 0])
        )
        .await
    }

    /// Transfer tokens to another account
    pub async fn transfer(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        signer_pubkeys: &[&Pubkey],
        amount: u64,
    ) -> TokenResult<()> {
        Self::process_ixs(
            &self.client,
            self.payer,
            &[instruction::transfer(
                &spl_token::id(),
                source,
                destination,
                authority,
                signer_pubkeys,
                amount,
            )?],
            &([] as [&Keypair; 0])
        )
        .await
    }
}
