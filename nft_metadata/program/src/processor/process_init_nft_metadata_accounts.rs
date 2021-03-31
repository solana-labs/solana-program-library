//! Program state processor

use crate::{
    error::NFTMetadataError,
    state::{
        nft_metadata::{
            NFTMetadata, CATEGORY_LENGTH, CREATOR_LENGTH, NAME_LENGTH, NFT_METADATA_VERSION,
            SYMBOL_LENGTH, URI_LENGTH,
        },
        nft_owner::{NFTOwner, NFT_OWNER_VERSION},
        PREFIX,
    },
    utils::{assert_initialized, assert_rent_exempt, assert_uninitialized},
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};
use spl_token::state::Mint;

/// Create a new accounts
pub fn process_init_nft_metadata_accounts(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: [u8; NAME_LENGTH],
    symbol: [u8; SYMBOL_LENGTH],
    uri: [u8; URI_LENGTH],
    category: [u8; CATEGORY_LENGTH],
    creator: [u8; CREATOR_LENGTH],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let nft_owner_info = next_account_info(account_info_iter)?;
    let nft_metadata_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_info)?;

    let mut nft_owner: NFTOwner = assert_uninitialized(nft_owner_info)?;
    let mut nft_metadata: NFTMetadata = assert_uninitialized(nft_metadata_info)?;
    let mint: Mint = assert_initialized(mint_info)?;
    match mint.mint_authority {
        solana_program::program_option::COption::None => {
            return Err(NFTMetadataError::InvalidMintAuthority.into());
        }
        solana_program::program_option::COption::Some(key) => {
            if *mint_authority_info.key != key {
                return Err(NFTMetadataError::InvalidMintAuthority.into());
            }
        }
    }

    if !mint_authority_info.is_signer {
        return Err(NFTMetadataError::NotMintAuthority.into());
    }

    assert_rent_exempt(rent, nft_owner_info)?;
    assert_rent_exempt(rent, nft_metadata_info)?;

    let (nft_metadata_address, _) = Pubkey::find_program_address(
        &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            mint_info.key.as_ref(),
        ],
        program_id,
    );
    if nft_metadata_info.key != &nft_metadata_address {
        return Err(NFTMetadataError::InvalidNFTMetadataKey.into());
    }

    let (nft_owner_address, _) = Pubkey::find_program_address(
        &[PREFIX.as_bytes(), program_id.as_ref(), &name, &symbol],
        program_id,
    );
    if nft_owner_info.key != &nft_owner_address {
        return Err(NFTMetadataError::InvalidNFTOwnerKey.into());
    }

    nft_owner.version = NFT_OWNER_VERSION;
    nft_owner.owner = *owner_info.key;
    nft_owner.metadata = *nft_metadata_info.key;

    nft_metadata.version = NFT_METADATA_VERSION;
    nft_metadata.mint = *mint_info.key;
    nft_metadata.name = name;
    nft_metadata.symbol = symbol;
    nft_metadata.uri = uri;
    nft_metadata.category = category;
    nft_metadata.creator = creator;

    NFTOwner::pack(nft_owner, &mut nft_owner_info.data.borrow_mut())?;
    NFTMetadata::pack(nft_metadata, &mut nft_metadata_info.data.borrow_mut())?;
    Ok(())
}
