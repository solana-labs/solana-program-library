//! Generic `Option` that can be used as a `Pod`s for types that can have
//! a `None` value.

use {
    bytemuck::{Pod, Zeroable},
    solana_program::{program_option::COption, pubkey::Pubkey},
};

/// Trait for types that can be `None`.
///
/// This trait is used to indicate that a type can be `None` according to a
/// specific value.
pub trait Nullable: Pod {
    /// Indicates whether the value is `None` or not.
    fn is_none(&self) -> bool;
}

/// A "pod-enabled" type that can be used as an `Option<T>` without
/// requiring extra space to indicate if the value is `Some` or `None`.
///
/// This can be used when a specific value of `T` indicates that its
/// value is `None`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct PodOption<T: Nullable>(T);

unsafe impl<T: Nullable> Pod for PodOption<T> {}

unsafe impl<T: Nullable> Zeroable for PodOption<T> {}

impl<T: Nullable> PodOption<T> {
    #[inline]
    pub fn new(value: T) -> Self {
        Self(value)
    }

    #[inline]
    pub fn to_option(&self) -> Option<&T> {
        if self.0.is_none() {
            None
        } else {
            Some(&self.0)
        }
    }

    #[inline]
    pub fn to_option_mut(&mut self) -> Option<&mut T> {
        if self.0.is_none() {
            None
        } else {
            Some(&mut self.0)
        }
    }
}

impl<T: Nullable> From<PodOption<T>> for Option<T> {
    fn from(value: PodOption<T>) -> Self {
        if value.0.is_none() {
            None
        } else {
            Some(value.0)
        }
    }
}

impl<T: Nullable> From<PodOption<T>> for COption<T> {
    fn from(value: PodOption<T>) -> Self {
        if value.0.is_none() {
            COption::None
        } else {
            COption::Some(value.0)
        }
    }
}

/// Simple macro for implementing the `Nullable` trait on int types.
///
/// The implementation assumes that the value `0` represents `None`.
macro_rules! impl_int_nullable {
    ( $I:ty ) => {
        impl Nullable for $I {
            fn is_none(&self) -> bool {
                *self == 0
            }
        }
    };
}

impl_int_nullable!(u8);
impl_int_nullable!(u16);
impl_int_nullable!(u32);
impl_int_nullable!(u64);
impl_int_nullable!(u128);

/// Implementation of `Nullable` for `Pubkey`.
///
/// The implementation assumes that the default value of `Pubkey` represents
/// `None`.
impl Nullable for Pubkey {
    fn is_none(&self) -> bool {
        self == &Pubkey::default()
    }
}

#[cfg(test)]
mod tests {

    use {super::*, crate::bytemuck::pod_slice_from_bytes, solana_program::sysvar};

    #[test]
    fn test_pod_option_u8() {
        let some_u8 = PodOption::new(42u8);
        assert_eq!(some_u8.to_option(), Some(&42u8));

        let none_u8 = PodOption::new(0u8);
        assert_eq!(none_u8.to_option(), None);

        let data = [42u8, 0u8, 100u8];

        let values = pod_slice_from_bytes::<PodOption<u8>>(&data).unwrap();
        assert_eq!(values[0].to_option(), Some(&42u8));
        assert_eq!(values[1].to_option(), None);
        assert_eq!(values[2].to_option(), Some(&100u8));
    }

    #[test]
    fn test_pod_option_u16() {
        let some_u16 = PodOption::new(1000u16);
        assert_eq!(some_u16.to_option(), Some(&1000u16));

        let none_u16 = PodOption::new(0u16);
        assert_eq!(none_u16.to_option(), None);

        let mut data = Vec::with_capacity(6);
        data.extend_from_slice(1024u16.to_le_bytes().as_ref());
        data.extend_from_slice(0u16.to_le_bytes().as_ref());
        data.extend_from_slice(1000u16.to_le_bytes().as_ref());

        let values = pod_slice_from_bytes::<PodOption<u16>>(&data).unwrap();
        assert_eq!(values[0].to_option(), Some(&1024u16));
        assert_eq!(values[1].to_option(), None);
        assert_eq!(values[2].to_option(), Some(&1000u16));
    }

    #[test]
    fn test_pod_option_u32() {
        let some_u32 = PodOption::new(10000u32);
        assert_eq!(some_u32.to_option(), Some(&10000u32));

        let none_u32 = PodOption::new(0u32);
        assert_eq!(none_u32.to_option(), None);

        let mut data = Vec::with_capacity(12);
        data.extend_from_slice(10024u32.to_le_bytes().as_ref());
        data.extend_from_slice(0u32.to_le_bytes().as_ref());
        data.extend_from_slice(10000u32.to_le_bytes().as_ref());

        let values = pod_slice_from_bytes::<PodOption<u32>>(&data).unwrap();
        assert_eq!(values[0].to_option(), Some(&10024u32));
        assert_eq!(values[1].to_option(), None);
        assert_eq!(values[2].to_option(), Some(&10000u32));
    }

    #[test]
    fn test_pod_option_u64() {
        let some_u64 = PodOption::new(1000u64);
        assert_eq!(some_u64.to_option(), Some(&1000u64));

        let none_u64 = PodOption::new(0u64);
        assert_eq!(none_u64.to_option(), None);

        let mut data = Vec::with_capacity(24);
        data.extend_from_slice(1000024u64.to_le_bytes().as_ref());
        data.extend_from_slice(0u64.to_le_bytes().as_ref());
        data.extend_from_slice(1000000u64.to_le_bytes().as_ref());

        let values = pod_slice_from_bytes::<PodOption<u64>>(&data).unwrap();
        assert_eq!(values[0].to_option(), Some(&1000024u64));
        assert_eq!(values[1].to_option(), None);
        assert_eq!(values[2].to_option(), Some(&1000000u64));
    }

    #[test]
    fn test_pod_option_u128() {
        let some_u128 = PodOption::new(100000000000000u128);
        assert_eq!(some_u128.to_option(), Some(&100000000000000u128));

        let none_u128 = PodOption::new(0u128);
        assert_eq!(none_u128.to_option(), None);

        let mut data = Vec::with_capacity(48);
        data.extend_from_slice(10000000000000024u128.to_le_bytes().as_ref());
        data.extend_from_slice(0u128.to_le_bytes().as_ref());
        data.extend_from_slice(100000000000000u128.to_le_bytes().as_ref());

        let values = pod_slice_from_bytes::<PodOption<u128>>(&data).unwrap();
        assert_eq!(values[0].to_option(), Some(&10000000000000024u128));
        assert_eq!(values[1].to_option(), None);
        assert_eq!(values[2].to_option(), Some(&100000000000000u128));
    }

    #[test]
    fn test_pod_option_pubkey() {
        let some_pubkey = PodOption::new(sysvar::ID);
        assert_eq!(some_pubkey.to_option(), Some(&sysvar::ID));

        let none_pubkey = PodOption::new(Pubkey::default());
        assert_eq!(none_pubkey.to_option(), None);

        let mut data = Vec::with_capacity(64);
        data.extend_from_slice(sysvar::ID.as_ref());
        data.extend_from_slice(&[0u8; 32]);

        let values = pod_slice_from_bytes::<PodOption<Pubkey>>(&data).unwrap();
        assert_eq!(values[0].to_option(), Some(&sysvar::ID));
        assert_eq!(values[1].to_option(), None);
    }
}
