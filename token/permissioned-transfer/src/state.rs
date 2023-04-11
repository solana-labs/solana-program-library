//! State transition types

use {
    crate::tlv::{pod_from_bytes, pod_from_bytes_mut, Discriminator, Value},
    bytemuck::{Pod, Zeroable},
    solana_program::{
        account_info::AccountInfo, instruction::AccountMeta, program_error::ProgramError,
        pubkey::Pubkey,
    },
};

pub(crate) const MAX_NUM_KEYS: usize = 3;
/// State for all pubkeys required to validate a transfer
// TODO this should work with any number of pubkeys, but I'm being lazy and want
// to use bytemuck to quickly prototype something.
// We need to implement more functions on this in order to properly add to the slice
// and all that, but it'll take some more time to get that working.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ExtraAccountMetas {
    /// Number of `Pubkey` instances in the slice
    pub length: u16,
    /// Slice of required pubkeys to validate a transfer, along with the normal
    /// checked-transfer accounts and this account.
    pub metas: [PodAccountMeta; MAX_NUM_KEYS],
}

/// The standard `bool` is not a `Pod`, define a replacement that is
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodBool(u8);
impl From<bool> for PodBool {
    fn from(b: bool) -> Self {
        Self(if b { 1 } else { 0 })
    }
}
impl From<&PodBool> for bool {
    fn from(b: &PodBool) -> Self {
        b.0 != 0
    }
}
impl From<PodBool> for bool {
    fn from(b: PodBool) -> Self {
        b.0 != 0
    }
}

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

impl From<PodAccountMeta> for AccountMeta {
    fn from(meta: PodAccountMeta) -> Self {
        Self {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer.into(),
            is_writable: meta.is_writable.into(),
        }
    }
}

/// First 8 bytes of `hash::hashv(&["permissioned-transfer:validation-pubkeys"])`
impl Value for ExtraAccountMetas {
    const TYPE: Discriminator = Discriminator::new([250, 175, 124, 64, 235, 120, 63, 195]);

    fn try_from_bytes(bytes: &[u8]) -> Result<&Self, ProgramError> {
        pod_from_bytes(bytes)
    }

    fn try_from_bytes_mut(bytes: &mut [u8]) -> Result<&mut Self, ProgramError> {
        pod_from_bytes_mut(bytes)
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{DISCRIMINATOR_LENGTH, NAMESPACE},
        solana_program::hash,
    };

    #[test]
    fn discriminator() {
        let preimage = hash::hashv(&[format!("{NAMESPACE}:validation-pubkeys").as_bytes()]);
        let discriminator =
            Discriminator::try_from(&preimage.as_ref()[..DISCRIMINATOR_LENGTH]).unwrap();
        assert_eq!(discriminator, ExtraAccountMetas::TYPE);
    }
}
