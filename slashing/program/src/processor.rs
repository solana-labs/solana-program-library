//! Program state processor

use {
    crate::{
        duplicate_block_proof::DuplicateBlockProofData, error::SlashingError,
        instruction::SlashingInstruction, state::SlashingProofData,
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

fn verify_proof_data<'a, T>(slot: Slot, pubkey: Pubkey, proof_data: &'a [u8]) -> ProgramResult
where
    T: SlashingProofData + Deserialize<'a>,
{
    if proof_data.len() < T::PROOF_TYPE.proof_account_length() {
        return Err(ProgramError::InvalidAccountData);
    }
    let proof_data: T =
        bincode::deserialize(proof_data).map_err(|_| SlashingError::DeserializationError)?;

    SlashingProofData::verify_proof(proof_data, slot, pubkey)?;

    // TODO: follow up PR will record this violation in context state account. just
    // log for now.
    msg!(
        "{} committed a {} violation in slot {}. This incident will be recorded",
        pubkey,
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
    let instruction = SlashingInstruction::unpack(input)?;
    let account_info_iter = &mut accounts.iter();
    let proof_data_info = next_account_info(account_info_iter);

    match instruction {
        SlashingInstruction::DuplicateBlockProof {
            offset,
            slot,
            node_pubkey,
        } => {
            msg!(
                "SlashingInstruction::DuplicateBlockProof {} {}",
                slot,
                node_pubkey
            );
            let proof_data = &proof_data_info?.data.borrow()[offset as usize..];
            verify_proof_data::<DuplicateBlockProofData>(slot, node_pubkey, proof_data)?;
            Ok(())
        }
    }
}
