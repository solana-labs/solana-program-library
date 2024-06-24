//! Generic `Option` that can be used as a `Pod` for types that can have
//! a designated `None` value.
//!
//! For example, a 64-bit unsigned integer can designate `0` as a `None` value.
//! This would be equivalent to
//! [`Option<NonZeroU64>`](https://doc.rust-lang.org/std/num/type.NonZeroU64.html)
//! and provide the same memory layout optimization.

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
