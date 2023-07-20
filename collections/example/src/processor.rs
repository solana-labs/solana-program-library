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
    spl_token_collections_interface::{
        error::TokenCollectionsError,
        instruction::{
            CreateCollection, CreateMember, Emit, ItemType, TokenCollectionsInstruction,
            UpdateCollectionAuthority, UpdateCollectionMaxSize,
        },
        state::{get_emit_slice, Collection, Member, OptionalNonZeroPubkey},
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
        .ok_or(TokenCollectionsError::ImmutableCollection)?;
    if update_authority != *update_authority_info.key {
        return Err(TokenCollectionsError::IncorrectUpdateAuthority.into());
    }
    Ok(())
}

/// Processes a [CreateCollection](enum.TokenCollectionsInstruction.html)
/// instruction.
pub fn process_create_collection(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: CreateCollection,
) -> ProgramResult {
    // Assumes one has already created a mint for the
    // collection.
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]`  Collection
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
            return Err(TokenCollectionsError::IncorrectMintAuthority.into());
        }
    }

    let collection = Collection::new(data.update_authority, data.max_size);
    let instance_size = get_instance_packed_len(&collection)?;

    // Allocate a TLV entry for the space and write it in
    let mut buffer = collection_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    state.alloc::<Collection>(instance_size)?;
    state.pack_variable_len_value(&collection)?;

    Ok(())
}

/// Processes an
/// [UpdateCollectionMaxSize](enum.TokenCollectionsInstruction.html)
/// instruction.
pub fn process_update_collection_max_size(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: UpdateCollectionMaxSize,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]`  Collection
    //   1. `[s]`  Update authority
    let collection_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    let mut collection = {
        let buffer = collection_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&buffer)?;
        state.get_variable_len_value::<Collection>()?
    };

    check_update_authority(update_authority_info, &collection.update_authority)?;

    // Update the max size
    collection.update_max_size(data.max_size)?;

    // Update the account, no realloc needed!
    realloc_and_pack_variable_len(collection_info, &collection)?;

    Ok(())
}

/// Processes a
/// [UpdateCollectionAuthority](enum.TokenCollectionsInstruction.html)
/// instruction.
pub fn process_update_collection_authority(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: UpdateCollectionAuthority,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]`  Collection
    //   1. `[s]`  Current update authority
    let collection_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    let mut collection = {
        let buffer = collection_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&buffer)?;
        state.get_variable_len_value::<Collection>()?
    };

    check_update_authority(update_authority_info, &collection.update_authority)?;

    // Update the authority
    collection.update_authority = data.new_authority;

    // Update the account, no realloc needed!
    realloc_and_pack_variable_len(collection_info, &collection)?;

    Ok(())
}

/// Processes a [CreateMember](enum.TokenCollectionsInstruction.html)
/// instruction.
pub fn process_create_member(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: CreateMember,
) -> ProgramResult {
    // Assumes the `Collection` has already been created,
    // as well as the mint for the member.
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]`  Member
    //   2. `[]`   Member Mint
    //   7. `[s]`  Member Mint authority
    //   3. `[w]`  Collection
    //   6. `[]`   Collection Mint
    //   7. `[s]`  Collection Mint authority
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
            return Err(TokenCollectionsError::IncorrectMintAuthority.into());
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
            return Err(TokenCollectionsError::IncorrectMintAuthority.into());
        }
    }

    if data.collection != *collection_info.key {
        return Err(TokenCollectionsError::IncorrectCollection.into());
    }

    // Increment the size of the collection
    let mut collection = {
        let buffer = collection_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&buffer)?;
        state.get_variable_len_value::<Collection>()?
    };
    collection.update_size(collection.size + 1)?;
    realloc_and_pack_variable_len(collection_info, &collection)?;

    // Create the new collection member
    let member = Member {
        collection: data.collection,
    };
    let instance_size = get_instance_packed_len(&member)?;

    // allocate a TLV entry for the space and write it in
    let mut buffer = member_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    state.alloc::<Member>(instance_size)?;
    state.pack_variable_len_value(&member)?;

    Ok(())
}

/// Processes an [Emit](enum.TokenCollectionsInstruction.html) instruction.
pub fn process_emit(program_id: &Pubkey, accounts: &[AccountInfo], data: Emit) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[]` Collection or Member account
    let print_info = next_account_info(account_info_iter)?;

    if print_info.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    let buffer = print_info.try_borrow_data()?;
    let state = TlvStateBorrowed::unpack(&buffer)?;

    // `print_type` determines which asset we're working with
    let item_bytes = match data.item_type {
        ItemType::Collection => state.get_bytes::<Collection>()?,
        ItemType::Member => state.get_bytes::<Member>()?,
    };

    if let Some(range) = get_emit_slice(item_bytes, data.start, data.end) {
        set_return_data(range);
    }

    Ok(())
}

/// Processes an [Instruction](enum.Instruction.html).
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = TokenCollectionsInstruction::unpack(input)?;

    match instruction {
        TokenCollectionsInstruction::CreateCollection(data) => {
            msg!("Instruction: CreateCollection");
            process_create_collection(program_id, accounts, data)
        }
        TokenCollectionsInstruction::UpdateCollectionMaxSize(data) => {
            msg!("Instruction: UpdateCollectionMaxSize");
            process_update_collection_max_size(program_id, accounts, data)
        }
        TokenCollectionsInstruction::UpdateCollectionAuthority(data) => {
            msg!("Instruction: UpdateCollectionAuthority");
            process_update_collection_authority(program_id, accounts, data)
        }
        TokenCollectionsInstruction::CreateMember(data) => {
            msg!("Instruction: CreateMember");
            process_create_member(program_id, accounts, data)
        }
        TokenCollectionsInstruction::Emit(data) => {
            msg!("Instruction: Emit");
            process_emit(program_id, accounts, data)
        }
    }
}
