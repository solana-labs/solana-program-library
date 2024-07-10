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
pub trait Nullable: Default + Pod {
    /// Indicates whether the value is `None` or not.
    fn is_none(&self) -> bool;

    /// Indicates whether the value is `Some`` value of type `T`` or not.
    fn is_some(&self) -> bool {
        !self.is_none()
    }
}

/// A "pod-enabled" type that can be used as an `Option<T>` without
/// requiring extra space to indicate if the value is `Some` or `None`.
///
/// This can be used when a specific value of `T` indicates that its
/// value is `None`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PodOption<T: Nullable>(T);

impl<T: Nullable> PodOption<T> {
    /// Returns the contained value as an `Option`.
    #[inline]
    pub fn get(self) -> Option<T> {
        if self.0.is_none() {
            None
        } else {
            Some(self.0)
        }
    }

    /// Returns the contained value as an `Option`.
    #[inline]
    pub fn as_ref(&self) -> Option<&T> {
        if self.0.is_none() {
            None
        } else {
            Some(&self.0)
        }
    }

    /// Returns the contained value as a mutable `Option`.
    #[inline]
    pub fn as_mut(&mut self) -> Option<&mut T> {
        if self.0.is_none() {
            None
        } else {
            Some(&mut self.0)
        }
    }
}

unsafe impl<T: Nullable> Pod for PodOption<T> {}

unsafe impl<T: Nullable> Zeroable for PodOption<T> {}

impl<T: Nullable> From<T> for PodOption<T> {
    fn from(value: T) -> Self {
        PodOption(value)
    }
}

impl<T: Nullable> From<Option<T>> for PodOption<T> {
    fn from(from: Option<T>) -> Self {
        match from {
            Some(value) => PodOption(value),
            None => PodOption(T::default()),
        }
    }
}

impl<T: Nullable> From<COption<T>> for PodOption<T> {
    fn from(from: COption<T>) -> Self {
        match from {
            COption::Some(value) => PodOption(value),
            COption::None => PodOption(T::default()),
        }
    }
}

/// Implementation of `Nullable` for `Pubkey`.
///
/// The implementation assumes that the default value of `Pubkey` represents
/// the `None` value.
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
        let some_pubkey = PodOption::from(sysvar::ID);
        assert_eq!(some_pubkey.get(), Some(sysvar::ID));

        let none_pubkey = PodOption::from(Pubkey::default());
        assert_eq!(none_pubkey.get(), None);

        let mut data = Vec::with_capacity(64);
        data.extend_from_slice(sysvar::ID.as_ref());
        data.extend_from_slice(&[0u8; 32]);

        let values = pod_slice_from_bytes::<PodOption<Pubkey>>(&data).unwrap();
        assert_eq!(values[0], PodOption::from(sysvar::ID));
        assert_eq!(values[1], PodOption::from(Pubkey::default()));

        let option_pubkey = Some(sysvar::ID);
        let pod_option_pubkey: PodOption<Pubkey> = option_pubkey.into();
        assert_eq!(pod_option_pubkey, PodOption::from(sysvar::ID));
        assert_eq!(pod_option_pubkey, PodOption::from(option_pubkey));

        let coption_pubkey = COption::Some(sysvar::ID);
        let pod_option_pubkey: PodOption<Pubkey> = coption_pubkey.into();
        assert_eq!(pod_option_pubkey, PodOption::from(sysvar::ID));
        assert_eq!(pod_option_pubkey, PodOption::from(coption_pubkey));
    }
}
