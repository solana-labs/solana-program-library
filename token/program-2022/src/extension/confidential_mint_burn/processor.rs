#[cfg(feature = "zk-ops")]
use spl_token_confidential_transfer_ciphertext_arithmetic as ciphertext_arithmetic;
use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            confidential_mint_burn::{
                instruction::{
                    BurnInstructionData, ConfidentialMintBurnInstruction, InitializeMintData,
                    MintInstructionData, RotateSupplyElGamalPubkeyData,
                    UpdateDecryptableSupplyData,
                },
                verify_proof::{verify_burn_proof, verify_mint_proof},
                ConfidentialMintBurn,
            },
            confidential_transfer::{ConfidentialTransferAccount, ConfidentialTransferMint},
            BaseStateWithExtensions, BaseStateWithExtensionsMut, PodStateWithExtensionsMut,
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
    solana_zk_sdk::{
        encryption::pod::{auth_encryption::PodAeCiphertext, elgamal::PodElGamalPubkey},
        zk_elgamal_proof_program::proof_data::{
            CiphertextCiphertextEqualityProofContext, CiphertextCiphertextEqualityProofData,
        },
    },
    spl_token_confidential_transfer_proof_extraction::instruction::verify_and_extract_context,
};

/// Processes an [InitializeMint] instruction.
fn process_initialize_mint(accounts: &[AccountInfo], data: &InitializeMintData) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;

    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(mint_data)?;
    let mint_burn_extension = mint.init_extension::<ConfidentialMintBurn>(true)?;

    mint_burn_extension.supply_elgamal_pubkey = data.supply_elgamal_pubkey;
    mint_burn_extension.decryptable_supply = data.decryptable_supply;

    Ok(())
}

