use crate::{
    error::TimelockError,
    state::{
        enums::TimelockStateStatus, timelock_program::TimelockProgram, timelock_set::TimelockSet,
    },
};
use arrayref::array_ref;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::Instruction,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    system_instruction::create_account,
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
    program_id: &Pubkey,
    perm_account_info: &AccountInfo<'a>,
    perm_validation_account_info: &AccountInfo<'a>,
    timelock_program_info: &AccountInfo<'a>,
    token_program_info: &AccountInfo<'a>,
    transfer_authority_info: &AccountInfo<'a>,
    timelock_authority_info: &AccountInfo<'a>,
) -> ProgramResult {
    msg!(
        "Args {:?} {:?} {:?} {:?} {:?} {:?} {:?}",
        program_id,
        perm_account_info.key,
        perm_validation_account_info.key,
        timelock_program_info.key,
        token_program_info.key,
        transfer_authority_info.key,
        timelock_authority_info.key
    );
    let _perm_account: Account = assert_initialized(perm_account_info)?;
    let _perm_validation: Account = assert_initialized(perm_validation_account_info)?;
    let (authority_key, bump_seed) =
        Pubkey::find_program_address(&[timelock_program_info.key.as_ref()], program_id);
    if timelock_authority_info.key != &authority_key {
        return Err(TimelockError::InvalidTimelockAuthority.into());
    }
    let authority_signer_seeds = &[timelock_program_info.key.as_ref(), &[bump_seed]];
    // If both accounts arent correct mint type, it explodes
    // If token amount is <1, it explodes. Perfect check.
    // If authority isnt right, it explodes.
    spl_token_transfer(TokenTransferParams {
        source: perm_account_info.clone(),
        destination: perm_validation_account_info.clone(),
        amount: 1,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;
    // Now give it back
    spl_token_transfer(TokenTransferParams {
        source: perm_validation_account_info.clone(),
        destination: perm_account_info.clone(),
        amount: 1,
        authority: timelock_authority_info.clone(),
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

/// Asserts a timelock set is in executing state.
pub fn assert_executing(timelock_set: &TimelockSet) -> ProgramResult {
    if timelock_set.state.status != TimelockStateStatus::Executing {
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

/// asserts timelock txn is in timelock set
pub fn assert_txn_in_set(
    timelock_set: &TimelockSet,
    timelock_txn_account_info: &AccountInfo,
) -> ProgramResult {
    let mut found: bool = false;
    for n in 0..timelock_set.state.timelock_transactions.len() {
        if timelock_set.state.timelock_transactions[n].to_bytes()
            == timelock_txn_account_info.key.to_bytes()
        {
            found = true;
            break;
        }
    }

    if !found {
        return Err(TimelockError::TimelockTransactionNotFoundError.into());
    }

    Ok(())
}

/// asserts that two accounts are equivalent
pub fn assert_account_equiv(acct: &AccountInfo, key: &Pubkey) -> ProgramResult {
    if acct.key != key {
        return Err(TimelockError::AccountsShouldMatch.into());
    }

    Ok(())
}

/// Assert the account has a matching mint
pub fn assert_mint_matching(acct: &Account, mint_info: &AccountInfo) -> ProgramResult {
    if acct.mint != *mint_info.key {
        return Err(TimelockError::MintsShouldMatch.into());
    }

    Ok(())
}

/// assert rent exempt
pub fn assert_rent_exempt(rent: &Rent, account_info: &AccountInfo) -> ProgramResult {
    if !rent.is_exempt(account_info.lamports(), account_info.data_len()) {
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

/// cheap assertion of mint serialization
#[inline(always)]
pub fn assert_cheap_mint_initialized(account_info: &AccountInfo) -> Result<(), ProgramError> {
    // In token program, 36, 8, 1, 1 is the layout, where the last 1 is initialized bit.
    // Not my favorite hack, but necessary to avoid stack size limitations caused by serializing entire Mint
    // to get at initialization check
    let index: usize = 36 + 8 + 1 + 1 - 1;
    if account_info.try_borrow_data().unwrap()[index] == 0 {
        return Err(TimelockError::Uninitialized.into());
    }
    Ok(())
}

/// cheap method to just pull supply off a mint
#[inline(always)]
pub fn pull_mint_supply(account_info: &AccountInfo) -> Result<u64, ProgramError> {
    // In token program, 36, 8, 1, 1 is the layout, where the first 8 is supply u64.
    // so we start at 36.
    let data = account_info.try_borrow_data().unwrap();
    let bytes = array_ref![data, 36, 8];

    Ok(u64::from_le_bytes(*bytes))
}

/// Cheap method to just grab mint Pubkey off token account, instead of deserializing entire thing
#[inline(always)]
pub fn get_mint_from_account(account_info: &AccountInfo) -> Result<Pubkey, ProgramError> {
    // Accounts have mint in first 32 bits.
    let data = account_info.try_borrow_data().unwrap();
    let key_data = array_ref![data, 0, 32];
    Ok(Pubkey::new_from_array(*key_data))
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

/// Issue a spl_token `Burn` instruction.
#[inline(always)]
pub fn execute(params: ExecuteParams<'_, '_>) -> ProgramResult {
    let ExecuteParams {
        instruction,
        authority_signer_seeds,
        account_infos,
    } = params;

    let result = invoke_signed(
        &instruction,
        &account_infos.as_slice(),
        &[authority_signer_seeds],
    );
    result.map_err(|_| TimelockError::ExecutionFailed.into())
}

/// Create account from scratch, stolen from Wormhole, slightly altered for my purposes
/// https://github.com/bartosz-lipinski/wormhole/blob/8478735ea7525043635524a62db2751e59d2bc38/solana/bridge/src/processor.rs#L1335
#[inline(always)]
pub fn create_account_raw<T: Pack>(
    accounts: &[AccountInfo],
    new_account: &Pubkey,
    payer: &Pubkey,
    owner: &Pubkey,
    seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    let size = T::LEN;
    let ix = create_account(
        payer,
        new_account,
        Rent::default().minimum_balance(size as usize),
        size as u64,
        owner,
    );
    invoke_signed(&ix, accounts, &[seeds])
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

/// ExecuteParams
pub struct ExecuteParams<'a: 'b, 'b> {
    /// Instruction
    pub instruction: Instruction,
    /// authority_signer_seeds
    pub authority_signer_seeds: &'b [&'b [u8]],
    /// Account infos
    pub account_infos: Vec<AccountInfo<'a>>,
}
