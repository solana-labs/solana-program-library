//! Processes raw RefDB instruction

use {
    solana_farm_sdk::{
        instruction::refdb::RefDbInstruction,
        program::{account, pda},
        refdb,
        refdb::RefDB,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
};

pub fn process_refdb_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction: RefDbInstruction,
) -> ProgramResult {
    msg!("Processing MainInstruction::RefDbInstruction");

    let accounts_iter = &mut accounts.iter();

    let signer_account = next_account_info(accounts_iter)?;
    let refdb_account = next_account_info(accounts_iter)?;

    let res = match instruction {
        RefDbInstruction::Init {
            name,
            reference_type,
            max_records,
            init_account,
        } => {
            if max_records == 0 {
                return Err(ProgramError::InvalidArgument);
            }
            if init_account {
                let (derived_address, bump_seed) =
                    Pubkey::find_program_address(&[name.as_bytes()], program_id);
                if derived_address != *refdb_account.key {
                    return Err(ProgramError::IncorrectProgramId);
                }
                let seeds = &[name.as_bytes(), &[bump_seed]];

                let data_size = refdb::StorageType::get_storage_size_for_records(
                    reference_type,
                    max_records as usize,
                );
                pda::check_pda_data_size(refdb_account, seeds, data_size, true)?;
                pda::check_pda_rent_exempt(signer_account, refdb_account, seeds, data_size, true)?;
                pda::check_pda_owner(program_id, refdb_account, seeds, true)?;
            }
            RefDB::init(*refdb_account.try_borrow_mut_data()?, &name, reference_type)
        }
        RefDbInstruction::Drop { close_account } => {
            let _ = RefDB::drop(*refdb_account.try_borrow_mut_data()?);
            if close_account {
                account::close_system_account(signer_account, refdb_account, program_id)?;
            }
            Ok(())
        }
        RefDbInstruction::Write { record } => {
            RefDB::write(*refdb_account.try_borrow_mut_data()?, &record).map(|_v| ())
        }
        RefDbInstruction::Delete { record } => {
            RefDB::delete(*refdb_account.try_borrow_mut_data()?, &record).map(|_v| ())
        }
    };

    msg!("MainInstruction::RefDbInstruction complete");

    res
}
