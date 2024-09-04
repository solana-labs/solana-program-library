use {
    super::ciphertext_extraction::BurnProofContextInfo,
    crate::{check_zk_elgamal_proof_program_account, error::TokenError},
    solana_zk_sdk::encryption::pod::elgamal::PodElGamalCiphertext,
};
#[cfg(feature = "zk-ops")]
use {
    super::ciphertext_extraction::{AuditableProofContextInfo, MintProofContextInfo},
    crate::check_system_program_account,
    //crate::extension::confidential_transfer::verify_proof::{
    //    verify_equality_proof, verify_transfer_range_proof,
    //},
    crate::proof::{decode_proof_instruction_context, verify_and_extract_context},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        msg,
        program::invoke,
        program_error::ProgramError,
        sysvar::instructions::get_instruction_relative,
    },
    solana_zk_sdk::encryption::pod::elgamal::PodElGamalPubkey,
    solana_zk_sdk::zk_elgamal_proof_program::instruction::{
        self as zk_token_proof_instruction, ContextStateInfo, ProofInstruction,
    },
    solana_zk_sdk::zk_elgamal_proof_program::proof_data::{
        BatchedGroupedCiphertext3HandlesValidityProofContext,
        BatchedGroupedCiphertext3HandlesValidityProofData, BatchedRangeProofContext,
        BatchedRangeProofU128Data, BatchedRangeProofU64Data,
        CiphertextCommitmentEqualityProofContext, CiphertextCommitmentEqualityProofData,
        ProofType,
    },
    solana_zk_sdk::zk_elgamal_proof_program::state::ProofContextState,
    spl_pod::bytemuck::pod_from_bytes,
    spl_pod::optional_keys::OptionalNonZeroElGamalPubkey,
    std::slice::Iter,
};

