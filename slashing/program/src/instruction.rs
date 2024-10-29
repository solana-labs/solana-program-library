//! Program instructions

use {
    crate::id,
    solana_program::{
        clock::Slot,
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    std::mem::size_of,
};

/// Instructions supported by the program
#[derive(Clone, Debug, PartialEq)]
pub enum SlashingInstruction {
    /// Submit a slashable violation proof for `node_pubkey`, which indicates
    /// that they submitted a duplicate block to the network
    ///
    ///
    /// Accounts expected by this instruction:
    /// 0. `[]` Proof account, must be previously initialized with the proof
    ///    data.
    ///
    /// We expect the proof account to be properly sized as to hold a duplicate
    /// block proof. See [ProofType] for sizing requirements.
    ///
    /// Deserializing the proof account from `offset` should result in a
    /// [DuplicateBlockProofData]
    DuplicateBlockProof {
        /// Offset into the proof account to begin reading, expressed as `u64`
        offset: u64,
        /// Slot for which the violation occured
        slot: Slot,
        /// Identity pubkey of the Node that signed the duplicate block
        node_pubkey: Pubkey,
    },
}

impl SlashingInstruction {
    /// Unpacks a byte buffer into a [SlashingInstruction].
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        const U64_BYTES: usize = 8;

        let (&tag, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;
        Ok(match tag {
            0 => {
                let offset = rest
                    .get(..U64_BYTES)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(ProgramError::InvalidInstructionData)?;
                let slot = rest
                    .get(U64_BYTES..2 * U64_BYTES)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(ProgramError::InvalidInstructionData)?;

                let node_pubkey = rest
                    .get(2 * U64_BYTES..)
                    .and_then(|slice| slice.try_into().ok())
                    .map(Pubkey::new_from_array)
                    .ok_or(ProgramError::InvalidInstructionData)?;

                Self::DuplicateBlockProof {
                    offset,
                    slot,
                    node_pubkey,
                }
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    /// Packs a [SlashingInstruction] into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match self {
            Self::DuplicateBlockProof {
                offset,
                slot,
                node_pubkey,
            } => {
                buf.push(0);
                buf.extend_from_slice(&offset.to_le_bytes());
                buf.extend_from_slice(&slot.to_le_bytes());
                buf.extend_from_slice(&node_pubkey.to_bytes());
            }
        };
        buf
    }
}

/// Create a `SlashingInstruction::DuplicateBlockProof` instruction
pub fn duplicate_block_proof(
    proof_account: &Pubkey,
    offset: u64,
    slot: Slot,
    node_pubkey: Pubkey,
) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![AccountMeta::new_readonly(*proof_account, false)],
        data: SlashingInstruction::DuplicateBlockProof {
            offset,
            slot,
            node_pubkey,
        }
        .pack(),
    }
}

#[cfg(test)]
mod tests {
    use {super::*, solana_program::program_error::ProgramError};

    const TEST_BYTES: [u8; 8] = [42; 8];

    #[test]
    fn serialize_duplicate_block_proof() {
        let offset = 34;
        let slot = 42;
        let node_pubkey = Pubkey::new_unique();
        let instruction = SlashingInstruction::DuplicateBlockProof {
            offset,
            slot,
            node_pubkey,
        };
        let mut expected = vec![0];
        expected.extend_from_slice(&offset.to_le_bytes());
        expected.extend_from_slice(&slot.to_le_bytes());
        expected.extend_from_slice(&node_pubkey.to_bytes());
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
