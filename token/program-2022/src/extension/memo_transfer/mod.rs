use {
    crate::{
        extension::{Extension, ExtensionType, StateWithExtensionsMut},
        pod::PodBool,
        state::Account,
    },
    bytemuck::{Pod, Zeroable},
    solana_program::program_error::ProgramError,
};

// Remove feature once sibling instruction syscall is available on all networks
#[cfg(feature = "sibling-instruction")]
use {
    crate::error::TokenError,
    solana_program::{instruction::get_processed_sibling_instruction, pubkey::Pubkey},
};

/// Memo Transfer extension instructions
pub mod instruction;

/// Memo Transfer extension processor
pub mod processor;

/// Memo Transfer extension for Accounts
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct MemoTransfer {
    /// Require transfers into this account to be accompanied by a memo
    pub require_incoming_transfer_memos: PodBool,
}
impl Extension for MemoTransfer {
    const TYPE: ExtensionType = ExtensionType::MemoTransfer;
}

/// Determine if a memo is required for transfers into this account
pub fn memo_required(account_state: &StateWithExtensionsMut<Account>) -> bool {
    if let Ok(extension) = account_state.get_extension::<MemoTransfer>() {
        return extension.require_incoming_transfer_memos.into();
    }
    false
}

/// Check if the previous sibling instruction is a memo
pub fn check_previous_sibling_instruction_is_memo() -> Result<(), ProgramError> {
    #[cfg(feature = "sibling-instruction")]
    {
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
    }
    Ok(())
}
