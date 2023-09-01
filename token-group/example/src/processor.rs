//! Program state processor

use {
    crate::state::{Collection, Edition},
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
            get_emit_slice, Emit, InitializeGroup, InitializeMember, ItemType,
            TokenGroupInterfaceInstruction, UpdateGroupAuthority, UpdateGroupMaxSize,
        },
        state::{Group, Member, SplTokenGroup},
    },
    spl_token_metadata_interface::{instruction::initialize, state::TokenMetadata},
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
        .ok_or(TokenGroupError::ImmutableGroup)?;
    if update_authority != *update_authority_info.key {
        return Err(TokenGroupError::IncorrectAuthority.into());
    }
    Ok(())
}

/// Processes a [InitializeGroup](enum.GroupInterfaceInstruction.html)
/// instruction for a `Collection`
pub fn process_initialize_collection(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: InitializeGroup<Collection>,
) -> ProgramResult {
    // Assumes one has already created a mint for the collection.
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]`  Collection (Group)
    //   1. `[]`   Mint
    //   2. `[s]`  Mint authority
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

    let collection = Group::new(data.update_authority, data.max_size, data.meta);
    let instance_size = get_instance_packed_len(&collection)?;

    // Allocate a TLV entry for the space and write it in
    let mut buffer = collection_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    state.alloc::<Group<Collection>>(instance_size, false)?;
    state.pack_first_variable_len_value(&collection)?;

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
    //   0. `[w]`  Collection (Group)
    //   1. `[s]`  Update authority
    let collection_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    let mut collection = {
        let buffer = collection_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&buffer)?;
        state.get_first_variable_len_value::<Group<Collection>>()?
    };

    check_update_authority(update_authority_info, &collection.update_authority)?;

    // Update the max size
    collection.update_max_size(data.max_size)?;

    // Update the account, no realloc needed!
    realloc_and_pack_first_variable_len(collection_info, &collection)?;

    Ok(())
}

/// Processes a
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
    //   0. `[w]`  Collection (Group)
    //   1. `[s]`  Current update authority
    let collection_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    let mut collection = {
        let buffer = collection_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&buffer)?;
        state.get_first_variable_len_value::<Group<Collection>>()?
    };

    check_update_authority(update_authority_info, &collection.update_authority)?;

    // Update the authority
    collection.update_authority = data.new_authority;

    // Update the account, no realloc needed!
    realloc_and_pack_first_variable_len(collection_info, &collection)?;

    Ok(())
}

/// Processes a [InitializeMember](enum.GroupInterfaceInstruction.html)
/// instruction for a `Collection`
pub fn process_initialize_collection_member(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: InitializeMember,
) -> ProgramResult {
    // For this group, we are going to assume the collection has been
    // initialized, and we're also assuming a mint _and_ metadata have been
    // created for the member.
    // Collection memebers in this example can have their own separate
    // metadata that differs from the metadata of the collection.
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]`  Collection Member (Member)
    //   1. `[]`   Collection Member (Member) Mint
    //   2. `[s]`  Collection Member (Member) Mint authority
    //   3. `[w]`  Collection (Group)
    //   4. `[]`   Collection (Group) Mint
    //   5. `[s]`  Collection (Group) Mint authority
    let member_info = next_account_info(account_info_iter)?;
    let member_mint_info = next_account_info(account_info_iter)?;
    let member_mint_authority_info = next_account_info(account_info_iter)?;
    let collection_info = next_account_info(account_info_iter)?;
    let collection_mint_info = next_account_info(account_info_iter)?;
    let collection_mint_authority_info = next_account_info(account_info_iter)?;

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

    // Mint checks on the collection
    {
        // IMPORTANT: this example program is designed to work with any
        // program that implements the SPL token interface, so there is no
        // ownership check on the mint account.
        let collection_mint_data = collection_mint_info.try_borrow_data()?;
        let collection_mint = StateWithExtensions::<Mint>::unpack(&collection_mint_data)?;

        if !collection_mint_authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if collection_mint.base.mint_authority.as_ref()
            != COption::Some(collection_mint_authority_info.key)
        {
            return Err(TokenGroupError::IncorrectAuthority.into());
        }
    }

    if data.group != *collection_info.key {
        return Err(TokenGroupError::IncorrectGroup.into());
    }

    // Increment the size of the collection
    let mut collection = {
        let buffer = collection_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&buffer)?;
        state.get_first_variable_len_value::<Group<Collection>>()?
    };
    let member_number = collection.increment_size()?;
    realloc_and_pack_first_variable_len(collection_info, &collection)?;

    // Initialize the new collection member
    let member = Member {
        group: data.group,
        member_number,
    };
    let instance_size = get_instance_packed_len(&member)?;

    // allocate a TLV entry for the space and write it in
    let mut buffer = member_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    state.alloc::<Member>(instance_size, false)?;
    state.pack_first_variable_len_value(&member)?;

    Ok(())
}

