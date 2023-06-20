//! The traits and types used to create a discriminator for a type

use {
    bytemuck::{Pod, Zeroable},
    solana_program::{hash, program_error::ProgramError},
};

/// A trait for managing 8-byte discriminators in a slab of bytes
pub trait HasDiscriminator {
    /// The 8-byte discriminator as a `[u8; 8]`
    const SPL_DISCRIMINATOR: Discriminator;
    /// The 8-byte discriminator as a slice (`&[u8]`)
    const SPL_DISCRIMINATOR_SLICE: &'static [u8] = Self::SPL_DISCRIMINATOR.as_slice();
}

/// Discriminator type
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct Discriminator([u8; Discriminator::LENGTH]);
impl Discriminator {
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
    /// Creates a new `Discriminator` from some hash input string literal
    pub fn new_with_hash_input(hash_input: &str) -> Self {
        let hash_bytes = hash::hashv(&[hash_input.as_bytes()]).to_bytes();
        let mut discriminator_bytes = [0u8; 8];
        discriminator_bytes.copy_from_slice(&hash_bytes[..8]);
        Self(discriminator_bytes)
    }
}
impl AsRef<[u8]> for Discriminator {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}
impl AsRef<[u8; Discriminator::LENGTH]> for Discriminator {
    fn as_ref(&self) -> &[u8; Discriminator::LENGTH] {
        &self.0
    }
}
impl From<u64> for Discriminator {
    fn from(from: u64) -> Self {
        Self(from.to_le_bytes())
    }
}
impl From<[u8; Self::LENGTH]> for Discriminator {
    fn from(from: [u8; Self::LENGTH]) -> Self {
        Self(from)
    }
}
impl TryFrom<&[u8]> for Discriminator {
    type Error = ProgramError;
    fn try_from(a: &[u8]) -> Result<Self, Self::Error> {
        <[u8; Self::LENGTH]>::try_from(a)
            .map(Self::from)
            .map_err(|_| ProgramError::InvalidAccountData)
    }
}
