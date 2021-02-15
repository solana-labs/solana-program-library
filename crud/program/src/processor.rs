//! Program state processor

use {
    crate::{
        error::CrudError,
        instruction::CrudInstruction,
        state::{AccountData, Data},
    },
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        program_pack::IsInitialized,
        pubkey::Pubkey,
        rent::Rent,
        sysvar::Sysvar,
    },
};

/// Instruction processor
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = CrudInstruction::try_from_slice(input)?;
    let account_info_iter = &mut accounts.iter();

    match instruction {
        CrudInstruction::Initialize => {
            msg!("CrudInstruction::Initialize");

            let data_info = next_account_info(account_info_iter)?;
            let owner_info = next_account_info(account_info_iter)?;
            let rent_sysvar_info = next_account_info(account_info_iter)?;
            let rent = &Rent::from_account_info(rent_sysvar_info)?;

            let mut account_data = AccountData::try_from_slice(*data_info.data.borrow())?;
            if account_data.is_initialized() {
                return Err(ProgramError::AccountAlreadyInitialized);
            }

            if !rent.is_exempt(data_info.lamports(), data_info.data_len()) {
                return Err(ProgramError::AccountNotRentExempt);
            }
            account_data.authority = *owner_info.key;
            account_data.version = AccountData::CURRENT_VERSION;
            account_data
                .serialize(&mut *data_info.data.borrow_mut())
                .map_err(|e| e.into())
        }

        CrudInstruction::Write { data } => {
            msg!("CrudInstruction::Write");
            let data_info = next_account_info(account_info_iter)?;
            let owner_info = next_account_info(account_info_iter)?;
            let mut account_data = AccountData::try_from_slice(&data_info.data.borrow())?;
            if account_data.authority != *owner_info.key {
                return Err(CrudError::IncorrectOwner.into());
            }
            if !owner_info.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }
            account_data.data = data;
            account_data
                .serialize(&mut *data_info.data.borrow_mut())
                .map_err(|e| e.into())
        }

        CrudInstruction::SetAuthority => {
            msg!("CrudInstruction::SetAuthority");
            let data_info = next_account_info(account_info_iter)?;
            let owner_info = next_account_info(account_info_iter)?;
            let new_authority_info = next_account_info(account_info_iter)?;
            let mut account_data = AccountData::try_from_slice(&data_info.data.borrow())?;
            if account_data.authority != *owner_info.key {
                return Err(CrudError::IncorrectOwner.into());
            }
            if !owner_info.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }
            account_data.authority = *new_authority_info.key;
            account_data
                .serialize(&mut *data_info.data.borrow_mut())
                .map_err(|e| e.into())
        }

        CrudInstruction::CloseAccount => {
            msg!("CrudInstruction::CloseAccount");
            let data_info = next_account_info(account_info_iter)?;
            let owner_info = next_account_info(account_info_iter)?;
            let destination_info = next_account_info(account_info_iter)?;
            let mut account_data = AccountData::try_from_slice(&data_info.data.borrow())?;
            if account_data.authority != *owner_info.key {
                return Err(CrudError::IncorrectOwner.into());
            }
            if !owner_info.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }
            let destination_starting_lamports = destination_info.lamports();
            let data_lamports = data_info.lamports();
            **data_info.lamports.borrow_mut() = 0;
            **destination_info.lamports.borrow_mut() = destination_starting_lamports
                .checked_add(data_lamports)
                .ok_or(CrudError::Overflow)?;
            account_data.data = Data::default();
            account_data
                .serialize(&mut *data_info.data.borrow_mut())
                .map_err(|e| e.into())
        }
    }
}
