use {
    crate::errors::AuctionError,
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        program_pack::Pack,
        program_error::ProgramError,
        pubkey::Pubkey,
        system_instruction,
        sysvar::{rent::Rent, Sysvar},
    },
    std::convert::TryInto,
};

pub fn assert_owned_by(account: &AccountInfo, owner: &Pubkey) -> ProgramResult {
    if account.owner != owner {
        Err(AuctionError::IncorrectOwner.into())
    } else {
        Ok(())
    }
}

pub fn assert_rent_exempt(rent: &Rent, account_info: &AccountInfo) -> ProgramResult {
    if !rent.is_exempt(account_info.lamports(), account_info.data_len()) {
        Err(AuctionError::NotRentExempt.into())
    } else {
        Ok(())
    }
}

pub fn assert_derivation(
    program_id: &Pubkey,
    account: &AccountInfo,
    path: &[&[u8]],
) -> Result<u8, ProgramError> {
    let (key, bump) = Pubkey::find_program_address(&path, program_id);
    if key != *account.key {
        return Err(AuctionError::DerivedKeyInvalid.into());
    }
    Ok(bump)
}

#[inline(always)]
pub fn create_or_allocate_account_raw<'a>(
    program_id: Pubkey,
    new_account_info: &AccountInfo<'a>,
    rent_sysvar_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    payer_info: &AccountInfo<'a>,
    size: usize,
    signer_seeds: &[&[u8]],
) -> Result<(), ProgramError> {
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
    msg!("Completed assignation!");

    Ok(())
}

///TokenTransferParams
pub struct TokenTransferParams<'a: 'b, 'b> {
    /// source
    pub source: AccountInfo<'a>,
    /// destination
    pub destination: AccountInfo<'a>,
    /// amount
    pub amount: u64,
    /// authority
    pub authority: AccountInfo<'a>,
    /// authority_signer_seeds
    pub authority_signer_seeds: Option<&'b [&'b [u8]]>,
    /// token_program
    pub token_program: AccountInfo<'a>,
}

#[inline(always)]
pub fn spl_token_transfer(params: TokenTransferParams<'_, '_>) -> ProgramResult {
    let TokenTransferParams {
        source,
        destination,
        authority,
        token_program,
        amount,
        authority_signer_seeds,
    } = params;

    match authority_signer_seeds {
        Some(seeds) => {
            invoke_signed(
                &spl_token::instruction::transfer(
                    token_program.key,
                    source.key,
                    destination.key,
                    authority.key,
                    &[],
                    amount,
                )?,
                &[source, destination, authority, token_program],
                &[seeds],
            )
            .map_err(|_| AuctionError::TokenTransferFailed.into())
        }

        None => {
            invoke(
                &spl_token::instruction::transfer(
                    token_program.key,
                    source.key,
                    destination.key,
                    authority.key,
                    &[],
                    amount,
                )?,
                &[source, destination, authority, token_program],
            )
            .map_err(|_| AuctionError::TokenTransferFailed.into())
        }
    }
}

/// TokenMintToParams
pub struct TokenCreateAccount<'a> {
    /// payer
    pub payer: AccountInfo<'a>,
    /// mint
    pub mint: AccountInfo<'a>,
    /// account
    pub account: AccountInfo<'a>,
    /// authority
    pub authority: AccountInfo<'a>,
    /// authority seeds
    pub authority_seeds: &'a [&'a [u8]],
    /// token_program
    pub token_program: AccountInfo<'a>,
    /// rent information
    pub rent: AccountInfo<'a>,
}

/// Create a new SPL token account.
#[inline(always)]
pub fn spl_token_create_account(params: TokenCreateAccount<'_>) -> ProgramResult {
    let TokenCreateAccount {
        payer,
        mint,
        account,
        authority,
        authority_seeds,
        token_program,
        rent,
    } = params;
    let size = spl_token::state::Account::LEN;
    let rent = &Rent::from_account_info(&rent)?;
    let required_lamports = rent
        .minimum_balance(size)
        .max(1)
        .saturating_sub(payer.lamports());

    invoke(
        &system_instruction::create_account(
            payer.key,
            account.key,
            required_lamports,
            size as u64,
            &spl_token::id(),
        ),
        &[payer, account.clone(), token_program],
    )?;

    invoke_signed(
        &spl_token::instruction::initialize_account(
            &spl_token::id(),
            account.key,
            mint.key,
            authority.key,
        )?,
        &[],
        &[authority_seeds],
    )?;

    Ok(())
}
