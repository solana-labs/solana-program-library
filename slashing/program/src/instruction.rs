//! Program instructions

use {
    crate::{id, state::ProofType},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
        system_program,
    },
    std::mem::size_of,
};

/// Instructions supported by the program
#[derive(Clone, Debug, PartialEq)]
pub enum SlashingInstruction<'a> {
    /// Create a new proof account
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writable]` Proof account, must be uninitialized
    /// 1. `[signer]` Fee payer for account creation
    /// 2. `[]` Proof authority
    InitializeProofAccount {
        /// [ProofType] indicating the size of the account
        proof_type: ProofType,
    },

    /// Write to the provided proof account
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writable]` Proof account, must be previously initialized
    /// 1. `[signer]` Proof authority
    Write {
        /// Offset to start writing proof, expressed as `u64`.
        offset: u64,
        /// Data to replace the existing proof data
        data: &'a [u8],
    },

    /// Close the provided proof account, draining lamports to recipient
    /// account
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writable]` Proof account, must be previously initialized
    /// 1. `[signer]` Proof authority
    /// 2. `[]` Receiver of account lamports
    CloseAccount,
}

impl<'a> SlashingInstruction<'a> {
    /// Unpacks a byte buffer into a [SlashingInstruction].
    pub fn unpack(input: &'a [u8]) -> Result<Self, ProgramError> {
        const U8_BYTES: usize = 1;
        const U32_BYTES: usize = 4;
        const U64_BYTES: usize = 8;

        let (&tag, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;
        Ok(match tag {
            0 => {
                let proof_type = rest
                    .get(..U8_BYTES)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u8::from_le_bytes)
                    .ok_or(ProgramError::InvalidInstructionData)?;
                let proof_type = ProofType::from(proof_type);
                Self::InitializeProofAccount { proof_type }
            }
            1 => {
                let offset = rest
                    .get(..U64_BYTES)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(ProgramError::InvalidInstructionData)?;
                let (length, data) = rest[U64_BYTES..].split_at(U32_BYTES);
                let length = u32::from_le_bytes(
                    length
                        .try_into()
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                ) as usize;

                Self::Write {
                    offset,
                    data: &data[..length],
                }
            }
            2 => Self::CloseAccount,
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    /// Packs a [SlashingInstruction] into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match self {
            Self::InitializeProofAccount { proof_type } => {
                let proof_type = u8::from(*proof_type);
                buf.push(0);
                buf.extend_from_slice(&proof_type.to_le_bytes());
            }
            Self::Write { offset, data } => {
                buf.push(1);
                buf.extend_from_slice(&offset.to_le_bytes());
                buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
                buf.extend_from_slice(data);
            }
            Self::CloseAccount => buf.push(2),
        };
        buf
    }
}

/// Create a `SlashingInstruction::InitializeProofAccount` instruction
pub fn initialize_proof_account(
    proof_account: &Pubkey,
    proof_type: ProofType,
    fee_payer: &Pubkey,
    authority: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*proof_account, true),
            AccountMeta::new(*fee_payer, true),
            AccountMeta::new_readonly(*authority, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: SlashingInstruction::InitializeProofAccount { proof_type }.pack(),
    }
}

/// Create a `SlashingInstruction::Write` instruction
pub fn write(proof_account: &Pubkey, signer: &Pubkey, offset: u64, data: &[u8]) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*proof_account, false),
            AccountMeta::new_readonly(*signer, true),
        ],
        data: SlashingInstruction::Write { offset, data }.pack(),
    }
}

/// Create a `SlashingInstruction::CloseAccount` instruction
pub fn close_account(proof_account: &Pubkey, signer: &Pubkey, receiver: &Pubkey) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*proof_account, false),
            AccountMeta::new_readonly(*signer, true),
            AccountMeta::new(*receiver, false),
        ],
        data: SlashingInstruction::CloseAccount.pack(),
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::state::tests::TEST_BYTES, solana_program::program_error::ProgramError};

    #[test]
    fn serialize_initialize_duplicate_block_proof() {
        let instruction = SlashingInstruction::InitializeProofAccount {
            proof_type: ProofType::DuplicateBlockProof,
        };
        let expected = vec![0, 0];
        assert_eq!(instruction.pack(), expected);
        assert_eq!(SlashingInstruction::unpack(&expected).unwrap(), instruction);
    }

    #[test]
    fn serialize_initialize_invalid_proof() {
        let instruction = SlashingInstruction::InitializeProofAccount {
            proof_type: ProofType::InvalidType,
        };
        let expected = vec![0, u8::MAX];
        assert_eq!(instruction.pack(), expected);
        assert_eq!(SlashingInstruction::unpack(&expected).unwrap(), instruction);
    }

    #[test]
    fn serialize_write() {
        let data = &TEST_BYTES;
        let offset = 0u64;
        let instruction = SlashingInstruction::Write { offset: 0, data };
        let mut expected = vec![1];
        expected.extend_from_slice(&offset.to_le_bytes());
        expected.extend_from_slice(&(data.len() as u32).to_le_bytes());
        expected.extend_from_slice(data);
        assert_eq!(instruction.pack(), expected);
        assert_eq!(SlashingInstruction::unpack(&expected).unwrap(), instruction);
    }

    #[test]
    fn serialize_close_account() {
        let instruction = SlashingInstruction::CloseAccount;
        let expected = vec![2];
        assert_eq!(instruction.pack(), expected);
        assert_eq!(SlashingInstruction::unpack(&expected).unwrap(), instruction);
    }

    #[test]
    fn deserialize_invalid_instruction() {
        let mut expected = vec![12];
        expected.extend_from_slice(&TEST_BYTES);
        let err: ProgramError = SlashingInstruction::unpack(&expected).unwrap_err();
        assert_eq!(err, ProgramError::InvalidInstructionData);
    }
}
