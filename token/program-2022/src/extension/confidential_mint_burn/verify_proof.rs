use {
    super::ciphertext_extraction::BurnProofContextInfo, crate::error::TokenError,
    solana_zk_sdk::encryption::pod::elgamal::PodElGamalCiphertext,
};
#[cfg(feature = "zk-ops")]
use {
    super::ciphertext_extraction::{AuditableProofContextInfo, MintProofContextInfo},
    crate::proof::{decode_proof_instruction_context, verify_and_extract_context},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        program_error::ProgramError,
        sysvar::instructions::get_instruction_relative,
    },
    solana_zk_sdk::encryption::pod::elgamal::PodElGamalPubkey,
    solana_zk_sdk::zk_elgamal_proof_program::instruction::ProofInstruction,
    solana_zk_sdk::zk_elgamal_proof_program::proof_data::{
        BatchedGroupedCiphertext3HandlesValidityProofContext,
        BatchedGroupedCiphertext3HandlesValidityProofData, BatchedRangeProofContext,
        BatchedRangeProofU128Data, BatchedRangeProofU64Data,
        CiphertextCommitmentEqualityProofContext, CiphertextCommitmentEqualityProofData,
    },
    spl_pod::optional_keys::OptionalNonZeroElGamalPubkey,
    std::slice::Iter,
};

/// Verify zero-knowledge proofs needed for a [ConfidentialMint] instruction and
/// return the corresponding proof context information.
#[cfg(feature = "zk-ops")]
pub fn verify_mint_proof(
    account_info_iter: &mut Iter<'_, AccountInfo<'_>>,
    proof_instruction_offset: i64,
) -> Result<MintProofContextInfo, ProgramError> {
    if proof_instruction_offset == 0 {
        let range_proof_context = verify_and_extract_context::<
            BatchedRangeProofU64Data,
            BatchedRangeProofContext,
        >(account_info_iter, proof_instruction_offset, None)?;
        let ciphertext_validity_proof_context =
            verify_and_extract_context::<
                BatchedGroupedCiphertext3HandlesValidityProofData,
                BatchedGroupedCiphertext3HandlesValidityProofContext,
            >(account_info_iter, proof_instruction_offset, None)?;

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

        Ok(MintProofContextInfo::verify_and_extract(
            &ciphertext_validity_proof_context,
            &range_proof_context,
        )?)
    }
}

/// Verify zero-knowledge proofs needed for a [ConfidentialBurn] instruction and
/// return the corresponding proof context information.
#[cfg(feature = "zk-ops")]
pub fn verify_burn_proof(
    account_info_iter: &mut Iter<'_, AccountInfo<'_>>,
    proof_instruction_offset: i64,
) -> Result<BurnProofContextInfo, ProgramError> {
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

        // The `TransferProofContextInfo` constructor verifies the consistency of the
        // individual proof context and generates a `TransferWithFeeProofInfo` struct
        // that is used to process the rest of the token-2022 logic.
        let transfer_proof_context = BurnProofContextInfo::verify_and_extract(
            &equality_proof_context,
            &ciphertext_validity_proof_context,
            &range_proof_context,
        )?;

        Ok(transfer_proof_context)
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

        Ok(BurnProofContextInfo::verify_and_extract(
            &equality_proof_context,
            &ciphertext_validity_proof_context,
            &range_proof_context,
        )?)
    }
}

/// Validates the auditor mint/burn amounts from the instruction against those
/// from zk-proofs
#[cfg(feature = "zk-ops")]
pub fn validate_auditor_ciphertext(
    auditor_elgamal_pubkey: &OptionalNonZeroElGamalPubkey,
    proof_context: &impl AuditableProofContextInfo,
    auditor_lo: &PodElGamalCiphertext,
    auditor_hi: &PodElGamalCiphertext,
) -> Result<(), ProgramError> {
    if let Some(auditor_pk) = Into::<Option<PodElGamalPubkey>>::into(*auditor_elgamal_pubkey) {
        // Check that the auditor encryption public key is consistent with what was
        // actually used to generate the zkp.
        if proof_context.auditor_pubkey() != &auditor_pk {
            return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
        }

        if auditor_lo != &proof_context.auditor_amount_lo()? {
            return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
        }
        if auditor_hi != &proof_context.auditor_amount_hi()? {
            return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
        }
    }

    Ok(())
}
