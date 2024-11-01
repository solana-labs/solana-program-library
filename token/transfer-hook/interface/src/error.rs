//! Error types

use {
    solana_decode_error::DecodeError,
    solana_msg::msg,
    solana_program_error::{PrintProgramError, ProgramError},
};

/// Errors that may be returned by the interface.
#[repr(u32)]
#[derive(Clone, Debug, Eq, thiserror::Error, num_derive::FromPrimitive, PartialEq)]
pub enum TransferHookError {
    /// Incorrect account provided
    #[error("Incorrect account provided")]
    IncorrectAccount = 2_110_272_652,
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
            TransferHookError::IncorrectAccount => {
                msg!("Incorrect account provided")
            }
            TransferHookError::MintHasNoMintAuthority => {
                msg!("Mint has no mint authority")
            }
            TransferHookError::IncorrectMintAuthority => {
                msg!("Incorrect mint authority has signed the instruction")
            }
            TransferHookError::ProgramCalledOutsideOfTransfer => {
                msg!("Program called outside of a token transfer")
            }
        }
    }
}
