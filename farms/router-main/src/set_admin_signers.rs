//! Initializes Main Router multisig with a new set of admin signatures

use {
    solana_farm_sdk::{
        id::main_router,
        program::{account, multisig, multisig::Multisig, pda},
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
};

pub fn set_admin_signers(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    min_signatures: u8,
) -> ProgramResult {
    msg!("Processing MainInstruction::SetAdminSigners");

    let accounts_iter = &mut accounts.iter();

    let signer_account = next_account_info(accounts_iter)?;
    let multisig_account = next_account_info(accounts_iter)?;
    let _system_program = next_account_info(accounts_iter)?;

    if account::is_empty(multisig_account)? {
        msg!("Init multisig account");
        let seeds: &[&[u8]] = &[b"multisig"];
        let _bump = pda::init_system_account(
            signer_account,
            multisig_account,
            &main_router::id(),
            &main_router::id(),
            seeds,
            Multisig::LEN,
        )?;
    } else {
        msg!("Update multisig account");
    }
    multisig::set_signers(multisig_account, accounts_iter.as_slice(), min_signatures)?;

    msg!("SetAdminSigners complete");

    Ok(())
}
