//! Program instructions

use crate::{state::Data, *};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};

/// Instructions supported by the program
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum CrudInstruction {
    /// Create a new document
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writeable, signer]` Data account, must be uninitialized
    /// 1. `[]` Document authority
    /// 2. `[]` Rent sysvar, to check for rent exemption
    Initialize,

    /// Write to the provided data account
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writeable]` Document account, must be previously initialized (version != 0)
    /// 1. `[signer]` Current authority of the document
    Write {
        /// Data to replace the existing document data
        data: Data,
    },

    /// Update the authority of the provided data account
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writeable]` Document account, must be previously initialized (version != 0)
    /// 1. `[signer]` Current authority of the document
    /// 2. `[]` New authority of the document
    SetAuthority,

    /// Close the provided document account, draining lamports to recipient account
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writeable]` Document account, must be previously initialized (version != 0)
    /// 1. `[signer]` Owner of the document
    /// 2. `[]` Receiver of account lamports
    CloseAccount,
}

/// Create a `CrudInstruction::Initialize` instruction
pub fn initialize(data_account: &Pubkey, owner: &Pubkey) -> Instruction {
    Instruction::new_with_borsh(
        id(),
        &CrudInstruction::Initialize,
        vec![
            AccountMeta::new(*data_account, true),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
    )
}

/// Create a `CrudInstruction::Write` instruction
pub fn write(data_account: &Pubkey, signer: &Pubkey, data: Data) -> Instruction {
    Instruction::new_with_borsh(
        id(),
        &CrudInstruction::Write { data },
        vec![
            AccountMeta::new(*data_account, false),
            AccountMeta::new_readonly(*signer, true),
        ],
    )
}

/// Create a `CrudInstruction::SetAuthority` instruction
pub fn set_authority(data_account: &Pubkey, signer: &Pubkey, new_authority: &Pubkey) -> Instruction {
    Instruction::new_with_borsh(
        id(),
        &CrudInstruction::SetAuthority,
        vec![
            AccountMeta::new(*data_account, false),
            AccountMeta::new_readonly(*signer, true),
            AccountMeta::new_readonly(*new_authority, false),
        ],
    )
}

/// Create a `CrudInstruction::CloseAccount` instruction
pub fn close_account(data_account: &Pubkey, signer: &Pubkey, receiver: &Pubkey) -> Instruction {
    Instruction::new_with_borsh(
        id(),
        &CrudInstruction::CloseAccount,
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
    fn serialize_initialize() {
        let instruction = CrudInstruction::Initialize;
        let expected = vec![0];
        assert_eq!(instruction.try_to_vec().unwrap(), expected);
        assert_eq!(
            CrudInstruction::try_from_slice(&expected).unwrap(),
            instruction
        );
    }

    #[test]
    fn serialize_write() {
        let instruction = CrudInstruction::Write { data: TEST_DATA };
        let mut expected = vec![1];
        expected.append(&mut TEST_DATA.try_to_vec().unwrap());
        assert_eq!(instruction.try_to_vec().unwrap(), expected);
        assert_eq!(
            CrudInstruction::try_from_slice(&expected).unwrap(),
            instruction
        );
    }

    #[test]
    fn serialize_set_authority() {
        let instruction = CrudInstruction::SetAuthority;
        let expected = vec![2];
        assert_eq!(instruction.try_to_vec().unwrap(), expected);
        assert_eq!(
            CrudInstruction::try_from_slice(&expected).unwrap(),
            instruction
        );
    }

    #[test]
    fn serialize_close_account() {
        let instruction = CrudInstruction::CloseAccount;
        let expected = vec![3];
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
        assert!(matches!(err, ProgramError::IOError(_)));
    }
}
