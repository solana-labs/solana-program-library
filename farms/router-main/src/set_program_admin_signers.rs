//! Initializes program multisig with a new set of admin signatures

use {
    solana_farm_sdk::{
        id::main_router,
        program::{account, multisig, multisig::Multisig, pda},
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        bpf_loader_upgradeable,
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        pubkey::Pubkey,
    },
};

pub fn set_program_admin_signers(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    min_signatures: u8,
) -> ProgramResult {
    msg!("Processing MainInstruction::SetProgramAdminSigners");

    let accounts_iter = &mut accounts.iter();

    let signer_account = next_account_info(accounts_iter)?;
    let multisig_account = next_account_info(accounts_iter)?;
    let target_program = next_account_info(accounts_iter)?;
    let program_buffer = next_account_info(accounts_iter)?;
    let _system_program = next_account_info(accounts_iter)?;
    let _bpf_loader = next_account_info(accounts_iter)?;

    if account::is_empty(multisig_account)? {
        // create new multisig account
        msg!("Init multisig account");
        let seeds: &[&[u8]] = &[b"multisig", target_program.key.as_ref()];
        let _bump = pda::init_system_account(
            signer_account,
            multisig_account,
            &main_router::id(),
            &main_router::id(),
            seeds,
            Multisig::LEN,
        )?;

        // change program upgrade authority to multisig
        let instruction = bpf_loader_upgradeable::set_buffer_authority(
            program_buffer.key,
            signer_account.key,
            multisig_account.key,
        );
        invoke(
            &instruction,
            &[
                program_buffer.clone(),
                signer_account.clone(),
                multisig_account.clone(),
            ],
        )?;
    } else {
        msg!("Update multisig account");
    }
    multisig::set_signers(multisig_account, accounts_iter.as_slice(), min_signatures)?;

    msg!("SetProgramAdminSigners complete");

    Ok(())
}
