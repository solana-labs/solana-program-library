#[cfg(feature = "zk-ops")]
use {
    super::verify_proof::validate_auditor_ciphertext, super::verify_proof::verify_burn_proof,
    spl_token_confidential_transfer_ciphertext_arithmetic as ciphertext_arithmetic,
};
use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            confidential_mint_burn::{
                instruction::{
                    BurnInstructionData, ConfidentialMintBurnInstruction, InitializeMintData,
                    MintInstructionData, RotateSupplyElGamalData, UpdateAuthorityData,
                },
                verify_proof::verify_mint_proof,
                ConfidentialMintBurn,
            },
            confidential_transfer::{ConfidentialTransferAccount, ConfidentialTransferMint},
            non_transferable::NonTransferableAccount,
            BaseStateWithExtensions, BaseStateWithExtensionsMut, PodStateWithExtensionsMut,
        },
        instruction::{decode_instruction_data, decode_instruction_type},
        pod::{PodAccount, PodMint},
        processor::Processor,
        proof::decode_proof_instruction_context,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvar::instructions::get_instruction_relative,
    },
    solana_zk_sdk::zk_elgamal_proof_program::{
        instruction::ProofInstruction,
        proof_data::{
            CiphertextCiphertextEqualityProofContext, CiphertextCiphertextEqualityProofData,
        },
    },
};

/// Processes an [InitializeMint] instruction.
fn process_initialize_mint(accounts: &[AccountInfo], data: &InitializeMintData) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;

    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(mint_data)?;
    let conf_mint_burn_ext = mint.init_extension::<ConfidentialMintBurn>(true)?;

    conf_mint_burn_ext.mint_authority = data.authority;
    conf_mint_burn_ext.supply_elgamal_pubkey = data.supply_elgamal_pubkey;

    Ok(())
}

/// Processes an [UpdateAuthority] instruction.
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
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(mint_data)?;
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

