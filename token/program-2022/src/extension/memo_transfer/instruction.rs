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

/// Required Memo Transfers extension instructions
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum RequiredMemoTransfersInstruction {
    /// Require memos for transfers into this Account. Adds the MemoTransfer
    /// extension to the Account, if it doesn't already exist.
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
    /// Stop requiring memos for transfers into this Account.
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
pub fn enable_required_transfer_memos(
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
        TokenInstruction::MemoTransferExtension,
        RequiredMemoTransfersInstruction::Enable,
        &(),
    ))
}

/// Create a `Disable` instruction
pub fn disable_required_transfer_memos(
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
        TokenInstruction::MemoTransferExtension,
        RequiredMemoTransfersInstruction::Disable,
        &(),
    ))
}
