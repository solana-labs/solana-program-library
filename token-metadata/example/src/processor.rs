//! Program state processor

use {
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh1::get_instance_packed_len,
        entrypoint::ProgramResult,
        msg,
        program::set_return_data,
        program_error::ProgramError,
        program_option::COption,
        pubkey::Pubkey,
    },
    spl_pod::optional_keys::OptionalNonZeroPubkey,
    spl_token_2022::{extension::StateWithExtensions, state::Mint},
    spl_token_metadata_interface::{
        error::TokenMetadataError,
        instruction::{
            Emit, Initialize, RemoveKey, TokenMetadataInstruction, UpdateAuthority, UpdateField,
        },
        state::TokenMetadata,
    },
    spl_type_length_value::state::{
        realloc_and_pack_first_variable_len, TlvState, TlvStateBorrowed, TlvStateMut,
    },
};

fn check_update_authority(
    update_authority_info: &AccountInfo,
    expected_update_authority: &OptionalNonZeroPubkey,
) -> Result<(), ProgramError> {
    if !update_authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let update_authority = Option::<Pubkey>::from(*expected_update_authority)
        .ok_or(TokenMetadataError::ImmutableMetadata)?;
    if update_authority != *update_authority_info.key {
        return Err(TokenMetadataError::IncorrectUpdateAuthority.into());
    }
    Ok(())
}

/// Processes a [Initialize](enum.TokenMetadataInstruction.html) instruction.
pub fn process_initialize(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: Initialize,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let metadata_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;

    // scope the mint authority check, in case the mint is in the same account!
    {
        // IMPORTANT: this example metadata program is designed to work with any
        // program that implements the SPL token interface, so there is no
        // ownership check on the mint account.
        let mint_data = mint_info.try_borrow_data()?;
        let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;

        if !mint_authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if mint.base.mint_authority.as_ref() != COption::Some(mint_authority_info.key) {
            return Err(TokenMetadataError::IncorrectMintAuthority.into());
        }
    }

    // get the required size, assumes that there's enough space for the entry
    let update_authority = OptionalNonZeroPubkey::try_from(Some(*update_authority_info.key))?;
    let token_metadata = TokenMetadata {
        name: data.name,
        symbol: data.symbol,
        uri: data.uri,
        update_authority,
        mint: *mint_info.key,
        ..Default::default()
    };
    let instance_size = get_instance_packed_len(&token_metadata)?;

    // allocate a TLV entry for the space and write it in
    let mut buffer = metadata_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    state.alloc::<TokenMetadata>(instance_size, false)?;
    state.pack_first_variable_len_value(&token_metadata)?;

    Ok(())
}

/// Processes an [UpdateField](enum.TokenMetadataInstruction.html) instruction.
pub fn process_update_field(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: UpdateField,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let metadata_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    // deserialize the metadata, but scope the data borrow since we'll probably
    // realloc the account
    let mut token_metadata = {
        let buffer = metadata_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&buffer)?;
        state.get_first_variable_len_value::<TokenMetadata>()?
    };

    check_update_authority(update_authority_info, &token_metadata.update_authority)?;

    // Update the field
    token_metadata.update(data.field, data.value);

    // Update / realloc the account
    realloc_and_pack_first_variable_len(metadata_info, &token_metadata)?;

    Ok(())
}

/// Processes a [RemoveKey](enum.TokenMetadataInstruction.html) instruction.
pub fn process_remove_key(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: RemoveKey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let metadata_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    // deserialize the metadata, but scope the data borrow since we'll probably
    // realloc the account
    let mut token_metadata = {
        let buffer = metadata_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&buffer)?;
        state.get_first_variable_len_value::<TokenMetadata>()?
    };

    check_update_authority(update_authority_info, &token_metadata.update_authority)?;
    if !token_metadata.remove_key(&data.key) && !data.idempotent {
        return Err(TokenMetadataError::KeyNotFound.into());
    }
    realloc_and_pack_first_variable_len(metadata_info, &token_metadata)?;

    Ok(())
}

/// Processes a [UpdateAuthority](enum.TokenMetadataInstruction.html)
/// instruction.
pub fn process_update_authority(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: UpdateAuthority,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let metadata_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    // deserialize the metadata, but scope the data borrow since we'll probably
    // realloc the account
    let mut token_metadata = {
        let buffer = metadata_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&buffer)?;
        state.get_first_variable_len_value::<TokenMetadata>()?
    };

    check_update_authority(update_authority_info, &token_metadata.update_authority)?;
    token_metadata.update_authority = data.new_authority;
    // Update the account, no realloc needed!
    realloc_and_pack_first_variable_len(metadata_info, &token_metadata)?;

    Ok(())
}

/// Processes an [Emit](enum.TokenMetadataInstruction.html) instruction.
pub fn process_emit(program_id: &Pubkey, accounts: &[AccountInfo], data: Emit) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let metadata_info = next_account_info(account_info_iter)?;

    if metadata_info.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    let buffer = metadata_info.try_borrow_data()?;
    let state = TlvStateBorrowed::unpack(&buffer)?;
    let metadata_bytes = state.get_first_bytes::<TokenMetadata>()?;

    if let Some(range) = TokenMetadata::get_slice(metadata_bytes, data.start, data.end) {
        set_return_data(range);
    }

    Ok(())
}

/// Processes an [Instruction](enum.Instruction.html).
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = TokenMetadataInstruction::unpack(input)?;

    match instruction {
        TokenMetadataInstruction::Initialize(data) => {
            msg!("Instruction: Initialize");
            process_initialize(program_id, accounts, data)
        }
        TokenMetadataInstruction::UpdateField(data) => {
            msg!("Instruction: UpdateField");
            process_update_field(program_id, accounts, data)
        }
        TokenMetadataInstruction::RemoveKey(data) => {
            msg!("Instruction: RemoveKey");
            process_remove_key(program_id, accounts, data)
        }
        TokenMetadataInstruction::UpdateAuthority(data) => {
            msg!("Instruction: UpdateAuthority");
            process_update_authority(program_id, accounts, data)
        }
        TokenMetadataInstruction::Emit(data) => {
            msg!("Instruction: Emit");
            process_emit(program_id, accounts, data)
        }
    }
}
