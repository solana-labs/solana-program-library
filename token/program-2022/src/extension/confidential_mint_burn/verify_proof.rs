use crate::error::TokenError;
#[cfg(feature = "zk-ops")]
use {
    crate::proof::verify_and_extract_context,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        program_error::ProgramError,
    },
    solana_zk_sdk::zk_elgamal_proof_program::proof_data::{
        BatchedGroupedCiphertext3HandlesValidityProofContext,
        BatchedGroupedCiphertext3HandlesValidityProofData, BatchedRangeProofContext,
        BatchedRangeProofU128Data, CiphertextCommitmentEqualityProofContext,
        CiphertextCommitmentEqualityProofData,
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
    let sysvar_account_info = if proof_instruction_offset != 0 {
        Some(next_account_info(account_info_iter)?)
    } else {
        None
    };

    let equality_proof_context = verify_and_extract_context::<
        CiphertextCommitmentEqualityProofData,
        CiphertextCommitmentEqualityProofContext,
    >(
        account_info_iter,
        proof_instruction_offset,
        sysvar_account_info,
    )?;

    let proof_instruction_offset = if proof_instruction_offset != 0 {
        proof_instruction_offset + 1
    } else {
        proof_instruction_offset
    };

    let ciphertext_validity_proof_context = verify_and_extract_context::<
        BatchedGroupedCiphertext3HandlesValidityProofData,
        BatchedGroupedCiphertext3HandlesValidityProofContext,
    >(
        account_info_iter,
        proof_instruction_offset,
        sysvar_account_info,
    )?;

    let proof_instruction_offset = if proof_instruction_offset != 0 {
        proof_instruction_offset + 1
    } else {
        proof_instruction_offset
    };

    let range_proof_context =
        verify_and_extract_context::<BatchedRangeProofU128Data, BatchedRangeProofContext>(
            account_info_iter,
            proof_instruction_offset,
            sysvar_account_info,
        )?;

    Ok(MintProofContext::verify_and_extract(
        &equality_proof_context,
        &ciphertext_validity_proof_context,
        &range_proof_context,
    )
    .map_err(|e| -> TokenError { e.into() })?)
}

/// Verify zero-knowledge proofs needed for a [ConfidentialBurn] instruction and
/// return the corresponding proof context information.
#[cfg(feature = "zk-ops")]
pub fn verify_burn_proof(
    account_info_iter: &mut Iter<'_, AccountInfo<'_>>,
    proof_instruction_offset: i64,
) -> Result<BurnProofContext, ProgramError> {
    let sysvar_account_info = if proof_instruction_offset != 0 {
        Some(next_account_info(account_info_iter)?)
    } else {
        None
    };

    let equality_proof_context = verify_and_extract_context::<
        CiphertextCommitmentEqualityProofData,
        CiphertextCommitmentEqualityProofContext,
    >(
        account_info_iter,
        proof_instruction_offset,
        sysvar_account_info,
    )?;

    let proof_instruction_offset = if proof_instruction_offset != 0 {
        proof_instruction_offset + 1
    } else {
        proof_instruction_offset
    };

    let ciphertext_validity_proof_context = verify_and_extract_context::<
        BatchedGroupedCiphertext3HandlesValidityProofData,
        BatchedGroupedCiphertext3HandlesValidityProofContext,
    >(
        account_info_iter,
        proof_instruction_offset,
        sysvar_account_info,
    )?;

    let proof_instruction_offset = if proof_instruction_offset != 0 {
        proof_instruction_offset + 1
    } else {
        proof_instruction_offset
    };

    let range_proof_context =
        verify_and_extract_context::<BatchedRangeProofU128Data, BatchedRangeProofContext>(
            account_info_iter,
            proof_instruction_offset,
            sysvar_account_info,
        )?;

    Ok(BurnProofContext::verify_and_extract(
        &equality_proof_context,
        &ciphertext_validity_proof_context,
        &range_proof_context,
    )
    .map_err(|e| -> TokenError { e.into() })?)
}
