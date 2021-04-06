use crate::state::{METADATA_KEY, NAME_SYMBOL_TUPLE_KEY};

use {
    crate::{
        error::MetadataError,
        instruction::MetadataInstruction,
        state::{
            Metadata, NameSymbolTuple, MAX_METADATA_LEN, MAX_NAME_LENGTH, MAX_OWNER_LEN,
            MAX_SYMBOL_LENGTH, MAX_URI_LENGTH, PREFIX,
        },
        utils::{assert_initialized, create_or_allocate_account_raw},
    },
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh::try_from_slice_unchecked,
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
    spl_token::state::Mint,
};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = MetadataInstruction::try_from_slice(input)?;
    match instruction {
        MetadataInstruction::CreateMetadataAccounts(args) => {
            msg!("Instruction: Create Metadata Accounts");
            process_create_metadata_accounts(
                program_id,
                accounts,
                args.data.name,
                args.data.symbol,
                args.data.uri,
                args.allow_duplication,
            )
        }
        MetadataInstruction::UpdateMetadataAccounts(args) => {
            msg!("Instruction: Update Metadata Accounts");
            process_update_metadata_accounts(
                program_id,
                accounts,
                args.uri,
                args.non_unique_specific_update_authority,
            )
        }
        MetadataInstruction::TransferUpdateAuthority => {
            msg!("Instruction: Transfer Update Authority");
            process_transfer_update_authority(program_id, accounts)
        }
    }
}

/// Create a new account instruction
pub fn process_create_metadata_accounts(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: String,
    symbol: String,
    uri: String,
    allow_duplication: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let name_symbol_account_info = next_account_info(account_info_iter)?;
    let metadata_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;
    let payer_account_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    if name.len() > MAX_NAME_LENGTH {
        return Err(MetadataError::NameTooLong.into());
    }

    if symbol.len() > MAX_SYMBOL_LENGTH {
        return Err(MetadataError::SymbolTooLong.into());
    }

    if uri.len() > MAX_URI_LENGTH {
        return Err(MetadataError::UriTooLong.into());
    }

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

    if metadata_account_info.key != &metadata_key {
        return Err(MetadataError::InvalidMetadataKey.into());
    }

    create_or_allocate_account_raw(
        *program_id,
        metadata_account_info,
        rent_info,
        system_account_info,
        payer_account_info,
        MAX_METADATA_LEN,
        metadata_authority_signer_seeds,
    )?;

    let mut metadata: Metadata = try_from_slice_unchecked(&metadata_account_info.data.borrow())?;
    metadata.mint = *mint_info.key;
    metadata.key = METADATA_KEY;
    metadata.data.name = name.to_owned();
    metadata.data.symbol = symbol.to_owned();
    metadata.data.uri = uri;
    metadata.non_unique_specific_update_authority = Some(*update_authority_info.key);

    if !allow_duplication {
        let name_symbol_seeds = &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            &name.as_bytes(),
            &symbol.as_bytes(),
        ];
        let (name_symbol_key, name_symbol_bump_seed) =
            Pubkey::find_program_address(name_symbol_seeds, program_id);
        let name_symbol_authority_signer_seeds = &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            &name.as_bytes(),
            &symbol.as_bytes(),
            &[name_symbol_bump_seed],
        ];

        if name_symbol_account_info.key != &name_symbol_key {
            return Err(MetadataError::InvalidNameSymbolKey.into());
        }

        // If this is a brand new NameSymbol, we can simply allocate and be on our way.
        // If it is an existing NameSymbol, we need to check that you are that authority and are the signer.
        if !name_symbol_account_info.try_data_is_empty()? {
            let name_symbol: NameSymbolTuple =
                try_from_slice_unchecked(&name_symbol_account_info.data.borrow_mut())?;
            if name_symbol.update_authority != *update_authority_info.key
                || !update_authority_info.is_signer
            {
                return Err(
                    MetadataError::UpdateAuthorityMustBeEqualToNameSymbolAuthorityAndSigner.into(),
                );
            }
        } else {
            create_or_allocate_account_raw(
                *program_id,
                name_symbol_account_info,
                rent_info,
                system_account_info,
                payer_account_info,
                MAX_OWNER_LEN,
                name_symbol_authority_signer_seeds,
            )?;
        }
        let mut name_symbol: NameSymbolTuple =
            try_from_slice_unchecked(&name_symbol_account_info.data.borrow())?;

        // Now this is 0'ed out, so it can be filtered on as a boolean filter for NFTs and other
        // Unique types
        metadata.non_unique_specific_update_authority = None;

        name_symbol.update_authority = *update_authority_info.key;
        name_symbol.key = NAME_SYMBOL_TUPLE_KEY;
        name_symbol.metadata = *metadata_account_info.key;
        name_symbol.serialize(&mut *name_symbol_account_info.data.borrow_mut())?;
    };

    metadata.serialize(&mut *metadata_account_info.data.borrow_mut())?;

    Ok(())
}

