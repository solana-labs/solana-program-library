//! Program state processor

use {
    crate::instruction::PermissionedTransferInstruction,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
};

/// Processes a [Validate](enum.PermissionedTransferInstruction.html) instruction.
pub fn process_validate(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let source_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let destination_account_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;

    Ok(())
}

/// Processes a [InitializeValidationPubkeys](enum.PermissionedTransferInstruction.html) instruction.
pub fn process_initialize_validation_pubkeys(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let validation_pubkeys_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;

    Ok(())
}

/// Processes an [Instruction](enum.Instruction.html).
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = PermissionedTransferInstruction::unpack(input)?;

    match instruction {
        PermissionedTransferInstruction::Validate { amount } => {
            msg!("Instruction: Validate");
            process_validate(program_id, accounts, amount)
        }
        PermissionedTransferInstruction::InitializeValidationPubkeys => {
            msg!("Instruction: InitializeValidationPubkeys");
            process_initialize_validation_pubkeys(program_id, accounts)
        }
    }
}

#[cfg(test)]
mod tests {
    // TODO
}
