//! Common accounts management functions

use {
    solana_farm_sdk::{
        program::pda,
        refdb,
        refdb::RefDB,
        string::{str_to_as64, ArrayString64},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
        pubkey::Pubkey,
    },
};

pub fn check_or_init_refdb<'a, 'b>(
    program_id: &Pubkey,
    signer_account: &'a AccountInfo<'b>,
    refdb_account: &'a AccountInfo<'b>,
    storage_type: refdb::StorageType,
    storage_size: usize,
    delete_mode: bool,
) -> ProgramResult {
    msg!("Executing check_or_init_refdb");

    // check address
    let type_name = storage_type.to_string();
    let (derived_address, bump_seed) = pda::find_refdb_pda(&type_name);

    if derived_address != *refdb_account.key {
        return Err(ProgramError::IncorrectProgramId);
    }
    let seeds = &[type_name.as_bytes(), &[bump_seed]];

    // check if account is initialized
    let data_size = if storage_size > 0 {
        storage_size
    } else {
        refdb::StorageType::get_storage_size_for_max_records(
            storage_type,
            refdb::ReferenceType::Pubkey,
        )
    };

    if !delete_mode {
        pda::check_pda_data_size(refdb_account, seeds, data_size, refdb::REFDB_ONCHAIN_INIT)?;
        pda::check_pda_rent_exempt(
            signer_account,
            refdb_account,
            seeds,
            data_size,
            refdb::REFDB_ONCHAIN_INIT,
        )?;
    }
    pda::check_pda_owner(program_id, refdb_account, seeds, refdb::REFDB_ONCHAIN_INIT)?;

    if !delete_mode {
        // check or init storage
        let data = &mut refdb_account.try_borrow_mut_data()?;
        if !RefDB::is_initialized(data) {
            msg!("Executing RefDB::init for {}", type_name);
            RefDB::init(
                data,
                &str_to_as64(&type_name)?,
                refdb::ReferenceType::Pubkey,
            )?;
            msg!("RefDB::init complete");
        }
    }

    msg!("check_or_init_refdb complete");

    Ok(())
}

pub fn check_or_init_refdb_target<'a, 'b>(
    program_id: &Pubkey,
    signer_account: &'a AccountInfo<'b>,
    target_account: &'a AccountInfo<'b>,
    storage_type: refdb::StorageType,
    data_name: &ArrayString64,
    data_size: usize,
    delete_mode: bool,
) -> ProgramResult {
    msg!("Executing check_or_init_refdb_target");

    // check address
    let type_name = storage_type.to_string();
    let (derived_address, bump_seed) = pda::find_target_pda(storage_type, data_name);

    if derived_address != *target_account.key {
        return Err(ProgramError::IncorrectProgramId);
    }
    let seeds = &[type_name.as_bytes(), data_name.as_bytes(), &[bump_seed]];

    if !delete_mode {
        pda::check_pda_data_size(target_account, seeds, data_size, true)?;
        pda::check_pda_rent_exempt(signer_account, target_account, seeds, data_size, true)?;
    }

    pda::check_pda_owner(program_id, target_account, seeds, true)?;

    msg!("check_or_init_refdb_target complete");

    Ok(())
}
