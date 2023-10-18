//! Token-group processor

use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            StateWithExtensions, update_authority::check_update_authority
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
        instruction::{
            InitializeGroup, TokenGroupInstruction, UpdateGroupMaxSize, UpdateGroupAuthority
        },
        state::{TokenGroup, TokenGroupMember},
    },
    spl_type_length_value::state::TlvStateMut,
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
    }

    // Allocate a TLV entry for the space and write it in
    // Assumes that there's enough SOL for the new rent-exemption
    let mut buffer = group_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    let (group, _) = state.init_value::<TokenGroup>(false)?;
    *group = TokenGroup::new(data.update_authority, data.max_size.into());

    Ok(())
}

/// Processes an
/// [UpdateGroupMaxSize](enum.GroupInterfaceInstruction.html)
/// instruction
pub fn process_update_group_max_size(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: UpdateGroupMaxSize,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let group_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    let mut buffer = group_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    let group = state.get_first_value_mut::<TokenGroup>()?;

    check_update_authority(update_authority_info, &group.update_authority)?;

    // Update the max size (zero-copy)
    group.update_max_size(data.max_size.into())?;

    Ok(())
}

/// Processes an
/// [UpdateGroupAuthority](enum.GroupInterfaceInstruction.html)
/// instruction
pub fn process_update_group_authority(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: UpdateGroupAuthority,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let group_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    let mut buffer = group_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    let mut group = state.get_first_value_mut::<TokenGroup>()?;

    check_update_authority(update_authority_info, &group.update_authority)?;

    // Update the authority (zero-copy)
    group.update_authority = data.new_authority;

    Ok(())
}

/// Processes an [InitializeMember](enum.GroupInterfaceInstruction.html)
/// instruction
pub fn process_initialize_group_member(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let member_info = next_account_info(account_info_iter)?;
    let member_mint_info = next_account_info(account_info_iter)?;
    let member_mint_authority_info = next_account_info(account_info_iter)?;
    let group_info = next_account_info(account_info_iter)?;
    let group_update_authority_info = next_account_info(account_info_iter)?;

    // scope the mint authority check, since the mint is in the same account!
    {
        // This check isn't really needed since we'll be writing into the account,
        // but auditors like it
        check_program_account(member_mint_info.owner)?;
        let member_mint_data = member_mint_info.try_borrow_data()?;
        let member_mint = StateWithExtensions::<Mint>::unpack(&member_mint_data)?;

        if !member_mint_authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if member_mint.base.mint_authority.as_ref() != COption::Some(member_mint_authority_info.key) {
            return Err(TokenGroupError::IncorrectMintAuthority.into());
        }
    }

    // Increment the size of the group
    let mut buffer = group_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    let group = state.get_first_value_mut::<TokenGroup>()?;

    check_update_authority(
        group_update_authority_info,
        &group.update_authority,
    )?;
    let member_number = group.increment_size()?;

    // Allocate a TLV entry for the space and write it in
    let mut buffer = member_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    let (member, _) = state.init_value::<TokenGroupMember>(false)?;
    *member = TokenGroupMember::new(*group_info.key, member_number);

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
        TokenGroupInstruction::UpdateGroupMaxSize(data) => {
            msg!("TokenGroupInstruction: UpdateGroupMaxSize");
            process_update_group_max_size(program_id, accounts, data)
        }
        TokenGroupInstruction::UpdateGroupAuthority(data) => {
            msg!("TokenGroupInstruction: UpdateGroupAuthority");
            process_update_group_authority(program_id, accounts, data)
        }
        TokenGroupInstruction::InitializeMember(_) => {
            msg!("TokenGroupInstruction: InitializeMember");
            process_initialize_group_member(program_id, accounts)
        }
    }
}
