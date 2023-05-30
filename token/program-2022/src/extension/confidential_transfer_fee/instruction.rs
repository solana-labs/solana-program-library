#[cfg(feature = "proof-program")]
use crate::extension::confidential_transfer::instruction::{
    verify_withdraw_withheld_tokens, WithdrawWithheldTokensData,
};
use {
    crate::{
        check_program_account,
        instruction::{encode_instruction, TokenInstruction},
        pod::{EncryptionPubkey, OptionalNonZeroPubkey},
    },
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvar,
    },
    std::convert::TryFrom,
};

/// Confidential Transfer extension instructions
#[derive(Clone, Copy, Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ConfidentialTransferFeeInstruction {
    /// Initializes confidential transfer fees for a mint.
    ///
    /// The `ConfidentialTransferFeeInstruction::InitializeConfidentialTransferFeeConfig`
    /// instruction requires no signers and MUST be included within the same Transaction as
    /// `TokenInstruction::InitializeMint`. Otherwise another party can initialize the
    /// configuration.
    ///
    /// The instruction fails if the `TokenInstruction::InitializeMint` instruction has already
    /// executed for the mint.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The SPL Token mint.
    ///
    /// Data expected by this instruction:
    ///   `InitializeConfidentialTransferFeeConfigData`
    ///
    InitializeConfidentialTransferFeeConfig,
}

/// Data expected by `InitializeConfidentialTransferFeeConfig`
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct InitializeConfidentialTransferFeeConfigData {
    /// confidential transfer fee authority
    pub authority: OptionalNonZeroPubkey,

    /// ElGamal public key used to encrypt withheld fees.
    pub withdraw_withheld_authority_encryption_pubkey: EncryptionPubkey,
}

/// Create a `InitializeConfidentialTransferFeeConfig` instruction
pub fn initialize_confidential_transfer_fee_config(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: Option<Pubkey>,
    withdraw_withheld_authority_encryption_pubkey: EncryptionPubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![AccountMeta::new(*mint, false)];

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferFeeExtension,
        ConfidentialTransferFeeInstruction::InitializeConfidentialTransferFeeConfig,
        &InitializeConfidentialTransferFeeConfigData {
            authority: authority.try_into()?,
            withdraw_withheld_authority_encryption_pubkey,
        },
    ))
}
