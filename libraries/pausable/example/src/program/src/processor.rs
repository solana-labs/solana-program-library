//! Program state processor

use solana_program::{
    account_info::AccountInfo,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_ownable::processor as Ownable;
use spl_pausable::processor as Pausable;

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Process an [OwnableInstruction](enum.OwnableInstruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> Result<bool, ProgramError> {
        if let Some((tag, _)) = input.split_first() {
            msg!("*********************************************************");
            msg!("Client Program TAG {:?}", tag);
            match tag {
                0 => Ownable::initialize_ownership(program_id, accounts, 0)?,
                1 => Ownable::transfer_ownership(program_id, accounts, 0)?,
                2 => Ownable::renounce_ownership(program_id, accounts, 0)?,
                3 => Pausable::pause(program_id, accounts, 0)?,
                4 => Pausable::resume(program_id, accounts, 0)?,
                _ => {
                    msg!("Unknown action to perform!");
                    return Ok(false);
                }
            };
            return Ok(true)
        } else {
            msg!("No action to perform!");
            Ok(false)
        }
    }
}

