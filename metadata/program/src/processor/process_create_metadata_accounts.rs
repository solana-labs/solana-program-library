//! Program state processor

use crate::{
    error::MetadataError,
    state::{
        metadata::{Metadata, NAME_LENGTH, SYMBOL_LENGTH},
        owner::Owner,
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
pub fn process_create_metadata_accounts(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: [u8; NAME_LENGTH],
    symbol: [u8; SYMBOL_LENGTH],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let owner_info = next_account_info(account_info_iter)?;
    let metadata_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;
    let payer_account_info = next_account_info(account_info_iter)?;
    let program_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;

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

    let metadata_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        mint_info.key.as_ref(),
    ];
    let (metadata_key, metadata_bump_seed) =
        Pubkey::find_program_address(metadata_seeds, program_id);
    let metadata_authority_signer_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        mint_info.key.as_ref(),
        &[metadata_bump_seed],
    ];

    let owner_seeds = &[PREFIX.as_bytes(), program_id.as_ref(), &name, &symbol];
    let (owner_key, owner_bump_seed) = Pubkey::find_program_address(owner_seeds, program_id);
    let owner_authority_signer_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &name,
        &symbol,
        &[owner_bump_seed],
    ];

    msg!("Owner key comps {:?} {:?}", owner_key, owner_info.key);

    msg!(
        "Metadata key comps {:?} {:?}",
        metadata_key,
        metadata_info.key
    );

    create_account_raw::<Metadata>(
        &[
            payer_account_info.clone(),
            metadata_info.clone(),
            program_info.clone(),
            system_account_info.clone(),
        ],
        &metadata_key,
        payer_account_info.key,
        program_id,
        metadata_authority_signer_seeds,
    )?;
    create_account_raw::<Owner>(
        &[
            payer_account_info.clone(),
            owner_info.clone(),
            program_info.clone(),
            system_account_info.clone(),
        ],
        &owner_key,
        payer_account_info.key,
        program_id,
        owner_authority_signer_seeds,
    )?;

    Ok(())
}
