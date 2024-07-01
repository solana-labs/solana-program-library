#[cfg(feature = "zk-ops")]
use {
    super::ciphertext_extraction::MintProofContextInfo,
    crate::check_zk_token_proof_program_account,
    crate::extension::confidential_transfer::verify_proof::verify_ciphertext_validity_proof,
    crate::proof::decode_proof_instruction_context,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        program_error::ProgramError,
        sysvar::instructions::get_instruction_relative,
    },
    solana_zk_token_sdk::instruction::BatchedRangeProofContext,
    solana_zk_token_sdk::instruction::BatchedRangeProofU64Data,
    solana_zk_token_sdk::instruction::{
        BatchedGroupedCiphertext2HandlesValidityProofContext,
        BatchedGroupedCiphertext2HandlesValidityProofData,
    },
    solana_zk_token_sdk::zk_token_proof_instruction::ProofInstruction,
    solana_zk_token_sdk::{instruction::ProofType, zk_token_proof_state::ProofContextState},
    spl_pod::bytemuck::pod_from_bytes,
    std::slice::Iter,
};

/// Verify zero-knowledge proof needed for a [ConfigureAccount] instruction and
/// return the corresponding proof context.
#[cfg(feature = "zk-ops")]
pub fn verify_mint_proof(
    account_info_iter: &mut Iter<'_, AccountInfo<'_>>,
    proof_instruction_offset: i64,
) -> Result<MintProofContextInfo, ProgramError> {
    if proof_instruction_offset == 0 {
        // interpret `account_info` as a context state account
        let range_proof_account_info = next_account_info(account_info_iter)?;
        let cipher_text_validity_account_info = next_account_info(account_info_iter)?;
        let range_proof_context = verify_batched_u64_range_proof(range_proof_account_info)?;
        let ciphertext_validity_proof_context =
            verify_ciphertext_validity_proof(cipher_text_validity_account_info)?;

        Ok(MintProofContextInfo::verify_and_extract(
            &ciphertext_validity_proof_context,
            &range_proof_context,
        )?)
    } else {
        let sysvar_account_info = next_account_info(account_info_iter)?;
        let range_proof_instruction =
            get_instruction_relative(proof_instruction_offset, sysvar_account_info)?;

        let ciphertext_validity_instruction =
            get_instruction_relative(proof_instruction_offset + 1, sysvar_account_info)?;

        let range_proof_context = *decode_proof_instruction_context::<
            BatchedRangeProofU64Data,
            BatchedRangeProofContext,
        >(
            ProofInstruction::VerifyBatchedRangeProofU64,
            &range_proof_instruction,
        )?;

        let ciphertext_validity_proof_context = *decode_proof_instruction_context::<
            BatchedGroupedCiphertext2HandlesValidityProofData,
            BatchedGroupedCiphertext2HandlesValidityProofContext,
        >(
            ProofInstruction::VerifyGroupedCiphertext2HandlesValidity,
            &ciphertext_validity_instruction,
        )?;

        Ok(MintProofContextInfo::verify_and_extract(
            &ciphertext_validity_proof_context,
            &range_proof_context,
        )?)
    }
}

/// Verify and process batched u64 range proof for [ConfidentialMint] instruction
pub fn verify_batched_u64_range_proof(
    account_info: &AccountInfo<'_>,
) -> Result<BatchedRangeProofContext, ProgramError> {
    check_zk_token_proof_program_account(account_info.owner)?;
    let context_state_account_data = account_info.data.borrow();
    let range_proof_context_state =
        pod_from_bytes::<ProofContextState<BatchedRangeProofContext>>(&context_state_account_data)?;

    if range_proof_context_state.proof_type != ProofType::BatchedRangeProofU64.into() {
        return Err(ProgramError::InvalidInstructionData);
    }

    Ok(range_proof_context_state.proof_context)
}
