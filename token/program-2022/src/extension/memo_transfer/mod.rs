#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
use {
    crate::{
        error::TokenError,
        extension::{BaseState, BaseStateWithExtensions, Extension, ExtensionType},
    },
    bytemuck::{Pod, Zeroable},
    solana_program::{
        instruction::get_processed_sibling_instruction, program_error::ProgramError, pubkey::Pubkey,
    },
    spl_pod::primitives::PodBool,
};

/// Memo Transfer extension instructions
pub mod instruction;

/// Memo Transfer extension processor
pub mod processor;

/// Memo Transfer extension for Accounts
#[repr(C)]
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct MemoTransfer {
    /// Require transfers into this account to be accompanied by a memo
    pub require_incoming_transfer_memos: PodBool,
}
impl Extension for MemoTransfer {
    const TYPE: ExtensionType = ExtensionType::MemoTransfer;
}

/// Determine if a memo is required for transfers into this account
pub fn memo_required<BSE: BaseStateWithExtensions<S>, S: BaseState>(account_state: &BSE) -> bool {
    if let Ok(extension) = account_state.get_extension::<MemoTransfer>() {
        return extension.require_incoming_transfer_memos.into();
    }
    false
}

/// Check if the previous sibling instruction is a memo
pub fn check_previous_sibling_instruction_is_memo() -> Result<(), ProgramError> {
    let is_memo_program = |program_id: &Pubkey| -> bool {
        program_id == &spl_memo::id() || program_id == &spl_memo::v1::id()
    };
    let previous_instruction = get_processed_sibling_instruction(0);
    match previous_instruction {
        Some(instruction) if is_memo_program(&instruction.program_id) => {}
        _ => {
            return Err(TokenError::NoMemo.into());
        }
    }
    Ok(())
}
