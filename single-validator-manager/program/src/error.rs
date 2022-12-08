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

/// Errors that may be returned by the single validator manager program.
#[repr(u8)]
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum SingleValidatorManagerError {
    /// The instruction cannot be decoded
    #[error("InvalidInstruction")]
    InvalidInstruction,
    /// The account cannot be initialized because it is already being used.
    #[error("AlreadyInUse")]
    AlreadyInUse,
    /// The calculation failed.
    #[error("CalculationFailure")]
    CalculationFailure,
    /// Required signature is missing.
    #[error("SignatureMissing")]
    SignatureMissing,
    /// Stake account for this validator not found in the pool.
    #[error("ValidatorNotFound")]
    ValidatorNotFound,
    /// Stake account address not properly derived from the validator address.
    #[error("InvalidStakeAccountAddress")]
    InvalidStakeAccountAddress,
    /// Validator stake account is not found in the list.
    #[error("UnknownValidatorStakeAccount")]
    UnknownValidatorStakeAccount,
    /// Invalid validator stake list account.
    #[error("InvalidValidatorStakeList")]
    InvalidValidatorStakeList,
    /// Invalid manager fee account.
    #[error("InvalidFeeAccount")]
    InvalidFeeAccount,
    /// Stake account voting for this validator already exists in the pool.
    #[error("ValidatorAlreadyAdded")]
    ValidatorAlreadyAdded,
    /// Wrong minting authority set for mint pool account
    #[error("IncorrectMintAuthority")]
    IncorrectMintAuthority,
    /// Incorrect pool staker account.
    #[error("IncorrectStaker")]
    IncorrectStaker,
    /// Incorrect manager account.
    #[error("IncorrectManager")]
    IncorrectManager,
    /// Incorrect reserve account.
    #[error("IncorrectReserve")]
    IncorrectReserve,
    /// The lamports in the validator stake account is not equal to the minimum
    #[error("StakeLamportsNotEqualToMinimum")]
    StakeLamportsNotEqualToMinimum,
    /// Provided metadata account does not match metadata account derived for pool mint
    #[error("InvalidMetadataAccount")]
    InvalidMetadataAccount,
}
impl From<SingleValidatorManagerError> for ProgramError {
    fn from(e: SingleValidatorManagerError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for SingleValidatorManagerError {
    fn type_of() -> &'static str {
        "Single Validator Stake Pool Manager Error"
    }
}
impl PrintProgramError for SingleValidatorManagerError {
    fn print<E>(&self)
    where
        E: 'static
            + std::error::Error
            + DecodeError<E>
            + PrintProgramError
            + num_traits::FromPrimitive,
    {
        match self {
            SingleValidatorManagerError::InvalidInstruction => msg!("Error: The instruction cannot be decoded"),
            SingleValidatorManagerError::AlreadyInUse => msg!("Error: The account cannot be initialized because it is already being used"),
            SingleValidatorManagerError::CalculationFailure => msg!("Error: The calculation failed"),
            SingleValidatorManagerError::SignatureMissing => msg!("Error: Required signature is missing"),
            SingleValidatorManagerError::InvalidValidatorStakeList => msg!("Error: Invalid validator stake list account"),
            SingleValidatorManagerError::InvalidFeeAccount => msg!("Error: Invalid manager fee account"),
            SingleValidatorManagerError::ValidatorAlreadyAdded => msg!("Error: Stake account voting for this validator already exists in the pool"),
            SingleValidatorManagerError::ValidatorNotFound => msg!("Error: Stake account for this validator not found in the pool"),
            SingleValidatorManagerError::InvalidStakeAccountAddress => msg!("Error: Stake account address not properly derived from the validator address"),
            SingleValidatorManagerError::UnknownValidatorStakeAccount => {
                msg!("Error: Validator stake account is not found in the list storage")
            }
            SingleValidatorManagerError::IncorrectMintAuthority => msg!("Error: Incorrect mint authority set for mint pool account"),
            SingleValidatorManagerError::IncorrectStaker => msg!("Error: Incorrect pool staker account"),
            SingleValidatorManagerError::IncorrectManager => msg!("Error: Incorrect pool manager account"),
            SingleValidatorManagerError::IncorrectReserve => msg!("Error: Incorrect pool reserve account"),
            SingleValidatorManagerError::StakeLamportsNotEqualToMinimum => msg!("Error: The lamports in the validator stake account is not equal to the minimum"),
            SingleValidatorManagerError::InvalidMetadataAccount => msg!("Error: Metadata account derived from pool mint account does not match the one passed to program"),
        }
    }
}
