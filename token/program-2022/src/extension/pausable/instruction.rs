#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
use {
    crate::{
        check_program_account,
        instruction::{encode_instruction, TokenInstruction},
    },
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
};

/// Pausable extension instructions
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum PausableInstruction {
    /// Initialize the pausable extension for the given mint account
    ///
    /// Fails if the account has already been initialized, so must be called
    /// before `InitializeMint`.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]`  The mint account to initialize.
    ///
    /// Data expected by this instruction:
    ///   `crate::extension::pausable::instruction::InitializeInstructionData`
    Initialize,
    /// Pause minting, burning, and transferring for the mint.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The mint to update.
    ///   1. `[signer]` The mint's pause authority.
    ///
    ///   * Multisignature authority
    ///   0. `[writable]` The mint to update.
    ///   1. `[]` The mint's multisignature pause authority.
    ///   2. `..2+M` `[signer]` M signer accounts.
    Pause,
    /// Resume minting, burning, and transferring for the mint.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The mint to update.
    ///   1. `[signer]` The mint's pause authority.
    ///
    ///   * Multisignature authority
    ///   0. `[writable]` The mint to update.
    ///   1. `[]` The mint's multisignature pause authority.
    ///   2. `..2+M` `[signer]` M signer accounts.
    Resume,
}

/// Data expected by `PausableInstruction::Initialize`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct InitializeInstructionData {
    /// The public key for the account that can pause the mint
    pub authority: Pubkey,
}

/// Create an `Initialize` instruction
pub fn initialize(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![AccountMeta::new(*mint, false)];
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::PausableExtension,
        PausableInstruction::Initialize,
        &InitializeInstructionData {
            authority: *authority,
        },
    ))
}

/// Create a `Pause` instruction
pub fn pause(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new_readonly(*authority, signers.is_empty()),
    ];
    for signer_pubkey in signers.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::PausableExtension,
        PausableInstruction::Pause,
        &(),
    ))
}

/// Create a `Resume` instruction
pub fn resume(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new_readonly(*authority, signers.is_empty()),
    ];
    for signer_pubkey in signers.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::PausableExtension,
        PausableInstruction::Resume,
        &(),
    ))
}
