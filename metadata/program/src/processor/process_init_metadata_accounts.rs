//! Program state processor

use crate::{
    error::MetadataError,
    state::{
        metadata::{Metadata, METADATA_VERSION, NAME_LENGTH, SYMBOL_LENGTH, URI_LENGTH},
        owner::{Owner, OWNER_VERSION},
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
pub fn process_init_metadata_accounts(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: [u8; NAME_LENGTH],
    symbol: [u8; SYMBOL_LENGTH],
    uri: [u8; URI_LENGTH],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let owner_account_info = next_account_info(account_info_iter)?;
    let metadata_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_info)?;

    let mut owner: Owner = assert_uninitialized(owner_account_info)?;
    let mut metadata: Metadata = assert_uninitialized(metadata_account_info)?;
    let mint: Mint = assert_initialized(mint_info)?;
    match mint.mint_authority {
        solana_program::program_option::COption::None => {
            return Err(MetadataError::InvalidMintAuthority.into());
        }
        solana_program::program_option::COption::Some(key) => {
            if *mint_authority_info.key != key {
                return Err(MetadataError::InvalidMintAuthority.into());
            }
        }
    }

    if !mint_authority_info.is_signer {
        return Err(MetadataError::NotMintAuthority.into());
    }

    assert_rent_exempt(rent, owner_account_info)?;
    assert_rent_exempt(rent, metadata_account_info)?;

    let (metadata_address, _) = Pubkey::find_program_address(
        &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            mint_info.key.as_ref(),
        ],
        program_id,
    );
    if metadata_account_info.key != &metadata_address {
        return Err(MetadataError::InvalidMetadataKey.into());
    }

    let (owner_address, _) = Pubkey::find_program_address(
        &[PREFIX.as_bytes(), program_id.as_ref(), &name, &symbol],
        program_id,
    );
    if owner_account_info.key != &owner_address {
        return Err(MetadataError::InvalidOwnerKey.into());
    }

    owner.version = OWNER_VERSION;
    owner.owner = *owner_info.key;
    owner.metadata = *metadata_account_info.key;

    metadata.version = METADATA_VERSION;
    metadata.mint = *mint_info.key;
    metadata.name = name;
    metadata.symbol = symbol;
    metadata.uri = uri;

    Owner::pack(owner, &mut owner_account_info.data.borrow_mut())?;
    Metadata::pack(metadata, &mut metadata_account_info.data.borrow_mut())?;
    Ok(())
}
