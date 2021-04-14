use crate::utils::TokenMintToParams;

use {
    crate::{
        error::MetadataError,
        instruction::MetadataInstruction,
        state::{
            Edition, Key, Metadata, NameSymbolTuple, EDITION, MAX_EDITION_LEN, MAX_METADATA_LEN,
            MAX_NAME_LENGTH, MAX_NAME_SYMBOL_LEN, MAX_SYMBOL_LENGTH, MAX_URI_LENGTH, PREFIX,
        },
        utils::{
            assert_edition_valid, assert_initialized, assert_mint_authority_matches_mint,
            assert_rent_exempt, assert_update_authority_is_correct, create_or_allocate_account_raw,
            spl_token_mint_to, transfer_mint_authority,
        },
    },
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh::try_from_slice_unchecked,
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
        rent::Rent,
        sysvar::Sysvar,
    },
    spl_token::state::{Account, Mint},
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
        MetadataInstruction::CreateMasterRecord(args) => {
            msg!("Instruction: Create Master Record");
            process_create_master_record(program_id, accounts, args.max_editions)
        }
        MetadataInstruction::MintNewEditionFromMasterRecord => {
            msg!("Instruction: Mint New Edition from Master Record");
            process_mint_new_edition_from_master_record(program_id, accounts)
        }
        MetadataInstruction::MintTokenForEdition => {
            msg!("Instruction: Mint Token for Edition");
            process_mint_token_for_edition(program_id, accounts)
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
    assert_mint_authority_matches_mint(&mint, mint_authority_info)?;

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
    metadata.key = Key::MetadataV1;
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
                MAX_NAME_SYMBOL_LEN,
                name_symbol_authority_signer_seeds,
            )?;
        }
        let mut name_symbol: NameSymbolTuple =
            try_from_slice_unchecked(&name_symbol_account_info.data.borrow())?;

        // Now this is 0'ed out, so it can be filtered on as a boolean filter for NFTs and other
        // Unique types
        metadata.non_unique_specific_update_authority = None;

        name_symbol.update_authority = *update_authority_info.key;
        name_symbol.key = Key::NameSymbolTupleV1;
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

    assert_update_authority_is_correct(
        &metadata,
        metadata_account_info,
        Some(name_symbol_account_info),
        update_authority_info,
    )?;

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

    let account_info = next_account_info(account_info_iter)?;
    let current_update_authority_info = next_account_info(account_info_iter)?;
    let new_update_authority_info = next_account_info(account_info_iter)?;

    if account_info.data_len() == MAX_METADATA_LEN {
        let mut metadata: Metadata = try_from_slice_unchecked(&account_info.data.borrow())?;
        if metadata.non_unique_specific_update_authority != Some(*current_update_authority_info.key)
            || !current_update_authority_info.is_signer
            || metadata.non_unique_specific_update_authority == None
        {
            return Err(
                MetadataError::UpdateAuthorityMustBeEqualToMetadataAuthorityAndSigner.into(),
            );
        }

        metadata.non_unique_specific_update_authority = Some(*new_update_authority_info.key);

        metadata.serialize(&mut *account_info.data.borrow_mut())?;
    } else {
        let mut name_symbol: NameSymbolTuple =
            try_from_slice_unchecked(&account_info.data.borrow())?;

        if name_symbol.update_authority != *current_update_authority_info.key
            || !current_update_authority_info.is_signer
        {
            return Err(
                MetadataError::UpdateAuthorityMustBeEqualToNameSymbolAuthorityAndSigner.into(),
            );
        }

        name_symbol.update_authority = *new_update_authority_info.key;

        name_symbol.serialize(&mut *account_info.data.borrow_mut())?;
    }

    Ok(())
}

/// Create master record
pub fn process_create_master_record(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    max_editions: Option<u64>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let edition_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;
    let metadata_account_info = next_account_info(account_info_iter)?;
    let name_symbol_account_info = next_account_info(account_info_iter)?;
    let payer_account_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    let metadata: Metadata = try_from_slice_unchecked(&metadata_account_info.data.borrow())?;
    let mint: Mint = assert_initialized(mint_info)?;

    let edition_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &mint_info.key.as_ref(),
        EDITION.as_bytes(),
    ];
    let (edition_key, _) = Pubkey::find_program_address(edition_seeds, program_id);

    if edition_key != *edition_account_info.key {
        return Err(MetadataError::InvalidEditionKey.into());
    }

    assert_mint_authority_matches_mint(&mint, mint_authority_info)?;

    if metadata.mint != *mint_info.key {
        return Err(MetadataError::MintMismatch.into());
    }

    assert_update_authority_is_correct(
        &metadata,
        metadata_account_info,
        Some(name_symbol_account_info),
        update_authority_info,
    )?;

    if mint.supply != 1 {
        return Err(MetadataError::MasterRecordsMustHaveExactlyOneToken.into());
    }

    create_or_allocate_account_raw(
        *program_id,
        edition_account_info,
        rent_info,
        system_account_info,
        payer_account_info,
        MAX_EDITION_LEN,
        edition_seeds,
    )?;

    let mut edition: Edition = try_from_slice_unchecked(&edition_account_info.data.borrow())?;

    edition.key = Key::EditionV1;
    edition.master_record = None;
    edition.edition = 0;
    edition.edition_count = 0;
    edition.max_editions = max_editions;
    edition.serialize(&mut *edition_account_info.data.borrow_mut())?;

    transfer_mint_authority(
        edition_seeds,
        &edition_key,
        edition_account_info,
        mint_info,
        mint_authority_info,
        token_program_info,
    )?;
    Ok(())
}

