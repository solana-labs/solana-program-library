#[cfg(feature = "zk-ops")]
use {
    super::ciphertext_extraction::BurnProofContextInfo,
    super::verify_proof::validate_auditor_ciphertext,
    crate::extension::non_transferable::NonTransferable,
    solana_zk_token_sdk::zk_token_elgamal::ops as syscall,
};
use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            confidential_mint_burn::{
                ciphertext_extraction::mint_burn_amount_target_ciphertext,
                instruction::{
                    BurnInstructionData, ConfidentialMintBurnInstruction, InitializeMintData,
                    MintInstructionData, UpdateMintData,
                },
                verify_proof::verify_mint_proof,
                ConfidentialMintBurn,
            },
            confidential_transfer::{
                ConfidentialTransferAccount, ConfidentialTransferMint, DecryptableBalance,
            },
            non_transferable::NonTransferableAccount,
            BaseStateWithExtensions, BaseStateWithExtensionsMut, PodStateWithExtensions,
            PodStateWithExtensionsMut,
        },
        instruction::{decode_instruction_data, decode_instruction_type},
        pod::{PodAccount, PodMint},
        processor::Processor,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
};

/// Processes an [InitializeMint] instruction.
fn process_initialize_mint(accounts: &[AccountInfo], authority: Pubkey) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;

    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(mint_data)?;
    let mint = mint.init_extension::<ConfidentialMintBurn>(true)?;

    mint.mint_authority = authority;

    Ok(())
}

