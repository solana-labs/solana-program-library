//! Program state processor

use crate::{
    error::OwnableError,
    instruction::OwnableInstruction,
    ownable::Ownable,
};

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_option::COption,
    pubkey::Pubkey,
};

/// Process an [OwnableInstruction](enum.OwnableInstruction.html).
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = OwnableInstruction::unpack(input)?;
    match instruction {
        OwnableInstruction::InitializeOwnership => {
            msg!("Initialize ownership");
            initialize_ownership(program_id, accounts, 0)
        },
        OwnableInstruction::TransferOwnership => {
            msg!("Transfer ownership");
            transfer_ownership(program_id, accounts, 0)
        },
        OwnableInstruction::RenounceOwnership => {
            msg!("Renounce ownership");
            renounce_ownership(program_id, accounts, 0)
        },
    }
}

pub fn initialize_ownership(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    offset: usize,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let account = next_account_info(accounts_iter)?;
    let new_owner = next_account_info(accounts_iter)?.key;
    _set_ownership(program_id, account, new_owner, COption::None, offset)
}
    
pub fn transfer_ownership(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    offset: usize,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let account = next_account_info(accounts_iter)?;
    let current_owner = next_account_info(accounts_iter)?.key;
    let new_owner = next_account_info(accounts_iter)?.key;
    _set_ownership(program_id, account, new_owner, COption::Some(current_owner), offset)
}

/// Renounce ownership by setting the new owner to the account PublicKey
pub fn renounce_ownership(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    offset: usize,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let account = next_account_info(accounts_iter)?;
    let current_owner = next_account_info(accounts_iter)?.key;
    _set_ownership(program_id, account, account.key, COption::Some(current_owner), offset)
}

fn _set_ownership(
    program_id: &Pubkey,
    account: &AccountInfo,
    new_owner_key: &Pubkey,
    old_owner_key: COption<&Pubkey>,
    offset: usize,
) -> ProgramResult {
    if account.owner != program_id {
        msg!("Account was not created by this program");
        return Err(ProgramError::IncorrectProgramId.into());
    }

    let mut pgm = Ownable::unpack_starting_at(&account.data.borrow(), offset)?;
    validate_owner(pgm, old_owner_key)?;

    pgm.owner = COption::Some(*new_owner_key);

    Ownable::pack_starting_at(&pgm, &mut account.data.borrow_mut(), offset);
    Ok(())
}

pub fn validate_owner(pgm: Ownable, owner: COption<&Pubkey>) -> ProgramResult {
    match pgm.owner {
        COption::None => {
            msg!("Currently no owner so proceeding with operation");
            Ok(())
        },
        COption::Some(pgm_owner) => {
            match owner {
                COption::None => {
                    msg!("Program is owned but no owning account supplied");
                    Err(OwnableError::InvalidOwner.into())
                },
                COption::Some(owner) => {
                    if pgm_owner != *owner {
                        msg!("Program is not currently owned by that Account");
                        Err(OwnableError::InvalidOwner.into())
                    } else {
                        Ok(())
                    }
                },
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{
        program_pack::Pack,
        account::Account,
    };

    #[test]
    fn test_validate_owner() {
        let other = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let error = Err(OwnableError::InvalidOwner.into());

        // no owner
        let ownable = Ownable{ owner: COption::None };
        assert_eq!(Ok(()), validate_owner(ownable, COption::None));
        assert_eq!(Ok(()), validate_owner(ownable, COption::Some(&other)));

        // owner not supplied
        let ownable = Ownable{ owner: COption::Some(owner) };
        assert_eq!(error, validate_owner(ownable, COption::None));

        // correct owner
        assert_eq!(Ok(()), validate_owner(ownable, COption::Some(&owner)));

        // incorrect owner
        assert_eq!(error, validate_owner(ownable, COption::Some(&other)));
    }

    #[test]
    fn test_ownerable() {
        let program_id = Pubkey::new_unique();

        let owner0 = Pubkey::new_unique();
        let mut account0 = Account::default();
        let account0_info: AccountInfo = (&owner0, &mut account0).into();

        let offset: usize = 3;
        let owner1 = Pubkey::new_unique();
        let mut account1 = Account::new(0, offset + Ownable::get_packed_len(), &program_id);
        let account1_info: AccountInfo = (&owner1, &mut account1).into();

        let owner2 = Pubkey::new_unique();
        let mut account2 = Account::default();
        let account2_info: AccountInfo = (&owner2, &mut account2).into();
       
        assert_eq!(initialize_ownership(&program_id, &[
            account0_info.clone()
        ], 0), Err(ProgramError::NotEnoughAccountKeys));

        assert_eq!(initialize_ownership(&program_id, &[
            account0_info.clone(), account0_info.clone(),
        ], 0), Err(ProgramError::IncorrectProgramId));


        let ownable = Ownable::unpack_starting_at(&account1_info.data.borrow(), offset).unwrap();
        assert_eq!(COption::None, ownable.owner);

        assert_eq!(initialize_ownership(&program_id, &[
            account1_info.clone(), account0_info.clone(),
        ], offset), Ok(()));
        let ownable = Ownable::unpack_starting_at(&account1_info.data.borrow(), offset).unwrap();
        assert_eq!(ownable.owner, COption::Some(owner0));

        assert_eq!(initialize_ownership(&program_id, &[
            account1_info.clone(), account0_info.clone(),
        ], offset), Err(OwnableError::InvalidOwner.into()));

        assert_eq!(initialize_ownership(&program_id, &[
            account1_info.clone(), account1_info.clone(),
        ], offset), Err(OwnableError::InvalidOwner.into()));

        assert_eq!(initialize_ownership(&program_id, &[
            account1_info.clone(), account2_info.clone(),
        ], offset), Err(OwnableError::InvalidOwner.into()));


        // Transfer
        assert_eq!(transfer_ownership(&program_id, &[
            account1_info.clone(), account2_info.clone(), account2_info.clone(),
        ], offset), Err(OwnableError::InvalidOwner.into()));

        assert_eq!(transfer_ownership(&program_id, &[
            account1_info.clone(), account1_info.clone(), account2_info.clone(),
        ], offset), Err(OwnableError::InvalidOwner.into()));

        assert_eq!(transfer_ownership(&program_id, &[
            account1_info.clone(), account0_info.clone(), account2_info.clone(),
        ], offset), Ok(()));
        let ownable = Ownable::unpack_starting_at(&account1_info.data.borrow(), offset).unwrap();
        assert_eq!(ownable.owner, COption::Some(owner2));


        // Renounce
        assert_eq!(renounce_ownership(&program_id, &[
            account1_info.clone(), account0_info.clone(),
        ], offset), Err(OwnableError::InvalidOwner.into()));

        assert_eq!(renounce_ownership(&program_id, &[
            account1_info.clone(), account1_info.clone(),
        ], offset), Err(OwnableError::InvalidOwner.into()));

        assert_eq!(renounce_ownership(&program_id, &[
            account1_info.clone(), account2_info.clone(),
        ], offset), Ok(()));
        let ownable = Ownable::unpack_starting_at(&account1_info.data.borrow(), offset).unwrap();
        assert_eq!(ownable.owner, COption::Some(owner1));
    }
}
