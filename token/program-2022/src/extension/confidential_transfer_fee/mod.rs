use {
    crate::{
        error::TokenError,
        extension::{Extension, ExtensionType},
        pod::*,
    },
    bytemuck::{Pod, Zeroable},
    solana_program::entrypoint::ProgramResult,
    solana_zk_token_sdk::zk_token_elgamal::pod::ElGamalCiphertext,
};

/// Confidential transfer fee extension instructions
pub mod instruction;

/// Confidential transfer fee extension processor
pub mod processor;

/// ElGamal ciphertext containing a withheld fee
pub type EncryptedWithheldAmount = ElGamalCiphertext;

/// Confidential transfer fee extension data for mints
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ConfidentialTransferFeeConfig {
    /// Optional authority to set the withdraw withheld authority encryption key
    pub authority: OptionalNonZeroPubkey,
}

impl Extension for ConfidentialTransferFeeConfig {
    const TYPE: ExtensionType = ExtensionType::ConfidentialTransferFeeConfig;
}

/// Confidential transfer fee
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ConfidentialTransferFeeAmount {}

impl Extension for ConfidentialTransferFeeAmount {
    const TYPE: ExtensionType = ExtensionType::ConfidentialTransferFeeAmount;
}
