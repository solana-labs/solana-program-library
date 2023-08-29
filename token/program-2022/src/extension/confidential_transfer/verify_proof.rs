use {
    crate::{
        check_system_program_account, check_zk_token_proof_program_account,
        extension::confidential_transfer::{ciphertext_extraction::*, instruction::*, *},
        proof::decode_proof_instruction_context,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        msg,
        program::invoke,
        program_error::ProgramError,
        sysvar::instructions::get_instruction_relative,
    },
    solana_zk_token_sdk::zk_token_proof_instruction::{self, ContextStateInfo},
    std::slice::Iter,
};

/// Verify zero-knowledge proof needed for a [ConfigureAccount] instruction and return the
/// corresponding proof context.
pub fn verify_configure_account_proof(
    account_info_iter: &mut Iter<'_, AccountInfo<'_>>,
    proof_instruction_offset: i64,
) -> Result<PubkeyValidityProofContext, ProgramError> {
    if proof_instruction_offset == 0 {
        // interpret `account_info` as a context state account
        let context_state_account_info = next_account_info(account_info_iter)?;
        check_zk_token_proof_program_account(context_state_account_info.owner)?;
        let context_state_account_data = context_state_account_info.data.borrow();
        let context_state = pod_from_bytes::<ProofContextState<PubkeyValidityProofContext>>(
            &context_state_account_data,
        )?;

        if context_state.proof_type != ProofType::PubkeyValidity.into() {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(context_state.proof_context)
    } else {
        // interpret `account_info` as a sysvar
        let sysvar_account_info = next_account_info(account_info_iter)?;
        let zkp_instruction =
            get_instruction_relative(proof_instruction_offset, sysvar_account_info)?;
        Ok(*decode_proof_instruction_context::<
            PubkeyValidityData,
            PubkeyValidityProofContext,
        >(
            ProofInstruction::VerifyPubkeyValidity, &zkp_instruction
        )?)
    }
}

/// Verify zero-knowledge proof needed for a [EmptyAccount] instruction and return the
/// corresponding proof context.
pub fn verify_empty_account_proof(
    account_info_iter: &mut Iter<'_, AccountInfo<'_>>,
    proof_instruction_offset: i64,
) -> Result<ZeroBalanceProofContext, ProgramError> {
    if proof_instruction_offset == 0 {
        // interpret `account_info` as a context state account
        let context_state_account_info = next_account_info(account_info_iter)?;
        check_zk_token_proof_program_account(context_state_account_info.owner)?;
        let context_state_account_data = context_state_account_info.data.borrow();
        let context_state = pod_from_bytes::<ProofContextState<ZeroBalanceProofContext>>(
            &context_state_account_data,
        )?;

        if context_state.proof_type != ProofType::ZeroBalance.into() {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(context_state.proof_context)
    } else {
        // interpret `account_info` as a sysvar
        let sysvar_account_info = next_account_info(account_info_iter)?;
        let zkp_instruction =
            get_instruction_relative(proof_instruction_offset, sysvar_account_info)?;
        Ok(*decode_proof_instruction_context::<
            ZeroBalanceProofData,
            ZeroBalanceProofContext,
        >(
            ProofInstruction::VerifyZeroBalance, &zkp_instruction
        )?)
    }
}

/// Verify zero-knowledge proof needed for a [Withdraw] instruction and return the
/// corresponding proof context.
pub fn verify_withdraw_proof(
    account_info_iter: &mut Iter<'_, AccountInfo<'_>>,
    proof_instruction_offset: i64,
) -> Result<WithdrawProofContext, ProgramError> {
    if proof_instruction_offset == 0 {
        // interpret `account_info` as a context state account
        let context_state_account_info = next_account_info(account_info_iter)?;
        check_zk_token_proof_program_account(context_state_account_info.owner)?;
        let context_state_account_data = context_state_account_info.data.borrow();
        let context_state =
            pod_from_bytes::<ProofContextState<WithdrawProofContext>>(&context_state_account_data)?;

        if context_state.proof_type != ProofType::Withdraw.into() {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(context_state.proof_context)
    } else {
        // interpret `account_info` as a sysvar
        let sysvar_account_info = next_account_info(account_info_iter)?;
        let zkp_instruction =
            get_instruction_relative(proof_instruction_offset, sysvar_account_info)?;
        Ok(*decode_proof_instruction_context::<
            WithdrawData,
            WithdrawProofContext,
        >(
            ProofInstruction::VerifyWithdraw, &zkp_instruction
        )?)
    }
}

/// Verify zero-knowledge proof needed for a [Transfer] instruction without fee and return the
/// corresponding proof context.
///
/// This returns a `Result` type for an `Option<TransferProofContextInfo>` type. If the proof
/// verification fails, then the function returns a suitable error variant. If the proof succeeds
/// to verify, then the function returns a `TransferProofContextInfo` that is wrapped inside
/// `Ok(Some(TransferProofContextInfo))`. If `no_op_on_split_proof_context_state` is `true` and
/// some a split context state account is not initialized, then it returns `Ok(None)`.
#[cfg(feature = "zk-ops")]
pub fn verify_transfer_proof(
    account_info_iter: &mut Iter<'_, AccountInfo<'_>>,
    proof_instruction_offset: i64,
    split_proof_context_state_accounts: bool,
    no_op_on_split_proof_context_state: bool,
    close_split_context_state_on_execution: bool,
    source_decrypt_handles: &SourceDecryptHandles,
) -> Result<Option<TransferProofContextInfo>, ProgramError> {
    if proof_instruction_offset == 0 && split_proof_context_state_accounts {
        let equality_proof_context_state_account_info = next_account_info(account_info_iter)?;
        let ciphertext_validity_proof_context_state_account_info =
            next_account_info(account_info_iter)?;
        let range_proof_context_state_account_info = next_account_info(account_info_iter)?;

        if no_op_on_split_proof_context_state
            && check_system_program_account(equality_proof_context_state_account_info.owner).is_ok()
        {
            msg!("Equality proof context state account not initialized");
            return Ok(None);
        }

        if no_op_on_split_proof_context_state
            && check_system_program_account(
                ciphertext_validity_proof_context_state_account_info.owner,
            )
            .is_ok()
        {
            msg!("Ciphertext validity proof context state account not initialized");
            return Ok(None);
        }

        if no_op_on_split_proof_context_state
            && check_system_program_account(range_proof_context_state_account_info.owner).is_ok()
        {
            msg!("Range proof context state account not initialized");
            return Ok(None);
        }

        let equality_proof_context =
            verify_equality_proof(equality_proof_context_state_account_info)?;
        let ciphertext_validity_proof_context =
            verify_ciphertext_validity_proof(ciphertext_validity_proof_context_state_account_info)?;
        let range_proof_context = verify_range_proof(range_proof_context_state_account_info)?;

        let transfer_proof_context = TransferProofContextInfo::new(
            &equality_proof_context,
            &ciphertext_validity_proof_context,
            &range_proof_context,
            source_decrypt_handles,
        )?;

        if close_split_context_state_on_execution {
            let lamport_destination_account_info = next_account_info(account_info_iter)?;
            let context_state_account_authority_info = next_account_info(account_info_iter)?;

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

        Ok(Some(transfer_proof_context))
    } else if proof_instruction_offset == 0 && !split_proof_context_state_accounts {
        // interpret `account_info` as a context state account
        let context_state_account_info = next_account_info(account_info_iter)?;
        check_zk_token_proof_program_account(context_state_account_info.owner)?;
        let context_state_account_data = context_state_account_info.data.borrow();
        let context_state =
            pod_from_bytes::<ProofContextState<TransferProofContext>>(&context_state_account_data)?;

        if context_state.proof_type != ProofType::Transfer.into() {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Some(context_state.proof_context.into()))
    } else {
        // interpret `account_info` as sysvar
        let sysvar_account_info = next_account_info(account_info_iter)?;
        let zkp_instruction =
            get_instruction_relative(proof_instruction_offset, sysvar_account_info)?;
        let proof_context = (*decode_proof_instruction_context::<
            TransferData,
            TransferProofContext,
        >(ProofInstruction::VerifyTransfer, &zkp_instruction)?)
        .into();

        Ok(Some(proof_context))
    }
}

/// Verify zero-knowledge proof needed for a [Transfer] instruction with fee and return the
/// corresponding proof context.
pub fn verify_transfer_with_fee_proof(
    account_info_iter: &mut Iter<'_, AccountInfo<'_>>,
    proof_instruction_offset: i64,
    split_proof_context_state_accounts: bool,
) -> Result<TransferWithFeeProofContext, ProgramError> {
    if proof_instruction_offset == 0 && split_proof_context_state_accounts {
        // TODO: decode each context state accounts and check consistency between them
        unimplemented!()
    } else if proof_instruction_offset == 0 && !split_proof_context_state_accounts {
        // interpret `account_info` as a context state account
        let context_state_account_info = next_account_info(account_info_iter)?;
        check_zk_token_proof_program_account(context_state_account_info.owner)?;
        let context_state_account_data = context_state_account_info.data.borrow();
        let context_state = pod_from_bytes::<ProofContextState<TransferWithFeeProofContext>>(
            &context_state_account_data,
        )?;

        if context_state.proof_type != ProofType::TransferWithFee.into() {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(context_state.proof_context)
    } else {
        // interpret `account_info` as sysvar
        let sysvar_account_info = next_account_info(account_info_iter)?;
        let zkp_instruction =
            get_instruction_relative(proof_instruction_offset, sysvar_account_info)?;
        Ok(*decode_proof_instruction_context::<
            TransferWithFeeData,
            TransferWithFeeProofContext,
        >(
            ProofInstruction::VerifyTransferWithFee,
            &zkp_instruction,
        )?)
    }
}

/// Verify and process equality proof for [Transfer] and [TransferWithFee] instructions.
fn verify_equality_proof(
    account_info: &AccountInfo<'_>,
) -> Result<CiphertextCommitmentEqualityProofContext, ProgramError> {
    check_zk_token_proof_program_account(account_info.owner)?;
    let context_state_account_data = account_info.data.borrow();
    let equality_proof_context_state = pod_from_bytes::<
        ProofContextState<CiphertextCommitmentEqualityProofContext>,
    >(&context_state_account_data)?;

    if equality_proof_context_state.proof_type != ProofType::CiphertextCommitmentEquality.into() {
        return Err(ProgramError::InvalidInstructionData);
    }

    Ok(equality_proof_context_state.proof_context)
}

/// Verify and process ciphertext validity proof for [Transfer] and [TransferWithFee] instructions.
fn verify_ciphertext_validity_proof(
    account_info: &AccountInfo<'_>,
) -> Result<BatchedGroupedCiphertext2HandlesValidityProofContext, ProgramError> {
    check_zk_token_proof_program_account(account_info.owner)?;
    let context_state_account_data = account_info.data.borrow();
    let ciphertext_validity_proof_context_state = pod_from_bytes::<
        ProofContextState<BatchedGroupedCiphertext2HandlesValidityProofContext>,
    >(&context_state_account_data)?;

    if ciphertext_validity_proof_context_state.proof_type
        != ProofType::BatchedGroupedCiphertext2HandlesValidity.into()
    {
        return Err(ProgramError::InvalidInstructionData);
    }

    Ok(ciphertext_validity_proof_context_state.proof_context)
}

/// Verify and process range proof for [Transfer] and [TransferWithFee] instructions.
fn verify_range_proof(
    account_info: &AccountInfo<'_>,
) -> Result<BatchedRangeProofContext, ProgramError> {
    check_zk_token_proof_program_account(account_info.owner)?;
    let context_state_account_data = account_info.data.borrow();
    let range_proof_context_state =
        pod_from_bytes::<ProofContextState<BatchedRangeProofContext>>(&context_state_account_data)?;

    if range_proof_context_state.proof_type != ProofType::BatchedRangeProofU128.into() {
        return Err(ProgramError::InvalidInstructionData);
    }

    Ok(range_proof_context_state.proof_context)
}
