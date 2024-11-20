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
    ShredDeserializationError,

    /// Invalid shred variant
    #[error("Invalid shred variant")]
    InvalidShredVariant,

    /// Invalid merkle shred
    #[error("Invalid Merkle shred")]
    InvalidMerkleShred,

    /// Invalid duplicate block payload proof
    #[error("Invalid payload proof")]
    InvalidPayloadProof,

    /// Invalid duplicate block erasure meta proof
    #[error("Invalid erasure meta conflict")]
    InvalidErasureMetaConflict,

    /// Invalid duplicate block last index proof
    #[error("Invalid last index conflict")]
    InvalidLastIndexConflict,

    /// Invalid shred version on duplicate block proof shreds
    #[error("Invalid shred version")]
    InvalidShredVersion,

    /// Invalid signature on duplicate block proof shreds
    #[error("Invalid signature")]
    InvalidSignature,

    /// Legacy shreds are not supported
    #[error("Legacy shreds are not eligible for slashing")]
    LegacyShreds,

    /// Invalid shred type on duplicate block proof shreds
    #[error("Shred type mismatch")]
    ShredTypeMismatch,

    /// Invalid slot on duplicate block proof shreds
    #[error("Slot mismatch")]
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