/// Processes an [UpdateMint] instruction.
fn process_update_mint(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_authority: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(mint_data)?;
    let mint = mint.get_extension_mut::<ConfidentialMintBurn>()?;

    Processor::validate_owner(
        program_id,
        &mint.mint_authority,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    mint.mint_authority = new_authority;

    Ok(())
}

/// Processes a [ConfidentialMint] instruction.
#[cfg(feature = "zk-ops")]
fn process_confidential_mint(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &MintInstructionData,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    check_program_account(mint_info.owner)?;
    let mint_data = &mint_info.data.borrow_mut();
    let mint = PodStateWithExtensions::<PodMint>::unpack(mint_data)?;

    let Ok(conf_transf_ext) = mint.get_extension::<ConfidentialTransferMint>() else {
        msg!("confidential-mint-burn extension initialized on mint without confidential transfer extension");
        return Err(TokenError::ExtensionNotFound.into());
    };

    let Ok(conf_mint_ext) = mint.get_extension::<ConfidentialMintBurn>() else {
        msg!("attempted to confidentially mint tokens on mint without confidential mint-burn extension");
        return Err(TokenError::ExtensionNotFound.into());
    };

    Processor::validate_owner(
        program_id,
        &conf_mint_ext.mint_authority,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    if mint.get_extension::<NonTransferable>().is_ok() {
        return Err(TokenError::NonTransferable.into());
    }

    check_program_account(token_account_info.owner)?;
    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = PodStateWithExtensionsMut::<PodAccount>::unpack(token_account_data)?;

    if token_account.base.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }

    if token_account.base.mint != *mint_info.key {
        return Err(TokenError::MintMismatch.into());
    }

    // Wrapped SOL mint obviously not possible since
    // it'd enable creating SOL out of thin air
    assert!(!token_account.base.is_native());

    let proof_context = verify_mint_proof(
        account_info_iter,
        data.proof_instruction_offset as i64,
        false,
    )?;

    let confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    confidential_transfer_account.valid_as_destination()?;

    if proof_context.destination_pubkey != confidential_transfer_account.elgamal_pubkey {
        return Err(ProgramError::InvalidInstructionData);
    }

    validate_auditor_ciphertext(
        conf_transf_ext,
        &proof_context,
        &data.audit_amount_lo,
        &data.audit_amount_hi,
    )?;

    confidential_transfer_account.pending_balance_lo = syscall::add(
        &confidential_transfer_account.pending_balance_lo,
        &mint_burn_amount_target_ciphertext(&proof_context.ciphertext_lo),
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;
    confidential_transfer_account.pending_balance_hi = syscall::add(
        &confidential_transfer_account.pending_balance_hi,
        &mint_burn_amount_target_ciphertext(&proof_context.ciphertext_hi),
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;

    confidential_transfer_account.increment_pending_balance_credit_counter()?;

    Ok(())
}

/// Processes a [ConfidentialBurn] instruction.
#[cfg(feature = "zk-ops")]
fn process_confidential_burn(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &BurnInstructionData,
) -> ProgramResult {
    use super::verify_proof::verify_burn_proof;

    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;

    let mint_data = &mint_info.data.borrow();
    let mint = PodStateWithExtensions::<PodMint>::unpack(mint_data)?;

    if let Err(_e) = mint.get_extension::<ConfidentialMintBurn>() {
        msg!("attempted to confidentially burn tokens on mint without confidential mint-burn extension");
        return Err(TokenError::ExtensionNotFound.into());
    };

    // The zero-knowledge proof certifies that:
    //   1. the burn amount is encrypted in the correct form
    //   2. the source account has enough balance to burn the amount
    let proof_context = verify_burn_proof(
        account_info_iter,
        data.proof_instruction_offset as i64,
        false,
    )?;

    let authority_info = next_account_info(account_info_iter)?;

    process_source_for_transfer(
        program_id,
        token_account_info,
        mint_info,
        authority_info,
        account_info_iter.as_slice(),
        &proof_context,
        data.new_decryptable_available_balance,
    )?;

    validate_auditor_ciphertext(
        mint.get_extension::<ConfidentialTransferMint>()?,
        &proof_context,
        &data.auditor_lo,
        &data.auditor_hi,
    )?;

    Ok(())
}

/// Processes the changes for the sending party of a confidential transfer
#[allow(clippy::too_many_arguments)]
#[cfg(feature = "zk-ops")]
pub fn process_source_for_transfer(
    program_id: &Pubkey,
    source_account_info: &AccountInfo,
    mint_info: &AccountInfo,
    authority_info: &AccountInfo,
    signers: &[AccountInfo],
    proof_context: &BurnProofContextInfo,
    new_source_decryptable_available_balance: DecryptableBalance,
) -> ProgramResult {
    check_program_account(source_account_info.owner)?;
    let authority_info_data_len = authority_info.data_len();
    let token_account_data = &mut source_account_info.data.borrow_mut();
    let mut token_account = PodStateWithExtensionsMut::<PodAccount>::unpack(token_account_data)?;
    if token_account
        .get_extension::<NonTransferableAccount>()
        .is_ok()
    {
        return Err(TokenError::NonTransferable.into());
    }

    Processor::validate_owner(
        program_id,
        &token_account.base.owner,
        authority_info,
        authority_info_data_len,
        signers,
    )?;

    if token_account.base.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }

    if token_account.base.mint != *mint_info.key {
        return Err(TokenError::MintMismatch.into());
    }

    let confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    confidential_transfer_account.valid_as_source()?;

    // Check that the source encryption public key is consistent with what was
    // actually used to generate the zkp.
    if proof_context.burner_pubkey != confidential_transfer_account.elgamal_pubkey {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }

    let source_transfer_amount_lo =
        mint_burn_amount_target_ciphertext(&proof_context.ciphertext_lo);
    let source_transfer_amount_hi =
        mint_burn_amount_target_ciphertext(&proof_context.ciphertext_hi);

    let new_source_available_balance = syscall::subtract_with_lo_hi(
        &confidential_transfer_account.available_balance,
        &source_transfer_amount_lo,
        &source_transfer_amount_hi,
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;

    // Check that the computed available balance is consistent with what was
    // actually used to generate the zkp on the client side.
    if new_source_available_balance != proof_context.new_burner_ciphertext {
        return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
    }

    confidential_transfer_account.available_balance = new_source_available_balance;
    confidential_transfer_account.decryptable_available_balance =
        new_source_decryptable_available_balance;

    Ok(())
}

#[allow(dead_code)]
pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id)?;

    match decode_instruction_type(input)? {
        ConfidentialMintBurnInstruction::InitializeMint => {
            msg!("ConfidentialMintBurnInstruction::InitializeMint");
            let data = decode_instruction_data::<InitializeMintData>(input)?;
            process_initialize_mint(accounts, data.authority)
        }
        ConfidentialMintBurnInstruction::UpdateMint => {
            msg!("ConfidentialMintBurnInstruction::UpdateMint");
            let data = decode_instruction_data::<UpdateMintData>(input)?;
            process_update_mint(program_id, accounts, data.new_authority)
        }
        ConfidentialMintBurnInstruction::ConfidentialMint => {
            msg!("ConfidentialMintBurnInstruction::ConfidentialMint");
            let data = decode_instruction_data::<MintInstructionData>(input)?;
            process_confidential_mint(program_id, accounts, data)
        }
        ConfidentialMintBurnInstruction::ConfidentialBurn => {
            msg!("ConfidentialMintBurnInstruction::ConfidentialBurn");
            let data = decode_instruction_data::<BurnInstructionData>(input)?;
            process_confidential_burn(program_id, accounts, data)
        }
    }
}