/// Processes a [InitializeGroup](enum.GroupInterfaceInstruction.html)
/// instruction for an `Edition`
pub fn process_initialize_original(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: InitializeGroup<Edition>,
) -> ProgramResult {
    // Assumes one has already created a mint and a metadata account for the
    // original print
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]`  Original (Group)
    //   1. `[]`   Mint
    //   2. `[s]`  Mint authority
    let original_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;

    // Extra accounts expected by this instruction:
    //
    //   0. `[]`   Metadata
    let metadata_info = next_account_info(account_info_iter)?;

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
            return Err(TokenGroupError::IncorrectAuthority.into());
        }

        // IMPORTANT: metadata is passed as a separate account because it may be owned
        // by another program - separate from the program implementing the SPL token
        // interface _or_ the program implementing the SPL token editions interface.
        let metadata_pointer = mint.get_extension::<MetadataPointer>()?;
        let metadata_pointer_address = Option::<Pubkey>::from(metadata_pointer.metadata_address);
        if metadata_pointer_address != Some(*metadata_info.key) {
            return Err(TokenGroupError::IncorrectAccount.into());
        }
    }

    let original = Group::new(data.update_authority, data.max_size, data.meta);
    let instance_size = get_instance_packed_len(&original)?;

    // Allocate a TLV entry for the space and write it in
    let mut buffer = original_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    state.alloc::<Group<Edition>>(instance_size, false)?;
    state.pack_first_variable_len_value(&original)?;

    Ok(())
}

/// Processes an
/// [UpdateGroupMaxSize](enum.GroupInterfaceInstruction.html)
/// instruction for an `Edition`
pub fn process_update_edition_max_size(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: UpdateGroupMaxSize,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]`  Original (Group)
    //   1. `[s]`  Update authority
    let original_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    let mut original = {
        let buffer = original_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&buffer)?;
        state.get_first_variable_len_value::<Group<Edition>>()?
    };

    check_update_authority(update_authority_info, &original.update_authority)?;

    // Update the max size
    original.update_max_size(data.max_size)?;

    // Update the account, no realloc needed!
    realloc_and_pack_first_variable_len(original_info, &original)?;

    Ok(())
}

