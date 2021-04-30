use {
    crate::{
        error::MetadataError,
        processor::process_create_metadata_accounts,
        state::{
            Edition, Key, MasterEdition, Metadata, NameSymbolTuple, EDITION, MAX_EDITION_LEN,
            PREFIX,
        },
    },
    borsh::BorshSerialize,
    solana_program::{
        account_info::AccountInfo,
        borsh::try_from_slice_unchecked,
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        program_pack::{IsInitialized, Pack},
        pubkey::Pubkey,
        system_instruction,
        sysvar::{rent::Rent, Sysvar},
    },
    spl_token::{
        instruction::{set_authority, AuthorityType},
        state::Mint,
    },
    std::convert::TryInto,
};

/// assert initialized account
pub fn assert_initialized<T: Pack + IsInitialized>(
    account_info: &AccountInfo,
) -> Result<T, ProgramError> {
    let account: T = T::unpack_unchecked(&account_info.data.borrow())?;
    if !account.is_initialized() {
        Err(MetadataError::Uninitialized.into())
    } else {
        Ok(account)
    }
}

/// Create account almost from scratch, lifted from
/// https://github.com/solana-labs/solana-program-library/blob/7d4873c61721aca25464d42cc5ef651a7923ca79/associated-token-account/program/src/processor.rs#L51-L98
#[inline(always)]
pub fn create_or_allocate_account_raw<'a>(
    program_id: Pubkey,
    new_account_info: &AccountInfo<'a>,
    rent_sysvar_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    payer_info: &AccountInfo<'a>,
    size: usize,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    let rent = &Rent::from_account_info(rent_sysvar_info)?;
    let required_lamports = rent
        .minimum_balance(size)
        .max(1)
        .saturating_sub(new_account_info.lamports());

    if required_lamports > 0 {
        msg!("Transfer {} lamports to the new account", required_lamports);
        invoke(
            &system_instruction::transfer(&payer_info.key, new_account_info.key, required_lamports),
            &[
                payer_info.clone(),
                new_account_info.clone(),
                system_program_info.clone(),
            ],
        )?;
    }

    msg!("Allocate space for the account");
    invoke_signed(
        &system_instruction::allocate(new_account_info.key, size.try_into().unwrap()),
        &[new_account_info.clone(), system_program_info.clone()],
        &[&signer_seeds],
    )?;

    msg!("Assign the account to the owning program");
    invoke_signed(
        &system_instruction::assign(new_account_info.key, &program_id),
        &[new_account_info.clone(), system_program_info.clone()],
        &[&signer_seeds],
    )?;

    Ok(())
}

pub fn assert_update_authority_is_correct(
    metadata: &Metadata,
    metadata_account_info: &AccountInfo,
    ns_info: Option<&AccountInfo>,
    update_authority_info: &AccountInfo,
) -> ProgramResult {
    match metadata.non_unique_specific_update_authority {
        Some(val) => {
            if val != *update_authority_info.key {
                return Err(MetadataError::UpdateAuthorityIncorrect.into());
            }
        }
        None => {
            if let Some(name_symbol_account_info) = ns_info {
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
    }

    if !update_authority_info.is_signer {
        return Err(MetadataError::UpdateAuthorityIsNotSigner.into());
    }

    Ok(())
}

pub fn assert_mint_authority_matches_mint(
    mint: &Mint,
    mint_authority_info: &AccountInfo,
) -> ProgramResult {
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

    Ok(())
}

pub fn transfer_mint_authority<'a>(
    edition_seeds: &[&[u8]],
    edition_key: &Pubkey,
    edition_account_info: &AccountInfo<'a>,
    mint_info: &AccountInfo<'a>,
    mint_authority_info: &AccountInfo<'a>,
    token_program_info: &AccountInfo<'a>,
) -> ProgramResult {
    msg!("Setting mint authority");
    invoke_signed(
        &set_authority(
            token_program_info.key,
            mint_info.key,
            Some(edition_key),
            AuthorityType::MintTokens,
            mint_authority_info.key,
            &[&mint_authority_info.key],
        )
        .unwrap(),
        &[
            mint_authority_info.clone(),
            mint_info.clone(),
            token_program_info.clone(),
            edition_account_info.clone(),
        ],
        &[edition_seeds],
    )?;
    msg!("Setting freeze authority");
    invoke_signed(
        &set_authority(
            token_program_info.key,
            mint_info.key,
            Some(&edition_key),
            AuthorityType::FreezeAccount,
            mint_authority_info.key,
            &[&mint_authority_info.key],
        )
        .unwrap(),
        &[
            mint_authority_info.clone(),
            mint_info.clone(),
            token_program_info.clone(),
            edition_account_info.clone(),
        ],
        &[edition_seeds],
    )?;

    Ok(())
}

pub fn assert_rent_exempt(rent: &Rent, account_info: &AccountInfo) -> ProgramResult {
    if !rent.is_exempt(account_info.lamports(), account_info.data_len()) {
        Err(MetadataError::NotRentExempt.into())
    } else {
        Ok(())
    }
}

pub fn assert_edition_valid(
    program_id: &Pubkey,
    mint: &Pubkey,
    edition_account_info: &AccountInfo,
) -> ProgramResult {
    let edition_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &mint.as_ref(),
        EDITION.as_bytes(),
    ];
    let (edition_key, _) = Pubkey::find_program_address(edition_seeds, program_id);
    if edition_key != *edition_account_info.key {
        return Err(MetadataError::InvalidEditionKey.into());
    }

    Ok(())
}

