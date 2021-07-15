//! Error types

use num_derive::FromPrimitive;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

/// Errors that may be returned by the Token program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum TokenError {
    // 0
    /// Lamport balance below rent-exempt threshold.
    #[error("Lamport balance below rent-exempt threshold")]
    NotRentExempt,
    /// Insufficient funds for the operation requested.
    #[error("Insufficient funds")]
    InsufficientFunds,
    /// Invalid Mint.
    #[error("Invalid Mint")]
    InvalidMint,
    /// Account not associated with this Mint.
    #[error("Account not associated with this Mint")]
    MintMismatch,
    /// Owner does not match.
    #[error("Owner does not match")]
    OwnerMismatch,

    // 5
    /// This token's supply is fixed and new tokens cannot be minted.
    #[error("Fixed supply")]
    FixedSupply,
    /// The account cannot be initialized because it is already being used.
    #[error("Already in use")]
    AlreadyInUse,
    /// Invalid number of provided signers.
    #[error("Invalid number of provided signers")]
    InvalidNumberOfProvidedSigners,
    /// Invalid number of required signers.
    #[error("Invalid number of required signers")]
    InvalidNumberOfRequiredSigners,
    /// State is uninitialized.
    #[error("State is unititialized")]
    UninitializedState,

    // 10
    /// Instruction does not support native tokens
    #[error("Instruction does not support native tokens")]
    NativeNotSupported,
    /// Non-native account can only be closed if its balance is zero
    #[error("Non-native account can only be closed if its balance is zero")]
    NonNativeHasBalance,
    /// Invalid instruction
    #[error("Invalid instruction")]
    InvalidInstruction,
    /// State is invalid for requested operation.
    #[error("State is invalid for requested operation")]
    InvalidState,
    /// Operation overflowed
    #[error("Operation overflowed")]
    Overflow,

    // 15
    /// Account does not support specified authority type.
    #[error("Account does not support specified authority type")]
    AuthorityTypeNotSupported,
    /// This token mint cannot freeze accounts.
    #[error("This token mint cannot freeze accounts")]
    MintCannotFreeze,
    /// Account is frozen; all account operations will fail
    #[error("Account is frozen")]
    AccountFrozen,
    /// Mint decimals mismatch between the client and mint
    #[error("The provided decimals value different from the Mint decimals")]
    MintDecimalsMismatch,
    /// Instruction does not support non-native tokens
    #[error("Instruction does not support non-native tokens")]
    NonNativeNotSupported,
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
