//! Solana program utilities for Plain Old Data types
use {
    bytemuck::{Pod, Zeroable},
    solana_program::{program_error::ProgramError, pubkey::Pubkey},
};

// TODO Generic `PodOption` is tricky, since we need to be sure that it'll
// always take up the same amount of space.  A few ideas, since all of these are
// pubkeys at the moment:
//
//#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
//#[repr(transparent)]
//struct NonZeroPubkey(Pubkey);
//impl NonZeroPubkey {
//    fn new(pubkey: Pubkey) -> Option<Self> {
//        if pubkey == Pubkey::default() {
//            None
//        else {
//            Some(Self(pubkey))
//        }
//    }
//}
//type OptionNonZeroPubkey = Option<NonZeroPubkey>;
//#[allow(unsafe_code)]
//unsafe impl Pod for Option<NonZeroPubkey> {}
//#[allow(unsafe_code)]
//unsafe impl Zeroable for Option<NonZeroPubkey> {}
// fails because you can't impl a foreign trait on a foreign type, option

// Looks just like a pubkey and works like a Pod.
// Might be confusing?
//
// 2.
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
struct PodOptionPubkey {
    option: u8,
    pubkey: Pubkey,
}
// Doesn't work like an enum though
//
// 3.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C, u8)]
enum PodOption<T: Pod> {
    None,
    Some(T),
}
#[allow(unsafe_code)]
unsafe impl<T: Pod> Pod for PodOption<T> {}
#[allow(unsafe_code)]
unsafe impl<T: Pod> Zeroable for PodOption<T> {}
// This maintains the size, may be unclear to use.  We'll also have to reimplement
// all of the Option helpers, same as COption has to.
//
// I'm leaning towards number 3, and eventually adding the support to COption.

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

/// The standard `u16` can cause alignment issues when placed in a `Pod`, define a replacement that
/// is usable in all `Pod`s
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodU16([u8; 2]);
impl From<u16> for PodU16 {
    fn from(n: u16) -> Self {
        Self(n.to_le_bytes())
    }
}
impl From<PodU16> for u16 {
    fn from(pod: PodU16) -> Self {
        Self::from_le_bytes(pod.0)
    }
}

/// The standard `u64` can cause alignment issues when placed in a `Pod`, define a replacement that
/// is usable in all `Pod`s
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodU64([u8; 8]);
impl From<u64> for PodU64 {
    fn from(n: u64) -> Self {
        Self(n.to_le_bytes())
    }
}
impl From<PodU64> for u64 {
    fn from(pod: PodU64) -> Self {
        Self::from_le_bytes(pod.0)
    }
}

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
/// This function exists primary because `Option<T>` is not a `Pod`.
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
        assert!(pod_from_bytes::<PodOption<Pubkey>>(&[]).is_err());
        assert_eq!(
            PodOption::Some(Pubkey::new_from_array([1; 32])),
            *pod_from_bytes::<PodOption<Pubkey>>(&[1; 33]).unwrap()
        );
        assert_eq!(
            PodOption::None,
            *pod_from_bytes::<PodOption<Pubkey>>(&[0; 33]).unwrap()
        );
        assert!(pod_from_bytes::<PodOption<Pubkey>>(&[0; 32]).is_err());
        assert!(pod_from_bytes::<PodOption<Pubkey>>(&[1; 32]).is_err());
        assert!(pod_from_bytes::<PodOption<Pubkey>>(&[0; 34]).is_err());
        assert!(pod_from_bytes::<PodOption<Pubkey>>(&[1; 34]).is_err());
    }
}
