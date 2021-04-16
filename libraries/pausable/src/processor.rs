//! Program state processor

use crate::{
    instruction::PausableInstruction,
    pausable::Pausable,
};
use spl_ownable::processor::validate_owner;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_option::COption,
    pubkey::Pubkey,
};

/// Process an [PausableInstruction](enum.PausableInstruction.html).
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = PausableInstruction::unpack(input)?;
    match instruction {
        PausableInstruction::Pause => {
            msg!("Pause program operation"); 
            pause(program_id, accounts, 0)
        },
        PausableInstruction::Resume => {
            msg!("Resume program operation");
            resume(program_id, accounts, 0)
        },
    }
}

pub fn pause(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    offset: usize,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let account = next_account_info(accounts_iter)?;
    let owner = next_account_info(accounts_iter)?.key;
    _toggle(program_id, account, owner, true, offset)
}
    
pub fn resume(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    offset: usize,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let account = next_account_info(accounts_iter)?;
    let owner = next_account_info(accounts_iter)?.key;
    _toggle(program_id, account, owner, false, offset)
}

fn _toggle(
    program_id: &Pubkey,
    account: &AccountInfo,
    owner: &Pubkey,
    paused: bool,
    offset: usize,
) -> ProgramResult {
    if account.owner != program_id {
        msg!("Account was not created by this program");
        return Err(ProgramError::IncorrectProgramId.into());
    }

    let mut pgm = Pausable::unpack_starting_at(&account.data.borrow(), offset)?;
    validate_owner(pgm.ownable, COption::Some(owner))?;

    pgm.paused = paused;

    Pausable::pack_starting_at(&pgm, &mut account.data.borrow_mut(), offset);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use spl_ownable::{
        error::OwnableError,
        processor as Ownable,
    };
    use solana_sdk::{
        program_pack::Pack,
        account::Account,
    };


    #[test]
    fn test_pausable() {
        let program_id = Pubkey::new_unique();

        let owner0 = Pubkey::new_unique();
        let mut account0 = Account::default();
        let account0_info: AccountInfo = (&owner0, &mut account0).into();

        let owner1 = Pubkey::new_unique();
        let mut account1 = Account::new(0, Pausable::get_packed_len(), &program_id);
        let account1_info: AccountInfo = (&owner1, &mut account1).into();

        let owner2 = Pubkey::new_unique();
        let mut account2 = Account::default();
        let account2_info: AccountInfo = (&owner2, &mut account2).into();

        let _is_paused = || {
            Pausable::unpack(&account1_info.data.borrow()).unwrap().paused
        };
        assert!(!_is_paused());

        // Pause no owner
        assert_eq!(pause(&program_id, &[
            account1_info.clone(), account0_info.clone(),
        ], 0), Ok(()));
        assert!(_is_paused());

        // Resume no owner
        assert_eq!(resume(&program_id, &[
            account1_info.clone(), account0_info.clone(),
        ], 0), Ok(()));
        assert!(!_is_paused());

        // Initialize owner
        assert_eq!(Ownable::initialize_ownership(&program_id, &[
            account1_info.clone(), account0_info.clone(),
        ], 0), Ok(()));

        // Pause wrong owner
        assert_eq!(pause(&program_id, &[
            account1_info.clone(), account2_info.clone(),
        ], 0), Err(OwnableError::InvalidOwner.into()));

        // Pause correct owner
        assert_eq!(pause(&program_id, &[
            account1_info.clone(), account0_info.clone(),
        ], 0), Ok(()));
        assert!(_is_paused());

        // Resume wrong owner
        assert_eq!(resume(&program_id, &[
            account1_info.clone(), account2_info.clone(),
        ], 0), Err(OwnableError::InvalidOwner.into()));
        assert!(_is_paused());

        // Resume correct owner
        assert_eq!(resume(&program_id, &[
            account1_info.clone(), account0_info.clone(),
        ], 0), Ok(()));
        assert!(!_is_paused());
    }
}
