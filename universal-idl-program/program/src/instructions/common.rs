use solana_program::{
    account_info::AccountInfo, program::invoke, program_error::ProgramError, system_instruction,
    system_program,
};

pub fn close<'info>(
    info: &AccountInfo<'info>,
    sol_destination: &AccountInfo<'info>,
) -> Result<(), ProgramError> {
    // Transfer tokens from the account to the sol_destination.
    let dest_starting_lamports = sol_destination.lamports();
    **sol_destination.lamports.borrow_mut() =
        dest_starting_lamports.checked_add(info.lamports()).unwrap();
    **info.lamports.borrow_mut() = 0;

    info.assign(&system_program::ID);
    info.realloc(0, false).map_err(Into::into)
}

pub fn transfer<'a, 'info>(
    source: &'a AccountInfo<'info>,
    destination: &'a AccountInfo<'info>,
    lamports: u64,
) -> Result<(), ProgramError> {
    if source.owner == &system_program::ID {
        return transfer_from_keypair(source, destination, lamports);
    }

    **destination.lamports.borrow_mut() = destination.lamports().checked_add(lamports).unwrap();
    **source.lamports.borrow_mut() = source.lamports().checked_sub(lamports).unwrap();
    Ok(())
}

/// Handles lamport transfer when source is owned by SystemAccount
fn transfer_from_keypair<'a, 'info>(
    source: &'a AccountInfo<'info>,
    destination: &'a AccountInfo<'info>,
    lamports: u64,
) -> Result<(), ProgramError> {
    invoke(
        &system_instruction::transfer(source.key, destination.key, lamports),
        &[source.clone(), destination.clone()],
    )
}
