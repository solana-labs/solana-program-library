use {
    crate::error::BettingPoolError,
    // borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::AccountInfo,
        // borsh::try_from_slice_unchecked,
        entrypoint::ProgramResult,
        // msg,
        program_error::ProgramError,
        program_pack::{IsInitialized, Pack},
        pubkey::Pubkey,
        // system_instruction,
        // sysvar::{rent::Rent, Sysvar},
    },
    spl_token::state::Mint,
    // std::convert::TryInto,
};

pub fn assert_mint_authority_matches_mint(
    mint: &Mint,
    mint_authority_info: &AccountInfo,
) -> ProgramResult {
    match mint.mint_authority {
        solana_program::program_option::COption::None => {
            return Err(BettingPoolError::InvalidMintAuthority.into());
        }
        solana_program::program_option::COption::Some(key) => {
            if *mint_authority_info.key != key {
                return Err(BettingPoolError::InvalidMintAuthority.into());
            }
        }
    }

    if !mint_authority_info.is_signer {
        return Err(BettingPoolError::NotMintAuthority.into());
    }

    Ok(())
}

pub fn assert_keys_equal(key1: Pubkey, key2: Pubkey) -> ProgramResult {
    if key1 != key2 {
        Err(BettingPoolError::PublicKeyMismatch.into())
    } else {
        Ok(())
    }
}

pub fn assert_keys_unequal(key1: Pubkey, key2: Pubkey) -> ProgramResult {
    if key1 == key2 {
        Err(BettingPoolError::PublicKeysShouldBeUnique.into())
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
        Err(BettingPoolError::UninitializedAccount.into())
    } else {
        Ok(account)
    }
}

/// assert owned by
pub fn assert_owned_by(account: &AccountInfo, owner: &Pubkey) -> ProgramResult {
    if account.owner != owner {
        Err(BettingPoolError::IncorrectOwner.into())
    } else {
        Ok(())
    }
}
