//! Program state processor

use {
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh::get_instance_packed_len,
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        program_option::COption,
        pubkey::Pubkey,
    },
    spl_token_2022::{extension::StateWithExtensions, state::Mint},
    spl_token_metadata_interface::{
        error::TokenMetadataError,
        instruction::{
            Emit, Initialize, RemoveKey, TokenMetadataInstruction, UpdateAuthority, UpdateField,
        },
        state::{OptionalNonZeroPubkey, TokenMetadata},
    },
    spl_type_length_value::state::{TlvState, TlvStateBorrowed, TlvStateMut},
};

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
    state.alloc::<TokenMetadata>(instance_size)?;
    state.borsh_serialize(&token_metadata)?;

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
        state.borsh_deserialize::<TokenMetadata>()?
    };

    // check the update authority signed
    if !update_authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let update_authority = Option::<Pubkey>::from(token_metadata.update_authority.clone())
        .ok_or(TokenMetadataError::ImmutableMetadata)?;
    if update_authority != *update_authority_info.key {
        return Err(TokenMetadataError::IncorrectUpdateAuthority.into());
    }

    // Update the field
    let previous_size = get_instance_packed_len(&token_metadata)?;
    token_metadata.update(data.field, data.value);
    let new_size = get_instance_packed_len(&token_metadata)?;

    // Realloc the account and TLV entry
    let previous_account_size = metadata_info.try_data_len()?;
    if previous_size < new_size {
        // size increased, so realloc the account, then the TLV entry, then write data
        let additional_bytes = new_size
            .checked_sub(previous_size)
            .ok_or(ProgramError::AccountDataTooSmall)?;
        metadata_info.realloc(previous_account_size.saturating_add(additional_bytes), true)?;
        let mut buffer = metadata_info.try_borrow_mut_data()?;
        let mut state = TlvStateMut::unpack(&mut buffer)?;
        state.realloc::<TokenMetadata>(new_size)?;
        state.borsh_serialize(&token_metadata)?;
    } else {
        // do it backwards otherwise, write the state, realloc TLV, then the account
        let mut buffer = metadata_info.try_borrow_mut_data()?;
        let mut state = TlvStateMut::unpack(&mut buffer)?;
        state.borsh_serialize(&token_metadata)?;
        let removed_bytes = previous_size
            .checked_sub(new_size)
            .ok_or(ProgramError::AccountDataTooSmall)?;
        if removed_bytes > 0 {
            // we decreased the size, so need to realloc the TLV, then the account
            state.realloc::<TokenMetadata>(new_size)?;
            drop(state);
            drop(buffer);
            metadata_info.realloc(previous_account_size.saturating_sub(removed_bytes), false)?;
        }
    }

    Ok(())
}

/// Processes a [RemoveKey](enum.TokenMetadataInstruction.html) instruction.
pub fn process_remove_key(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: RemoveKey,
) -> ProgramResult {
    Ok(())
}

/// Processes a [UpdateAuthority](enum.TokenMetadataInstruction.html) instruction.
pub fn process_update_authority(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: UpdateAuthority,
) -> ProgramResult {
    Ok(())
}

/// Processes an [Emit](enum.TokenMetadataInstruction.html) instruction.
pub fn process_emit(program_id: &Pubkey, accounts: &[AccountInfo], data: Emit) -> ProgramResult {
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
