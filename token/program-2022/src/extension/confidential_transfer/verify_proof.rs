use {
    crate::{
        extension::{
            confidential_transfer::{ciphertext_extraction::*, instruction::*},
            transfer_fee::TransferFee,
        },
        proof::verify_and_extract_context,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        program_error::ProgramError,
    },
    std::slice::Iter,
};

/// Verify zero-knowledge proof needed for a [ConfigureAccount] instruction and
/// return the corresponding proof context.
pub fn verify_configure_account_proof<'a>(
    account_info_iter: &mut Iter<'a, AccountInfo<'a>>,
    proof_instruction_offset: i64,
) -> Result<PubkeyValidityProofContext, ProgramError> {
    verify_and_extract_context::<PubkeyValidityData, PubkeyValidityProofContext>(
        account_info_iter,
        proof_instruction_offset,
        None,
    )
}

/// Verify zero-knowledge proof needed for a [EmptyAccount] instruction and
/// return the corresponding proof context.
pub fn verify_empty_account_proof<'a>(
    account_info_iter: &mut Iter<'a, AccountInfo<'a>>,
    proof_instruction_offset: i64,
) -> Result<ZeroBalanceProofContext, ProgramError> {
    verify_and_extract_context::<ZeroBalanceProofData, ZeroBalanceProofContext>(
        account_info_iter,
        proof_instruction_offset,
        None,
    )
}

/// Verify zero-knowledge proof needed for a [Withdraw] instruction and return
/// the corresponding proof context.
pub fn verify_withdraw_proof<'a>(
    account_info_iter: &mut Iter<'a, AccountInfo<'a>>,
    proof_instruction_offset: i64,
) -> Result<WithdrawProofContext, ProgramError> {
    verify_and_extract_context::<WithdrawData, WithdrawProofContext>(
        account_info_iter,
        proof_instruction_offset,
        None,
    )
}

/// Verify zero-knowledge proof needed for a [Transfer] instruction without fee
/// and return the corresponding proof context.
#[cfg(feature = "zk-ops")]
pub fn verify_transfer_proof<'a>(
    account_info_iter: &mut Iter<'a, AccountInfo<'a>>,
    equality_proof_instruction_offset: i64,
    ciphertext_validity_proof_instruction_offset: i64,
    range_proof_instruction_offset: i64,
    source_decrypt_handles: &SourceDecryptHandles,
) -> Result<TransferProofContextInfo, ProgramError> {
    let sysvar_account_info = if equality_proof_instruction_offset != 0
        || ciphertext_validity_proof_instruction_offset != 0
        || range_proof_instruction_offset != 0
    {
        Some(next_account_info(account_info_iter)?)
    } else {
        None
    };

    let equality_proof_context = verify_and_extract_context::<
        CiphertextCommitmentEqualityProofData,
        CiphertextCommitmentEqualityProofContext,
    >(
        account_info_iter,
        equality_proof_instruction_offset,
        sysvar_account_info,
    )?;

    let ciphertext_validity_proof_context = verify_and_extract_context::<
        BatchedGroupedCiphertext2HandlesValidityProofData,
        BatchedGroupedCiphertext2HandlesValidityProofContext,
    >(
        account_info_iter,
        equality_proof_instruction_offset,
        sysvar_account_info,
    )?;

    let range_proof_context =
        verify_and_extract_context::<BatchedRangeProofU128Data, BatchedRangeProofContext>(
            account_info_iter,
            equality_proof_instruction_offset,
            sysvar_account_info,
        )?;

    // The `TransferProofContextInfo` constructor verifies the consistency of the
    // individual proof context and generates a `TransferWithFeeProofInfo` struct
    // that is used to process the rest of the token-2022 logic.
    let transfer_proof_context = TransferProofContextInfo::verify_and_extract(
        &equality_proof_context,
        &ciphertext_validity_proof_context,
        &range_proof_context,
        source_decrypt_handles,
    )?;

    Ok(transfer_proof_context)
}

/// Verify zero-knowledge proof needed for a [Transfer] instruction with fee and
/// return the corresponding proof context.
#[cfg(feature = "zk-ops")]
pub fn verify_transfer_with_fee_proof<'a>(
    account_info_iter: &mut Iter<'a, AccountInfo<'a>>,
    equality_proof_instruction_offset: i64,
    transfer_amount_ciphertext_validity_proof_instruction_offset: i64,
    fee_sigma_proof_instruction_offset: i64,
    fee_ciphertext_validity_proof_instruction_offset: i64,
    range_proof_instruction_offset: i64,
    source_decrypt_handles: &SourceDecryptHandles,
    fee_parameters: &TransferFee,
) -> Result<TransferWithFeeProofContextInfo, ProgramError> {
    let sysvar_account_info = if equality_proof_instruction_offset != 0
        || transfer_amount_ciphertext_validity_proof_instruction_offset != 0
        || fee_sigma_proof_instruction_offset != 0
        || fee_ciphertext_validity_proof_instruction_offset != 0
        || range_proof_instruction_offset != 0
    {
        Some(next_account_info(account_info_iter)?)
    } else {
        None
    };

    let equality_proof_context = verify_and_extract_context::<
        CiphertextCommitmentEqualityProofData,
        CiphertextCommitmentEqualityProofContext,
    >(
        account_info_iter,
        equality_proof_instruction_offset,
        sysvar_account_info,
    )?;

    let transfer_amount_ciphertext_validity_proof_context = verify_and_extract_context::<
        BatchedGroupedCiphertext2HandlesValidityProofData,
        BatchedGroupedCiphertext2HandlesValidityProofContext,
    >(
        account_info_iter,
        equality_proof_instruction_offset,
        sysvar_account_info,
    )?;

    let fee_sigma_proof_context =
        verify_and_extract_context::<FeeSigmaProofData, FeeSigmaProofContext>(
            account_info_iter,
            equality_proof_instruction_offset,
            sysvar_account_info,
        )?;

    let fee_ciphertext_validity_proof_context = verify_and_extract_context::<
        BatchedGroupedCiphertext2HandlesValidityProofData,
        BatchedGroupedCiphertext2HandlesValidityProofContext,
    >(
        account_info_iter,
        equality_proof_instruction_offset,
        sysvar_account_info,
    )?;

    let range_proof_context =
        verify_and_extract_context::<BatchedRangeProofU256Data, BatchedRangeProofContext>(
            account_info_iter,
            equality_proof_instruction_offset,
            sysvar_account_info,
        )?;

    // The `TransferWithFeeProofContextInfo` constructor verifies the consistency of
    // the individual proof context and generates a
    // `TransferWithFeeProofInfo` struct that is used to process the rest of
    // the token-2022 logic. The consistency check includes verifying
    // whether the fee-related zkps were generated with respect to the correct fee
    // parameter that is stored in the mint extension.
    let transfer_with_fee_proof_context = TransferWithFeeProofContextInfo::verify_and_extract(
        &equality_proof_context,
        &transfer_amount_ciphertext_validity_proof_context,
        &fee_sigma_proof_context,
        &fee_ciphertext_validity_proof_context,
        &range_proof_context,
        source_decrypt_handles,
        fee_parameters,
    )?;

    Ok(transfer_with_fee_proof_context)
}
