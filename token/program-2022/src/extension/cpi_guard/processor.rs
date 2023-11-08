use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            cpi_guard::{in_cpi, instruction::CpiGuardInstruction, CpiGuard},
            StateWithExtensionsMut,
        },
        instruction::decode_instruction_type,
        processor::Processor,
        state::Account,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
};

/// Toggle the CpiGuard extension, initializing the extension if not already
/// present.
fn process_toggle_cpi_guard(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    enable: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    let owner_info_data_len = owner_info.data_len();

    let mut account_data = token_account_info.data.borrow_mut();
    let mut account = StateWithExtensionsMut::<Account>::unpack(&mut account_data)?;

    Processor::validate_owner(
        program_id,
        &account.base.owner,
        owner_info,
        owner_info_data_len,
        account_info_iter.as_slice(),
    )?;

    if in_cpi() {
        return Err(TokenError::CpiGuardSettingsLocked.into());
    }

    let extension = if let Ok(extension) = account.get_extension_mut::<CpiGuard>() {
        extension
    } else {
        account.init_extension::<CpiGuard>(true)?
    };
    extension.lock_cpi = enable.into();
    Ok(())
}

pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id)?;

    match decode_instruction_type(input)? {
        CpiGuardInstruction::Enable => {
            msg!("CpiGuardInstruction::Enable");
            process_toggle_cpi_guard(program_id, accounts, true /* enable */)
        }
        CpiGuardInstruction::Disable => {
            msg!("CpiGuardInstruction::Disable");
            process_toggle_cpi_guard(program_id, accounts, false /* disable */)
        }
    }
}
