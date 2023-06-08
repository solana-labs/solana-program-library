//! Discriminator for differentiating account types, the "Type" in the
//! Type-Length-Value structure. Since the word "type" is reserved in Rust,
//! we use the term "Discriminator" and "Type" interchangeably.

use {
    bytemuck::{Pod, Zeroable},
    solana_program::program_error::ProgramError,
};

/// Trait to be implemented by all value types in the TLV structure, specifying
/// just the discriminator
pub trait TlvDiscriminator {
    /// Associated value type discriminator, checked at the start of TLV entries
    const TLV_DISCRIMINATOR: Discriminator;
}

/// Discriminator used as the type in the TLV structure
/// Type in TLV structure
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
