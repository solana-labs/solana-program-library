//! Program state processor

use {
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh::get_instance_packed_len,
        entrypoint::ProgramResult,
        msg,
        program::{invoke, set_return_data},
        program_error::ProgramError,
        program_option::COption,
        pubkey::Pubkey,
    },
    spl_token_2022::{
        extension::{
            metadata_pointer::MetadataPointer, BaseStateWithExtensions, StateWithExtensions,
        },
        state::Mint,
    },
    spl_token_editions_interface::{
        error::TokenEditionsError,
        instruction::{
            CreateOriginal, CreateReprint, Emit, PrintType, TokenEditionsInstruction,
            UpdateOriginalAuthority, UpdateOriginalMaxSupply,
        },
        state::{get_emit_slice, OptionalNonZeroPubkey, Original, Reprint},
    },
    spl_token_metadata_interface::{instruction::initialize, state::TokenMetadata},
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
    // Assumes one has already created a mint and a metadata account for the
    // original print.
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]` Original
    //   1. `[]` Metadata
    //   2. `[]` Mint
    //   3. `[s]` Mint authority
    let original_info = next_account_info(account_info_iter)?;
    let metadata_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;

    // Mint & metadata checks
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

        // IMPORTANT: metadata is passed as a separate account because it may be owned
        // by another program - separate from the program implementing the SPL token
        // interface _or_ the program implementing the SPL token editions interface.
        let metadata_pointer = mint.get_extension::<MetadataPointer>()?;
        let metadata_pointer_address = Option::<Pubkey>::from(metadata_pointer.metadata_address);
        if metadata_pointer_address != Some(*metadata_info.key) {
            return Err(TokenEditionsError::IncorrectMetadata.into());
        }
    }

    let original_print = Original::new(data.update_authority, data.max_supply);
    let instance_size = get_instance_packed_len(&original_print)?;

    // Allocate a TLV entry for the space and write it in
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

    // Accounts expected by this instruction:
    //
    //   0. `[w]` Original
    //   1. `[s]` Update authority
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

    // Accounts expected by this instruction:
    //
    //   0. `[w]` Original
    //   1. `[s]` Current update authority
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
    data: CreateReprint,
) -> ProgramResult {
    // Assumes the `Original` print has already been created,
    // as well as the mint and metadata for the original print.
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]` Reprint
    //   1. `[w]` Reprint Metadata
    //   2. `[]` Reprint Mint
    //   3. `[w]` Original
    //   4. `[s]` Update authority
    //   5. `[]` Original Metadata
    //   6. `[]` Original Mint
    //   7. `[s]` Mint authority
    //   8. `[]` Metadata program
    //   8..8+M `[]` `M` additional accounts, written in validation account data
    let reprint_info = next_account_info(account_info_iter)?;
    let reprint_metadata_info = next_account_info(account_info_iter)?;
    let reprint_mint_info = next_account_info(account_info_iter)?;
    let original_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;
    let original_metadata_info = next_account_info(account_info_iter)?;
    let original_mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;
    let metadata_program_info = next_account_info(account_info_iter)?;
    // No additional accounts required in this example

    // Mint & metadata checks on the original
    let token_metadata = {
        // IMPORTANT: this example program is designed to work with any
        // program that implements the SPL token interface, so there is no
        // ownership check on the mint account.
        let original_mint_data = original_mint_info.try_borrow_data()?;
        let original_mint = StateWithExtensions::<Mint>::unpack(&original_mint_data)?;

        if !mint_authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if original_mint.base.mint_authority.as_ref() != COption::Some(mint_authority_info.key) {
            return Err(TokenEditionsError::IncorrectMintAuthority.into());
        }

        // IMPORTANT: metadata is passed as a separate account because it may be owned
        // by another program - separate from the program implementing the SPL token
        // interface _or_ the program implementing the SPL token editions interface.
        let metadata_pointer = original_mint.get_extension::<MetadataPointer>()?;
        let metadata_pointer_address = Option::<Pubkey>::from(metadata_pointer.metadata_address);
        if metadata_pointer_address != Some(*original_metadata_info.key) {
            return Err(TokenEditionsError::IncorrectMetadata.into());
        }

        original_mint.get_variable_len_extension::<TokenMetadata>()?
    };

    // Mint & metadata checks on the reprint
    {
        // IMPORTANT: this example program is designed to work with any
        // program that implements the SPL token interface, so there is no
        // ownership check on the mint account.
        let reprint_mint_data = reprint_mint_info.try_borrow_data()?;
        let reprint_mint = StateWithExtensions::<Mint>::unpack(&reprint_mint_data)?;

        if !mint_authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if reprint_mint.base.mint_authority.as_ref() != COption::Some(mint_authority_info.key) {
            return Err(TokenEditionsError::IncorrectMintAuthority.into());
        }

        // IMPORTANT: metadata is passed as a separate account because it may be owned
        // by another program - separate from the program implementing the SPL token
        // interface _or_ the program implementing the SPL token editions interface.
        let metadata_pointer = reprint_mint.get_extension::<MetadataPointer>()?;
        let metadata_pointer_address = Option::<Pubkey>::from(metadata_pointer.metadata_address);
        if metadata_pointer_address != Some(*reprint_metadata_info.key) {
            return Err(TokenEditionsError::IncorrectMetadata.into());
        }
    }

    let mut original_print = {
        let buffer = original_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&buffer)?;
        state.get_variable_len_value::<Original>()?
    };

    check_update_authority(update_authority_info, &original_print.update_authority)?;

    if data.original != *original_info.key {
        return Err(TokenEditionsError::IncorrectOriginal.into());
    }

    // Update the current supply
    let copy = original_print.supply + 1;
    original_print.update_supply(copy)?;
    realloc_and_pack_variable_len(original_info, &original_print)?;

    // Create the reprint metadata from the original metadata
    let cpi_instruction = initialize(
        metadata_program_info.key,
        reprint_mint_info.key,
        update_authority_info.key,
        reprint_mint_info.key,
        mint_authority_info.key,
        token_metadata.name,
        token_metadata.symbol,
        token_metadata.uri,
    );
    let cpi_account_infos = &[
        reprint_mint_info.clone(),
        update_authority_info.clone(),
        reprint_mint_info.clone(),
        mint_authority_info.clone(),
    ];
    invoke(&cpi_instruction, cpi_account_infos)?;

    // Create the reprint
    let reprint = Reprint {
        original: *original_info.key,
        copy,
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

    // Accounts expected by this instruction:
    //
    //   0. `[]` Original _or_ Reprint account
    let print_info = next_account_info(account_info_iter)?;

    if print_info.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    let buffer = print_info.try_borrow_data()?;
    let state = TlvStateBorrowed::unpack(&buffer)?;

    // `print_type` determines which asset we're working with
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
