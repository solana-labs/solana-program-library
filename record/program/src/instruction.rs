//! Program instructions

use crate::id;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};

/// Instructions supported by the program
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum RecordInstruction {
    /// Create a new document
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writable]` Data account, must be uninitialized
    /// 1. `[]` Document authority
    /// 2. `[]` Rent sysvar, to check for rent exemption
    Initialize,

    /// Write to the provided data account
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writable]` Document account, must be previously initialized (version != 0)
    /// 1. `[signer]` Current document authority
    Write {
        /// Offset to start writing data, expressed as `u64`.
        offset: u64,
        /// Data to replace the existing document data
        data: Vec<u8>,
    },

    /// Update the authority of the provided data account
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writable]` Document account, must be previously initialized (version != 0)
    /// 1. `[signer]` Current authority of the document
    /// 2. `[]` New authority of the document
    SetAuthority,

    /// Close the provided document account, draining lamports to recipient account
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writable]` Document account, must be previously initialized (version != 0)
    /// 1. `[signer]` Document authority
    /// 2. `[]` Receiver of account lamports
    CloseAccount,
}

/// Create a `RecordInstruction::Initialize` instruction
pub fn initialize(data_account: &Pubkey, authority: &Pubkey) -> Instruction {
    Instruction::new_with_borsh(
        id(),
        &RecordInstruction::Initialize,
        vec![
            AccountMeta::new(*data_account, false),
            AccountMeta::new_readonly(*authority, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
    )
}

/// Create a `RecordInstruction::Write` instruction
pub fn write(data_account: &Pubkey, signer: &Pubkey, offset: u64, data: Vec<u8>) -> Instruction {
    Instruction::new_with_borsh(
        id(),
        &RecordInstruction::Write { offset, data },
        vec![
            AccountMeta::new(*data_account, false),
            AccountMeta::new_readonly(*signer, true),
        ],
    )
}

/// Create a `RecordInstruction::SetAuthority` instruction
pub fn set_authority(
    data_account: &Pubkey,
    signer: &Pubkey,
    new_authority: &Pubkey,
) -> Instruction {
    Instruction::new_with_borsh(
        id(),
        &RecordInstruction::SetAuthority,
        vec![
            AccountMeta::new(*data_account, false),
            AccountMeta::new_readonly(*signer, true),
            AccountMeta::new_readonly(*new_authority, false),
        ],
    )
}

/// Create a `RecordInstruction::CloseAccount` instruction
pub fn close_account(data_account: &Pubkey, signer: &Pubkey, receiver: &Pubkey) -> Instruction {
    Instruction::new_with_borsh(
        id(),
        &RecordInstruction::CloseAccount,
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
        let instruction = RecordInstruction::Initialize;
        let expected = vec![0];
        assert_eq!(instruction.try_to_vec().unwrap(), expected);
        assert_eq!(
            RecordInstruction::try_from_slice(&expected).unwrap(),
            instruction
        );
    }

    #[test]
    fn serialize_write() {
        let data = TEST_DATA.try_to_vec().unwrap();
        let offset = 0u64;
        let instruction = RecordInstruction::Write {
            offset: 0,
            data: data.clone(),
        };
        let mut expected = vec![1];
        expected.extend_from_slice(&offset.to_le_bytes());
        expected.append(&mut data.try_to_vec().unwrap());
        assert_eq!(instruction.try_to_vec().unwrap(), expected);
        assert_eq!(
            RecordInstruction::try_from_slice(&expected).unwrap(),
            instruction
        );
    }

    #[test]
    fn serialize_set_authority() {
        let instruction = RecordInstruction::SetAuthority;
        let expected = vec![2];
        assert_eq!(instruction.try_to_vec().unwrap(), expected);
        assert_eq!(
            RecordInstruction::try_from_slice(&expected).unwrap(),
            instruction
        );
    }

    #[test]
    fn serialize_close_account() {
        let instruction = RecordInstruction::CloseAccount;
        let expected = vec![3];
        assert_eq!(instruction.try_to_vec().unwrap(), expected);
        assert_eq!(
            RecordInstruction::try_from_slice(&expected).unwrap(),
            instruction
        );
    }

    #[test]
    fn deserialize_invalid_instruction() {
        let mut expected = vec![12];
        expected.append(&mut TEST_DATA.try_to_vec().unwrap());
        let err: ProgramError = RecordInstruction::try_from_slice(&expected)
            .unwrap_err()
            .into();
        assert!(matches!(err, ProgramError::IOError(_)));
    }
}
