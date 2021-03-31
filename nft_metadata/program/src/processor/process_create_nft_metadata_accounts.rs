//! Program state processor

use crate::{
    error::NFTMetadataError,
    state::{
        nft_metadata::{NFTMetadata, NAME_LENGTH, SYMBOL_LENGTH},
        nft_owner::NFTOwner,
        PREFIX,
    },
    utils::{assert_initialized, create_account_raw},
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};
use spl_token::state::Mint;

/// Create a new accounts
pub fn process_create_nft_metadata_accounts(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: [u8; NAME_LENGTH],
    symbol: [u8; SYMBOL_LENGTH],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let nft_owner_info = next_account_info(account_info_iter)?;
    let nft_metadata_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;
    let payer_account_info = next_account_info(account_info_iter)?;
    let program_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;

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

    let nft_metadata_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        mint_info.key.as_ref(),
    ];
    let (nft_metadata_key, nft_metadata_bump_seed) =
        Pubkey::find_program_address(nft_metadata_seeds, program_id);
    let nft_metadata_authority_signer_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        mint_info.key.as_ref(),
        &[nft_metadata_bump_seed],
    ];

    let nft_owner_seeds = &[PREFIX.as_bytes(), program_id.as_ref(), &name, &symbol];
    let (nft_owner_key, nft_owner_bump_seed) =
        Pubkey::find_program_address(nft_owner_seeds, program_id);
    let nft_owner_authority_signer_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &name,
        &symbol,
        &[nft_owner_bump_seed],
    ];

    msg!(
        "Owner key comps {:?} {:?}",
        nft_owner_key,
        nft_owner_info.key
    );

    msg!(
        "Metadata key comps {:?} {:?}",
        nft_metadata_key,
        nft_metadata_info.key
    );

    create_account_raw::<NFTMetadata>(
        &[
            payer_account_info.clone(),
            nft_metadata_info.clone(),
            program_info.clone(),
            system_account_info.clone(),
        ],
        &nft_metadata_key,
        payer_account_info.key,
        program_id,
        nft_metadata_authority_signer_seeds,
    )?;
    create_account_raw::<NFTOwner>(
        &[
            payer_account_info.clone(),
            nft_owner_info.clone(),
            program_info.clone(),
            system_account_info.clone(),
        ],
        &nft_owner_key,
        payer_account_info.key,
        program_id,
        nft_owner_authority_signer_seeds,
    )?;

    Ok(())
}
