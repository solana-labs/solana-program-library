use crate::error::MetadataError;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    system_instruction::create_account,
    sysvar::rent::Rent,
};

/// assert rent exempt
pub fn assert_rent_exempt(rent: &Rent, account_info: &AccountInfo) -> ProgramResult {
    if !rent.is_exempt(account_info.lamports(), account_info.data_len()) {
        Err(MetadataError::NotRentExempt.into())
    } else {
        Ok(())
    }
}

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

/// Create account from scratch, stolen from Wormhole, slightly altered for my purposes
/// https://github.com/bartosz-lipinski/wormhole/blob/8478735ea7525043635524a62db2751e59d2bc38/solana/bridge/src/processor.rs#L1335
#[inline(always)]
pub fn create_account_raw<T>(
    accounts: &[AccountInfo],
    new_account: &Pubkey,
    payer: &Pubkey,
    owner: &Pubkey,
    seeds: &[&[u8]],
    size: u64,
) -> Result<(), ProgramError> {
    let ix = create_account(
        payer,
        new_account,
        Rent::default().minimum_balance(size as usize),
        size as u64,
        owner,
    );
    invoke_signed(&ix, accounts, &[seeds])
}
