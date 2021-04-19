//! Program state processor

pub mod processor;

use crate::{
    processor::Processor,
};
use spl_pausable::pausable::Pausable;

use byteorder::{ByteOrder, LittleEndian};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_pack::Pack,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use std::mem;

// Declare and export the program's entrypoint
entrypoint!(process_instruction);

// Program entrypoint's implementation
fn process_instruction(
    program_id: &Pubkey, // Public key of the account the hello world program was loaded into
    accounts: &[AccountInfo], // The program metadata and account to say hello to
    instruction_data: &[u8], // instructions
) -> ProgramResult {
    msg!("Helloworld Pausable Rust program entrypoint");
    match Processor::process(program_id, accounts, instruction_data) {
        Ok(true) => {
            msg!("Process completed and we are done.");
            return Ok(())
        },
        Ok(false) => msg!("it was not an ownable or pausable instruction, proceeding with Greeting"),
        Err(e) => {
            msg!("Process failed with ERROR: {:?}", &e.to_string());
            return Err(e)
        }
    }
    msg!("Time to say Hello");

    let accounts_iter = &mut accounts.iter();
    let security = next_account_info(accounts_iter)?; // TODO don't need this
    let pgm = Pausable::unpack_from_slice(&security.data.borrow())?;
    if pgm.paused {
        msg!("Program is PAUSED!");
        return Ok(());
    }

    let greeted = next_account_info(accounts_iter)?;
    if greeted.owner != program_id {
        msg!("Greeted account does not have the correct program id");
        return Err(ProgramError::IncorrectProgramId);
    }

    // The data must be large enough to hold a u32 count
    if greeted.try_data_len()? < mem::size_of::<u32>() {
        msg!("Greeted account data length too small for u32");
        return Err(ProgramError::InvalidAccountData);
    }

    // Increment and store the number of times the account has been greeted
    let mut data = greeted.try_borrow_mut_data()?;
    let mut num_greets = LittleEndian::read_u32(&data);
    msg!("incrementing num greets");
    num_greets += 1;
    LittleEndian::write_u32(&mut data[0..], num_greets);

    msg!("Hello! {:?} times", num_greets);

    Ok(())
}


// Sanity tests
#[cfg(test)]
mod test {
    use super::*;
    use solana_program::clock::Epoch;

    #[test]
    fn test_sanity() {
        let program_id = Pubkey::default();
        let key = Pubkey::default();
        let mut lamports = 0;
        let mut data = vec![0; mem::size_of::<u32>()];
        LittleEndian::write_u32(&mut data, 0);
        let owner = Pubkey::default();
        let account = AccountInfo::new(
            &key,
            false,
            true,
            &mut lamports,
            &mut data,
            &owner,
            false,
            Epoch::default(),
        );
        let instruction_data: Vec<u8> = Vec::new();

        let accounts = vec![account];

        assert_eq!(LittleEndian::read_u32(&accounts[0].data.borrow()), 0);
        process_instruction(&program_id, &accounts, &instruction_data).unwrap();
        assert_eq!(LittleEndian::read_u32(&accounts[0].data.borrow()), 1);
        process_instruction(&program_id, &accounts, &instruction_data).unwrap();
        assert_eq!(LittleEndian::read_u32(&accounts[0].data.borrow()), 2);
    }
}
