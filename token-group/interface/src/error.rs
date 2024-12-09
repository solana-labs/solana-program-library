//! Interface error types

use {
    solana_decode_error::DecodeError,
    solana_msg::msg,
    solana_program_error::{PrintProgramError, ProgramError},
};

/// Errors that may be returned by the interface.
#[repr(u32)]
#[derive(Clone, Debug, Eq, thiserror::Error, num_derive::FromPrimitive, PartialEq)]
pub enum TokenGroupError {
    /// Size is greater than proposed max size
    #[error("Size is greater than proposed max size")]
    SizeExceedsNewMaxSize = 3_406_457_176,
    /// Size is greater than max size
    #[error("Size is greater than max size")]
    SizeExceedsMaxSize,
    /// Group is immutable
    #[error("Group is immutable")]
    ImmutableGroup,
    /// Incorrect mint authority has signed the instruction
    #[error("Incorrect mint authority has signed the instruction")]
    IncorrectMintAuthority,
    /// Incorrect update authority has signed the instruction
    #[error("Incorrect update authority has signed the instruction")]
    IncorrectUpdateAuthority,
    /// Member account should not be the same as the group account
    #[error("Member account should not be the same as the group account")]
    MemberAccountIsGroupAccount,
}

impl From<TokenGroupError> for ProgramError {
    fn from(e: TokenGroupError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for TokenGroupError {
    fn type_of() -> &'static str {
        "TokenGroupError"
    }
}

impl PrintProgramError for TokenGroupError {
    fn print<E>(&self)
    where
        E: 'static
            + std::error::Error
            + DecodeError<E>
            + PrintProgramError
            + num_traits::FromPrimitive,
    {
        match self {
            TokenGroupError::SizeExceedsNewMaxSize => {
                msg!("Size is greater than proposed max size")
            }
            TokenGroupError::SizeExceedsMaxSize => {
                msg!("Size is greater than max size")
            }
            TokenGroupError::ImmutableGroup => {
                msg!("Group is immutable")
            }
            TokenGroupError::IncorrectMintAuthority => {
                msg!("Incorrect mint authority has signed the instruction",)
            }
            TokenGroupError::IncorrectUpdateAuthority => {
                msg!("Incorrect update authority has signed the instruction",)
            }
            TokenGroupError::MemberAccountIsGroupAccount => {
                msg!("Member account should not be the same as the group account",)
            }
        }
    }
}
