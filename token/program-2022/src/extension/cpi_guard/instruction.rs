#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
use {
    crate::{
        check_program_account,
        instruction::{encode_instruction, TokenInstruction},
    },
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
};

/// CPI Guard extension instructions
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum CpiGuardInstruction {
    /// Lock certain token operations from taking place within CPI for this
    /// Account, namely:
    /// * Transfer and Burn must go through a delegate.
    /// * CloseAccount can only return lamports to owner.
    /// * SetAuthority can only be used to remove an existing close authority.
    /// * Approve is disallowed entirely.
    ///
    /// In addition, CPI Guard cannot be enabled or disabled via CPI.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The account to update.
    ///   1. `[signer]` The account's owner.
    ///
    ///   * Multisignature authority
    ///   0. `[writable]` The account to update.
    ///   1. `[]` The account's multisignature owner.
    ///   2. ..2+M `[signer]` M signer accounts.
    Enable,
    /// Allow all token operations to happen via CPI as normal.
    ///
    /// Implicitly initializes the extension in the case where it is not
    /// present.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The account to update.
    ///   1. `[signer]` The account's owner.
    ///
    ///   * Multisignature authority
    ///   0. `[writable]` The account to update.
    ///   1. `[]`  The account's multisignature owner.
    ///   2. ..2+M `[signer]` M signer accounts.
    Disable,
}

/// Create an `Enable` instruction
pub fn enable_cpi_guard(
    token_program_id: &Pubkey,
    account: &Pubkey,
    owner: &Pubkey,
    signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*account, false),
        AccountMeta::new_readonly(*owner, signers.is_empty()),
    ];
    for signer_pubkey in signers.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::CpiGuardExtension,
        CpiGuardInstruction::Enable,
        &(),
    ))
}

/// Create a `Disable` instruction
pub fn disable_cpi_guard(
    token_program_id: &Pubkey,
    account: &Pubkey,
    owner: &Pubkey,
    signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*account, false),
        AccountMeta::new_readonly(*owner, signers.is_empty()),
    ];
    for signer_pubkey in signers.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::CpiGuardExtension,
        CpiGuardInstruction::Disable,
        &(),
    ))
}
