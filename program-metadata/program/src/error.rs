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

    /// Program authority must sign
    #[error("ProgramAuthorityMustBeSigner")]
    ProgramAuthorityMustBeSigner,

    /// Program must match program data
    #[error("ProgramDoesNotMatchProgramData")]
    ProgramDoesNotMatchProgramData,

    /// Metadata's key must match seed of ['metadata', target_program id, metadata_name] provided
    #[error(" Metadata's key must match seed of ['metadata', target_program id, metadata_name] provided")]
    InvalidMetadataAccount,

    /// Metadata's key must match seed of ['metadata', target_program id, effective_slot] provided
    #[error(" Metadata's key must match seed of ['metadata', target_program id, effective_slot] provided")]
    InvalidIdlAccount,

    /// Name too long
    #[error("Name too long")]
    NameTooLong,

    /// Value too long
    #[error("Value too long")]
    ValueTooLong,

    /// IDL url too long
    #[error("IDL url too long")]
    IDLUrlTooLong,

    /// Source url too long
    #[error("Source url too long")]
    SourceUrlTooLong,

    /// Custom layout url too long
    #[error("Custom layout url too long")]
    CustomLayoutUrlTooLong,

    /// Update Authority given does not match
    #[error("Update Authority given does not match")]
    UpdateAuthorityIncorrect,
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
