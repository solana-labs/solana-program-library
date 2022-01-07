use {
    crate::{id, instruction::TokenInstruction, pod::*},
    bytemuck::Pod,
    num_derive::{FromPrimitive, ToPrimitive},
    num_traits::{FromPrimitive, ToPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
    },
};

/// Confidential Transfer extension instructions
#[derive(Clone, Copy, Debug, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum ConfidentialTransferInstruction {
    /// TODO: inline `zk_token_program::instructions::ZkTokenInstruction` here
    Todo,
}

pub(crate) fn decode_instruction_type(
    input: &[u8],
) -> Result<ConfidentialTransferInstruction, ProgramError> {
    if input.is_empty() {
        Err(ProgramError::InvalidInstructionData)
    } else {
        FromPrimitive::from_u8(input[0]).ok_or(ProgramError::InvalidInstructionData)
    }
}

pub(crate) fn decode_instruction_data<T: Pod>(input: &[u8]) -> Result<&T, ProgramError> {
    if input.is_empty() {
        Err(ProgramError::InvalidInstructionData)
    } else {
        pod_from_bytes(&input[1..])
    }
}

fn encode_instruction<T: Pod>(
    accounts: Vec<AccountMeta>,
    instruction_type: ConfidentialTransferInstruction,
    instruction_data: &T,
) -> Instruction {
    let mut data = TokenInstruction::ConfidentialTransferExtension.pack();
    data.push(ToPrimitive::to_u8(&instruction_type).unwrap());
    data.extend_from_slice(bytemuck::bytes_of(instruction_data));
    Instruction {
        program_id: id(),
        accounts,
        data,
    }
}

/// Create a `Todo` instruction
pub fn todo() -> Instruction {
    encode_instruction(vec![], ConfidentialTransferInstruction::Todo, &())
}
