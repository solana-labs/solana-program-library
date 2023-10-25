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
    spl_token_2022::{
        extension::{
            metadata_pointer::MetadataPointer, BaseStateWithExtensions, StateWithExtensions,
        },
        state::Mint,
    },
    spl_token_group_interface::{
        error::TokenGroupError,
        instruction::{InitializeGroup, TokenGroupInstruction},
        state::{TokenGroup, TokenGroupMember},
    },
    spl_token_metadata_interface::state::TokenMetadata,
    spl_type_length_value::state::TlvStateMut,
};

fn check_update_authority(
    update_authority_info: &AccountInfo,
    expected_update_authority: &OptionalNonZeroPubkey,
) -> ProgramResult {
    if !update_authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let update_authority = Option::<Pubkey>::from(*expected_update_authority)
        .ok_or(TokenGroupError::ImmutableGroup)?;
    if update_authority != *update_authority_info.key {
        return Err(TokenGroupError::IncorrectUpdateAuthority.into());
    }
    Ok(())
}

/// Checks that a mint is valid and contains metadata.
fn check_mint_and_metadata(
    mint_info: &AccountInfo,
    mint_authority_info: &AccountInfo,
) -> ProgramResult {
    let mint_data = mint_info.try_borrow_data()?;
    let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;

    if !mint_authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if mint.base.mint_authority.as_ref() != COption::Some(mint_authority_info.key) {
        return Err(TokenGroupError::IncorrectMintAuthority.into());
    }

    let metadata_pointer = mint.get_extension::<MetadataPointer>()?;
    let metadata_pointer_address = Option::<Pubkey>::from(metadata_pointer.metadata_address);

    // If the metadata is inside the mint (Token2022), make sure it contains
    // valid TokenMetadata
    if metadata_pointer_address == Some(*mint_info.key) {
        mint.get_variable_len_extension::<TokenMetadata>()?;
    }

    Ok(())
}

/// Processes an [InitializeGroup](enum.GroupInterfaceInstruction.html)
/// instruction to initialize a collection.
pub fn process_initialize_collection(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: InitializeGroup,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let collection_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;

    check_mint_and_metadata(mint_info, mint_authority_info)?;

    // Initialize the collection
    let mut buffer = collection_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    let (collection, _) = state.init_value::<TokenGroup>(false)?;
    *collection = TokenGroup::new(mint_info.key, data.update_authority, data.max_size.into());

    Ok(())
}

/// Processes an [InitializeMember](enum.GroupInterfaceInstruction.html)
/// instruction
pub fn process_initialize_collection_member(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let member_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;
    let collection_info = next_account_info(account_info_iter)?;
    let collection_update_authority_info = next_account_info(account_info_iter)?;

    check_mint_and_metadata(mint_info, mint_authority_info)?;

    if member_info.key == collection_info.key {
        return Err(TokenGroupError::MemberAccountIsGroupAccount.into());
    }

    let mut buffer = collection_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    let collection = state.get_first_value_mut::<TokenGroup>()?;

    check_update_authority(
        collection_update_authority_info,
        &collection.update_authority,
    )?;
    let member_number = collection.increment_size()?;

    let mut buffer = member_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;

    // This program uses `allow_repetition: true` because the same mint can be
    // a member of multiple collections.
    let (member, _) = state.init_value::<TokenGroupMember>(/* allow_repetition */ true)?;
    *member = TokenGroupMember::new(mint_info.key, collection_info.key, member_number);

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
            // Same functionality as the example program
            spl_token_group_example::processor::process_update_group_max_size(
                program_id, accounts, data,
            )
        }
        TokenGroupInstruction::UpdateGroupAuthority(data) => {
            msg!("Instruction: UpdateCollectionAuthority");
            // Same functionality as the example program
            spl_token_group_example::processor::process_update_group_authority(
                program_id, accounts, data,
            )
        }
        TokenGroupInstruction::InitializeMember(_) => {
            msg!("Instruction: InitializeCollectionMember");
            process_initialize_collection_member(program_id, accounts)
        }
    }
}
