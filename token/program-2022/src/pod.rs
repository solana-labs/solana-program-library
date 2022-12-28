//! Solana program utilities for Plain Old Data types
use {
    bytemuck::{Pod, Zeroable},
    solana_program::{program_error::ProgramError, program_option::COption, pubkey::Pubkey},
    solana_zk_token_sdk::zk_token_elgamal::pod,
    std::convert::TryFrom,
};

/// A Pubkey that encodes `None` as all `0`, meant to be usable as a Pod type,
/// similar to all NonZero* number types from the bytemuck library.
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct OptionalNonZeroPubkey(Pubkey);
impl TryFrom<Option<Pubkey>> for OptionalNonZeroPubkey {
    type Error = ProgramError;
    fn try_from(p: Option<Pubkey>) -> Result<Self, Self::Error> {
        match p {
            None => Ok(Self(Pubkey::default())),
            Some(pubkey) => {
                if pubkey == Pubkey::default() {
                    Err(ProgramError::InvalidArgument)
                } else {
                    Ok(Self(pubkey))
                }
            }
        }
    }
}
impl TryFrom<COption<Pubkey>> for OptionalNonZeroPubkey {
    type Error = ProgramError;
    fn try_from(p: COption<Pubkey>) -> Result<Self, Self::Error> {
        match p {
            COption::None => Ok(Self(Pubkey::default())),
            COption::Some(pubkey) => {
                if pubkey == Pubkey::default() {
                    Err(ProgramError::InvalidArgument)
                } else {
                    Ok(Self(pubkey))
                }
            }
        }
    }
}
impl From<OptionalNonZeroPubkey> for Option<Pubkey> {
    fn from(p: OptionalNonZeroPubkey) -> Self {
        if p.0 == Pubkey::default() {
            None
        } else {
            Some(p.0)
        }
    }
}
impl From<OptionalNonZeroPubkey> for COption<Pubkey> {
    fn from(p: OptionalNonZeroPubkey) -> Self {
        if p.0 == Pubkey::default() {
            COption::None
        } else {
            COption::Some(p.0)
        }
    }
}

/// ElGamal public key used for encryption
pub type EncryptionPubkey = pod::ElGamalPubkey;
/// An EncryptionPubkey that encodes `None` as all `0`, meant to be usable as a Pod type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct OptionalNonZeroEncryptionPubkey(EncryptionPubkey);
impl OptionalNonZeroEncryptionPubkey {
    /// Checks equality between an OptionalNonZeroEncryptionPubkey and an EncryptionPubkey when
    /// interpreted as bytes.
    pub fn equals(&self, other: &EncryptionPubkey) -> bool {
        &self.0 == other
    }
}
impl TryFrom<Option<EncryptionPubkey>> for OptionalNonZeroEncryptionPubkey {
    type Error = ProgramError;
    fn try_from(p: Option<EncryptionPubkey>) -> Result<Self, Self::Error> {
        match p {
            None => Ok(Self(EncryptionPubkey::default())),
            Some(encryption_pubkey) => {
                if encryption_pubkey == EncryptionPubkey::default() {
                    Err(ProgramError::InvalidArgument)
                } else {
                    Ok(Self(encryption_pubkey))
                }
            }
        }
    }
}
impl From<OptionalNonZeroEncryptionPubkey> for Option<EncryptionPubkey> {
    fn from(p: OptionalNonZeroEncryptionPubkey) -> Self {
        if p.0 == EncryptionPubkey::default() {
            None
        } else {
            Some(p.0)
        }
    }
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

/// Simple macro for implementing conversion functions between Pod* ints and standard ints.
///
/// The standard int types can cause alignment issues when placed in a `Pod`,
/// so these replacements are usable in all `Pod`s.
macro_rules! impl_int_conversion {
    ($P:ty, $I:ty) => {
        impl From<$I> for $P {
            fn from(n: $I) -> Self {
                Self(n.to_le_bytes())
            }
        }
        impl From<$P> for $I {
            fn from(pod: $P) -> Self {
                Self::from_le_bytes(pod.0)
            }
        }
    };
}

/// `u16` type that can be used in `Pod`s
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodU16([u8; 2]);
impl_int_conversion!(PodU16, u16);

/// `i16` type that can be used in `Pod`s
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodI16([u8; 2]);
impl_int_conversion!(PodI16, i16);

/// `u64` type that can be used in `Pod`s
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodU64([u8; 8]);
impl_int_conversion!(PodU64, u64);

/// `i64` type that can be used in `Pod`s
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodI64([u8; 8]);
impl_int_conversion!(PodI64, i64);

/// On-chain size of a `Pod` type
pub fn pod_get_packed_len<T: Pod>() -> usize {
    std::mem::size_of::<T>()
}

/// Convert a `Pod` into a slice (zero copy)
pub fn pod_bytes_of<T: Pod>(t: &T) -> &[u8] {
    bytemuck::bytes_of(t)
}

/// Convert a slice into a `Pod` (zero copy)
pub fn pod_from_bytes<T: Pod>(bytes: &[u8]) -> Result<&T, ProgramError> {
    bytemuck::try_from_bytes(bytes).map_err(|_| ProgramError::InvalidArgument)
}

/// Maybe convert a slice into a `Pod` (zero copy)
///
/// Returns `None` if the slice is empty, but `Err` if all other lengths but `get_packed_len()`
/// This function exists primarily because `Option<T>` is not a `Pod`.
pub fn pod_maybe_from_bytes<T: Pod>(bytes: &[u8]) -> Result<Option<&T>, ProgramError> {
    if bytes.is_empty() {
        Ok(None)
    } else {
        bytemuck::try_from_bytes(bytes)
            .map(Some)
            .map_err(|_| ProgramError::InvalidArgument)
    }
}

/// Convert a slice into a mutable `Pod` (zero copy)
pub fn pod_from_bytes_mut<T: Pod>(bytes: &mut [u8]) -> Result<&mut T, ProgramError> {
    bytemuck::try_from_bytes_mut(bytes).map_err(|_| ProgramError::InvalidArgument)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pod_bool() {
        assert!(pod_from_bytes::<PodBool>(&[]).is_err());
        assert!(pod_from_bytes::<PodBool>(&[0, 0]).is_err());

        for i in 0..=u8::MAX {
            assert_eq!(i != 0, bool::from(pod_from_bytes::<PodBool>(&[i]).unwrap()));
        }
    }

    #[test]
    fn test_pod_u64() {
        assert!(pod_from_bytes::<PodU64>(&[]).is_err());
        assert_eq!(
            1u64,
            u64::from(*pod_from_bytes::<PodU64>(&[1, 0, 0, 0, 0, 0, 0, 0]).unwrap())
        );
    }

    #[test]
    fn test_pod_option() {
        assert_eq!(
            Some(Pubkey::new_from_array([1; 32])),
            Option::<Pubkey>::from(*pod_from_bytes::<OptionalNonZeroPubkey>(&[1; 32]).unwrap())
        );
        assert_eq!(
            None,
            Option::<Pubkey>::from(*pod_from_bytes::<OptionalNonZeroPubkey>(&[0; 32]).unwrap())
        );
        assert!(pod_from_bytes::<OptionalNonZeroPubkey>(&[]).is_err());
        assert!(pod_from_bytes::<OptionalNonZeroPubkey>(&[0; 1]).is_err());
        assert!(pod_from_bytes::<OptionalNonZeroPubkey>(&[1; 1]).is_err());
    }
}
