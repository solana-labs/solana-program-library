//! Removes Fund's metadata from chain

use {
    crate::refdb_init::{check_or_init_refdb, check_or_init_refdb_target},
    solana_farm_sdk::{
        program::account::close_system_account, refdb, refdb::RefDB, string::ArrayString64,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
};

pub fn remove_fund(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: &ArrayString64,
    refdb_index: Option<u32>,
) -> ProgramResult {
    msg!("Processing MainInstruction::RemoveFund");

    // validate accounts
    let accounts_iter = &mut accounts.iter();

    let signer_account = next_account_info(accounts_iter)?;
    let _multisig_account = next_account_info(accounts_iter)?;
    let refdb_account = next_account_info(accounts_iter)?;
    let target_account = next_account_info(accounts_iter)?;

    check_or_init_refdb(
        program_id,
        signer_account,
        refdb_account,
        refdb::StorageType::Fund,
        0,
        true,
    )?;
    check_or_init_refdb_target(
        program_id,
        signer_account,
        target_account,
        refdb::StorageType::Fund,
        name,
        0,
        true,
    )?;

    // update ref storage
    msg!("Updating refdb storage");
    let _ = RefDB::delete_with_name(*refdb_account.try_borrow_mut_data()?, name, refdb_index);

    // close metadata account
    msg!("Closing metadata account");
    close_system_account(signer_account, target_account, program_id)?;

    msg!("RemoveFund complete");

    Ok(())
}