/// Processes an [RotateSupplyElGamal] instruction.
#[cfg(feature = "zk-ops")]
fn process_rotate_supply_elgamal_pubkey(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &RotateSupplyElGamalPubkeyData,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(mint_data)?;
    let mint_authority = mint.base.mint_authority;
    let mint_burn_extension = mint.get_extension_mut::<ConfidentialMintBurn>()?;

    let proof_context = verify_and_extract_context::<
        CiphertextCiphertextEqualityProofData,
        CiphertextCiphertextEqualityProofContext,
    >(
        account_info_iter,
        data.proof_instruction_offset as i64,
        None,
    )?;

    let supply_elgamal_pubkey: Option<PodElGamalPubkey> =
        mint_burn_extension.supply_elgamal_pubkey.into();
    let Some(supply_elgamal_pubkey) = supply_elgamal_pubkey else {
        return Err(TokenError::InvalidState.into());
    };

    if !supply_elgamal_pubkey.eq(&proof_context.first_pubkey) {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }
    if mint_burn_extension.confidential_supply != proof_context.first_ciphertext {
        return Err(ProgramError::InvalidInstructionData);
    }

    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();
    let authority = mint_authority.ok_or(TokenError::NoAuthorityExists)?;

    Processor::validate_owner(
        program_id,
        &authority,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    mint_burn_extension.supply_elgamal_pubkey = proof_context.second_pubkey;
    mint_burn_extension.confidential_supply = proof_context.second_ciphertext;

    Ok(())
}

/// Processes an [UpdateAuthority] instruction.
fn process_update_decryptable_supply(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_decryptable_supply: PodAeCiphertext,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(mint_data)?;
    let mint_authority = mint.base.mint_authority;
    let mint_burn_extension = mint.get_extension_mut::<ConfidentialMintBurn>()?;

    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();
    let authority = mint_authority.ok_or(TokenError::NoAuthorityExists)?;

    Processor::validate_owner(
        program_id,
        &authority,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    mint_burn_extension.decryptable_supply = new_decryptable_supply;

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

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(mint_data)?;
    let mint_authority = mint.base.mint_authority;

    let auditor_elgamal_pubkey = mint
        .get_extension::<ConfidentialTransferMint>()?
        .auditor_elgamal_pubkey;
    let mint_burn_extension = mint.get_extension_mut::<ConfidentialMintBurn>()?;

    let proof_context = verify_mint_proof(
        account_info_iter,
        data.equality_proof_instruction_offset,
        data.ciphertext_validity_proof_instruction_offset,
        data.range_proof_instruction_offset,
    )?;

    check_program_account(token_account_info.owner)?;
    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = PodStateWithExtensionsMut::<PodAccount>::unpack(token_account_data)?;

    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();
    let authority = mint_authority.ok_or(TokenError::NoAuthorityExists)?;

    Processor::validate_owner(
        program_id,
        &authority,
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

    assert!(!token_account.base.is_native());

    let confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    confidential_transfer_account.valid_as_destination()?;

    if proof_context.mint_pubkeys.destination != confidential_transfer_account.elgamal_pubkey {
        return Err(ProgramError::InvalidInstructionData);
    }

    if let Some(auditor_pubkey) = Option::<PodElGamalPubkey>::from(auditor_elgamal_pubkey) {
        if auditor_pubkey != proof_context.mint_pubkeys.auditor {
            return Err(ProgramError::InvalidInstructionData);
        }
    }

    confidential_transfer_account.pending_balance_lo = ciphertext_arithmetic::add(
        &confidential_transfer_account.pending_balance_lo,
        &proof_context
            .mint_amount_ciphertext_lo
            .try_extract_ciphertext(0)
            .map_err(|_| ProgramError::InvalidAccountData)?,
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;
    confidential_transfer_account.pending_balance_hi = ciphertext_arithmetic::add(
        &confidential_transfer_account.pending_balance_hi,
        &proof_context
            .mint_amount_ciphertext_hi
            .try_extract_ciphertext(0)
            .map_err(|_| ProgramError::InvalidAccountData)?,
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;

    confidential_transfer_account.increment_pending_balance_credit_counter()?;

    // update supply
    if mint_burn_extension.supply_elgamal_pubkey != proof_context.mint_pubkeys.supply {
        return Err(ProgramError::InvalidInstructionData);
    }
    let current_supply = mint_burn_extension.confidential_supply;
    mint_burn_extension.confidential_supply = ciphertext_arithmetic::add_with_lo_hi(
        &current_supply,
        &proof_context
            .mint_amount_ciphertext_lo
            .try_extract_ciphertext(2)
            .map_err(|_| ProgramError::InvalidAccountData)?,
        &proof_context
            .mint_amount_ciphertext_hi
            .try_extract_ciphertext(2)
            .map_err(|_| ProgramError::InvalidAccountData)?,
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;
    mint_burn_extension.decryptable_supply = data.new_decryptable_supply;

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
    let mint_burn_extension = mint.get_extension_mut::<ConfidentialMintBurn>()?;

    let proof_context = verify_burn_proof(
        account_info_iter,
        data.equality_proof_instruction_offset,
        data.ciphertext_validity_proof_instruction_offset,
        data.range_proof_instruction_offset,
    )?;

    check_program_account(token_account_info.owner)?;
    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = PodStateWithExtensionsMut::<PodAccount>::unpack(token_account_data)?;

    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

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
    if proof_context.burn_pubkeys.source != confidential_transfer_account.elgamal_pubkey {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }

    let burn_amount_lo = &proof_context
        .burn_amount_ciphertext_lo
        .try_extract_ciphertext(0)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let burn_amount_hi = &proof_context
        .burn_amount_ciphertext_hi
        .try_extract_ciphertext(0)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    let new_source_available_balance = ciphertext_arithmetic::subtract_with_lo_hi(
        &confidential_transfer_account.available_balance,
        burn_amount_lo,
        burn_amount_hi,
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;

    // Check that the computed available balance is consistent with what was
    // actually used to generate the zkp on the client side.
    if new_source_available_balance != proof_context.remaining_balance_ciphertext {
        return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
    }

    confidential_transfer_account.available_balance = new_source_available_balance;
    confidential_transfer_account.decryptable_available_balance =
        data.new_decryptable_available_balance;

    if let Some(auditor_pubkey) = Option::<PodElGamalPubkey>::from(auditor_elgamal_pubkey) {
        if auditor_pubkey != proof_context.burn_pubkeys.auditor {
            return Err(ProgramError::InvalidInstructionData);
        }
    }

    // update supply
    if mint_burn_extension.supply_elgamal_pubkey != proof_context.burn_pubkeys.supply {
        return Err(ProgramError::InvalidInstructionData);
    }
    let current_supply = mint_burn_extension.confidential_supply;
    mint_burn_extension.confidential_supply = ciphertext_arithmetic::subtract_with_lo_hi(
        &current_supply,
        &proof_context
            .burn_amount_ciphertext_lo
            .try_extract_ciphertext(2)
            .map_err(|_| ProgramError::InvalidAccountData)?,
        &proof_context
            .burn_amount_ciphertext_hi
            .try_extract_ciphertext(2)
            .map_err(|_| ProgramError::InvalidAccountData)?,
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;

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
        ConfidentialMintBurnInstruction::RotateSupplyElGamalPubkey => {
            msg!("ConfidentialMintBurnInstruction::RotateSupplyElGamal");
            let data = decode_instruction_data::<RotateSupplyElGamalPubkeyData>(input)?;
            process_rotate_supply_elgamal_pubkey(program_id, accounts, data)
        }
        ConfidentialMintBurnInstruction::UpdateDecryptableSupply => {
            msg!("ConfidentialMintBurnInstruction::UpdateDecryptableSupply");
            let data = decode_instruction_data::<UpdateDecryptableSupplyData>(input)?;
            process_update_decryptable_supply(program_id, accounts, data.new_decryptable_supply)
        }
        ConfidentialMintBurnInstruction::Mint => {
            msg!("ConfidentialMintBurnInstruction::ConfidentialMint");
            let data = decode_instruction_data::<MintInstructionData>(input)?;
            process_confidential_mint(program_id, accounts, data)
        }
        ConfidentialMintBurnInstruction::Burn => {
            msg!("ConfidentialMintBurnInstruction::ConfidentialBurn");
            let data = decode_instruction_data::<BurnInstructionData>(input)?;
            process_confidential_burn(program_id, accounts, data)
        }
    }
}
