//! Struct for managing extra required account configs, ie. defining accounts
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
/// This can either be a standard `AccountMeta` or a PDA.
/// Can be used in TLV-encoded data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ExtraAccountMeta {
    /// Discriminator to tell whether this represents a standard
    /// `AccountMeta` or a PDA
    pub discriminator: u8,
    /// This `address_config` field can either be the pubkey of the account
    /// or the seeds used to derive the pubkey from provided inputs
    pub address_config: [u8; 32],
    /// Whether the account should sign
    pub is_signer: PodBool,
    /// Whether the account should be writable
    pub is_writable: PodBool,
}
impl ExtraAccountMeta {
    /// Create a `ExtraAccountMeta` from a public key,
    /// thus representing a standard `AccountMeta`
    pub fn new_with_pubkey(
        pubkey: &Pubkey,
        is_signer: bool,
        is_writable: bool,
    ) -> Result<Self, ProgramError> {
        Ok(Self {
            discriminator: 0,
            address_config: pubkey.to_bytes(),
            is_signer: is_signer.into(),
            is_writable: is_writable.into(),
        })
    }

    /// Create a `ExtraAccountMeta` from a list of seed configurations,
    /// thus representing a PDA
    pub fn new_with_seeds(
        seeds: &[Seed],
        is_signer: bool,
        is_writable: bool,
    ) -> Result<Self, ProgramError> {
        Ok(Self {
            discriminator: 1,
            address_config: Seed::pack_into_address_config(seeds)?,
            is_signer: is_signer.into(),
            is_writable: is_writable.into(),
        })
    }
}

// Conversions to `ExtraAccountMeta`
impl From<&AccountMeta> for ExtraAccountMeta {
    fn from(meta: &AccountMeta) -> Self {
        Self {
            discriminator: 0,
            address_config: meta.pubkey.to_bytes(),
            is_signer: meta.is_signer.into(),
            is_writable: meta.is_writable.into(),
        }
    }
}
impl From<&AccountInfo<'_>> for ExtraAccountMeta {
    fn from(account_info: &AccountInfo) -> Self {
        Self {
            discriminator: 0,
            address_config: account_info.key.to_bytes(),
            is_signer: account_info.is_signer.into(),
            is_writable: account_info.is_writable.into(),
        }
    }
}

// Conversions from `ExtraAccountMeta`
impl TryFrom<&ExtraAccountMeta> for AccountMeta {
    type Error = ProgramError;

    fn try_from(pod: &ExtraAccountMeta) -> Result<Self, Self::Error> {
        if pod.discriminator == 0 {
            Ok(AccountMeta {
                pubkey: Pubkey::try_from(pod.address_config)
                    .map_err(|_| ProgramError::from(AccountResolutionError::InvalidPubkey))?,
                is_signer: pod.is_signer.into(),
                is_writable: pod.is_writable.into(),
            })
        } else {
            Err(AccountResolutionError::AccountTypeNotAccountMeta.into())
        }
    }
}
