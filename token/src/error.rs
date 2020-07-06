//! Error types

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use solana_sdk::{
    info,
    program_error::{PrintProgramError, ProgramError},
    program_utils::DecodeError,
};
use thiserror::Error;

/// Errors that may be returned by the Token program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum TokenError {
    /// Insufficient funds for the operation requested.
    #[error("insufficient funds")]
    InsufficientFunds,
    /// Token types of the provided accounts don't match.
    #[error("token mismatch")]
    TokenMismatch,
    /// Owner was not a signing member of the instruction.
    #[error("no owner")]
    NoOwner,
    /// This token's supply is fixed and new tokens cannot be minted.
    #[error("fixed supply")]
    FixedSupply,
    /// The account cannot be initialized because it is already being used.
    #[error("AlreadyInUse")]
    AlreadyInUse,
    /// An owner is required if supply is zero.
    #[error("An owner is required if supply is zero")]
    OwnerRequiredIfNoInitialSupply,
}
impl From<TokenError> for ProgramError {
    fn from(e: TokenError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for TokenError {
    fn type_of() -> &'static str {
        "TokenError"
    }
}
impl PrintProgramError for TokenError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            TokenError::InsufficientFunds => info!("Error: insufficient funds"),
            TokenError::TokenMismatch => info!("Error: token mismatch"),
            TokenError::NoOwner => info!("Error: no owner"),
            TokenError::FixedSupply => info!("Error: the total supply of this token is fixed"),
            TokenError::AlreadyInUse => info!("Error: account or token already in use"),
            TokenError::OwnerRequiredIfNoInitialSupply => {
                info!("Error: An owner is required if supply is zero")
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn return_token_error_as_program_error() -> ProgramError {
        TokenError::TokenMismatch.into()
    }

    #[test]
    fn test_print_error() {
        let error = return_token_error_as_program_error();
        error.print::<TokenError>();
    }

    #[test]
    #[should_panic(expected = "Custom(1)")]
    fn test_error_unwrap() {
        Err::<(), ProgramError>(return_token_error_as_program_error()).unwrap();
    }
}