/// Processes an [RotateSupplyElGamal] instruction.
#[cfg(feature = "zk-ops")]
fn process_rotate_supply_elgamal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &RotateSupplyElGamalData,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(mint_data)?;
    let mint = mint.get_extension_mut::<ConfidentialMintBurn>()?;

    Processor::validate_owner(
        program_id,
        &mint.mint_authority,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    if data.proof_location != 1 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let sysvar_account_info = next_account_info(account_info_iter)?;
    let equality_proof_instruction =
        get_instruction_relative(data.proof_location as i64, sysvar_account_info)?;

    let proof_context = decode_proof_instruction_context::<
        CiphertextCiphertextEqualityProofData,
        CiphertextCiphertextEqualityProofContext,
    >(
        account_info_iter,
        ProofInstruction::VerifyCiphertextCiphertextEquality,
        &equality_proof_instruction,
    )?;

    if !mint
        .supply_elgamal_pubkey
        .equals(&proof_context.first_pubkey)
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }
    if mint.confidential_supply != proof_context.first_ciphertext {
        return Err(ProgramError::InvalidInstructionData);
    }

    mint.supply_elgamal_pubkey = Some(proof_context.second_pubkey).try_into()?;
    mint.confidential_supply = proof_context.second_ciphertext;

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
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(mint_data)?;

    let auditor_elgamal_pubkey = mint
        .get_extension::<ConfidentialTransferMint>()?
        .auditor_elgamal_pubkey;
    let conf_mint_ext = mint.get_extension_mut::<ConfidentialMintBurn>()?;

    Processor::validate_owner(
        program_id,
        &conf_mint_ext.mint_authority,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

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
    )?;

    if token_account
        .get_extension::<NonTransferableAccount>()
        .is_ok()
    {
        return Err(TokenError::NonTransferable.into());
    }

    let confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    confidential_transfer_account.valid_as_destination()?;

    if proof_context.mint_to_pubkey != confidential_transfer_account.elgamal_pubkey {
        return Err(ProgramError::InvalidInstructionData);
    }

    validate_auditor_ciphertext(
        &auditor_elgamal_pubkey,
        &proof_context,
        &data.audit_amount_lo,
        &data.audit_amount_hi,
    )?;

    confidential_transfer_account.pending_balance_lo = ciphertext_arithmetic::add(
        &confidential_transfer_account.pending_balance_lo,
        &proof_context
            .ciphertext_lo
            .0
            .try_extract_ciphertext(0)
            .map_err(|_| ProgramError::InvalidAccountData)?,
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;
    confidential_transfer_account.pending_balance_hi = ciphertext_arithmetic::add(
        &confidential_transfer_account.pending_balance_hi,
        &proof_context
            .ciphertext_hi
            .0
            .try_extract_ciphertext(0)
            .map_err(|_| ProgramError::InvalidAccountData)?,
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;

    confidential_transfer_account.increment_pending_balance_credit_counter()?;

    // update supply
    if conf_mint_ext.supply_elgamal_pubkey.is_some() {
        let current_supply = conf_mint_ext.confidential_supply;
        conf_mint_ext.confidential_supply = ciphertext_arithmetic::add_with_lo_hi(
            &current_supply,
            &proof_context
                .ciphertext_lo
                .0
                .try_extract_ciphertext(2)
                .map_err(|_| ProgramError::InvalidAccountData)?,
            &proof_context
                .ciphertext_hi
                .0
                .try_extract_ciphertext(2)
                .map_err(|_| ProgramError::InvalidAccountData)?,
        )
        .ok_or(TokenError::CiphertextArithmeticFailed)?;
    }

    Ok(())
}

/// Processes a [ConfidentialBurn] instruction.
#[cfg(feature = "zk-ops")]
fn process_confidential_burn(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &BurnInstructionData,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(mint_data)?;

    let auditor_elgamal_pubkey = mint
        .get_extension::<ConfidentialTransferMint>()?
        .auditor_elgamal_pubkey;
    let conf_mint_ext = mint.get_extension_mut::<ConfidentialMintBurn>()?;

    // The zero-knowledge proof certifies that:
    //   1. the burn amount is encrypted in the correct form
    //   2. the source account has enough balance to burn the amount
    let proof_context = verify_burn_proof(
        account_info_iter,
        data.proof_instruction_offset as i64,
    )?;

    let authority_info = next_account_info(account_info_iter)?;

    check_program_account(token_account_info.owner)?;
    let authority_info_data_len = authority_info.data_len();
    let token_account_data = &mut token_account_info.data.borrow_mut();
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
        account_info_iter.as_slice(),
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
        &proof_context
            .ciphertext_lo
            .0
            .try_extract_ciphertext(0)
            .map_err(|_| ProgramError::InvalidAccountData)?;
    let source_transfer_amount_hi =
        &proof_context
            .ciphertext_hi
            .0
            .try_extract_ciphertext(0)
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let new_source_available_balance = ciphertext_arithmetic::subtract_with_lo_hi(
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
        data.new_decryptable_available_balance;

    validate_auditor_ciphertext(
        &auditor_elgamal_pubkey,
        &proof_context,
        &data.auditor_lo,
        &data.auditor_hi,
    )?;

    // update supply
    if conf_mint_ext.supply_elgamal_pubkey.is_some() {
        let current_supply = conf_mint_ext.confidential_supply;
        conf_mint_ext.confidential_supply = ciphertext_arithmetic::subtract_with_lo_hi(
            &current_supply,
            &proof_context
                .ciphertext_lo
                .0
                .try_extract_ciphertext(2)
                .map_err(|_| ProgramError::InvalidAccountData)?,
            &proof_context
                .ciphertext_hi
                .0
                .try_extract_ciphertext(2)
                .map_err(|_| ProgramError::InvalidAccountData)?,
        )
        .ok_or(TokenError::CiphertextArithmeticFailed)?;
    }

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
            process_initialize_mint(accounts, data)
        }
        ConfidentialMintBurnInstruction::UpdateAuthority => {
            msg!("ConfidentialMintBurnInstruction::UpdateMint");
            let data = decode_instruction_data::<UpdateAuthorityData>(input)?;
            process_update_mint(program_id, accounts, data.new_authority)
        }
        ConfidentialMintBurnInstruction::RotateSupplyElGamal => {
            msg!("ConfidentialMintBurnInstruction::RotateSupplyElGamal");
            let data = decode_instruction_data::<RotateSupplyElGamalData>(input)?;
            process_rotate_supply_elgamal(program_id, accounts, data)
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
