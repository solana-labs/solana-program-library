use crate::state::METADATA_ENTRY_SIZE;
use {
    crate::error::MetadataError,
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        msg,
        program::invoke_signed,
        program_error::ProgramError,
        sysvar::{rent::Rent, Sysvar},
    },
    spl_name_service::instruction::{
        create as create_name_instruction, delete as delete_name_instruction,
        update as update_name_instruction, NameRegistryInstruction,
    },
};

pub const NAMESERVICE_HEADER_SIZE: usize = 96;
pub const PROGRAMDATA_OFFSET: usize = 4;
pub const PROGRAMDATA_AUTHORITY_KEY_OFFSET: usize = 13;

pub fn compute_name_service_account_lamports<'a>(
    size: usize,
    rent_sysvar_info: &AccountInfo<'a>,
    name_account_info: &AccountInfo<'a>,
) -> Result<u64, ProgramError> {
    let rent = &Rent::from_account_info(rent_sysvar_info)?;
    let required_lamports = rent
        .minimum_balance(size + NAMESERVICE_HEADER_SIZE)
        .max(1)
        .saturating_sub(name_account_info.lamports());

    Ok(required_lamports as u64)
}

pub fn delete_name_service_account<'a>(
    name_service_info: &AccountInfo<'a>,
    name_account_info: &AccountInfo<'a>,
    class_account_info: &AccountInfo<'a>,
    refund_info: &AccountInfo<'a>,
    class_authority_signer_seeds: &[&[u8]],
) -> ProgramResult {
    let delete_ix = delete_name_instruction(
        *name_service_info.key,
        *name_account_info.key,
        *class_account_info.key,
        *refund_info.key,
    )?;

    invoke_signed(
        &delete_ix,
        &[
            name_account_info.clone(),
            class_account_info.clone(),
            refund_info.clone(),
        ],
        &[class_authority_signer_seeds],
    )?;

    Ok(())
}

pub fn create_name_service_account<'a>(
    rent_sysvar_info: &AccountInfo<'a>,
    name_account_info: &AccountInfo<'a>,
    payer_account_info: &AccountInfo<'a>,
    name_service_info: &AccountInfo<'a>,
    system_account_info: &AccountInfo<'a>,
    class_account_info: &AccountInfo<'a>,
    class_authority_signer_seeds: &[&[u8]],
    hashed_name: &Vec<u8>,
    size: usize,
) -> ProgramResult {
    let lamports =
        compute_name_service_account_lamports(size, rent_sysvar_info, name_account_info)?;

    msg!(&lamports.to_string());

    let create_ix = create_name_instruction(
        *name_service_info.key,
        NameRegistryInstruction::Create {
            hashed_name: hashed_name.to_vec(),
            lamports,
            space: METADATA_ENTRY_SIZE as u32,
        },
        *name_account_info.key,
        *payer_account_info.key,
        *class_account_info.key,
        Some(*class_account_info.key),
        None,
        None,
    )?;

    invoke_signed(
        &create_ix,
        &[
            system_account_info.clone(),
            payer_account_info.clone(),
            name_account_info.clone(),
            class_account_info.clone(),
        ],
        &[class_authority_signer_seeds],
    )?;

    Ok(())
}

pub fn update_name_service_account<'a>(
    name_account_info: &AccountInfo<'a>,
    class_account_info: &AccountInfo<'a>,
    name_service_info: &AccountInfo<'a>,
    class_authority_signer_seeds: &[&[u8]],
    data: &Vec<u8>,
) -> ProgramResult {
    let update_ix = update_name_instruction(
        *name_service_info.key,
        0,
        data.to_vec(),
        *name_account_info.key,
        *class_account_info.key,
    )?;

    invoke_signed(
        &update_ix,
        &[name_account_info.clone(), class_account_info.clone()],
        &[class_authority_signer_seeds],
    )?;

    Ok(())
}

// We check the program data belongs to the target program
pub fn assert_program_matches_program_data_address(
    target_program_info: &AccountInfo,
    target_program_program_data_info: &AccountInfo,
) -> Result<(), ProgramError> {
    let target_program_data = target_program_info.data.borrow();
    let extracted_program_data_key = &target_program_data[PROGRAMDATA_OFFSET..];
    let program_data_key = target_program_program_data_info.key.to_bytes();

    if extracted_program_data_key != program_data_key {
        return Err(MetadataError::ProgramDoesNotMatchProgramData.into());
    }

    Ok(())
}

// We check the program data to see if target program authority
// matches program data
pub fn assert_program_authority_has_authority_over_program(
    target_program_authority_info: &AccountInfo,
    target_program_program_data_info: &AccountInfo,
) -> Result<(), ProgramError> {
    let program_data = target_program_program_data_info.data.borrow();
    let extracted_authority_key =
        &program_data[PROGRAMDATA_AUTHORITY_KEY_OFFSET..PROGRAMDATA_AUTHORITY_KEY_OFFSET + 32];
    let program_authority_key = target_program_authority_info.key.to_bytes();

    if extracted_authority_key != program_authority_key {
        return Err(MetadataError::UpdateAuthorityIncorrect.into());
    }

    Ok(())
}