/// Processes a
/// [UpdateGroupAuthority](enum.GroupInterfaceInstruction.html)
/// instruction for an `Edition`
pub fn process_update_edition_authority(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: UpdateGroupAuthority,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]`  Original (Group)
    //   1. `[s]`  Current update authority
    let original_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    let mut original = {
        let buffer = original_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&buffer)?;
        state.get_first_variable_len_value::<Group<Edition>>()?
    };

    check_update_authority(update_authority_info, &original.update_authority)?;

    // Update the authority
    original.update_authority = data.new_authority;

    // Update the account, no realloc needed!
    realloc_and_pack_first_variable_len(original_info, &original)?;

    Ok(())
}

/// Processes a [InitializeMember](enum.GroupInterfaceInstruction.html)
/// instruction for an `Edition`
pub fn process_initialize_reprint(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: InitializeMember,
) -> ProgramResult {
    // For this group, we are going to assume the original print has been
    // initialized, but _only_ a mint has been created for the reprint.
    // We will then copy the original print's metadata to the reprint's
    // metadata, creating a copy!
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]`  Reprint (Member)
    //   1. `[]`   Reprint (Member) Mint
    //   2. `[s]`  Reprint (Member) Mint authority
    //   3. `[w]`  Original (Group)
    //   4. `[]`   Original (Group) Mint
    //   5. `[s]`  Original (Group) Mint authority
    let reprint_info = next_account_info(account_info_iter)?;
    let reprint_mint_info = next_account_info(account_info_iter)?;
    let reprint_mint_authority_info = next_account_info(account_info_iter)?;
    let original_info = next_account_info(account_info_iter)?;
    let original_mint_info = next_account_info(account_info_iter)?;
    let original_mint_authority_info = next_account_info(account_info_iter)?;

    // Extra accounts expected by this instruction:
    //
    //   0. `[w]`  Reprint Metadata
    //   1. `[]`   Reprint Metadata Update Authoriy
    //   2. `[]`   Original Metadata
    //   3. `[]`   Token Metadata Program
    let reprint_metadata_info = next_account_info(account_info_iter)?;
    let reprint_metadata_update_authority_info = next_account_info(account_info_iter)?;
    let original_metadata_info = next_account_info(account_info_iter)?;
    let metadata_program_info = next_account_info(account_info_iter)?;

    // Mint & metadata checks on the original
    let token_metadata = {
        // IMPORTANT: this example program is designed to work with any
        // program that implements the SPL token interface, so there is no
        // ownership check on the mint account.
        let original_mint_data = original_mint_info.try_borrow_data()?;
        let original_mint = StateWithExtensions::<Mint>::unpack(&original_mint_data)?;

        if !original_mint_authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if original_mint.base.mint_authority.as_ref()
            != COption::Some(original_mint_authority_info.key)
        {
            return Err(TokenGroupError::IncorrectAuthority.into());
        }

        // IMPORTANT: metadata is passed as a separate account because it may be owned
        // by another program - separate from the program implementing the SPL token
        // interface _or_ the program implementing the SPL token editions interface.
        let metadata_pointer = original_mint.get_extension::<MetadataPointer>()?;
        let metadata_pointer_address = Option::<Pubkey>::from(metadata_pointer.metadata_address);
        if metadata_pointer_address != Some(*original_metadata_info.key) {
            return Err(TokenGroupError::IncorrectAccount.into());
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

        if !reprint_mint_authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if reprint_mint.base.mint_authority.as_ref()
            != COption::Some(reprint_mint_authority_info.key)
        {
            return Err(TokenGroupError::IncorrectAuthority.into());
        }

        // IMPORTANT: metadata is passed as a separate account because it may be owned
        // by another program - separate from the program implementing the SPL token
        // interface _or_ the program implementing the SPL token editions interface.
        let metadata_pointer = reprint_mint.get_extension::<MetadataPointer>()?;
        let metadata_pointer_address = Option::<Pubkey>::from(metadata_pointer.metadata_address);
        if metadata_pointer_address != Some(*reprint_metadata_info.key) {
            return Err(TokenGroupError::IncorrectAccount.into());
        }
    }

    let mut original_print = {
        let buffer = original_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&buffer)?;
        state.get_first_variable_len_value::<Group<Edition>>()?
    };

    if data.group != *original_info.key {
        return Err(TokenGroupError::IncorrectAccount.into());
    }

    // Update the current supply
    let member_number = original_print.increment_size()?;
    realloc_and_pack_first_variable_len(original_info, &original_print)?;

    // Initialize the reprint metadata from the original metadata
    let cpi_instruction = initialize(
        metadata_program_info.key,
        reprint_mint_info.key,
        reprint_metadata_update_authority_info.key,
        reprint_mint_info.key,
        reprint_mint_authority_info.key,
        token_metadata.name,
        token_metadata.symbol,
        token_metadata.uri,
    );
    let cpi_account_infos = &[
        reprint_mint_info.clone(),
        reprint_metadata_update_authority_info.clone(),
        reprint_mint_info.clone(),
        reprint_mint_authority_info.clone(),
    ];
    invoke(&cpi_instruction, cpi_account_infos)?;

    // Initialize the reprint
    let reprint = Member {
        group: *original_info.key,
        member_number,
    };
    let instance_size = get_instance_packed_len(&reprint)?;

    // allocate a TLV entry for the space and write it in
    let mut buffer = reprint_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    state.alloc::<Member>(instance_size, false)?;
    state.pack_first_variable_len_value(&reprint)?;

    Ok(())
}

