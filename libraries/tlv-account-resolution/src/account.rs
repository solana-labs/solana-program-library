//! Structs/enums for managing "required account state", ie. defining accounts
//! required for your interface program, which can be  `AccountMeta`s - which
//! have fixed addresses - or PDAs - which have addresses derived from a
//! collection of seeds

use {
    crate::{error::AccountResolutionError, seeds::Seed},
    bytemuck::{Pod, Zeroable},
    solana_program::{
        account_info::AccountInfo, instruction::AccountMeta, program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_type_length_value::pod::PodBool,
};

/// `Pod` type for defining a required account in a validation account.
///
/// This can either be a standard `AccountMeta` or a `AccountMetaPda`.
/// Can be used in TLV-encoded data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PodAccountMeta {
    /// Discriminator to tell whether this represents a standard
    /// `AccountMeta` or an `AccountMetaPda`
    pub discriminator: u8,
    /// This `address_config` field can either be the pubkey of the account
    /// or the seeds used to derive the pubkey from provided inputs
    pub address_config: [u8; 32],
    /// Whether the account should sign
    pub is_signer: PodBool,
    /// Whether the account should be writable
    pub is_writable: PodBool,
}

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
            seeds: Seed::pack_into_array(seeds)?,
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

// Conversions to `PodAccountMeta`
impl From<&AccountMeta> for PodAccountMeta {
    fn from(meta: &AccountMeta) -> Self {
        Self {
            discriminator: 0,
            address_config: meta.pubkey.to_bytes(),
            is_signer: meta.is_signer.into(),
            is_writable: meta.is_writable.into(),
        }
    }
}
impl From<&AccountInfo<'_>> for PodAccountMeta {
    fn from(account_info: &AccountInfo) -> Self {
        Self {
            discriminator: 0,
            address_config: account_info.key.to_bytes(),
            is_signer: account_info.is_signer.into(),
            is_writable: account_info.is_writable.into(),
        }
    }
}
impl From<&AccountMetaPda> for PodAccountMeta {
    fn from(pda: &AccountMetaPda) -> Self {
        Self {
            discriminator: 1,
            address_config: pda.seeds,
            is_signer: pda.is_signer.into(),
            is_writable: pda.is_writable.into(),
        }
    }
}
impl From<&RequiredAccount> for PodAccountMeta {
    fn from(required_account: &RequiredAccount) -> Self {
        match required_account {
            RequiredAccount::Account {
                pubkey,
                is_signer,
                is_writable,
            } => Self {
                discriminator: 0,
                address_config: pubkey.to_bytes(),
                is_signer: PodBool::from(*is_signer),
                is_writable: PodBool::from(*is_writable),
            },
            RequiredAccount::Pda {
                seeds,
                is_signer,
                is_writable,
            } => Self {
                discriminator: 1,
                address_config: Seed::pack_into_array(seeds).unwrap(),
                is_signer: PodBool::from(*is_signer),
                is_writable: PodBool::from(*is_writable),
            },
        }
    }
}

// Conversions from `PodAccountMeta`
impl TryFrom<&PodAccountMeta> for AccountMeta {
    type Error = ProgramError;

    fn try_from(pod: &PodAccountMeta) -> Result<Self, Self::Error> {
        if pod.discriminator == 0 {
            Ok(AccountMeta {
                pubkey: Pubkey::try_from(pod.address_config)
                    .map_err(|_| ProgramError::from(AccountResolutionError::InvalidPubkey))?,
                is_signer: pod.is_signer.into(),
                is_writable: pod.is_writable.into(),
            })
        } else {
            Err(AccountResolutionError::RequiredAccountNotAccountMeta.into())
        }
    }
}
impl TryFrom<&PodAccountMeta> for AccountMetaPda {
    type Error = ProgramError;

    fn try_from(pod: &PodAccountMeta) -> Result<Self, Self::Error> {
        if pod.discriminator == 1 {
            Ok(AccountMetaPda {
                seeds: pod.address_config,
                is_signer: pod.is_signer.into(),
                is_writable: pod.is_writable.into(),
            })
        } else {
            Err(AccountResolutionError::RequiredAccountNotPda.into())
        }
    }
}
impl TryFrom<&PodAccountMeta> for RequiredAccount {
    type Error = ProgramError;

    fn try_from(pod: &PodAccountMeta) -> Result<Self, Self::Error> {
        if pod.discriminator == 0 {
            Ok(RequiredAccount::Account {
                pubkey: Pubkey::try_from(pod.address_config)
                    .map_err(|_| ProgramError::from(AccountResolutionError::InvalidPubkey))?,
                is_signer: pod.is_signer.into(),
                is_writable: pod.is_writable.into(),
            })
        } else if pod.discriminator == 1 {
            Ok(RequiredAccount::Pda {
                seeds: Seed::unpack_array(&pod.address_config)?,
                is_signer: pod.is_signer.into(),
                is_writable: pod.is_writable.into(),
            })
        } else {
            Err(AccountResolutionError::InvalidAccountType.into())
        }
    }
}

// `PartialEq` for `RequiredAccount`
impl PartialEq<AccountMeta> for RequiredAccount {
    fn eq(&self, other: &AccountMeta) -> bool {
        match *self {
            Self::Account {
                pubkey,
                is_signer,
                is_writable,
            } => {
                pubkey == other.pubkey
                    && is_signer == other.is_signer
                    && is_writable == other.is_writable
            }
            Self::Pda { .. } => false,
        }
    }
}
impl PartialEq<AccountInfo<'_>> for RequiredAccount {
    fn eq(&self, other: &AccountInfo<'_>) -> bool {
        match *self {
            Self::Account {
                pubkey,
                is_signer,
                is_writable,
            } => {
                pubkey == *other.key
                    && is_signer == other.is_signer
                    && is_writable == other.is_writable
            }
            Self::Pda { .. } => false,
        }
    }
}
impl PartialEq<AccountMetaPda> for RequiredAccount {
    fn eq(&self, other: &AccountMetaPda) -> bool {
        match self {
            Self::Account { .. } => false,
            Self::Pda {
                seeds,
                is_signer,
                is_writable,
            } => {
                let unpacked_seeds = Seed::unpack_array(&other.seeds).unwrap();
                seeds == &unpacked_seeds
                    && is_signer == &other.is_signer
                    && is_writable == &other.is_writable
            }
        }
    }
}

// Conversions to `RequiredAccount`
impl From<&AccountInfo<'_>> for RequiredAccount {
    fn from(account_info: &AccountInfo<'_>) -> Self {
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
impl TryFrom<&AccountMetaPda> for RequiredAccount {
    type Error = ProgramError;

    fn try_from(pda: &AccountMetaPda) -> Result<Self, Self::Error> {
        Ok(Self::Pda {
            seeds: Seed::unpack_array(&pda.seeds)?,
            is_signer: pda.is_signer,
            is_writable: pda.is_writable,
        })
    }
}

// Conversions from `RequiredAccount`
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
                seeds: Seed::pack_into_array(seeds)?,
                is_signer: *is_signer,
                is_writable: *is_writable,
            }),
            RequiredAccount::Account { .. } => {
                Err(AccountResolutionError::RequiredAccountNotPda.into())
            }
        }
    }
}
