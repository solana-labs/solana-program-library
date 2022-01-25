//! Saves Vault's metadata on-chain

use {
    crate::refdb_init::{check_or_init_refdb, check_or_init_refdb_target},
    solana_farm_sdk::{refdb, refdb::RefDB, vault::Vault},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
};

pub fn add_vault(program_id: &Pubkey, accounts: &[AccountInfo], vault: &Vault) -> ProgramResult {
    msg!("Processing MainInstruction::AddVault {}", vault.name);

    // validate accounts
    let accounts_iter = &mut accounts.iter();

    let signer_account = next_account_info(accounts_iter)?;
    let refdb_account = next_account_info(accounts_iter)?;
    let target_account = next_account_info(accounts_iter)?;

    check_or_init_refdb(
        program_id,
        signer_account,
        refdb_account,
        refdb::StorageType::Vault,
        0,
        false,
    )?;
    check_or_init_refdb_target(
        program_id,
        signer_account,
        target_account,
        refdb::StorageType::Vault,
        &vault.name,
        vault.get_size(),
        false,
    )?;

    // update refdb storage
    msg!("Updating refdb storage");
    RefDB::write(
        *refdb_account.try_borrow_mut_data()?,
        &refdb::Record {
            index: vault.refdb_index,
            counter: vault.refdb_counter,
            tag: refdb::StorageType::Vault as u16,
            name: vault.name,
            reference: refdb::Reference::Pubkey {
                data: *target_account.key,
            },
        },
    )?;

    // fill in data
    msg!("Writing metadata account");
    vault.pack(*target_account.try_borrow_mut_data()?)?;

    msg!("AddVault complete");

    Ok(())
}
