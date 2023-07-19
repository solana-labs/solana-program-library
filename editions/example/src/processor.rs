//! Program state processor

use {
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh::get_instance_packed_len,
        entrypoint::ProgramResult,
        msg,
        program::set_return_data,
        program_error::ProgramError,
        program_option::COption,
        pubkey::Pubkey,
    },
    spl_token_2022::{extension::StateWithExtensions, state::Mint},
    spl_token_editions_interface::{
        error::TokenEditionsError,
        instruction::{
            CreateOriginal, CreateReprint, Emit, PrintType, TokenEditionsInstruction,
            UpdateOriginalAuthority, UpdateOriginalMaxSupply,
        },
        state::{get_emit_slice, OptionalNonZeroPubkey, Original, Reprint},
    },
    spl_type_length_value::state::{
        realloc_and_pack_variable_len, TlvState, TlvStateBorrowed, TlvStateMut,
    },
};

fn check_update_authority(
    update_authority_info: &AccountInfo,
    expected_update_authority: &OptionalNonZeroPubkey,
) -> Result<(), ProgramError> {
    if !update_authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let update_authority = Option::<Pubkey>::from(expected_update_authority.clone())
        .ok_or(TokenEditionsError::ImmutablePrint)?;
    if update_authority != *update_authority_info.key {
        return Err(TokenEditionsError::IncorrectUpdateAuthority.into());
    }
    Ok(())
}

/// Processes a [CreateOriginal](enum.TokenEditionsInstruction.html)
/// instruction.
pub fn process_create_original(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: CreateOriginal,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let original_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;
    let _metadata_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;

    // scope the mint authority check, in case the mint is in the same account!
    {
        // IMPORTANT: this example program is designed to work with any
        // program that implements the SPL token interface, so there is no
        // ownership check on the mint account.
        let mint_data = mint_info.try_borrow_data()?;
        let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;

        if !mint_authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if mint.base.mint_authority.as_ref() != COption::Some(mint_authority_info.key) {
            return Err(TokenEditionsError::IncorrectMintAuthority.into());
        }
    }

    /* Check metadata account exists and is not empty */

    // get the required size, assumes that there's enough space for the entry
    let update_authority = OptionalNonZeroPubkey::try_from(Some(*update_authority_info.key))?;
    let original_print = Original::new(update_authority, data.max_supply);

    let instance_size = get_instance_packed_len(&original_print)?;

    // allocate a TLV entry for the space and write it in
    let mut buffer = original_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    state.alloc::<Original>(instance_size)?;
    state.pack_variable_len_value(&original_print)?;

    Ok(())
}

/// Processes an [UpdateOriginalMaxSupply](enum.TokenEditionsInstruction.html)
/// instruction.
pub fn process_update_original_max_supply(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: UpdateOriginalMaxSupply,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let original_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    let mut original_print = {
        let buffer = original_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&buffer)?;
        state.get_variable_len_value::<Original>()?
    };

    check_update_authority(update_authority_info, &original_print.update_authority)?;

    // Update the max supply
    original_print.update_max_supply(data.max_supply)?;

    // Update the account, no realloc needed!
    realloc_and_pack_variable_len(original_info, &original_print)?;

    Ok(())
}

/// Processes a [UpdateOriginalAuthority](enum.TokenEditionsInstruction.html)
/// instruction.
pub fn process_update_original_authority(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: UpdateOriginalAuthority,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let original_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    let mut original_print = {
        let buffer = original_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&buffer)?;
        state.get_variable_len_value::<Original>()?
    };

    check_update_authority(update_authority_info, &original_print.update_authority)?;

    // Update the authority
    original_print.update_authority = data.new_authority;

    // Update the account, no realloc needed!
    realloc_and_pack_variable_len(original_info, &original_print)?;

    Ok(())
}

/// Processes a [CreateReprint](enum.TokenEditionsInstruction.html) instruction.
pub fn process_create_reprint(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _data: CreateReprint,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let reprint_info = next_account_info(account_info_iter)?;
    let _reprint_metadata_info = next_account_info(account_info_iter)?;
    let _reprint_mint_info = next_account_info(account_info_iter)?;
    let original_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;
    let _original_metadata_info = next_account_info(account_info_iter)?;
    let _original_mint_info = next_account_info(account_info_iter)?;
    let _mint_authority_info = next_account_info(account_info_iter)?;

    let mut original_print = {
        let buffer = original_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&buffer)?;
        state.get_variable_len_value::<Original>()?
    };

    check_update_authority(update_authority_info, &original_print.update_authority)?;

    // Update the current supply
    original_print.update_supply(original_print.supply + 1)?;

    // Update the account, no realloc needed!
    realloc_and_pack_variable_len(original_info, &original_print)?;

    let reprint = Reprint {
        original: *original_info.key,
        copy: 1,
    };

    let instance_size = get_instance_packed_len(&reprint)?;

    // allocate a TLV entry for the space and write it in
    let mut buffer = reprint_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    state.alloc::<Original>(instance_size)?;
    state.pack_variable_len_value(&reprint)?;

    Ok(())
}

/// Processes an [Emit](enum.TokenEditionsInstruction.html) instruction.
pub fn process_emit(program_id: &Pubkey, accounts: &[AccountInfo], data: Emit) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let print_info = next_account_info(account_info_iter)?;

    if print_info.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    let buffer = print_info.try_borrow_data()?;
    let state = TlvStateBorrowed::unpack(&buffer)?;
    let print_bytes = match data.print_type {
        PrintType::Original => state.get_bytes::<Original>()?,
        PrintType::Reprint => state.get_bytes::<Reprint>()?,
    };

    if let Some(range) = get_emit_slice(print_bytes, data.start, data.end) {
        set_return_data(range);
    }

    Ok(())
}

/// Processes an [Instruction](enum.Instruction.html).
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = TokenEditionsInstruction::unpack(input)?;

    match instruction {
        TokenEditionsInstruction::CreateOriginal(data) => {
            msg!("Instruction: CreateOriginal");
            process_create_original(program_id, accounts, data)
        }
        TokenEditionsInstruction::UpdateOriginalMaxSupply(data) => {
            msg!("Instruction: UpdateOriginalMaxSupply");
            process_update_original_max_supply(program_id, accounts, data)
        }
        TokenEditionsInstruction::UpdateOriginalAuthority(data) => {
            msg!("Instruction: UpdateOriginalAuthority");
            process_update_original_authority(program_id, accounts, data)
        }
        TokenEditionsInstruction::CreateReprint(data) => {
            msg!("Instruction: CreateReprint");
            process_create_reprint(program_id, accounts, data)
        }
        TokenEditionsInstruction::Emit(data) => {
            msg!("Instruction: Emit");
            process_emit(program_id, accounts, data)
        }
    }
}