/// Processes an [Emit](enum.InterfaceBaseInstruction.html) instruction.
pub fn process_emit<G>(program_id: &Pubkey, accounts: &[AccountInfo], data: Emit) -> ProgramResult
where
    G: SplTokenGroup,
{
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[]` Group or Member account
    let asset_info = next_account_info(account_info_iter)?;

    if asset_info.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    let buffer = asset_info.try_borrow_data()?;
    let state = TlvStateBorrowed::unpack(&buffer)?;

    let item_bytes = match data.item_type {
        ItemType::Group => state.get_first_bytes::<Group<G>>()?,
        ItemType::Member => state.get_first_bytes::<Member>()?,
    };

    if let Some(range) = get_emit_slice(item_bytes, data.start, data.end) {
        set_return_data(range);
    }

    Ok(())
}

/// Processor for a `Collection`
fn process_collection_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction: TokenGroupInterfaceInstruction<Collection>,
) -> ProgramResult {
    match instruction {
        TokenGroupInterfaceInstruction::InitializeGroup(data) => {
            msg!("Instruction: InitializeCollection");
            process_initialize_collection(program_id, accounts, data)
        }
        TokenGroupInterfaceInstruction::UpdateGroupMaxSize(data) => {
            msg!("Instruction: UpdateCollectionMaxSize");
            process_update_collection_max_size(program_id, accounts, data)
        }
        TokenGroupInterfaceInstruction::UpdateGroupAuthority(data) => {
            msg!("Instruction: UpdateCollectionAuthority");
            process_update_collection_authority(program_id, accounts, data)
        }
        TokenGroupInterfaceInstruction::InitializeMember(data) => {
            msg!("Instruction: InitializeCollectionMember");
            process_initialize_collection_member(program_id, accounts, data)
        }
        TokenGroupInterfaceInstruction::Emit(data) => {
            msg!("Instruction: Emit");
            process_emit::<Collection>(program_id, accounts, data)
        }
    }
}

/// Processor for an `Edition`
fn process_edition_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction: TokenGroupInterfaceInstruction<Edition>,
) -> ProgramResult {
    match instruction {
        TokenGroupInterfaceInstruction::InitializeGroup(data) => {
            msg!("Instruction: InitializeOriginal");
            process_initialize_original(program_id, accounts, data)
        }
        TokenGroupInterfaceInstruction::UpdateGroupMaxSize(data) => {
            msg!("Instruction: UpdateEditionMaxSize");
            process_update_edition_max_size(program_id, accounts, data)
        }
        TokenGroupInterfaceInstruction::UpdateGroupAuthority(data) => {
            msg!("Instruction: UpdateEditionAuthority");
            process_update_edition_authority(program_id, accounts, data)
        }
        TokenGroupInterfaceInstruction::InitializeMember(data) => {
            msg!("Instruction: InitializeReprint");
            process_initialize_reprint(program_id, accounts, data)
        }
        TokenGroupInterfaceInstruction::Emit(data) => {
            msg!("Instruction: Emit");
            process_emit::<Edition>(program_id, accounts, data)
        }
    }
}

/// Processes an `SplTokenGroupInstruction`
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    if TokenGroupInterfaceInstruction::<Collection>::peek(input) {
        process_collection_instruction(
            program_id,
            accounts,
            TokenGroupInterfaceInstruction::<Collection>::unpack(input)?,
        )
    } else if TokenGroupInterfaceInstruction::<Edition>::peek(input) {
        process_edition_instruction(
            program_id,
            accounts,
            TokenGroupInterfaceInstruction::<Edition>::unpack(input)?,
        )
    } else {
        Err(ProgramError::InvalidInstructionData)
    }
}
