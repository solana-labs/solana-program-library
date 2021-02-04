//! Program instructions

use crate::state::Document;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    msg,
    program_error::ProgramError,
    program_pack::{Pack, Sealed},
    pubkey::Pubkey,
    sysvar,
};

/// Instructions supported by the Feature Proposal program
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum CrudInstruction {
    /// Create a new document
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writeable, signer]` Document account, must be uninitialized
    /// 1. `[]` Rent sysvar, to check for rent exemption
    Create {
        /// Data to be filled into the account
        document: Document,
    },

    /// Update the provided document account
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writeable]` Document account, must be previously initialized (version != 0)
    /// 1. `[signer]` Current owner of the document
    Update {
        /// Data to replace the existing document data
        document: Document,
    },
}

/// Create a `CrudInstruction::Create` instruction
pub fn create(
    document_address: &Pubkey,
    document: Document
) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*document_address, true),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: CrudInstruction::Create {
            document
        }
        .pack_into_vec(),
    }
}

/// Create a `CrudInstruction::Update` instruction
pub fn update(document_address: &Pubkey, signer: &Pubkey, updated_document: Document) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*document_address, false),
            AccountMeta::new_readonly(*signer, true),
        ],
        data: CrudInstruction::Update {
            document
        }.pack_into_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_bytes() {
    }

    #[test]
    fn test_serialize_large_slice() {
    }

    #[test]
    fn state_deserialize_invalid() {
    }
}
