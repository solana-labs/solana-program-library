//! Program implementation

use {
    solana_account_info::{next_account_info, AccountInfo},
    solana_program_error::{ProgramError, ProgramResult},
    solana_pubkey::Pubkey,
};

solana_program_entrypoint::entrypoint!(process_instruction);
fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let source_account_info = next_account_info(account_info_iter)?;
    let _mint_info = next_account_info(account_info_iter)?;
    let _destination_account_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let _extra_account_metas_info = next_account_info(account_info_iter)?;

    let source_account_info_again = next_account_info(account_info_iter)?;
    let authority_info_again = next_account_info(account_info_iter)?;

    if source_account_info.key != source_account_info_again.key {
        return Err(ProgramError::InvalidAccountData);
    }

    if source_account_info_again.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    if authority_info.key != authority_info_again.key {
        return Err(ProgramError::InvalidAccountData);
    }

    if authority_info.is_signer {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}
