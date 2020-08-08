//! Error types

use num_derive::FromPrimitive;
use solana_sdk::{decode_error::DecodeError, program_error::ProgramError};
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
    /// Instruction does not support non-native tokens
    #[error("Instruction does not support non-native tokens")]
    NonNativeNotSupported,
    /// Invalid instruction
    #[error("Invalid instruction")]
    InvalidInstruction,
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
