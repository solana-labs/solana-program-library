//! Program state processor

use crate::{
    error::NFTMetadataError,
    state::{
        nft_metadata::{NFTMetadata, CATEGORY_LENGTH, CREATOR_LENGTH, URI_LENGTH},
        nft_owner::NFTOwner,
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
pub fn process_update_nft_metadata_accounts(
    _: &Pubkey,
    accounts: &[AccountInfo],
    uri: [u8; URI_LENGTH],
    category: [u8; CATEGORY_LENGTH],
    creator: [u8; CREATOR_LENGTH],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let nft_metadata_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    let nft_owner_info = next_account_info(account_info_iter)?;

    let nft_owner: NFTOwner = assert_initialized(nft_owner_info)?;
    let mut nft_metadata: NFTMetadata = assert_initialized(nft_metadata_info)?;

    if nft_owner.metadata != *nft_metadata_info.key {
        return Err(NFTMetadataError::InvalidMetadataForNFTOwner.into());
    }

    if nft_owner.owner != *owner_info.key {
        return Err(NFTMetadataError::NFTOwnerNotOwner.into());
    }

    if !owner_info.is_signer {
        return Err(NFTMetadataError::OwnerIsNotSigner.into());
    }

    nft_metadata.uri = uri;
    nft_metadata.category = category;
    nft_metadata.creator = creator;

    NFTMetadata::pack(nft_metadata, &mut nft_metadata_info.data.borrow_mut())?;
    Ok(())
}
