//! Program state processor

use {
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        program_error::ProgramError,
        program_option::COption,
        pubkey::Pubkey,
    },
    spl_pod::optional_keys::OptionalNonZeroPubkey,
    spl_token_2022::{
        extension::{
            metadata_pointer::MetadataPointer, BaseStateWithExtensions, StateWithExtensions,
        },
        state::Mint,
    },
    spl_token_group_interface::{
        error::TokenGroupError,
        instruction::{
            InitializeGroup, TokenGroupInstruction, UpdateGroupAuthority, UpdateGroupMaxSize,
        },
        state::{TokenGroup, TokenGroupMember},
    },
    spl_token_metadata_interface::state::TokenMetadata,
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

/// Processes a [InitializeMember](enum.GroupInterfaceInstruction.html)
/// instruction for an `Edition`.
///
/// This function demonstrates using this interface for editions as well.
fn process_initialize_edition_reprint(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    // Here we are going to assume the original has been created and
    // initialized as a group, then we can use the original to print "reprints"
    // from it.
    // We're also assuming a mint _and_ metadata have been created for _both_
    // the original and the reprint.
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]`   Reprint (Member)
    //   1. `[]`    Reprint (Member) Mint
    //   2. `[s]`   Reprint (Member) Mint authority
    //   3. `[w]`   Original (Group)
    //   4. `[s]`   Original (Group) update authority
    let reprint_info = next_account_info(account_info_iter)?;
    // Note this particular example _also_ requires the mint to be writable!
    let reprint_mint_info = next_account_info(account_info_iter)?;
    let reprint_mint_authority_info = next_account_info(account_info_iter)?;
    let original_info = next_account_info(account_info_iter)?;
    let original_update_authority_info = next_account_info(account_info_iter)?;

    // Additional accounts expected by this instruction:
    //
    //   5. `[]`    Original (Group) Mint
    //   6. `[]`    SPL Token 2022 program
    let original_mint_info = next_account_info(account_info_iter)?;
    let _program_2022_info = next_account_info(account_info_iter)?;

    // Mint & metadata checks on the original
    let original_token_metadata = {
        // IMPORTANT: this example program is designed to work with any
        // program that implements the SPL token interface, so there is no
        // ownership check on the mint account.
        let original_mint_data = original_mint_info.try_borrow_data()?;
        let original_mint = StateWithExtensions::<Mint>::unpack(&original_mint_data)?;

        // Make sure the metadata pointer is pointing to the mint itself
        let metadata_pointer = original_mint.get_extension::<MetadataPointer>()?;
        let metadata_pointer_address = Option::<Pubkey>::from(metadata_pointer.metadata_address);
        if metadata_pointer_address != Some(*original_mint_info.key) {
            return Err(ProgramError::InvalidAccountData);
        }

        // Extract the token metadata
        original_mint.get_variable_len_extension::<TokenMetadata>()?
    };

    // Mint checks on the reprint
    {
        // IMPORTANT: this example program is designed to work with any
        // program that implements the SPL token interface, so there is no
        // ownership check on the mint account.
        let reprint_mint_data = reprint_mint_info.try_borrow_data()?;
        let reprint_mint = StateWithExtensions::<Mint>::unpack(&reprint_mint_data)?;

        if !reprint_mint_authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if reprint_mint.base.mint_authority.as_ref()
            != COption::Some(reprint_mint_authority_info.key)
        {
            return Err(TokenGroupError::IncorrectAuthority.into());
        }

        // Make sure the metadata pointer is pointing to the mint itself
        let metadata_pointer = reprint_mint.get_extension::<MetadataPointer>()?;
        let metadata_pointer_address = Option::<Pubkey>::from(metadata_pointer.metadata_address);
        if metadata_pointer_address != Some(*reprint_mint_info.key) {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    // Increment the size of the editions
    let mut buffer = original_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    let original = state.get_first_value_mut::<TokenGroup>()?;

    check_update_authority(original_update_authority_info, &original.update_authority)?;
    let reprint_number = original.increment_size()?;

    // Allocate a TLV entry for the space and write it in
    let mut buffer = reprint_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    let (reprint, _) = state.init_value::<TokenGroupMember>(false)?;
    *reprint = TokenGroupMember::new(*original_info.key, reprint_number);

    // Use the original metadata to initialize the reprint metadata
    let cpi_instruction = spl_token_metadata_interface::instruction::initialize(
        &spl_token_2022::id(),
        reprint_mint_info.key,
        original_update_authority_info.key,
        reprint_mint_info.key,
        reprint_mint_authority_info.key,
        original_token_metadata.name,
        original_token_metadata.symbol,
        original_token_metadata.uri,
    );
    let cpi_account_infos = &[
        reprint_mint_info.clone(),
        original_update_authority_info.clone(),
        reprint_mint_info.clone(),
        reprint_mint_authority_info.clone(),
    ];
    invoke(&cpi_instruction, cpi_account_infos)?;

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
            // For demonstration purposes, we'll use the number of accounts
            // provided to determine which type of member to initialize.
            if accounts.len() == 5 {
                msg!("Instruction: InitializeCollectionMember");
                process_initialize_collection_member(program_id, accounts)
            } else {
                msg!("Instruction: InitializeEdition");
                process_initialize_edition_reprint(program_id, accounts)
            }
        }
    }
}
