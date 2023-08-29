//! Error types

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
/// 
/// Note: Error codes range from 30_000 - (30_000 + n)
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum TransferHookError {
    /// Incorrect account provided
    #[error("Incorrect account provided")]
    IncorrectAccount = 30_000, // Error code offset
    /// Mint has no mint authority
    #[error("Mint has no mint authority")]
    MintHasNoMintAuthority,
    /// Incorrect mint authority has signed the instruction
    #[error("Incorrect mint authority has signed the instruction")]
    IncorrectMintAuthority,
    /// Program called outside of a token transfer
    #[error("Program called outside of a token transfer")]
    ProgramCalledOutsideOfTransfer,
}
impl From<TransferHookError> for ProgramError {
    fn from(e: TransferHookError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for TransferHookError {
    fn type_of() -> &'static str {
        "TransferHookError"
    }
}

impl PrintProgramError for TransferHookError {
    fn print<E>(&self)
    where
        E: 'static
            + std::error::Error
            + DecodeError<E>
            + PrintProgramError
            + num_traits::FromPrimitive,
    {
        match self {
            Self::IncorrectAccount => msg!("Incorrect account provided"),
            Self::MintHasNoMintAuthority => msg!("Mint has no mint authority"),
            Self::IncorrectMintAuthority => {
                msg!("Incorrect mint authority has signed the instruction")
            }
            Self::ProgramCalledOutsideOfTransfer => {
                msg!("Program called outside of a token transfer")
            }
        }
    }
}
