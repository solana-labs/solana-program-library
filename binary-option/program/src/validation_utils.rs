use {
    crate::error::BinaryOptionError,
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        program_error::ProgramError,
        program_pack::{IsInitialized, Pack},
        pubkey::Pubkey,
    },
    spl_token::state::Mint,
};

pub fn assert_mint_authority_matches_mint(
    mint: &Mint,
    mint_authority_info: &AccountInfo,
) -> ProgramResult {
    match mint.mint_authority {
        solana_program::program_option::COption::None => {
            return Err(BinaryOptionError::InvalidMintAuthority.into());
        }
        solana_program::program_option::COption::Some(key) => {
            if *mint_authority_info.key != key {
                return Err(BinaryOptionError::InvalidMintAuthority.into());
            }
        }
    }

    if !mint_authority_info.is_signer {
        return Err(BinaryOptionError::NotMintAuthority.into());
    }

    Ok(())
}

pub fn assert_keys_equal(key1: Pubkey, key2: Pubkey) -> ProgramResult {
    if key1 != key2 {
        Err(BinaryOptionError::PublicKeyMismatch.into())
    } else {
        Ok(())
    }
}

pub fn assert_keys_unequal(key1: Pubkey, key2: Pubkey) -> ProgramResult {
    if key1 == key2 {
        Err(BinaryOptionError::PublicKeysShouldBeUnique.into())
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
        Err(BinaryOptionError::UninitializedAccount.into())
    } else {
        Ok(account)
    }
}

/// assert owned by
pub fn assert_owned_by(account: &AccountInfo, owner: &Pubkey) -> ProgramResult {
    if account.owner != owner {
        Err(BinaryOptionError::IncorrectOwner.into())
    } else {
        Ok(())
    }
}
