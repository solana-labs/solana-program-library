use std::{convert::TryInto, mem::size_of};

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::invoke_signed,
    program_error::{PrintProgramError, ProgramError},
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
};

/// Creates an 'InitTimelockProgram' instruction.
pub fn release_escrow_instruction(
    program_id: Pubkey,
    authority: Pubkey,
    source: Pubkey,
    destination: Pubkey,
    token_program: Pubkey,
    amount: u64,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(source, false),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(authority, false),
            AccountMeta::new_readonly(token_program, false),
        ],
        data: HelloWorldInstruction::ReleaseEscrow { amount }.pack(),
    }
}

use num_derive::FromPrimitive;
use thiserror::Error;

/// Errors that may be returned by the Timelock program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum HelloWorldError {
    /// Invalid instruction data passed in.
    #[error("Failed to unpack instruction data")]
    InstructionUnpackError,

    /// Invalid authority
    #[error("Invalid authority")]
    InvalidAuthority,

    /// TokenTransferFailed
    #[error("TokenTransferFailed")]
    TokenTransferFailed,
}

impl PrintProgramError for HelloWorldError {
    fn print<E>(&self) {
        msg!(&self.to_string());
    }
}

impl From<HelloWorldError> for ProgramError {
    fn from(e: HelloWorldError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for HelloWorldError {
    fn type_of() -> &'static str {
        "HelloWorldEscrow Error"
    }
}

/// Used for telling caller what type of format you want back
#[derive(Clone, PartialEq)]
pub enum Format {
    /// JSON format
    JSON,
    /// MsgPack format
    MsgPack,
}
impl Default for Format {
    fn default() -> Self {
        Format::JSON
    }
}

/// Instructions supported by the Timelock program.
#[derive(Clone)]
pub enum HelloWorldInstruction {
    /// Release all tokens in an escrow to the destination account.
    ReleaseEscrow {
        ///Amount
        amount: u64,
    },

    /// Seed Escrow
    SeedEscrow {
        ///Amount
        amount: u64,
    },
}

impl HelloWorldInstruction {
    /// Unpacks a byte buffer into a [TimelockInstruction](enum.TimelockInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input
            .split_first()
            .ok_or(HelloWorldError::InstructionUnpackError)?;
        Ok(match tag {
            0 => {
                let (amount, rest) = Self::unpack_u64(rest)?;
                Self::ReleaseEscrow { amount }
            }
            1 => {
                let (amount, rest) = Self::unpack_u64(rest)?;
                Self::SeedEscrow { amount }
            }
            _ => return Err(HelloWorldError::InstructionUnpackError.into()),
        })
    }

    fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
        if input.len() >= 8 {
            let (amount, rest) = input.split_at(8);
            let amount = amount
                .get(..8)
                .and_then(|slice| slice.try_into().ok())
                .map(u64::from_le_bytes)
                .ok_or(HelloWorldError::InstructionUnpackError)?;
            Ok((amount, rest))
        } else {
            Err(HelloWorldError::InstructionUnpackError.into())
        }
    }

    /// Packs a [TimelockInstruction](enum.TimelockInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());

        match self {
            Self::ReleaseEscrow { amount } => {
                buf.push(0);
                buf.extend_from_slice(&amount.to_le_bytes());
            }

            Self::SeedEscrow { amount } => {
                buf.push(1);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
        }
        buf
    }
}

///Release escrow
pub fn release_escrow(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let source = next_account_info(account_info_iter)?;
    let destination = next_account_info(account_info_iter)?;
    let authority = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    let (authority_key, bump_seed) =
        Pubkey::find_program_address(&[source.key.as_ref()], program_id);
    if authority.key != &authority_key {
        return Err(HelloWorldError::InvalidAuthority.into());
    }
    let authority_signer_seeds = &[source.key.as_ref(), &[bump_seed]];

    spl_token_transfer(TokenTransferParams {
        source: source.clone(),
        destination: destination.clone(),
        amount: 0,
        authority: authority.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program.clone(),
    })?;
    Ok(())
}

/// Seed escrow
pub fn seed_escrow(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let transfer_authority = next_account_info(account_info_iter)?;
    let source = next_account_info(account_info_iter)?;
    let destination = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    spl_token_transfer(TokenTransferParams {
        source: source.clone(),
        destination: destination.clone(),
        amount: amount,
        authority: transfer_authority.clone(),
        authority_signer_seeds: &[],
        token_program: token_program.clone(),
    })?;

    Ok(())
}

/// Issue a spl_token `Transfer` instruction.
#[inline(always)]
pub fn spl_token_transfer(params: TokenTransferParams<'_, '_>) -> ProgramResult {
    let TokenTransferParams {
        source,
        destination,
        authority,
        token_program,
        amount,
        authority_signer_seeds,
    } = params;
    let result = invoke_signed(
        &spl_token::instruction::transfer(
            token_program.key,
            source.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?,
        &[source, destination, authority, token_program],
        &[authority_signer_seeds],
    );
    result.map_err(|_| HelloWorldError::TokenTransferFailed.into())
}

///TokenTransferParams
pub struct TokenTransferParams<'a: 'b, 'b> {
    /// source
    pub source: AccountInfo<'a>,
    /// destination
    pub destination: AccountInfo<'a>,
    /// amount
    pub amount: u64,
    /// authority
    pub authority: AccountInfo<'a>,
    /// authority_signer_seeds
    pub authority_signer_seeds: &'b [&'b [u8]],
    /// token_program
    pub token_program: AccountInfo<'a>,
}
/// TokenMintToParams
pub struct TokenMintToParams<'a: 'b, 'b> {
    /// mint
    pub mint: AccountInfo<'a>,
    /// destination
    pub destination: AccountInfo<'a>,
    /// amount
    pub amount: u64,
    /// authority
    pub authority: AccountInfo<'a>,
    /// authority_signer_seeds
    pub authority_signer_seeds: &'b [&'b [u8]],
    /// token_program
    pub token_program: AccountInfo<'a>,
}
