//! Utility functions to simplify the handling of ZK ElGamal proof program
//! instruction data in SPL crates

use {
    bytemuck::Pod,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        instruction::Instruction,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvar::instructions::get_instruction_relative,
    },
    solana_zk_sdk::zk_elgamal_proof_program::{
        self,
        instruction::ProofInstruction,
        proof_data::{ProofType, ZkProofData},
        state::ProofContextState,
    },
    spl_pod::bytemuck::pod_from_bytes,
    std::{num::NonZeroI8, slice::Iter},
};

/// Checks that the supplied program ID is correct for the ZK ElGamal proof
/// program
pub fn check_zk_elgamal_proof_program_account(
    zk_elgamal_proof_program_id: &Pubkey,
) -> ProgramResult {
    if zk_elgamal_proof_program_id != &solana_zk_sdk::zk_elgamal_proof_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

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
    if instruction.program_id != zk_elgamal_proof_program::id()
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

        let record_account_data = record_account.data.borrow();
        let raw_proof_data = record_account_data
            .get(start_offset..end_offset)
            .ok_or(ProgramError::AccountDataTooSmall)?;

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

impl<'a, T> ProofLocation<'a, T> {
    /// Returns true if the proof location is an instruction offset
    pub fn is_instruction_offset(&self) -> bool {
        match self {
            Self::InstructionOffset(_, _) => true,
            Self::ContextStateAccount(_) => false,
        }
    }
}

/// A proof data type to distinguish between proof data included as part of
/// zk-token proof instruction data and proof data stored in a record account.
#[derive(Clone, Copy)]
pub enum ProofData<'a, T> {
    /// The proof data
    InstructionData(&'a T),
    /// The address of a record account containing the proof data and its byte
    /// offset
    RecordAccount(&'a Pubkey, u32),
}

/// Verify zero-knowledge proof and return the corresponding proof context.
pub fn verify_and_extract_context<'a, T: Pod + ZkProofData<U>, U: Pod>(
    account_info_iter: &mut Iter<'_, AccountInfo<'a>>,
    proof_instruction_offset: i64,
    sysvar_account_info: Option<&'_ AccountInfo<'a>>,
) -> Result<U, ProgramError> {
    if proof_instruction_offset == 0 {
        // interpret `account_info` as a context state account
        let context_state_account_info = next_account_info(account_info_iter)?;
        check_zk_elgamal_proof_program_account(context_state_account_info.owner)?;
        let context_state_account_data = context_state_account_info.data.borrow();
        let context_state = pod_from_bytes::<ProofContextState<U>>(&context_state_account_data)?;

        if context_state.proof_type != T::PROOF_TYPE.into() {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(context_state.proof_context)
    } else {
        // if sysvar account is not provided, then get the sysvar account
        let sysvar_account_info = if let Some(sysvar_account_info) = sysvar_account_info {
            sysvar_account_info
        } else {
            next_account_info(account_info_iter)?
        };
        let zkp_instruction =
            get_instruction_relative(proof_instruction_offset, sysvar_account_info)?;
        let expected_proof_type = zk_proof_type_to_instruction(T::PROOF_TYPE)?;
        Ok(decode_proof_instruction_context::<T, U>(
            account_info_iter,
            expected_proof_type,
            &zkp_instruction,
        )?)
    }
}

/// Converts a zk proof type to a corresponding ZK ElGamal proof program
/// instruction that verifies the proof.
pub fn zk_proof_type_to_instruction(
    proof_type: ProofType,
) -> Result<ProofInstruction, ProgramError> {
    match proof_type {
        ProofType::ZeroCiphertext => Ok(ProofInstruction::VerifyZeroCiphertext),
        ProofType::CiphertextCiphertextEquality => {
            Ok(ProofInstruction::VerifyCiphertextCiphertextEquality)
        }
        ProofType::PubkeyValidity => Ok(ProofInstruction::VerifyPubkeyValidity),
        ProofType::BatchedRangeProofU64 => Ok(ProofInstruction::VerifyBatchedRangeProofU64),
        ProofType::BatchedRangeProofU128 => Ok(ProofInstruction::VerifyBatchedRangeProofU128),
        ProofType::BatchedRangeProofU256 => Ok(ProofInstruction::VerifyBatchedRangeProofU256),
        ProofType::CiphertextCommitmentEquality => {
            Ok(ProofInstruction::VerifyCiphertextCommitmentEquality)
        }
        ProofType::GroupedCiphertext2HandlesValidity => {
            Ok(ProofInstruction::VerifyGroupedCiphertext2HandlesValidity)
        }
        ProofType::BatchedGroupedCiphertext2HandlesValidity => {
            Ok(ProofInstruction::VerifyBatchedGroupedCiphertext2HandlesValidity)
        }
        ProofType::PercentageWithCap => Ok(ProofInstruction::VerifyPercentageWithCap),
        ProofType::GroupedCiphertext3HandlesValidity => {
            Ok(ProofInstruction::VerifyGroupedCiphertext3HandlesValidity)
        }
        ProofType::BatchedGroupedCiphertext3HandlesValidity => {
            Ok(ProofInstruction::VerifyBatchedGroupedCiphertext3HandlesValidity)
        }
        ProofType::Uninitialized => Err(ProgramError::InvalidInstructionData),
    }
}
