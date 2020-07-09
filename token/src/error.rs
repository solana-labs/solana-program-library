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
    #[error("Insufficient funds")]
    InsufficientFunds,
    /// Account not associated with this Mint.
    #[error("Account not associated with this Mint")]
    MintMismatch,
    /// Owner does not match.
    #[error("Owner does not match")]
    OwnerMismatch,
    /// This token's supply is fixed and new tokens cannot be minted.
    #[error("Fixed supply")]
    FixedSupply,
    /// The account cannot be initialized because it is already being used.
    #[error("AlreadyInUse")]
    AlreadyInUse,
    /// An owner is required if initial supply is zero.
    #[error("An owner is required if supply is zero")]
    OwnerRequiredIfNoInitialSupply,
    /// Invalid number of provided signers.
    #[error("Invalid number of provided signers")]
    InvalidNumberOfProvidedSigners,
    /// Invalid number of required signers.
    #[error("Invalid number of required signers")]
    InvalidNumberOfRequiredSigners,
    /// State is uninitialized.
    #[error("State is unititialized")]
    UninitializedState,
    /// Instruction does not support native tokens
    #[error("Instruction does not support native tokens")]
    NativeNotSupported,
    /// Invalid instruction
    #[error("Invalid instruction")]
    InvalidInstruction,
    /// Non-native account can only be closed if its balance is zero
    #[error("Non-native account can only be closed if its balance is zero")]
    NonNativeHasBalance,
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
            TokenError::MintMismatch => info!("Error: Account not associated with this Mint"),
            TokenError::OwnerMismatch => info!("Error: owner does not match"),
            TokenError::FixedSupply => info!("Error: the total supply of this token is fixed"),
            TokenError::AlreadyInUse => info!("Error: account or token already in use"),
            TokenError::OwnerRequiredIfNoInitialSupply => {
                info!("Error: An owner is required if supply is zero")
            }
            TokenError::InvalidNumberOfProvidedSigners => {
                info!("Error: Invalid number of provided signers")
            }
            TokenError::InvalidNumberOfRequiredSigners => {
                info!("Error: Invalid number of required signers")
            }
            TokenError::UninitializedState => info!("Error: State is uninitialized"),
            TokenError::NativeNotSupported => {
                info!("Error: Instruction does not support native tokens")
            }
            TokenError::InvalidInstruction => info!("Error: Invalid instruction"),
            TokenError::NonNativeHasBalance => {
                info!("Non-native account can only be closed if its balance is zero")
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn return_token_error_as_program_error() -> ProgramError {
        TokenError::MintMismatch.into()
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
