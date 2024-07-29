//! Helper for processing instruction data from ZK Token proof program

use {
    bytemuck::Pod,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        instruction::Instruction,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    solana_zk_token_sdk::{
        instruction::ZkProofData, zk_token_proof_instruction::ProofInstruction,
        zk_token_proof_program,
    },
    std::{num::NonZeroI8, slice::Iter},
};

/// If a proof is to be read from a record account, the proof instruction data
/// must be 5 bytes: 1 byte for the proof type and 4 bytes for the u32 offset
const INSTRUCTION_DATA_LENGTH_WITH_RECORD_ACCOUNT: usize = 5;

/// Decodes the proof context data associated with a zero-knowledge proof
/// instruction.
pub fn decode_proof_instruction_context<T: Pod + ZkProofData<U>, U: Pod>(
    account_info_iter: &mut Iter<'_, AccountInfo<'_>>,
    expected: ProofInstruction,
    instruction: &Instruction,
) -> Result<U, ProgramError> {
    if instruction.program_id != zk_token_proof_program::id()
        || ProofInstruction::instruction_type(&instruction.data) != Some(expected)
    {
        msg!("Unexpected proof instruction");
        return Err(ProgramError::InvalidInstructionData);
    }

    // If the instruction data size is exactly 5 bytes, then interpret it as an
    // offset byte for a record account. This behavior is identical to that of
    // the ZK ElGamal proof program.
    if instruction.data.len() == INSTRUCTION_DATA_LENGTH_WITH_RECORD_ACCOUNT {
        let record_account = next_account_info(account_info_iter)?;

        // first byte is the proof type
        let start_offset = u32::from_le_bytes(instruction.data[1..].try_into().unwrap()) as usize;
        let end_offset = start_offset
            .checked_add(std::mem::size_of::<T>())
            .ok_or(ProgramError::InvalidAccountData)?;
        let raw_proof_data = &record_account.data.borrow()[start_offset..end_offset];

        bytemuck::try_from_bytes::<T>(raw_proof_data)
            .map(|proof_data| *ZkProofData::context_data(proof_data))
            .map_err(|_| ProgramError::InvalidAccountData)
    } else {
        ProofInstruction::proof_data::<T, U>(&instruction.data)
            .map(|proof_data| *ZkProofData::context_data(proof_data))
            .ok_or(ProgramError::InvalidInstructionData)
    }
}

/// A proof location type meant to be used for arguments to instruction
/// constructors.
#[derive(Clone, Copy)]
pub enum ProofLocation<'a, T> {
    /// The proof is included in the same transaction of a corresponding
    /// token-2022 instruction.
    InstructionOffset(NonZeroI8, ProofData<'a, T>),
    /// The proof is pre-verified into a context state account.
    ContextStateAccount(&'a Pubkey),
}

/// A proof data type to distinguish between proof data included as part of
/// instruction data and proof data stored in a record account.
#[derive(Clone, Copy)]
pub enum ProofData<'a, T> {
    /// The proof data
    ProofData(&'a T),
    /// The address of a record account containing the proof data and its byte
    /// offset
    RecordAccount(&'a Pubkey, u32),
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
