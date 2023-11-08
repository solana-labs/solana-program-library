//! Generic Token Account, copied from spl_token::state
// Remove all of this and use spl-token's version once token 3.4.0 is released
use {
    crate::state::AccountState,
    solana_program::pubkey::{Pubkey, PUBKEY_BYTES},
};

const SPL_TOKEN_ACCOUNT_MINT_OFFSET: usize = 0;
const SPL_TOKEN_ACCOUNT_OWNER_OFFSET: usize = 32;

/// A trait for token Account structs to enable efficiently unpacking various
/// fields without unpacking the complete state.
pub trait GenericTokenAccount {
    /// Check if the account data is a valid token account
    fn valid_account_data(account_data: &[u8]) -> bool;

    /// Call after account length has already been verified to unpack the
    /// account owner
    fn unpack_account_owner_unchecked(account_data: &[u8]) -> &Pubkey {
        Self::unpack_pubkey_unchecked(account_data, SPL_TOKEN_ACCOUNT_OWNER_OFFSET)
    }

    /// Call after account length has already been verified to unpack the
    /// account mint
    fn unpack_account_mint_unchecked(account_data: &[u8]) -> &Pubkey {
        Self::unpack_pubkey_unchecked(account_data, SPL_TOKEN_ACCOUNT_MINT_OFFSET)
    }

    /// Call after account length has already been verified to unpack a Pubkey
    /// at the specified offset. Panics if `account_data.len()` is less than
    /// `PUBKEY_BYTES`
    fn unpack_pubkey_unchecked(account_data: &[u8], offset: usize) -> &Pubkey {
        bytemuck::from_bytes(&account_data[offset..offset + PUBKEY_BYTES])
    }

    /// Unpacks an account's owner from opaque account data.
    fn unpack_account_owner(account_data: &[u8]) -> Option<&Pubkey> {
        if Self::valid_account_data(account_data) {
            Some(Self::unpack_account_owner_unchecked(account_data))
        } else {
            None
        }
    }

    /// Unpacks an account's mint from opaque account data.
    fn unpack_account_mint(account_data: &[u8]) -> Option<&Pubkey> {
        if Self::valid_account_data(account_data) {
            Some(Self::unpack_account_mint_unchecked(account_data))
        } else {
            None
        }
    }
}

/// The offset of state field in Account's C representation
pub const ACCOUNT_INITIALIZED_INDEX: usize = 108;

/// Check if the account data buffer represents an initialized account.
/// This is checking the `state` (AccountState) field of an Account object.
pub fn is_initialized_account(account_data: &[u8]) -> bool {
    *account_data
        .get(ACCOUNT_INITIALIZED_INDEX)
        .unwrap_or(&(AccountState::Uninitialized as u8))
        != AccountState::Uninitialized as u8
}
