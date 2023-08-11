use {
    crate::{
        check_zk_token_proof_program_account,
        extension::confidential_transfer::{instruction::*, *},
        proof::decode_proof_instruction_context,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        program_error::ProgramError,
        sysvar::instructions::get_instruction_relative,
    },
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
pub fn verify_transfer_proof(
    account_info_iter: &mut Iter<'_, AccountInfo<'_>>,
    proof_instruction_offset: i64,
    split_proof_context_state_accounts: bool,
) -> Result<TransferProofContext, ProgramError> {
    if proof_instruction_offset == 0 && split_proof_context_state_accounts {
        // TODO: decode each context state accounts and check consistency between them
        unimplemented!()
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

        Ok(context_state.proof_context)
    } else {
        // interpret `account_info` as sysvar
        let sysvar_account_info = next_account_info(account_info_iter)?;
        let zkp_instruction =
            get_instruction_relative(proof_instruction_offset, sysvar_account_info)?;
        Ok(*decode_proof_instruction_context::<
            TransferData,
            TransferProofContext,
        >(
            ProofInstruction::VerifyTransfer, &zkp_instruction
        )?)
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
