pub mod account;
pub mod clock;
pub mod multisig;
pub mod pda;
pub mod protocol;

use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, sysvar::instructions,
};

/// Returns true if currently executing instruction is the only instruction in the transaction
pub fn is_single_instruction(sysvar_account: &AccountInfo) -> Result<bool, ProgramError> {
    if &instructions::id() != sysvar_account.key {
        return Err(ProgramError::UnsupportedSysvar);
    }
    Ok(
        instructions::load_current_index_checked(sysvar_account)? == 0
            && instructions::load_instruction_at_checked(1, sysvar_account).is_err(),
    )
}

/// Returns true if currently executing instruction is first or last instruction in the transaction
pub fn is_first_or_last_instruction(sysvar_account: &AccountInfo) -> Result<bool, ProgramError> {
    if &instructions::id() != sysvar_account.key {
        return Err(ProgramError::UnsupportedSysvar);
    }
    let instruction_index = instructions::load_current_index_checked(sysvar_account)?;
    Ok(instruction_index == 0
        || instructions::load_instruction_at_checked(
            instruction_index as usize + 1,
            sysvar_account,
        )
        .is_err())
}

/// Returns true if currently executing instruction is the last instruction in the transaction
pub fn is_last_instruction(sysvar_account: &AccountInfo) -> Result<bool, ProgramError> {
    if &instructions::id() != sysvar_account.key {
        return Err(ProgramError::UnsupportedSysvar);
    }
    let instruction_index = instructions::load_current_index_checked(sysvar_account)?;
    Ok(
        instructions::load_instruction_at_checked(instruction_index as usize + 1, sysvar_account)
            .is_err(),
    )
}
