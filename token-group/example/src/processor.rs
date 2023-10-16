//! Program state processor

use {
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        program_option::COption,
        pubkey::Pubkey,
    },
    spl_pod::optional_keys::OptionalNonZeroPubkey,
    spl_token_2022::{extension::StateWithExtensions, state::Mint},
    spl_token_group_interface::{
        error::TokenGroupError,
        instruction::{
            InitializeGroup, TokenGroupInstruction, UpdateGroupAuthority, UpdateGroupMaxSize,
        },
        state::{TokenGroup, TokenGroupMember},
    },
    spl_type_length_value::state::TlvStateMut,
};

fn check_update_authority(
    update_authority_info: &AccountInfo,
    expected_update_authority: &OptionalNonZeroPubkey,
) -> Result<(), ProgramError> {
    if !update_authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let update_authority = Option::<Pubkey>::from(*expected_update_authority)
        .ok_or(TokenGroupError::ImmutableGroup)?;
    if update_authority != *update_authority_info.key {
        return Err(TokenGroupError::IncorrectAuthority.into());
    }
    Ok(())
}

/// Processes an [InitializeGroup](enum.GroupInterfaceInstruction.html)
/// instruction for a `Collection`
pub fn process_initialize_collection(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: InitializeGroup,
) -> ProgramResult {
    // Assumes one has already created a mint for the collection.
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]`   Collection (Group)
    //   1. `[]`    Mint
    //   2. `[s]`   Mint authority
    let collection_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;

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
            return Err(TokenGroupError::IncorrectAuthority.into());
        }
    }

    // Allocate a TLV entry for the space and write it in
    let mut buffer = collection_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    let (collection, _) = state.init_value::<TokenGroup>(false)?;
    *collection = TokenGroup::new(data.update_authority, data.max_size.into());

    Ok(())
}

/// Processes an
/// [UpdateGroupMaxSize](enum.GroupInterfaceInstruction.html)
/// instruction for a `Collection`
pub fn process_update_collection_max_size(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: UpdateGroupMaxSize,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]`   Collection (Group)
    //   1. `[s]`   Update authority
    let collection_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    let mut buffer = collection_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    let collection = state.get_first_value_mut::<TokenGroup>()?;

    check_update_authority(update_authority_info, &collection.update_authority)?;

    // Update the max size (zero-copy)
    collection.update_max_size(data.max_size.into())?;

    Ok(())
}

/// Processes an
/// [UpdateGroupAuthority](enum.GroupInterfaceInstruction.html)
/// instruction for a `Collection`
pub fn process_update_collection_authority(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: UpdateGroupAuthority,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]`   Collection (Group)
    //   1. `[s]`   Current update authority
    let collection_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    let mut buffer = collection_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    let mut collection = state.get_first_value_mut::<TokenGroup>()?;

    check_update_authority(update_authority_info, &collection.update_authority)?;

    // Update the authority (zero-copy)
    collection.update_authority = data.new_authority;

    Ok(())
}

/// Processes an [InitializeMember](enum.GroupInterfaceInstruction.html)
/// instruction for a `Collection`
pub fn process_initialize_collection_member(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    // For this group, we are going to assume the collection has been
    // initialized, and we're also assuming a mint has been created for the
    // member.
    // Collection members in this example can have their own separate
    // metadata that differs from the metadata of the collection, since
    // metadata is not involved here.
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]`   Collection Member (Member)
    //   1. `[]`    Collection Member (Member) Mint
    //   2. `[s]`   Collection Member (Member) Mint authority
    //   3. `[w]`   Collection (Group)
    //   4. `[s]`   Collection (Group) update authority
    let member_info = next_account_info(account_info_iter)?;
    let member_mint_info = next_account_info(account_info_iter)?;
    let member_mint_authority_info = next_account_info(account_info_iter)?;
    let collection_info = next_account_info(account_info_iter)?;
    let collection_update_authority_info = next_account_info(account_info_iter)?;

    // Mint checks on the member
    {
        // IMPORTANT: this example program is designed to work with any
        // program that implements the SPL token interface, so there is no
        // ownership check on the mint account.
        let member_mint_data = member_mint_info.try_borrow_data()?;
        let member_mint = StateWithExtensions::<Mint>::unpack(&member_mint_data)?;

        if !member_mint_authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if member_mint.base.mint_authority.as_ref() != COption::Some(member_mint_authority_info.key)
        {
            return Err(TokenGroupError::IncorrectAuthority.into());
        }
    }

    // Increment the size of the collection
    let mut buffer = collection_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    let collection = state.get_first_value_mut::<TokenGroup>()?;

    check_update_authority(
        collection_update_authority_info,
        &collection.update_authority,
    )?;
    let member_number = collection.increment_size()?;

    // Allocate a TLV entry for the space and write it in
    let mut buffer = member_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    let (member, _) = state.init_value::<TokenGroupMember>(false)?;
    *member = TokenGroupMember::new(*collection_info.key, member_number);

    Ok(())
}

/// Processes an `SplTokenGroupInstruction`
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = TokenGroupInstruction::unpack(input)?;
    match instruction {
        TokenGroupInstruction::InitializeGroup(data) => {
            msg!("Instruction: InitializeCollection");
            process_initialize_collection(program_id, accounts, data)
        }
        TokenGroupInstruction::UpdateGroupMaxSize(data) => {
            msg!("Instruction: UpdateCollectionMaxSize");
            process_update_collection_max_size(program_id, accounts, data)
        }
        TokenGroupInstruction::UpdateGroupAuthority(data) => {
            msg!("Instruction: UpdateCollectionAuthority");
            process_update_collection_authority(program_id, accounts, data)
        }
        TokenGroupInstruction::InitializeMember(_) => {
            msg!("Instruction: InitializeCollectionMember");
            process_initialize_collection_member(program_id, accounts)
        }
    }
}
