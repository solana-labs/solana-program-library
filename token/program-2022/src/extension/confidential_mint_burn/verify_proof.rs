use crate::error::TokenError;
#[cfg(feature = "zk-ops")]
use {
    crate::proof::{decode_proof_instruction_context, verify_and_extract_context},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        program_error::ProgramError,
        sysvar::instructions::get_instruction_relative,
    },
    solana_zk_sdk::zk_elgamal_proof_program::instruction::ProofInstruction,
    solana_zk_sdk::zk_elgamal_proof_program::proof_data::{
        BatchedGroupedCiphertext3HandlesValidityProofContext,
        BatchedGroupedCiphertext3HandlesValidityProofData, BatchedRangeProofContext,
        BatchedRangeProofU128Data, BatchedRangeProofU64Data,
        CiphertextCommitmentEqualityProofContext, CiphertextCommitmentEqualityProofData,
    },
    spl_token_confidential_transfer_proof_extraction::burn::BurnProofContext,
    spl_token_confidential_transfer_proof_extraction::mint::MintProofContext,
    std::slice::Iter,
};

/// Verify zero-knowledge proofs needed for a [ConfidentialMint] instruction and
/// return the corresponding proof context information.
#[cfg(feature = "zk-ops")]
pub fn verify_mint_proof(
    account_info_iter: &mut Iter<'_, AccountInfo<'_>>,
    proof_instruction_offset: i64,
) -> Result<MintProofContext, ProgramError> {
    if proof_instruction_offset == 0 {
        let equality_proof_context = verify_and_extract_context::<
            CiphertextCommitmentEqualityProofData,
            CiphertextCommitmentEqualityProofContext,
        >(account_info_iter, proof_instruction_offset, None)?;

        let ciphertext_validity_proof_context =
            verify_and_extract_context::<
                BatchedGroupedCiphertext3HandlesValidityProofData,
                BatchedGroupedCiphertext3HandlesValidityProofContext,
            >(account_info_iter, proof_instruction_offset, None)?;

        let range_proof_context = verify_and_extract_context::<
            BatchedRangeProofU128Data,
            BatchedRangeProofContext,
        >(account_info_iter, proof_instruction_offset, None)?;

        Ok(MintProofContext::verify_and_extract(
            &equality_proof_context,
            &ciphertext_validity_proof_context,
            &range_proof_context,
        )
        .map_err(|e| -> TokenError { e.into() })?)
    } else {
        let sysvar_account_info = next_account_info(account_info_iter)?;

        let equality_proof_instruction =
            get_instruction_relative(proof_instruction_offset, sysvar_account_info)?;
        let ciphertext_validity_instruction =
            get_instruction_relative(proof_instruction_offset + 1, sysvar_account_info)?;
        let range_proof_instruction =
            get_instruction_relative(proof_instruction_offset + 2, sysvar_account_info)?;

        let equality_proof_context = decode_proof_instruction_context::<
            CiphertextCommitmentEqualityProofData,
            CiphertextCommitmentEqualityProofContext,
        >(
            account_info_iter,
            ProofInstruction::VerifyBatchedRangeProofU64,
            &equality_proof_instruction,
        )?;

        let range_proof_context =
            decode_proof_instruction_context::<BatchedRangeProofU64Data, BatchedRangeProofContext>(
                account_info_iter,
                ProofInstruction::VerifyBatchedRangeProofU64,
                &range_proof_instruction,
            )?;

        let ciphertext_validity_proof_context = decode_proof_instruction_context::<
            BatchedGroupedCiphertext3HandlesValidityProofData,
            BatchedGroupedCiphertext3HandlesValidityProofContext,
        >(
            account_info_iter,
            ProofInstruction::VerifyGroupedCiphertext2HandlesValidity,
            &ciphertext_validity_instruction,
        )?;

        Ok(MintProofContext::verify_and_extract(
            &equality_proof_context,
            &ciphertext_validity_proof_context,
            &range_proof_context,
        )
        .map_err(|e| -> TokenError { e.into() })?)
    }
}

/// Verify zero-knowledge proofs needed for a [ConfidentialBurn] instruction and
/// return the corresponding proof context information.
#[cfg(feature = "zk-ops")]
pub fn verify_burn_proof(
    account_info_iter: &mut Iter<'_, AccountInfo<'_>>,
    proof_instruction_offset: i64,
) -> Result<BurnProofContext, ProgramError> {
    if proof_instruction_offset == 0 {
        let equality_proof_context = verify_and_extract_context::<
            CiphertextCommitmentEqualityProofData,
            CiphertextCommitmentEqualityProofContext,
        >(account_info_iter, proof_instruction_offset, None)?;

        let ciphertext_validity_proof_context =
            verify_and_extract_context::<
                BatchedGroupedCiphertext3HandlesValidityProofData,
                BatchedGroupedCiphertext3HandlesValidityProofContext,
            >(account_info_iter, proof_instruction_offset, None)?;

        let range_proof_context = verify_and_extract_context::<
            BatchedRangeProofU128Data,
            BatchedRangeProofContext,
        >(account_info_iter, proof_instruction_offset, None)?;

        Ok(BurnProofContext::verify_and_extract(
            &equality_proof_context,
            &ciphertext_validity_proof_context,
            &range_proof_context,
        )
        .map_err(|e| -> TokenError { e.into() })?)
    } else {
        let sysvar_account_info = next_account_info(account_info_iter)?;
        let equality_proof_instruction =
            get_instruction_relative(proof_instruction_offset, sysvar_account_info)?;
        let range_proof_instruction =
            get_instruction_relative(proof_instruction_offset + 1, sysvar_account_info)?;

        let ciphertext_validity_instruction =
            get_instruction_relative(proof_instruction_offset + 2, sysvar_account_info)?;

        let equality_proof_context = decode_proof_instruction_context::<
            CiphertextCommitmentEqualityProofData,
            CiphertextCommitmentEqualityProofContext,
        >(
            account_info_iter,
            ProofInstruction::VerifyCiphertextCommitmentEquality,
            &equality_proof_instruction,
        )?;

        let range_proof_context = decode_proof_instruction_context::<
            BatchedRangeProofU128Data,
            BatchedRangeProofContext,
        >(
            account_info_iter,
            ProofInstruction::VerifyBatchedRangeProofU128,
            &range_proof_instruction,
        )?;

        let ciphertext_validity_proof_context = decode_proof_instruction_context::<
            BatchedGroupedCiphertext3HandlesValidityProofData,
            BatchedGroupedCiphertext3HandlesValidityProofContext,
        >(
            account_info_iter,
            ProofInstruction::VerifyGroupedCiphertext2HandlesValidity,
            &ciphertext_validity_instruction,
        )?;

        Ok(BurnProofContext::verify_and_extract(
            &equality_proof_context,
            &ciphertext_validity_proof_context,
            &range_proof_context,
        )
        .map_err(|e| -> TokenError { e.into() })?)
    }
}
