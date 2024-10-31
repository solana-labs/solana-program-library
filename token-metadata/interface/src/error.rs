//! Interface error types

use {
    solana_decode_error::DecodeError,
    solana_msg::msg,
    solana_program_error::{PrintProgramError, ProgramError},
};

/// Errors that may be returned by the interface.
#[repr(u32)]
#[derive(Clone, Debug, Eq, thiserror::Error, num_derive::FromPrimitive, PartialEq)]
pub enum TokenMetadataError {
    /// Incorrect account provided
    #[error("Incorrect account provided")]
    IncorrectAccount = 901_952_957,
    /// Mint has no mint authority
    #[error("Mint has no mint authority")]
    MintHasNoMintAuthority,
    /// Incorrect mint authority has signed the instruction
    #[error("Incorrect mint authority has signed the instruction")]
    IncorrectMintAuthority,
    /// Incorrect metadata update authority has signed the instruction
    #[error("Incorrect metadata update authority has signed the instruction")]
    IncorrectUpdateAuthority,
    /// Token metadata has no update authority
    #[error("Token metadata has no update authority")]
    ImmutableMetadata,
    /// Key not found in metadata account
    #[error("Key not found in metadata account")]
    KeyNotFound,
}

impl From<TokenMetadataError> for ProgramError {
    fn from(e: TokenMetadataError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for TokenMetadataError {
    fn type_of() -> &'static str {
        "TokenMetadataError"
    }
}

impl PrintProgramError for TokenMetadataError {
    fn print<E>(&self)
    where
        E: 'static
            + std::error::Error
            + DecodeError<E>
            + PrintProgramError
            + num_traits::FromPrimitive,
    {
        match self {
            TokenMetadataError::IncorrectAccount => {
                msg!("Incorrect account provided")
            }
            TokenMetadataError::MintHasNoMintAuthority => {
                msg!("Mint has no mint authority")
            }
            TokenMetadataError::IncorrectMintAuthority => {
                msg!("Incorrect mint authority has signed the instruction",)
            }
            TokenMetadataError::IncorrectUpdateAuthority => {
                msg!("Incorrect metadata update authority has signed the instruction",)
            }
            TokenMetadataError::ImmutableMetadata => {
                msg!("Token metadata has no update authority")
            }
            TokenMetadataError::KeyNotFound => {
                msg!("Key not found in metadata account")
            }
        }
    }
}
