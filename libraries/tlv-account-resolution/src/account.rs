//! Structs for managing "required account state", ie. defining
//! accounts required for your interface program, which can be
//! `AccountMeta`s - which have fixed addresses - or PDAs -
//! which have addresses derived from a collection of seeds

use {
    crate::{error::AccountResolutionError, seeds::Seed},
    solana_program::{
        account_info::AccountInfo, instruction::AccountMeta, program_error::ProgramError,
        pubkey::Pubkey,
    },
};

/// Struct designed to serve as an `AccountMeta` but for a PDA.
///
/// Similar to `AccountMeta` in structure, but instead of a
/// fixed address uses seed configurations for deriving the PDA
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccountMetaPda {
    /// Seed configurations for the PDA
    pub seeds: [u8; 32],
    /// Whether the account should sign
    pub is_signer: bool,
    /// Whether the account should be writable
    pub is_writable: bool,
}

impl AccountMetaPda {
    /// Initialize a new `AccountMetaPda` by providing seed configs
    pub fn new(seeds: &[Seed], is_signer: bool, is_writable: bool) -> Result<Self, ProgramError> {
        Ok(Self {
            seeds: Seed::pack_slice(seeds)?,
            is_signer,
            is_writable,
        })
    }
}

/// Enum that binds together the two types of required accounts
/// possible in a TLV-based validation account.
#[derive(Clone, Debug, PartialEq)]
pub enum RequiredAccount {
    /// Mimics the `AccountMeta` type, which has a fixed address
    Account {
        /// The account's public key
        pubkey: Pubkey,
        /// Whether the account should sign
        is_signer: bool,
        /// Whether the account should be writable
        is_writable: bool,
    },
    /// Designed for a Program-Derived Address account, which
    /// derives its address from a set of seeds
    Pda {
        /// The seeds used to derive the account's PDA
        seeds: Vec<Seed>,
        /// Whether the account should sign
        is_signer: bool,
        /// Whether the account should be writable
        is_writable: bool,
    },
}

impl TryFrom<&RequiredAccount> for AccountMeta {
    type Error = ProgramError;

    fn try_from(value: &RequiredAccount) -> Result<Self, Self::Error> {
        match value {
            RequiredAccount::Account {
                pubkey,
                is_signer,
                is_writable,
            } => Ok(Self {
                pubkey: *pubkey,
                is_signer: *is_signer,
                is_writable: *is_writable,
            }),
            RequiredAccount::Pda { .. } => {
                Err(AccountResolutionError::RequiredAccountNotAccountMeta.into())
            }
        }
    }
}

impl TryFrom<&RequiredAccount> for AccountMetaPda {
    type Error = ProgramError;

    fn try_from(value: &RequiredAccount) -> Result<Self, Self::Error> {
        match value {
            RequiredAccount::Pda {
                seeds,
                is_signer,
                is_writable,
            } => Ok(Self {
                seeds: Seed::pack_slice(seeds)?,
                is_signer: *is_signer,
                is_writable: *is_writable,
            }),
            RequiredAccount::Account { .. } => {
                Err(AccountResolutionError::RequiredAccountNotPda.into())
            }
        }
    }
}

impl From<&AccountInfo<'_>> for RequiredAccount {
    fn from(account_info: &AccountInfo<'_>) -> Self {
        Self::Account {
            pubkey: *account_info.key,
            is_signer: account_info.is_signer,
            is_writable: account_info.is_writable,
        }
    }
}

impl From<AccountInfo<'_>> for RequiredAccount {
    fn from(account_info: AccountInfo<'_>) -> Self {
        Self::Account {
            pubkey: *account_info.key,
            is_signer: account_info.is_signer,
            is_writable: account_info.is_writable,
        }
    }
}

impl From<&AccountMeta> for RequiredAccount {
    fn from(meta: &AccountMeta) -> Self {
        Self::Account {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        }
    }
}

impl From<AccountMeta> for RequiredAccount {
    fn from(meta: AccountMeta) -> Self {
        Self::Account {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        }
    }
}

impl TryFrom<&AccountMetaPda> for RequiredAccount {
    type Error = ProgramError;

    fn try_from(pda: &AccountMetaPda) -> Result<Self, Self::Error> {
        Ok(Self::Pda {
            seeds: Seed::unpack_to_vec(&pda.seeds)?,
            is_signer: pda.is_signer,
            is_writable: pda.is_writable,
        })
    }
}

impl TryFrom<AccountMetaPda> for RequiredAccount {
    type Error = ProgramError;

    fn try_from(pda: AccountMetaPda) -> Result<Self, Self::Error> {
        Ok(Self::Pda {
            seeds: Seed::unpack_to_vec(&pda.seeds)?,
            is_signer: pda.is_signer,
            is_writable: pda.is_writable,
        })
    }
}
