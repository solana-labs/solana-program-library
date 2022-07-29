//! Saves Fund's metadata on-chain

use {
    crate::refdb_init::{check_or_init_refdb, check_or_init_refdb_target},
    solana_farm_sdk::{fund::Fund, refdb, refdb::RefDB, traits::Packed},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
};

pub fn add_fund(program_id: &Pubkey, accounts: &[AccountInfo], fund: &Fund) -> ProgramResult {
    msg!("Processing MainInstruction::AddFund {}", fund.name);

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
        false,
    )?;
    check_or_init_refdb_target(
        program_id,
        signer_account,
        target_account,
        refdb::StorageType::Fund,
        &fund.name,
        fund.get_size(),
        false,
    )?;

    // update refdb storage
    msg!("Updating refdb storage");
    RefDB::write(
        *refdb_account.try_borrow_mut_data()?,
        &refdb::Record {
            index: fund.refdb_index,
            counter: fund.refdb_counter,
            tag: refdb::StorageType::Fund as u16,
            name: fund.name,
            reference: refdb::Reference::Pubkey {
                data: *target_account.key,
            },
        },
    )?;

    // fill in data
    msg!("Writing metadata account");
    fund.pack(*target_account.try_borrow_mut_data()?)?;

    msg!("AddFund complete");

    Ok(())
}
