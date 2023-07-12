//! Pod types to be used with bytemuck for zero-copy serde

use {
    crate::{
        account::{AccountMetaPda, RequiredAccount},
        error::AccountResolutionError,
        seeds::Seed,
    },
    bytemuck::{Pod, Zeroable},
    solana_program::{account_info::AccountInfo, instruction::AccountMeta, pubkey::Pubkey},
    spl_type_length_value::pod::PodBool,
};

/// The standard `AccountMeta` is not a `Pod`, define a replacement that is
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

impl TryFrom<&PodAccountMeta> for AccountMeta {
    type Error = ProgramError;

    fn try_from(pod: &PodAccountMeta) -> Result<Self, Self::Error> {
        if pod.discriminator == 0 {
            Ok(AccountMeta {
                pubkey: Pubkey::new(&pod.address_config),
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
                pubkey: Pubkey::new(&pod.address_config),
                is_signer: pod.is_signer.into(),
                is_writable: pod.is_writable.into(),
            })
        } else {
            Ok(RequiredAccount::Pda {
                seeds: Seed::unpack_to_vec(&pod.address_config)?,
                is_signer: pod.is_signer.into(),
                is_writable: pod.is_writable.into(),
            })
        }
    }
}

/// Unfortunately this has to be its own trait in order for the
/// trait constraint in `ExtraAccountMetas::init` to work properly.
///
/// The `?` can't resolve to a `ProgramError` using just `TryFrom<T>`
pub trait TryFromAccountType<T>: Sized {
    /// Mimics the functionality of `try_from(T)` for `PodAccountMeta`
    fn try_from_account(value: T) -> Result<Self, ProgramError>;
}

impl TryFromAccountType<&AccountInfo<'_>> for PodAccountMeta {
    fn try_from_account(account_info: &AccountInfo<'_>) -> Result<Self, ProgramError> {
        Ok(PodAccountMeta {
            discriminator: 0,
            address_config: account_info.key.to_bytes(),
            is_signer: account_info.is_signer.into(),
            is_writable: account_info.is_writable.into(),
        })
    }
}

impl TryFromAccountType<&AccountMeta> for PodAccountMeta {
    fn try_from_account(meta: &AccountMeta) -> Result<Self, ProgramError> {
        Ok(PodAccountMeta {
            discriminator: 0,
            address_config: meta.pubkey.to_bytes(),
            is_signer: meta.is_signer.into(),
            is_writable: meta.is_writable.into(),
        })
    }
}

impl TryFromAccountType<&AccountMetaPda> for PodAccountMeta {
    fn try_from_account(pda: &AccountMetaPda) -> Result<Self, ProgramError> {
        Ok(PodAccountMeta {
            discriminator: 1,
            address_config: pda.seeds,
            is_signer: pda.is_signer.into(),
            is_writable: pda.is_writable.into(),
        })
    }
}

impl TryFromAccountType<&RequiredAccount> for PodAccountMeta {
    fn try_from_account(value: &RequiredAccount) -> Result<Self, ProgramError> {
        match value {
            RequiredAccount::Account {
                pubkey,
                is_signer,
                is_writable,
            } => Ok(PodAccountMeta {
                discriminator: 0,
                address_config: pubkey.to_bytes(),
                is_signer: is_signer.into(),
                is_writable: is_writable.into(),
            }),
            RequiredAccount::Pda {
                seeds,
                is_signer,
                is_writable,
            } => Ok(PodAccountMeta {
                discriminator: 1,
                address_config: Seed::pack_slice(seeds)?,
                is_signer: is_signer.into(),
                is_writable: is_writable.into(),
            }),
        }
    }
}
