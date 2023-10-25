//! Token-group processor

use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            alloc_and_serialize, group_pointer::GroupPointer, BaseStateWithExtensions,
            StateWithExtensions,
        },
        state::Mint,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        program_option::COption,
        pubkey::Pubkey,
    },
    spl_token_group_interface::{
        error::TokenGroupError,
        instruction::{InitializeGroup, TokenGroupInstruction},
        state::TokenGroup,
    },
};

/// Processes a [InitializeGroup](enum.TokenGroupInstruction.html) instruction.
pub fn process_initialize_group(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: InitializeGroup,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let group_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;

    // check that the mint and group accounts are the same, since the group
    // extension should only describe itself
    if group_info.key != mint_info.key {
        msg!("Group configurations for a mint must be initialized in the mint itself.");
        return Err(TokenError::MintMismatch.into());
    }

    // scope the mint authority check, since the mint is in the same account!
    {
        // This check isn't really needed since we'll be writing into the account,
        // but auditors like it
        check_program_account(mint_info.owner)?;
        let mint_data = mint_info.try_borrow_data()?;
        let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;

        if !mint_authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if mint.base.mint_authority.as_ref() != COption::Some(mint_authority_info.key) {
            return Err(TokenGroupError::IncorrectMintAuthority.into());
        }

        if mint.get_extension::<GroupPointer>().is_err() {
            msg!(
                "A mint with group configurations must have the group-pointer extension \
                 initialized"
            );
            return Err(TokenError::InvalidExtensionCombination.into());
        }
    }

    // Allocate a TLV entry for the space and write it in
    // Assumes that there's enough SOL for the new rent-exemption
    let group = TokenGroup::new(mint_info.key, data.update_authority, data.max_size.into());
    alloc_and_serialize::<Mint, TokenGroup>(group_info, &group, false)?;

    Ok(())
}

/// Processes an [Instruction](enum.Instruction.html).
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction: TokenGroupInstruction,
) -> ProgramResult {
    match instruction {
        TokenGroupInstruction::InitializeGroup(data) => {
            msg!("TokenGroupInstruction: InitializeGroup");
            process_initialize_group(program_id, accounts, data)
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
