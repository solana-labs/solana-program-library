//! Error types

use {
    solana_program::{
        decode_error::DecodeError,
        msg,
        program_error::{PrintProgramError, ProgramError},
    },
    thiserror::Error,
};

/// Errors that may be returned by the SinglePool program.
#[derive(Clone, Debug, Eq, Error, num_derive::FromPrimitive, PartialEq)]
pub enum SinglePoolError {
    // 0.
    /// Provided pool stake account does not match stake account derived for validator vote account.
    #[error("InvalidPoolStakeAccount")]
    InvalidPoolStakeAccount,
    /// Provided pool authority does not match authority derived for validator vote account.
    #[error("InvalidPoolAuthority")]
    InvalidPoolAuthority,
    /// Provided pool mint does not match mint derived for validator vote account.
    #[error("InvalidPoolMint")]
    InvalidPoolMint,
    /// Provided metadata account does not match metadata account derived for pool mint.
    #[error("InvalidMetadataAccount")]
    InvalidMetadataAccount,
    /// Authorized withdrawer provided for metadata update does not match the vote account.
    #[error("InvalidMetadataSigner")]
    InvalidMetadataSigner,

    // 5.
    /// Not enough lamports provided for deposit to result in one pool token.
    #[error("DepositTooSmall")]
    DepositTooSmall,
    /// Not enough pool tokens provided to withdraw stake worth one lamport.
    #[error("WithdrawalTooSmall")]
    WithdrawalTooSmall,
    /// Not enough stake to cover the provided quantity of pool tokens.
    /// (Generally this should not happen absent user error, but may if the minimum delegation increases.)
    #[error("WithdrawalTooLarge")]
    WithdrawalTooLarge,
    /// Required signature is missing.
    #[error("SignatureMissing")]
    SignatureMissing,
    /// Stake account is not in the state expected by the program.
    #[error("WrongStakeState")]
    WrongStakeState,

    // 10.
    /// Unsigned subtraction crossed the zero.
    #[error("ArithmeticOverflow")]
    ArithmeticOverflow,
    /// A calculation failed unexpectedly.
    /// (This error should never be surfaced; it stands in for failure conditions that should never be reached.)
    #[error("UnexpectedMathError")]
    UnexpectedMathError,
    /// The V0_23_5 vote account type is unsupported and should be upgraded via `convert_to_current()`.
    #[error("LegacyVoteAccount")]
    LegacyVoteAccount,
    /// Failed to parse vote account.
    #[error("UnparseableVoteAccount")]
    UnparseableVoteAccount,
    /// Incorrect number of lamports provided for rent-exemption when initializing.
    #[error("WrongRentAmount")]
    WrongRentAmount,

    // 15.
    /// Attempted to deposit from or withdraw to pool stake account.
    #[error("InvalidPoolAccountUsage")]
    InvalidPoolAccountUsage,
}
impl From<SinglePoolError> for ProgramError {
    fn from(e: SinglePoolError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for SinglePoolError {
    fn type_of() -> &'static str {
        "Single-Validator Stake Pool Error"
    }
}
impl PrintProgramError for SinglePoolError {
    fn print<E>(&self)
    where
        E: 'static
            + std::error::Error
            + DecodeError<E>
            + PrintProgramError
            + num_traits::FromPrimitive,
    {
        match self {
            SinglePoolError::InvalidPoolStakeAccount =>
                msg!("Error: Provided pool stake account does not match stake account derived for validator vote account."),
            SinglePoolError::InvalidPoolAuthority =>
                msg!("Error: Provided pool authority does not match authority derived for validator vote account."),
            SinglePoolError::InvalidPoolMint =>
                msg!("Error: Provided pool mint does not match mint derived for validator vote account."),
            SinglePoolError::InvalidMetadataAccount =>
                msg!("Error: Provided metadata account does not match metadata account derived for pool mint."),
            SinglePoolError::InvalidMetadataSigner =>
                msg!("Error: Authorized withdrawer provided for metadata update does not match the vote account."),
            SinglePoolError::DepositTooSmall =>
                msg!("Error: Not enough lamports provided for deposit to result in one pool token."),
            SinglePoolError::WithdrawalTooSmall =>
                msg!("Error: Not enough pool tokens provided to withdraw stake worth one lamport."),
            SinglePoolError::WithdrawalTooLarge =>
                msg!("Error: Not enough stake to cover the provided quantity of pool tokens. \
                     (Generally this should not happen absent user error, but may if the minimum delegation increases.)"),
            SinglePoolError::SignatureMissing => msg!("Error: Required signature is missing."),
            SinglePoolError::WrongStakeState => msg!("Error: Stake account is not in the state expected by the program."),
            SinglePoolError::ArithmeticOverflow => msg!("Error: Unsigned subtraction crossed the zero."),
            SinglePoolError::UnexpectedMathError =>
                msg!("Error: A calculation failed unexpectedly. \
                     (This error should never be surfaced; it stands in for failure conditions that should never be reached.)"),
            SinglePoolError::UnparseableVoteAccount => msg!("Error: Failed to parse vote account."),
            SinglePoolError::LegacyVoteAccount =>
                msg!("Error: The V0_23_5 vote account type is unsupported and should be upgraded via `convert_to_current()`."),
            SinglePoolError::WrongRentAmount =>
                msg!("Error: Incorrect number of lamports provided for rent-exemption when initializing."),
            SinglePoolError::InvalidPoolAccountUsage =>
                msg!("Error: Attempted to deposit from or withdraw to pool stake account."),
        }
    }
}
