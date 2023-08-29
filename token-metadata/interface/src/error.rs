//! Interface error types

use {
    num_derive::FromPrimitive,
    solana_program::{
        decode_error::DecodeError,
        msg,
        program_error::{PrintProgramError, ProgramError},
    },
    thiserror::Error,
};

/// Errors that may be returned by the interface.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum TokenMetadataError {
    /// Incorrect account provided
    #[error("Incorrect account provided")]
    IncorrectAccount,
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
impl TokenMetadataError {
    /// Offset to avoid conflict with implementing program error codes
    const PROGRAM_ERROR_OFFSET: u32 = 40000;

    /// Returns the error code
    pub fn error_code(self) -> u32 {
        (self as u32).saturating_add(Self::PROGRAM_ERROR_OFFSET)
    }
}

impl From<TokenMetadataError> for ProgramError {
    fn from(e: TokenMetadataError) -> Self {
        ProgramError::Custom(e.error_code())
    }
}

impl<T> DecodeError<T> for TokenMetadataError {
    fn type_of() -> &'static str {
        "TokenError"
    }
}

impl PrintProgramError for TokenMetadataError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + num_traits::FromPrimitive,
    {
        msg!("{}", self);
    }
}
