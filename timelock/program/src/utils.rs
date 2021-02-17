use crate::{
    error::TimelockError,
    state::{
        enums::TimelockStateStatus,
        timelock_program::{TimelockProgram, TIMELOCK_VERSION},
        timelock_set::{TimelockSet, TIMELOCK_SET_VERSION},
    },
};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::rent::Rent,
};
use spl_token::state::Account;

/* TODO come back to this conundrum later..
pub fn get_authority_signer_seeds<'a>(
    timelock_program_account_info: &'a AccountInfo<'a>,
    timelock_program_authority_info: &'a AccountInfo<'a>,
    program_id: &'a Pubkey,
) -> Result<&'a [&'a [u8]; 2], ProgramError> {
    let (authority_key, bump_seed) =
        Pubkey::find_program_address(&[timelock_program_account_info.key.as_ref()], program_id);
    if timelock_program_authority_info.key != &authority_key {
        return Err(TimelockError::InvalidTimelockAuthority.into());
    }
    let authority_signer_seeds = &[timelock_program_account_info.key.as_ref(), &[bump_seed]];
    Ok(&*authority_signer_seeds)
}*/

/// Attempts to transfer the token to the timelock set's validation account and back to the person again.
/// Can only be done if done in a transaction that has authority to do so. Serves as a check
/// That the person is who they say they are!
pub fn assert_is_permissioned<'a>(
    perm_account_info: &AccountInfo<'a>,
    perm_validation_account_info: &AccountInfo<'a>,
    timelock_program_info: &AccountInfo<'a>,
    token_program_info: &AccountInfo<'a>,
) -> ProgramResult {
    let _perm_account: Account = assert_initialized(perm_account_info)?;
    let _perm_validation: Account = assert_initialized(perm_validation_account_info)?;

    let (_, bump_seed) =
        Pubkey::find_program_address(&[perm_account_info.key.as_ref()], timelock_program_info.key);

    let authority_signer_seeds = &[timelock_program_info.key.as_ref(), &[bump_seed]];

    // If both accounts arent correct mint type, it explodes
    // If token amount is <1, it explodes. Perfect check.
    // If authority isnt right, it explodes.
    spl_token_transfer(TokenTransferParams {
        source: perm_account_info.clone(),
        destination: perm_validation_account_info.clone(),
        amount: 1,
        authority: timelock_program_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    // Now give it back

    spl_token_transfer(TokenTransferParams {
        source: perm_validation_account_info.clone(),
        destination: perm_account_info.clone(),
        amount: 1,
        authority: timelock_program_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    Ok(())
}

/// Asserts a timelock set is in a state that can be edited - if its voting or executing, cant touch it.
pub fn assert_not_in_voting_or_executing(timelock_set: &TimelockSet) -> ProgramResult {
    if timelock_set.state.status == TimelockStateStatus::Voting
        || timelock_set.state.status == TimelockStateStatus::Executing
    {
        return Err(TimelockError::InvalidTimelockSetStateError.into());
    }
    Ok(())
}

/// Asserts a timelock set is in voting state.
pub fn assert_voting(timelock_set: &TimelockSet) -> ProgramResult {
    if timelock_set.state.status != TimelockStateStatus::Voting {
        return Err(TimelockError::InvalidTimelockSetStateError.into());
    }
    Ok(())
}

/// Asserts a timelock set is in draft state.
pub fn assert_draft(timelock_set: &TimelockSet) -> ProgramResult {
    if timelock_set.state.status != TimelockStateStatus::Draft {
        return Err(TimelockError::InvalidTimelockSetStateError.into());
    }
    Ok(())
}

/// Asserts the proper mint key is being used.
pub fn assert_proper_signatory_mint(
    timelock_set: &TimelockSet,
    signatory_mint_account_info: &AccountInfo,
) -> ProgramResult {
    if timelock_set.signatory_mint != *signatory_mint_account_info.key {
        return Err(TimelockError::InvalidSignatoryMintError.into());
    }
    Ok(())
}

/// Asserts token_program is correct program
pub fn assert_token_program_is_correct(
    timelock_program: &TimelockProgram,
    token_program_info: &AccountInfo,
) -> ProgramResult {
    if &timelock_program.token_program_id != token_program_info.key {
        return Err(TimelockError::InvalidTokenProgram.into());
    };
    Ok(())
}

/// Asserts the timelock program and timelock set are running the same version constants as this code
/// Otherwise throws an error telling user to find different version on the block chain for these accounts that is compatible
pub fn assert_same_version_as_program(
    timelock_program: &TimelockProgram,
    timelock_set: &TimelockSet,
) -> ProgramResult {
    if timelock_program.version != TIMELOCK_VERSION {
        return Err(TimelockError::InvalidTimelockVersionError.into());
    }
    if timelock_set.version != TIMELOCK_SET_VERSION {
        return Err(TimelockError::InvalidTimelockSetVersionError.into());
    }

    Ok(())
}
/// assert rent exempt
pub fn assert_rent_exempt(rent: &Rent, account_info: &AccountInfo) -> ProgramResult {
    if !rent.is_exempt(account_info.lamports(), account_info.data_len()) {
        msg!(&rent.minimum_balance(account_info.data_len()).to_string());
        Err(TimelockError::NotRentExempt.into())
    } else {
        Ok(())
    }
}
/// assert ununitialized account
pub fn assert_uninitialized<T: Pack + IsInitialized>(
    account_info: &AccountInfo,
) -> Result<T, ProgramError> {
    let account: T = T::unpack_unchecked(&account_info.data.borrow())?;
    if account.is_initialized() {
        Err(TimelockError::AlreadyInitialized.into())
    } else {
        Ok(account)
    }
}

/// assert initialized account
pub fn assert_initialized<T: Pack + IsInitialized>(
    account_info: &AccountInfo,
) -> Result<T, ProgramError> {
    let account: T = T::unpack_unchecked(&account_info.data.borrow())?;
    if !account.is_initialized() {
        Err(TimelockError::Uninitialized.into())
    } else {
        Ok(account)
    }
}

/// Unpacks a spl_token `Mint`.
pub fn unpack_mint(data: &[u8]) -> Result<spl_token::state::Mint, TimelockError> {
    spl_token::state::Mint::unpack(data).map_err(|_| TimelockError::InvalidTokenMint)
}

/// Issue a spl_token `InitializeMint` instruction.
#[inline(always)]
pub fn spl_token_init_mint(params: TokenInitializeMintParams<'_, '_>) -> ProgramResult {
    let TokenInitializeMintParams {
        mint,
        rent,
        authority,
        token_program,
        decimals,
    } = params;
    let ix = spl_token::instruction::initialize_mint(
        token_program.key,
        mint.key,
        authority,
        None,
        decimals,
    )?;
    let result = invoke(&ix, &[mint, rent, token_program]);
    result.map_err(|_| TimelockError::TokenInitializeMintFailed.into())
}

/// Issue a spl_token `InitializeAccount` instruction.
#[inline(always)]
pub fn spl_token_init_account(params: TokenInitializeAccountParams<'_>) -> ProgramResult {
    let TokenInitializeAccountParams {
        account,
        mint,
        owner,
        rent,
        token_program,
    } = params;
    let ix = spl_token::instruction::initialize_account(
        token_program.key,
        account.key,
        mint.key,
        owner.key,
    )?;
    let result = invoke(&ix, &[account, mint, owner, rent, token_program]);
    result.map_err(|_| TimelockError::TokenInitializeAccountFailed.into())
}

/// Issue a spl_token `Transfer` instruction.
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
    let result = invoke_signed(
        &spl_token::instruction::transfer(
            token_program.key,
            source.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?,
        &[source, destination, authority, token_program],
        &[authority_signer_seeds],
    );
    result.map_err(|_| TimelockError::TokenTransferFailed.into())
}

/// Issue a spl_token `MintTo` instruction.
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
    result.map_err(|_| TimelockError::TokenMintToFailed.into())
}

/// Issue a spl_token `Burn` instruction.
#[inline(always)]
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
    result.map_err(|_| TimelockError::TokenBurnFailed.into())
}

/// TokenInitializeMintParams
pub struct TokenInitializeMintParams<'a: 'b, 'b> {
    /// mint
    pub mint: AccountInfo<'a>,
    /// rent
    pub rent: AccountInfo<'a>,
    /// authority
    pub authority: &'b Pubkey,
    /// decimals
    pub decimals: u8,
    /// token_program
    pub token_program: AccountInfo<'a>,
}

/// TokenInitializeAccountParams
pub struct TokenInitializeAccountParams<'a> {
    /// account
    pub account: AccountInfo<'a>,
    /// mint
    pub mint: AccountInfo<'a>,
    /// owner
    pub owner: AccountInfo<'a>,
    /// rent
    pub rent: AccountInfo<'a>,
    /// token_program
    pub token_program: AccountInfo<'a>,
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
    pub authority_signer_seeds: &'b [&'b [u8]],
    /// token_program
    pub token_program: AccountInfo<'a>,
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
