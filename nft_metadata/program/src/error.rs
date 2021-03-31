//! Error types

use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

/// Errors that may be returned by the NFTMetadata program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum NFTMetadataError {
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

    /// NFT Metadata's key must match seed of ['metadata', program id, mint] provided
    #[error("NFT Metadata's key must match seed of ['metadata', program id, mint] provided")]
    InvalidNFTMetadataKey,

    /// NFT Owner's key must match seed of ['metadata', program id, name, symbol] provided
    #[error(
        "NFT Metadata's key must match seed of ['metadata', program id, name, symbol] provided"
    )]
    InvalidNFTOwnerKey,

    /// This nft owner does not own this nft metadata
    #[error("This nft owner does not own this nft metadata")]
    InvalidMetadataForNFTOwner,

    /// Owner given does not match owner key on NFT Owner
    #[error("Owner given does not match owner key on NFT Owner")]
    NFTOwnerNotOwner,

    /// Owner needs to be signer to update NFT metadata
    #[error("Owner needs to be signer to update NFT metadata")]
    OwnerIsNotSigner,

    /// You must be the mint authority and signer on this transaction to create it's metadata
    #[error(
        "You must be the mint authority and signer on this transaction to create it's metadata"
    )]
    NotMintAuthority,

    /// Mint authority provided does not match the authority on the mint
    #[error("Mint authority provided does not match the authority on the mint")]
    InvalidMintAuthority,
}

impl PrintProgramError for NFTMetadataError {
    fn print<E>(&self) {
        msg!(&self.to_string());
    }
}

impl From<NFTMetadataError> for ProgramError {
    fn from(e: NFTMetadataError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for NFTMetadataError {
    fn type_of() -> &'static str {
        "NFTMetadata Error"
    }
}
