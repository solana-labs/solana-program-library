//! State transition types

use crate::{error::TokenError, instruction::MAX_SIGNERS, option::COption};
use solana_sdk::{program_error::ProgramError, pubkey::Pubkey};
use std::mem::size_of;

/// Mint data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mint {
    /// Optional authority used to mint new tokens. The mint authority may only be provided during
    /// mint creation. If no mint authority is present then the mint has a fixed supply and no
    /// further tokens may be minted.
    pub mint_authority: COption<Pubkey>,
    /// Number of base 10 digits to the right of the decimal place.
    pub decimals: u8,
    /// Is `true` if this structure has been initialized
    pub is_initialized: bool,
    /// Optional authority to freeze token accounts.
    pub freeze_authority: COption<Pubkey>,
}
impl IsInitialized for Mint {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

/// Account data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Account {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub amount: u64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: COption<Pubkey>,
    /// The account's state
    pub state: AccountState,
    /// Is this a native token
    pub is_native: bool,
    /// The amount delegated
    pub delegated_amount: u64,
}
impl Account {
    /// Checks if account is frozen
    pub fn is_frozen(&self) -> bool {
        self.state == AccountState::Frozen
    }
}
impl IsInitialized for Account {
    fn is_initialized(&self) -> bool {
        self.state != AccountState::Uninitialized
    }
}

/// Account state.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AccountState {
    /// Account is not yet initialized
    Uninitialized,
    /// Account is initialized; the account owner and/or delegate may perform permitted operations
    /// on this account
    Initialized,
    /// Account has been frozen by the mint freeze authority. Neither the account owner nor
    /// the delegate are able to perform operations on this account.
    Frozen,
}

impl Default for AccountState {
    fn default() -> Self {
        AccountState::Uninitialized
    }
}

/// Multisignature data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Multisig {
    /// Number of signers required
    pub m: u8,
    /// Number of valid signers
    pub n: u8,
    /// Is `true` if this structure has been initialized
    pub is_initialized: bool,
    /// Signer public keys
    pub signers: [Pubkey; MAX_SIGNERS],
}
impl IsInitialized for Multisig {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

/// Check is a token state is initialized
pub trait IsInitialized {
    /// Is initialized
    fn is_initialized(&self) -> bool;
}

/// Unpacks a token state from a bytes buffer while assuring that the state is initialized.
pub fn unpack<T: IsInitialized>(input: &mut [u8]) -> Result<&mut T, ProgramError> {
    let mut_ref: &mut T = unpack_unchecked(input)?;
    if !mut_ref.is_initialized() {
        return Err(TokenError::UninitializedState.into());
    }
    Ok(mut_ref)
}
/// Unpacks a token state from a bytes buffer without checking that the state is initialized.
pub fn unpack_unchecked<T: IsInitialized>(input: &mut [u8]) -> Result<&mut T, ProgramError> {
    if input.len() != size_of::<T>() {
        return Err(ProgramError::InvalidAccountData);
    }
    #[allow(clippy::cast_ptr_alignment)]
    Ok(unsafe { &mut *(&mut input[0] as *mut u8 as *mut T) })
}
