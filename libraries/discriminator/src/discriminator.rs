//! The traits and types used to create a discriminator for a type

use {
    bytemuck::{Pod, Zeroable},
    solana_program::{hash, program_error::ProgramError},
};

/// A trait for managing 8-byte discriminators in a slab of bytes
pub trait SplDiscriminate {
    /// The 8-byte discriminator as a `[u8; 8]`
    const SPL_DISCRIMINATOR: ArrayDiscriminator;
    /// The 8-byte discriminator as a slice (`&[u8]`)
    const SPL_DISCRIMINATOR_SLICE: &'static [u8] = Self::SPL_DISCRIMINATOR.as_slice();
}

/// Array Discriminator type
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct ArrayDiscriminator([u8; ArrayDiscriminator::LENGTH]);
impl ArrayDiscriminator {
    /// Size for discriminator in account data
    pub const LENGTH: usize = 8;
    /// Uninitialized variant of a discriminator
    pub const UNINITIALIZED: Self = Self::new([0; Self::LENGTH]);
    /// Creates a discriminator from an array
    pub const fn new(value: [u8; Self::LENGTH]) -> Self {
        Self(value)
    }
    /// Get the array as a const slice
    pub const fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
    /// Creates a new `ArrayDiscriminator` from some hash input string literal
    pub fn new_with_hash_input(hash_input: &str) -> Self {
        let hash_bytes = hash::hashv(&[hash_input.as_bytes()]).to_bytes();
        let mut discriminator_bytes = [0u8; 8];
        discriminator_bytes.copy_from_slice(&hash_bytes[..8]);
        Self(discriminator_bytes)
    }
}
impl AsRef<[u8]> for ArrayDiscriminator {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}
impl AsRef<[u8; ArrayDiscriminator::LENGTH]> for ArrayDiscriminator {
    fn as_ref(&self) -> &[u8; ArrayDiscriminator::LENGTH] {
        &self.0
    }
}
impl From<u64> for ArrayDiscriminator {
    fn from(from: u64) -> Self {
        Self(from.to_le_bytes())
    }
}
impl From<[u8; Self::LENGTH]> for ArrayDiscriminator {
    fn from(from: [u8; Self::LENGTH]) -> Self {
        Self(from)
    }
}
impl TryFrom<&[u8]> for ArrayDiscriminator {
    type Error = ProgramError;
    fn try_from(a: &[u8]) -> Result<Self, Self::Error> {
        <[u8; Self::LENGTH]>::try_from(a)
            .map(Self::from)
            .map_err(|_| ProgramError::InvalidAccountData)
    }
}
impl From<ArrayDiscriminator> for [u8; 8] {
    fn from(from: ArrayDiscriminator) -> Self {
        from.0
    }
}
impl From<ArrayDiscriminator> for u64 {
    fn from(from: ArrayDiscriminator) -> Self {
        u64::from_le_bytes(from.0)
    }
}
