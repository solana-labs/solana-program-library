//! Program state processor

use {
    crate::{
        duplicate_block_proof::DuplicateBlockProofData,
        error::SlashingError,
        instruction::{
            decode_instruction_data, decode_instruction_type, DuplicateBlockProofInstructionData,
            SlashingInstruction,
        },
        state::SlashingProofData,
    },
    serde::Deserialize,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        clock::Slot,
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
};

fn verify_proof_data<'a, T>(slot: Slot, pubkey: &Pubkey, proof_data: &'a [u8]) -> ProgramResult
where
    T: SlashingProofData + Deserialize<'a>,
{
    if proof_data.len() < T::PROOF_TYPE.proof_account_length() {
        return Err(ProgramError::InvalidAccountData);
    }
    let proof_data: T =
        bincode::deserialize(proof_data).map_err(|_| SlashingError::ShredDeserializationError)?;

    SlashingProofData::verify_proof(proof_data, slot, pubkey)?;

    // TODO: follow up PR will record this violation in context state account. just
    // log for now.
    msg!(
        "{} violation verified in slot {}. This incident will be recorded",
        T::PROOF_TYPE.violation_str(),
        slot
    );
    Ok(())
}

/// Instruction processor
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction_type = decode_instruction_type(input)?;
    let account_info_iter = &mut accounts.iter();
    let proof_data_info = next_account_info(account_info_iter);

    match instruction_type {
        SlashingInstruction::DuplicateBlockProof => {
            let data = decode_instruction_data::<DuplicateBlockProofInstructionData>(input)?;
            let proof_data = &proof_data_info?.data.borrow()[u64::from(data.offset) as usize..];
            verify_proof_data::<DuplicateBlockProofData>(
                data.slot.into(),
                &data.node_pubkey,
                proof_data,
            )?;
            Ok(())
        }
    }
}
