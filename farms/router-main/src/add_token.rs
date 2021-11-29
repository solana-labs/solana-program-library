//! Saves Token's metadata on-chain

use {
    crate::refdb_init::{check_or_init_refdb, check_or_init_refdb_target},
    solana_farm_sdk::{refdb, refdb::RefDB, token::Token},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
};

pub fn add_token(program_id: &Pubkey, accounts: &[AccountInfo], token: &Token) -> ProgramResult {
    msg!("Processing MainInstruction::AddToken {}", token.name);

    // validate accounts
    let accounts_iter = &mut accounts.iter();

    let signer_account = next_account_info(accounts_iter)?;
    let refdb_account = next_account_info(accounts_iter)?;
    let target_account = next_account_info(accounts_iter)?;

    check_or_init_refdb(
        program_id,
        signer_account,
        refdb_account,
        refdb::StorageType::Token,
        0,
        false,
    )?;
    check_or_init_refdb_target(
        program_id,
        signer_account,
        target_account,
        refdb::StorageType::Token,
        &token.name,
        token.get_size(),
        false,
    )?;

    // update refdb storage
    msg!("Updating refdb storage");
    RefDB::write(
        *refdb_account.try_borrow_mut_data()?,
        &refdb::Record {
            index: token.refdb_index,
            counter: token.refdb_counter,
            tag: refdb::StorageType::Token as u16,
            name: token.name,
            reference: refdb::Reference::Pubkey {
                data: *target_account.key,
            },
        },
    )?;

    // fill in data
    msg!("Writing metadata account");
    token.pack(*target_account.try_borrow_mut_data()?)?;

    msg!("AddToken complete");

    Ok(())
}
