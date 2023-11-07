use {
    crate::{
        error::TokenError,
        extension::{Extension, ExtensionType},
    },
    bytemuck::{Pod, Zeroable},
    solana_program::entrypoint::ProgramResult,
    solana_zk_token_sdk::zk_token_elgamal::pod::{ElGamalCiphertext, ElGamalPubkey, FeeEncryption},
    spl_pod::{optional_keys::OptionalNonZeroPubkey, primitives::PodBool},
};

/// Confidential transfer fee extension instructions
pub mod instruction;

/// Confidential transfer fee extension processor
pub mod processor;

/// Confidential Transfer Fee extension account information needed for
/// instructions
#[cfg(not(target_os = "solana"))]
pub mod account_info;

/// ElGamal ciphertext containing a transfer fee
pub type EncryptedFee = FeeEncryption;
/// ElGamal ciphertext containing a withheld fee in an account
pub type EncryptedWithheldAmount = ElGamalCiphertext;

/// Confidential transfer fee extension data for mints
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ConfidentialTransferFeeConfig {
    /// Optional authority to set the withdraw withheld authority ElGamal key
    pub authority: OptionalNonZeroPubkey,

    /// Withheld fees from accounts must be encrypted with this ElGamal key.
    ///
    /// Note that whoever holds the ElGamal private key for this ElGamal public
    /// key has the ability to decode any withheld fee amount that are
    /// associated with accounts. When combined with the fee parameters, the
    /// withheld fee amounts can reveal information about transfer amounts.
    pub withdraw_withheld_authority_elgamal_pubkey: ElGamalPubkey,

    /// If `false`, the harvest of withheld tokens to mint is rejected.
    pub harvest_to_mint_enabled: PodBool,

    /// Withheld confidential transfer fee tokens that have been moved to the
    /// mint for withdrawal.
    pub withheld_amount: EncryptedWithheldAmount,
}

impl Extension for ConfidentialTransferFeeConfig {
    const TYPE: ExtensionType = ExtensionType::ConfidentialTransferFeeConfig;
}

/// Confidential transfer fee
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ConfidentialTransferFeeAmount {
    /// Amount withheld during confidential transfers, to be harvest to the mint
    pub withheld_amount: EncryptedWithheldAmount,
}

impl Extension for ConfidentialTransferFeeAmount {
    const TYPE: ExtensionType = ExtensionType::ConfidentialTransferFeeAmount;
}

impl ConfidentialTransferFeeAmount {
    /// Check if a confidential transfer fee account is in a closable state.
    pub fn closable(&self) -> ProgramResult {
        if self.withheld_amount == EncryptedWithheldAmount::zeroed() {
            Ok(())
        } else {
            Err(TokenError::ConfidentialTransferFeeAccountHasWithheldFee.into())
        }
    }
}
