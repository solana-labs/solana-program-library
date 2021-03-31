//! Error types

use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

/// Errors that may be returned by the Metadata program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum MetadataError {
    /// Invalid instruction data passed in.
    #[error("Failed to unpack instruction data")]
    InstructionUnpackError,

    /// Lamport balance below rent-exempt threshold.
    #[error("Lamport balance below rent-exempt threshold")]
    NotRentExempt,

    /// Already initialized
    #[error("Already initialized")]
    AlreadyInitialized,

    /// Uninitialized
    #[error("Uninitialized")]
    Uninitialized,

    ///  Metadata's key must match seed of ['metadata', program id, mint] provided
    #[error(" Metadata's key must match seed of ['metadata', program id, mint] provided")]
    InvalidMetadataKey,

    ///  Owner's key must match seed of ['metadata', program id, name, symbol] provided
    #[error(" Metadata's key must match seed of ['metadata', program id, name, symbol] provided")]
    InvalidOwnerKey,

    /// This  owner does not own this  metadata
    #[error("This  owner does not own this  metadata")]
    InvalidMetadataForOwner,

    /// Owner given does not match owner key on  Owner
    #[error("Owner given does not match owner key on  Owner")]
    OwnerNotOwner,

    /// Owner needs to be signer to update  metadata
    #[error("Owner needs to be signer to update  metadata")]
    OwnerIsNotSigner,

    /// You must be the mint authority and signer on this transaction to create it's metadata
    #[error(
        "You must be the mint authority and signer on this transaction to create it's metadata"
    )]
    NotMintAuthority,

    /// Mint authority provided does not match the authority on the mint
    #[error("Mint authority provided does not match the authority on the mint")]
    InvalidMintAuthority,

    /// Name too long
    #[error("Name too long")]
    NameTooLong,

    /// Symbol too long
    #[error("Symbol too long")]
    SymbolTooLong,

    /// URI too long
    #[error("URI too long")]
    UriTooLong,
}

impl PrintProgramError for MetadataError {
    fn print<E>(&self) {
        msg!(&self.to_string());
    }
}

impl From<MetadataError> for ProgramError {
    fn from(e: MetadataError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for MetadataError {
    fn type_of() -> &'static str {
        "Metadata Error"
    }
}