pub fn process_mint_new_edition_from_master_record(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let new_metadata_account_info = next_account_info(account_info_iter)?;
    let new_edition_account_info = next_account_info(account_info_iter)?;
    let master_edition_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;
    let payer_account_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;
    let master_metadata_account_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    let master_metadata: Metadata =
        try_from_slice_unchecked(&master_metadata_account_info.data.borrow())?;
    let mut master_edition: Edition =
        try_from_slice_unchecked(&master_edition_account_info.data.borrow())?;
    let mint: Mint = assert_initialized(mint_info)?;

    assert_mint_authority_matches_mint(&mint, mint_authority_info)?;

    assert_update_authority_is_correct(
        &master_metadata,
        master_metadata_account_info,
        None,
        update_authority_info,
    )?;

    assert_edition_valid(
        program_id,
        &master_metadata.mint,
        master_edition_account_info,
    )?;

    let edition_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &mint_info.key.as_ref(),
        EDITION.as_bytes(),
    ];
    let (edition_key, _) = Pubkey::find_program_address(edition_seeds, program_id);
    if edition_key != *new_edition_account_info.key {
        return Err(MetadataError::InvalidEditionKey.into());
    }

    if let Some(max) = master_edition.max_editions {
        if master_edition.edition_count >= max {
            return Err(MetadataError::MaxEditionsMintedAlready.into());
        }
    }

    master_edition.edition_count += 1;
    master_edition.serialize(&mut *master_edition_account_info.data.borrow_mut())?;

    if mint.supply != 0 {
        return Err(MetadataError::NewlyMintedEditionsMustHaveZeroTokens.into());
    }

    // create the metadata the normal way...
    process_create_metadata_accounts(
        program_id,
        &[
            system_account_info.clone(),
            new_metadata_account_info.clone(),
            mint_info.clone(),
            mint_authority_info.clone(),
            payer_account_info.clone(),
            update_authority_info.clone(),
            system_account_info.clone(),
            rent_info.clone(),
        ],
        master_metadata.data.name,
        master_metadata.data.symbol,
        master_metadata.data.uri,
        false,
    )?;

    create_or_allocate_account_raw(
        *program_id,
        new_edition_account_info,
        rent_info,
        system_account_info,
        payer_account_info,
        MAX_EDITION_LEN,
        edition_seeds,
    )?;

    let mut new_edition: Edition =
        try_from_slice_unchecked(&new_edition_account_info.data.borrow())?;
    new_edition.key = Key::EditionV1;
    new_edition.master_record = Some(*master_edition_account_info.key);
    new_edition.edition = master_edition.edition_count;
    new_edition.serialize(&mut *new_edition_account_info.data.borrow_mut())?;

    // Now make sure this mint can never be used by anybody else.
    transfer_mint_authority(
        edition_seeds,
        &edition_key,
        new_edition_account_info,
        mint_info,
        mint_authority_info,
        token_program_account_info,
    )?;

    Ok(())
}

pub fn process_mint_token_for_edition(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;
    let destination_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;
    let edition_account_info = next_account_info(account_info_iter)?;
    let master_edition_account_info = next_account_info(account_info_iter)?;
    let master_metadata_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_info)?;

    let master_metadata: Metadata =
        try_from_slice_unchecked(&master_metadata_account_info.data.borrow())?;
    let mint: Mint = assert_initialized(mint_info)?;
    let edition: Edition = try_from_slice_unchecked(&edition_account_info.data.borrow())?;

    if mint.supply >= 1 {
        return Err(MetadataError::EditionAlreadyMinted.into());
    }

    assert_update_authority_is_correct(
        &master_metadata,
        master_metadata_account_info,
        None,
        update_authority_info,
    )?;

    assert_edition_valid(
        program_id,
        &master_metadata.mint,
        master_edition_account_info,
    )?;

    if let Some(record) = edition.master_record {
        if record != *master_edition_account_info.key {
            return Err(MetadataError::MasterRecordMismatch.into());
        }
    }

    // We look out for you here, make sure your account is safe and won't brick your token.
    assert_rent_exempt(rent, destination_info)?;

    let destination: Account = assert_initialized(destination_info)?;

    if destination.mint != *mint_info.key {
        return Err(MetadataError::DestinationMintMismatch.into());
    }

    let edition_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &mint_info.key.as_ref(),
        EDITION.as_bytes(),
    ];
    let (edition_key, _) = Pubkey::find_program_address(edition_seeds, program_id);
    if edition_key != *edition_account_info.key {
        return Err(MetadataError::InvalidEditionKey.into());
    }

    spl_token_mint_to(TokenMintToParams {
        mint: mint_info.clone(),
        destination: destination_info.clone(),
        authority: edition_account_info.clone(),
        token_program: token_program_account_info.clone(),
        amount: 1,
        authority_signer_seeds: edition_seeds,
    })?;

    Ok(())
}