pub fn mint_limited_edition<'a>(
    program_id: &Pubkey,
    new_metadata_account_info: &AccountInfo<'a>,
    new_edition_account_info: &AccountInfo<'a>,
    master_edition_account_info: &AccountInfo<'a>,
    mint_info: &AccountInfo<'a>,
    mint_authority_info: &AccountInfo<'a>,
    payer_account_info: &AccountInfo<'a>,
    update_authority_info: &AccountInfo<'a>,
    master_metadata_account_info: &AccountInfo<'a>,
    token_program_account_info: &AccountInfo<'a>,
    system_account_info: &AccountInfo<'a>,
    rent_info: &AccountInfo<'a>,
) -> ProgramResult {
    let master_metadata: Metadata =
        try_from_slice_unchecked(&master_metadata_account_info.data.borrow())?;
    let mut master_edition: MasterEdition =
        try_from_slice_unchecked(&master_edition_account_info.data.borrow())?;
    let mint: Mint = assert_initialized(mint_info)?;

    assert_mint_authority_matches_mint(&mint, mint_authority_info)?;

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
    let (edition_key, bump_seed) = Pubkey::find_program_address(edition_seeds, program_id);
    if edition_key != *new_edition_account_info.key {
        return Err(MetadataError::InvalidEditionKey.into());
    }

    if let Some(max) = master_edition.max_supply {
        if master_edition.supply >= max {
            return Err(MetadataError::MaxEditionsMintedAlready.into());
        }
    }

    master_edition.supply += 1;
    master_edition.serialize(&mut *master_edition_account_info.data.borrow_mut())?;

    if mint.supply != 1 {
        return Err(MetadataError::EditionsMustHaveExactlyOneToken.into());
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
        true,
    )?;

    let edition_authority_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &mint_info.key.as_ref(),
        EDITION.as_bytes(),
        &[bump_seed],
    ];

    create_or_allocate_account_raw(
        *program_id,
        new_edition_account_info,
        rent_info,
        system_account_info,
        payer_account_info,
        MAX_EDITION_LEN,
        edition_authority_seeds,
    )?;

    let mut new_edition: Edition =
        try_from_slice_unchecked(&new_edition_account_info.data.borrow())?;
    new_edition.key = Key::EditionV1;
    new_edition.parent = *master_edition_account_info.key;
    new_edition.edition = master_edition.supply;
    new_edition.serialize(&mut *new_edition_account_info.data.borrow_mut())?;

    // Now make sure this mint can never be used by anybody else.
    transfer_mint_authority(
        edition_authority_seeds,
        &edition_key,
        new_edition_account_info,
        mint_info,
        mint_authority_info,
        token_program_account_info,
    )?;

    Ok(())
}

pub fn spl_token_burn(params: TokenBurnParams<'_, '_>) -> ProgramResult {
    let TokenBurnParams {
        mint,
        source,
        authority,
        token_program,
        amount,
        authority_signer_seeds,
    } = params;
    let result = invoke_signed(
        &spl_token::instruction::burn(
            token_program.key,
            source.key,
            mint.key,
            authority.key,
            &[],
            amount,
        )?,
        &[source, mint, authority, token_program],
        &[authority_signer_seeds],
    );
    result.map_err(|_| MetadataError::TokenBurnFailed.into())
}

/// TokenBurnParams
pub struct TokenBurnParams<'a: 'b, 'b> {
    /// mint
    pub mint: AccountInfo<'a>,
    /// source
    pub source: AccountInfo<'a>,
    /// amount
    pub amount: u64,
    /// authority
    pub authority: AccountInfo<'a>,
    /// authority_signer_seeds
    pub authority_signer_seeds: &'b [&'b [u8]],
    /// token_program
    pub token_program: AccountInfo<'a>,
}

pub fn spl_token_mint_to(params: TokenMintToParams<'_, '_>) -> ProgramResult {
    let TokenMintToParams {
        mint,
        destination,
        authority,
        token_program,
        amount,
        authority_signer_seeds,
    } = params;
    let result = invoke_signed(
        &spl_token::instruction::mint_to(
            token_program.key,
            mint.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?,
        &[mint, destination, authority, token_program],
        &[authority_signer_seeds],
    );
    result.map_err(|_| MetadataError::TokenMintToFailed.into())
}

/// TokenMintToParams
pub struct TokenMintToParams<'a: 'b, 'b> {
    /// mint
    pub mint: AccountInfo<'a>,
    /// destination
    pub destination: AccountInfo<'a>,
    /// amount
    pub amount: u64,
    /// authority
    pub authority: AccountInfo<'a>,
    /// authority_signer_seeds
    pub authority_signer_seeds: &'b [&'b [u8]],
    /// token_program
    pub token_program: AccountInfo<'a>,
}