/// Update existing account instruction
pub fn process_update_metadata_accounts(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    uri: String,
    non_unique_specific_update_authority: Option<Pubkey>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let metadata_account_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;
    let name_symbol_account_info = next_account_info(account_info_iter)?;

    if uri.len() > MAX_URI_LENGTH {
        return Err(MetadataError::UriTooLong.into());
    }
    let mut metadata: Metadata = try_from_slice_unchecked(&metadata_account_info.data.borrow())?;

    // Even if you're a metadata that doesn't use this, you need to send it up with proper key.
    let name_symbol_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &metadata.data.name.as_bytes(),
        &metadata.data.symbol.as_bytes(),
    ];
    let (name_symbol_key, _) = Pubkey::find_program_address(name_symbol_seeds, program_id);

    if name_symbol_key != *name_symbol_account_info.key {
        return Err(MetadataError::InvalidNameSymbolKey.into());
    }

    match metadata.non_unique_specific_update_authority {
        Some(val) => {
            if val != *update_authority_info.key {
                return Err(MetadataError::UpdateAuthorityIncorrect.into());
            }
        }
        None => {
            let name_symbol: NameSymbolTuple =
                try_from_slice_unchecked(&name_symbol_account_info.data.borrow())?;

            if name_symbol.metadata != *metadata_account_info.key {
                return Err(MetadataError::InvalidMetadataForNameSymbolTuple.into());
            }

            if name_symbol.update_authority != *update_authority_info.key {
                return Err(MetadataError::UpdateAuthorityIncorrect.into());
            }
        }
    }

    if !update_authority_info.is_signer {
        return Err(MetadataError::UpdateAuthorityIsNotSigner.into());
    }

    metadata.data.uri = uri;

    // Only set it if it's specifically a duplicable metadata (not an NFT kind) which can be
    // determined by the presence of this field already.
    if metadata.non_unique_specific_update_authority.is_some() {
        metadata.non_unique_specific_update_authority = non_unique_specific_update_authority
    }

    metadata.serialize(&mut *metadata_account_info.data.borrow_mut())?;
    Ok(())
}

/// Transfer update authority
pub fn process_transfer_update_authority(_: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let name_symbol_account_info = next_account_info(account_info_iter)?;
    let current_update_authority_info = next_account_info(account_info_iter)?;
    let new_update_authority_info = next_account_info(account_info_iter)?;

    let mut name_symbol: NameSymbolTuple =
        try_from_slice_unchecked(&name_symbol_account_info.data.borrow())?;

    if name_symbol.update_authority != *current_update_authority_info.key
        || !current_update_authority_info.is_signer
    {
        return Err(MetadataError::UpdateAuthorityMustBeEqualToNameSymbolAuthorityAndSigner.into());
    }

    name_symbol.update_authority = *new_update_authority_info.key;

    name_symbol.serialize(&mut *name_symbol_account_info.data.borrow_mut())?;

    Ok(())
}
