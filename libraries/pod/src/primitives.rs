//! primitive types that can be used in `Pod`s
use bytemuck::{Pod, Zeroable};

#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};

/// The standard `bool` is not a `Pod`, define a replacement that is
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(from = "bool", into = "bool"))]
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
#[macro_export]
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
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(from = "i16", into = "i16"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodI16([u8; 2]);
impl_int_conversion!(PodI16, i16);

/// `u32` type that can be used in `Pod`s
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodU32([u8; 4]);
impl_int_conversion!(PodU32, u32);

/// `u64` type that can be used in `Pod`s
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(from = "u64", into = "u64"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodU64([u8; 8]);
impl_int_conversion!(PodU64, u64);

/// `i64` type that can be used in `Pod`s
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodI64([u8; 8]);
impl_int_conversion!(PodI64, i64);

#[cfg(test)]
mod tests {
    use {super::*, crate::bytemuck::*};

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
}