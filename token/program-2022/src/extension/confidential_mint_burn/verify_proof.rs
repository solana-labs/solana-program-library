use crate::error::TokenError;
#[cfg(feature = "zk-ops")]
use {
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
    spl_token_confidential_transfer_proof_extraction::{
        burn::BurnProofContext, instruction::verify_and_extract_context, mint::MintProofContext,
    },
    std::slice::Iter,
};

/// Verify zero-knowledge proofs needed for a [ConfidentialMint] instruction and
/// return the corresponding proof context information.
#[cfg(feature = "zk-ops")]
pub fn verify_mint_proof(
    account_info_iter: &mut Iter<'_, AccountInfo<'_>>,
    equality_proof_instruction_offset: i8,
    ciphertext_validity_proof_instruction_offset: i8,
    range_proof_instruction_offset: i8,
) -> Result<MintProofContext, ProgramError> {
    let sysvar_account_info = if equality_proof_instruction_offset != 0 {
        Some(next_account_info(account_info_iter)?)
    } else {
        None
    };

    let equality_proof_context = verify_and_extract_context::<
        CiphertextCommitmentEqualityProofData,
        CiphertextCommitmentEqualityProofContext,
    >(
        account_info_iter,
        equality_proof_instruction_offset as i64,
        sysvar_account_info,
    )?;

    let ciphertext_validity_proof_context = verify_and_extract_context::<
        BatchedGroupedCiphertext3HandlesValidityProofData,
        BatchedGroupedCiphertext3HandlesValidityProofContext,
    >(
        account_info_iter,
        ciphertext_validity_proof_instruction_offset as i64,
        sysvar_account_info,
    )?;

    let range_proof_context =
        verify_and_extract_context::<BatchedRangeProofU128Data, BatchedRangeProofContext>(
            account_info_iter,
            range_proof_instruction_offset as i64,
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
    equality_proof_instruction_offset: i8,
    ciphertext_validity_proof_instruction_offset: i8,
    range_proof_instruction_offset: i8,
) -> Result<BurnProofContext, ProgramError> {
    let sysvar_account_info = if equality_proof_instruction_offset != 0 {
        Some(next_account_info(account_info_iter)?)
    } else {
        None
    };

    let equality_proof_context = verify_and_extract_context::<
        CiphertextCommitmentEqualityProofData,
        CiphertextCommitmentEqualityProofContext,
    >(
        account_info_iter,
        equality_proof_instruction_offset as i64,
        sysvar_account_info,
    )?;

    let ciphertext_validity_proof_context = verify_and_extract_context::<
        BatchedGroupedCiphertext3HandlesValidityProofData,
        BatchedGroupedCiphertext3HandlesValidityProofContext,
    >(
        account_info_iter,
        ciphertext_validity_proof_instruction_offset as i64,
        sysvar_account_info,
    )?;

    let range_proof_context =
        verify_and_extract_context::<BatchedRangeProofU128Data, BatchedRangeProofContext>(
            account_info_iter,
            range_proof_instruction_offset as i64,
            sysvar_account_info,
        )?;

    Ok(BurnProofContext::verify_and_extract(
        &equality_proof_context,
        &ciphertext_validity_proof_context,
        &range_proof_context,
    )
    .map_err(|e| -> TokenError { e.into() })?)
}
