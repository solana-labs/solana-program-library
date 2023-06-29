//! Helper for processing instruction data from ZK Token proof program

use {
    bytemuck::Pod,
    solana_program::{instruction::Instruction, msg, program_error::ProgramError},
    solana_zk_token_sdk::{
        instruction::ZkProofData, zk_token_proof_instruction::ProofInstruction,
        zk_token_proof_program,
    },
};

/// Decodes the proof context data associated with a zero-knowledge proof instruction.
pub fn decode_proof_instruction_context<T: Pod + ZkProofData<U>, U: Pod>(
    expected: ProofInstruction,
    instruction: &Instruction,
) -> Result<&U, ProgramError> {
    if instruction.program_id != zk_token_proof_program::id()
        || ProofInstruction::instruction_type(&instruction.data) != Some(expected)
    {
        msg!("Unexpected proof instruction");
        return Err(ProgramError::InvalidInstructionData);
    }

    ProofInstruction::proof_data::<T, U>(&instruction.data)
        .map(ZkProofData::context_data)
        .ok_or(ProgramError::InvalidInstructionData)
}
