//! Program instructions

use {
    crate::{error::SlashingError, id},
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        clock::Slot,
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_pod::{
        bytemuck::{pod_from_bytes, pod_get_packed_len},
        primitives::PodU64,
    },
};

/// Instructions supported by the program
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, TryFromPrimitive, IntoPrimitive)]
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
    ///
    /// Data expected by this instruction:
    ///   DuplicateBlockProofInstructionData
    DuplicateBlockProof,
}

/// Data expected by
/// `SlashingInstruction::DuplicateBlockProof`
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct DuplicateBlockProofInstructionData {
    /// Offset into the proof account to begin reading, expressed as `u64`
    pub(crate) offset: PodU64,
    /// Slot for which the violation occured
    pub(crate) slot: PodU64,
    /// Identity pubkey of the Node that signed the duplicate block
    pub(crate) node_pubkey: Pubkey,
}

/// Utility function for encoding instruction data
pub(crate) fn encode_instruction<D: Pod>(
    accounts: Vec<AccountMeta>,
    instruction: SlashingInstruction,
    instruction_data: &D,
) -> Instruction {
    let mut data = vec![u8::from(instruction)];
    data.extend_from_slice(bytemuck::bytes_of(instruction_data));
    Instruction {
        program_id: id(),
        accounts,
        data,
    }
}

/// Utility function for decoding just the instruction type
pub(crate) fn decode_instruction_type(input: &[u8]) -> Result<SlashingInstruction, ProgramError> {
    if input.is_empty() {
        Err(ProgramError::InvalidInstructionData)
    } else {
        SlashingInstruction::try_from(input[0])
            .map_err(|_| SlashingError::InvalidInstruction.into())
    }
}

/// Utility function for decoding instruction data
pub(crate) fn decode_instruction_data<T: Pod>(input_with_type: &[u8]) -> Result<&T, ProgramError> {
    if input_with_type.len() != pod_get_packed_len::<T>().saturating_add(1) {
        Err(ProgramError::InvalidInstructionData)
    } else {
        pod_from_bytes(&input_with_type[1..])
    }
}

/// Create a `SlashingInstruction::DuplicateBlockProof` instruction
pub fn duplicate_block_proof(
    proof_account: &Pubkey,
    offset: u64,
    slot: Slot,
    node_pubkey: Pubkey,
) -> Instruction {
    encode_instruction(
        vec![AccountMeta::new_readonly(*proof_account, false)],
        SlashingInstruction::DuplicateBlockProof,
        &DuplicateBlockProofInstructionData {
            offset: PodU64::from(offset),
            slot: PodU64::from(slot),
            node_pubkey,
        },
    )
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
        let instruction = duplicate_block_proof(&Pubkey::new_unique(), offset, slot, node_pubkey);
        let mut expected = vec![0];
        expected.extend_from_slice(&offset.to_le_bytes());
        expected.extend_from_slice(&slot.to_le_bytes());
        expected.extend_from_slice(&node_pubkey.to_bytes());
        assert_eq!(instruction.data, expected);

        assert_eq!(
            SlashingInstruction::DuplicateBlockProof,
            decode_instruction_type(&instruction.data).unwrap()
        );
        let instruction_data: &DuplicateBlockProofInstructionData =
            decode_instruction_data(&instruction.data).unwrap();

        assert_eq!(instruction_data.offset, offset.into());
        assert_eq!(instruction_data.slot, slot.into());
        assert_eq!(instruction_data.node_pubkey, node_pubkey);
    }

    #[test]
    fn deserialize_invalid_instruction() {
        let mut expected = vec![12];
        expected.extend_from_slice(&TEST_BYTES);
        let err: ProgramError = decode_instruction_type(&expected).unwrap_err();
        assert_eq!(err, SlashingError::InvalidInstruction.into());
    }
}
