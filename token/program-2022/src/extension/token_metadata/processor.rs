//! Token-metadata processor

use {
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
    spl_token_metadata_interface::{
        instruction::{
            Emit, Initialize, RemoveKey, TokenMetadataInstruction, UpdateAuthority, UpdateField,
        },
    },
};

/// Processes a [Initialize](enum.TokenMetadataInstruction.html) instruction.
pub fn process_initialize(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _data: Initialize,
) -> ProgramResult {
    Ok(())
}

/// Processes an [UpdateField](enum.TokenMetadataInstruction.html) instruction.
pub fn process_update_field(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _data: UpdateField,
) -> ProgramResult {
    Ok(())
}

/// Processes a [RemoveKey](enum.TokenMetadataInstruction.html) instruction.
pub fn process_remove_key(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _data: RemoveKey,
) -> ProgramResult {
    Ok(())
}

/// Processes a [UpdateAuthority](enum.TokenMetadataInstruction.html) instruction.
pub fn process_update_authority(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _data: UpdateAuthority,
) -> ProgramResult {
    Ok(())
}

/// Processes an [Emit](enum.TokenMetadataInstruction.html) instruction.
pub fn process_emit(_program_id: &Pubkey, _accounts: &[AccountInfo], _data: Emit) -> ProgramResult {
    Ok(())
}

/// Processes an [Instruction](enum.Instruction.html).
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction: TokenMetadataInstruction,
) -> ProgramResult {
    match instruction {
        TokenMetadataInstruction::Initialize(data) => {
            msg!("TokenMetadataInstruction: Initialize");
            process_initialize(program_id, accounts, data)
        }
        TokenMetadataInstruction::UpdateField(data) => {
            msg!("TokenMetadataInstruction: UpdateField");
            process_update_field(program_id, accounts, data)
        }
        TokenMetadataInstruction::RemoveKey(data) => {
            msg!("TokenMetadataInstruction: RemoveKey");
            process_remove_key(program_id, accounts, data)
        }
        TokenMetadataInstruction::UpdateAuthority(data) => {
            msg!("TokenMetadataInstruction: UpdateAuthority");
            process_update_authority(program_id, accounts, data)
        }
        TokenMetadataInstruction::Emit(data) => {
            msg!("TokenMetadataInstruction: Emit");
            process_emit(program_id, accounts, data)
        }
    }
}
