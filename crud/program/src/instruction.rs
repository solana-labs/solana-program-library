//! Program instructions

use crate::{state::Data, *};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
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
    /// 0. `[writeable, signer]` Data account, must be uninitialized
    /// 1. `[]` Document owner
    /// 2. `[]` Rent sysvar, to check for rent exemption
    Create {
        /// Data to be filled into the account
        data: Data,
    },

    /// Update the provided document account
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writeable]` Document account, must be previously initialized (version != 0)
    /// 1. `[signer]` Current owner of the document
    Update {
        /// Data to replace the existing document data
        data: Data,
    },

    /// Delete the provided document account, draining lamports to recipient account
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writeable]` Document account, must be previously initialized (version != 0)
    /// 1. `[signer]` Owner of the document
    /// 2. `[]` Receiver of drained lamports
    Delete,
}

/// Create a `CrudInstruction::Create` instruction
pub fn create(data_account: &Pubkey, owner: &Pubkey, data: Data) -> Instruction {
    Instruction::new_from_borsh(
        id(),
        &CrudInstruction::Create { data },
        vec![
            AccountMeta::new(*data_account, true),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
    )
}

/// Create a `CrudInstruction::Update` instruction
pub fn update(data_account: &Pubkey, signer: &Pubkey, data: Data) -> Instruction {
    Instruction::new_from_borsh(
        id(),
        &CrudInstruction::Update { data },
        vec![
            AccountMeta::new(*data_account, false),
            AccountMeta::new_readonly(*signer, true),
        ],
    )
}

/// Create a `CrudInstruction::Delete` instruction
pub fn delete(data_account: &Pubkey, signer: &Pubkey, receiver: &Pubkey) -> Instruction {
    Instruction::new_from_borsh(
        id(),
        &CrudInstruction::Delete,
        vec![
            AccountMeta::new(*data_account, false),
            AccountMeta::new_readonly(*signer, true),
            AccountMeta::new(*receiver, false),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::tests::TEST_DATA;
    use solana_program::program_error::ProgramError;

    #[test]
    fn serialize_create() {
        let instruction = CrudInstruction::Create { data: TEST_DATA };
        let mut expected = vec![0];
        expected.append(&mut TEST_DATA.try_to_vec().unwrap());
        assert_eq!(instruction.try_to_vec().unwrap(), expected);
        assert_eq!(
            CrudInstruction::try_from_slice(&expected).unwrap(),
            instruction
        );
    }

    #[test]
    fn serialize_update() {
        let instruction = CrudInstruction::Update { data: TEST_DATA };
        let mut expected = vec![1];
        expected.append(&mut TEST_DATA.try_to_vec().unwrap());
        assert_eq!(instruction.try_to_vec().unwrap(), expected);
        assert_eq!(
            CrudInstruction::try_from_slice(&expected).unwrap(),
            instruction
        );
    }

    #[test]
    fn deserialize_invalid_instruction() {
        let mut expected = vec![12];
        expected.append(&mut TEST_DATA.try_to_vec().unwrap());
        let err: ProgramError = CrudInstruction::try_from_slice(&expected)
            .unwrap_err()
            .into();
        assert!(matches!(err, ProgramError::SerializationError(_)));
    }

    #[test]
    fn serialize_delete() {
        let instruction = CrudInstruction::Delete;
        let expected = vec![2];
        assert_eq!(instruction.try_to_vec().unwrap(), expected);
        assert_eq!(
            CrudInstruction::try_from_slice(&expected).unwrap(),
            instruction
        );
    }
}
