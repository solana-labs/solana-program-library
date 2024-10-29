//! Error types

use {
    num_derive::FromPrimitive,
    solana_program::{decode_error::DecodeError, program_error::ProgramError},
    thiserror::Error,
};

/// Errors that may be returned by the program.
#[derive(Clone, Copy, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum SlashingError {
    /// Shred deserialization error
    #[error("Deserialization error")]
    DeserializationError,

    /// Invalid shred variant
    #[error("Invalid shred variant")]
    InvalidShredVariant,

    /// Invalid merkle shred
    #[error("Invalid Merkle shred")]
    InvalidMerkleShred,

    /// Invalid duplicate block payload proof
    #[error("invalid duplicate shreds")]
    InvalidPayloadProof,

    /// Invalid duplicate block erasure meta proof
    #[error("invalid erasure meta conflict")]
    InvalidErasureMetaConflict,

    /// Invalid duplicate block last index proof
    #[error("invalid last index conflict")]
    InvalidLastIndexConflict,

    /// Invalid shred version on duplicate block proof shreds
    #[error("invalid shred version")]
    InvalidShredVersion,

    /// Invalid signature on duplicate block proof shreds
    #[error("invalid signature")]
    InvalidSignature,

    /// Legacy shreds are not supported
    #[error("Legacy shreds are not eligible for slashing")]
    LegacyShreds,

    /// Invalid shred type on duplicate block proof shreds
    #[error("shred type mismatch")]
    ShredTypeMismatch,

    /// Invalid slot on duplicate block proof shreds
    #[error("slot mismatch")]
    SlotMismatch,
}

impl From<SlashingError> for ProgramError {
    fn from(e: SlashingError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for SlashingError {
    fn type_of() -> &'static str {
        "Slashing Error"
    }
}
