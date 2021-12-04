//! Saves Pool's metadata on-chain

use {
    crate::refdb_init::{check_or_init_refdb, check_or_init_refdb_target},
    solana_farm_sdk::{pool::Pool, refdb, refdb::RefDB},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
};

pub fn add_pool(program_id: &Pubkey, accounts: &[AccountInfo], pool: &Pool) -> ProgramResult {
    msg!("Processing MainInstruction::AddPool {}", pool.name);

    // validate accounts
    let accounts_iter = &mut accounts.iter();

    let signer_account = next_account_info(accounts_iter)?;
    let refdb_account = next_account_info(accounts_iter)?;
    let target_account = next_account_info(accounts_iter)?;

    check_or_init_refdb(
        program_id,
        signer_account,
        refdb_account,
        refdb::StorageType::Pool,
        0,
        false,
    )?;
    check_or_init_refdb_target(
        program_id,
        signer_account,
        target_account,
        refdb::StorageType::Pool,
        &pool.name,
        pool.get_size(),
        false,
    )?;

    // update refdb storage
    msg!("Updating refdb storage");
    RefDB::write(
        *refdb_account.try_borrow_mut_data()?,
        &refdb::Record {
            index: pool.refdb_index,
            counter: pool.refdb_counter,
            tag: refdb::StorageType::Pool as u16,
            name: pool.name,
            reference: refdb::Reference::Pubkey {
                data: *target_account.key,
            },
        },
    )?;

    // fill in data
    msg!("Writing metadata account");
    pool.pack(*target_account.try_borrow_mut_data()?)?;

    msg!("AddPool complete");

    Ok(())
}
