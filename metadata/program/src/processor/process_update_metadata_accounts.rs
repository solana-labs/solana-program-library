//! Program state processor

use crate::{
    error::MetadataError,
    state::{
        metadata::{Metadata, URI_LENGTH},
        owner::Owner,
    },
    utils::assert_initialized,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
};

/// Update existing accounts
pub fn process_update_metadata_accounts(
    _: &Pubkey,
    accounts: &[AccountInfo],
    uri: [u8; URI_LENGTH],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let metadata_account_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    let owner_account_info = next_account_info(account_info_iter)?;

    let owner: Owner = assert_initialized(owner_account_info)?;
    let mut metadata: Metadata = assert_initialized(metadata_account_info)?;

    if owner.metadata != *metadata_account_info.key {
        return Err(MetadataError::InvalidMetadataForOwner.into());
    }

    if owner.owner != *owner_info.key {
        return Err(MetadataError::OwnerNotOwner.into());
    }

    if !owner_info.is_signer {
        return Err(MetadataError::OwnerIsNotSigner.into());
    }

    metadata.uri = uri;

    Metadata::pack(metadata, &mut metadata_account_info.data.borrow_mut())?;
    Ok(())
}
