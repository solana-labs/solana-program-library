//! Pod types to be used with bytemuck for zero-copy serde

use {
    bytemuck::{Pod, Zeroable},
    solana_program::{account_info::AccountInfo, instruction::AccountMeta, pubkey::Pubkey},
    spl_type_length_value::pod::PodBool,
};

/// The standard `AccountMeta` is not a `Pod`, define a replacement that is
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PodAccountMeta {
    /// The pubkey of the account
    pub pubkey: Pubkey,
    /// Whether the account should sign
    pub is_signer: PodBool,
    /// Whether the account should be writable
    pub is_writable: PodBool,
}
impl PartialEq<AccountInfo<'_>> for PodAccountMeta {
    fn eq(&self, other: &AccountInfo) -> bool {
        self.pubkey == *other.key
            && self.is_signer == other.is_signer.into()
            && self.is_writable == other.is_writable.into()
    }
}

impl From<&AccountInfo<'_>> for PodAccountMeta {
    fn from(account_info: &AccountInfo) -> Self {
        Self {
            pubkey: *account_info.key,
            is_signer: account_info.is_signer.into(),
            is_writable: account_info.is_writable.into(),
        }
    }
}

impl From<&AccountMeta> for PodAccountMeta {
    fn from(meta: &AccountMeta) -> Self {
        Self {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer.into(),
            is_writable: meta.is_writable.into(),
        }
    }
}

impl From<&PodAccountMeta> for AccountMeta {
    fn from(meta: &PodAccountMeta) -> Self {
        Self {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer.into(),
            is_writable: meta.is_writable.into(),
        }
    }
}
