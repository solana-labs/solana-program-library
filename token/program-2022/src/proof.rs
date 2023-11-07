//! Helper for processing instruction data from ZK Token proof program

use {
    bytemuck::Pod,
    solana_program::{instruction::Instruction, msg, program_error::ProgramError, pubkey::Pubkey},
    solana_zk_token_sdk::{
        instruction::ZkProofData, zk_token_proof_instruction::ProofInstruction,
        zk_token_proof_program,
    },
    std::num::NonZeroI8,
};

/// Decodes the proof context data associated with a zero-knowledge proof
/// instruction.
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

/// A proof location type meant to be used for arguments to instruction
/// constructors.
#[derive(Clone, Copy)]
pub enum ProofLocation<'a, T> {
    /// The proof is included in the same transaction of a corresponding
    /// token-2022 instruction.
    InstructionOffset(NonZeroI8, &'a T),
    /// The proof is pre-verified into a context state account.
    ContextStateAccount(&'a Pubkey),
}

/// Instruction options for when using split context state accounts
#[derive(Clone, Copy)]
pub struct SplitContextStateAccountsConfig {
    /// If true, execute no op when an associated split proof context state
    /// account is not initialized. Otherwise, fail on an uninitialized
    /// context state account.
    pub no_op_on_uninitialized_split_context_state: bool,
    /// Close associated context states after a complete execution of the
    /// transfer instruction.
    pub close_split_context_state_on_execution: bool,
}