/// Verify zero-knowledge proofs needed for a [ConfidentialMint] instruction and
/// return the corresponding proof context information.
#[cfg(feature = "zk-ops")]
pub fn verify_mint_proof(
    account_info_iter: &mut Iter<'_, AccountInfo<'_>>,
    proof_instruction_offset: i64,
    close_split_context_state_on_execution: bool,
) -> Result<MintProofContextInfo, ProgramError> {
    if proof_instruction_offset == 0 {
        // interpret `account_info` as a context state account
        let range_proof_account_info = next_account_info(account_info_iter)?;
        let cipher_text_validity_account_info = next_account_info(account_info_iter)?;
        let range_proof_context = verify_batched_u64_range_proof(range_proof_account_info)?;
        let ciphertext_validity_proof_context =
            verify_3_ciphertext_validity_proof(cipher_text_validity_account_info)?;

        if close_split_context_state_on_execution {
            let lamport_destination_account_info = next_account_info(account_info_iter)?;
            let context_state_account_authority_info = next_account_info(account_info_iter)?;
            let _zk_token_proof_program = next_account_info(account_info_iter)?;

            msg!("Closing equality proof context state account");
            invoke(
                &zk_token_proof_instruction::close_context_state(
                    ContextStateInfo {
                        context_state_account: cipher_text_validity_account_info.key,
                        context_state_authority: context_state_account_authority_info.key,
                    },
                    lamport_destination_account_info.key,
                ),
                &[
                    cipher_text_validity_account_info.clone(),
                    lamport_destination_account_info.clone(),
                    context_state_account_authority_info.clone(),
                ],
            )?;

            msg!("Closing range proof context state account");
            invoke(
                &zk_token_proof_instruction::close_context_state(
                    ContextStateInfo {
                        context_state_account: range_proof_account_info.key,
                        context_state_authority: context_state_account_authority_info.key,
                    },
                    lamport_destination_account_info.key,
                ),
                &[
                    range_proof_account_info.clone(),
                    lamport_destination_account_info.clone(),
                    context_state_account_authority_info.clone(),
                ],
            )?;
        }

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

        let range_proof_context = decode_proof_instruction_context::<
            BatchedRangeProofU64Data,
            BatchedRangeProofContext,
        >(
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
    close_split_context_state_on_execution: bool,
) -> Result<BurnProofContextInfo, ProgramError> {
    if proof_instruction_offset == 0 {
        let equality_proof_context_state_account_info = next_account_info(account_info_iter)?;
        let ciphertext_validity_proof_context_state_account_info =
            next_account_info(account_info_iter)?;
        let range_proof_context_state_account_info = next_account_info(account_info_iter)?;

        if check_system_program_account(equality_proof_context_state_account_info.owner).is_ok() {
            msg!("Equality proof context state account not initialized");
            return Err(ProgramError::UninitializedAccount);
        }

        if check_system_program_account(ciphertext_validity_proof_context_state_account_info.owner)
            .is_ok()
        {
            msg!("Ciphertext validity proof context state account not initialized");
            return Err(ProgramError::UninitializedAccount);
        }

        if check_system_program_account(range_proof_context_state_account_info.owner).is_ok() {
            msg!("Range proof context state account not initialized");
            return Err(ProgramError::UninitializedAccount);
        }

        let equality_proof_context = verify_and_extract_context::<
            CiphertextCommitmentEqualityProofData,
            CiphertextCommitmentEqualityProofContext,
        >(
            account_info_iter,
            proof_instruction_offset,
            None,
        )?;
        let ciphertext_validity_proof_context = verify_3_ciphertext_validity_proof(
            ciphertext_validity_proof_context_state_account_info,
        )?;

        let range_proof_context =
            verify_and_extract_context::<BatchedRangeProofU128Data, BatchedRangeProofContext>(
                account_info_iter,
                proof_instruction_offset,
                None,
            )?;

        // The `TransferProofContextInfo` constructor verifies the consistency of the
        // individual proof context and generates a `TransferWithFeeProofInfo` struct
        // that is used to process the rest of the token-2022 logic.
        let transfer_proof_context = BurnProofContextInfo::verify_and_extract(
            &equality_proof_context,
            &ciphertext_validity_proof_context,
            &range_proof_context,
        )?;

        if close_split_context_state_on_execution {
            let lamport_destination_account_info = next_account_info(account_info_iter)?;
            let context_state_account_authority_info = next_account_info(account_info_iter)?;
            let _zk_token_proof_program = next_account_info(account_info_iter)?;

            msg!("Closing equality proof context state account");
            invoke(
                &zk_token_proof_instruction::close_context_state(
                    ContextStateInfo {
                        context_state_account: equality_proof_context_state_account_info.key,
                        context_state_authority: context_state_account_authority_info.key,
                    },
                    lamport_destination_account_info.key,
                ),
                &[
                    equality_proof_context_state_account_info.clone(),
                    lamport_destination_account_info.clone(),
                    context_state_account_authority_info.clone(),
                ],
            )?;

            msg!("Closing ciphertext validity proof context state account");
            invoke(
                &zk_token_proof_instruction::close_context_state(
                    ContextStateInfo {
                        context_state_account: ciphertext_validity_proof_context_state_account_info
                            .key,
                        context_state_authority: context_state_account_authority_info.key,
                    },
                    lamport_destination_account_info.key,
                ),
                &[
                    ciphertext_validity_proof_context_state_account_info.clone(),
                    lamport_destination_account_info.clone(),
                    context_state_account_authority_info.clone(),
                ],
            )?;

            msg!("Closing range proof context state account");
            invoke(
                &zk_token_proof_instruction::close_context_state(
                    ContextStateInfo {
                        context_state_account: range_proof_context_state_account_info.key,
                        context_state_authority: context_state_account_authority_info.key,
                    },
                    lamport_destination_account_info.key,
                ),
                &[
                    range_proof_context_state_account_info.clone(),
                    lamport_destination_account_info.clone(),
                    context_state_account_authority_info.clone(),
                ],
            )?;
        }

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

/// Verify and process batched u64 range proof for [ConfidentialMint]
/// instruction
pub fn verify_batched_u64_range_proof(
    account_info: &AccountInfo<'_>,
) -> Result<BatchedRangeProofContext, ProgramError> {
    check_zk_elgamal_proof_program_account(account_info.owner)?;
    let context_state_account_data = account_info.data.borrow();
    let range_proof_context_state =
        pod_from_bytes::<ProofContextState<BatchedRangeProofContext>>(&context_state_account_data)?;

    if range_proof_context_state.proof_type != ProofType::BatchedRangeProofU64.into() {
        return Err(ProgramError::InvalidInstructionData);
    }

    Ok(range_proof_context_state.proof_context)
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

/// Verify and process ciphertext validity proof for [ConfidentialMint] and
/// [ConfidentialBurn] instructions.
pub fn verify_3_ciphertext_validity_proof(
    account_info: &AccountInfo<'_>,
) -> Result<BatchedGroupedCiphertext3HandlesValidityProofContext, ProgramError> {
    check_zk_elgamal_proof_program_account(account_info.owner)?;
    let context_state_account_data = account_info.data.borrow();
    let ciphertext_validity_proof_context_state = pod_from_bytes::<
        ProofContextState<BatchedGroupedCiphertext3HandlesValidityProofContext>,
    >(&context_state_account_data)?;

    if ciphertext_validity_proof_context_state.proof_type
        != ProofType::BatchedGroupedCiphertext3HandlesValidity.into()
    {
        return Err(ProgramError::InvalidInstructionData);
    }

    Ok(ciphertext_validity_proof_context_state.proof_context)
}
