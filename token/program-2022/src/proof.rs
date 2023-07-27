//! Helper for processing instruction data from ZK Token proof program

use {
    bytemuck::Pod,
    solana_program::{instruction::Instruction, msg, program_error::ProgramError, pubkey::Pubkey},
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

/// An `i8` type guaranteed to be non-zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NonZeroI8(i8);
impl TryFrom<i8> for NonZeroI8 {
    type Error = ProgramError;
    fn try_from(n: i8) -> Result<Self, Self::Error> {
        if n == 0 {
            Err(ProgramError::InvalidArgument)
        } else {
            Ok(Self(n))
        }
    }
}
impl From<NonZeroI8> for i8 {
    fn from(n: NonZeroI8) -> Self {
        n.0
    }
}

/// A proof location type meant to be used for arguments to instruction constructors.
#[derive(Clone, Copy)]
pub enum ProofLocation<'a, T> {
    /// The proof is included in the same transaction of a corresponding token-2022 instruction.
    InstructionOffset(NonZeroI8, &'a T),
    /// The proof is pre-verified into a context state account.
    ContextStateAccount(&'a Pubkey),
}
