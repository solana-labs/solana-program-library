//! Crate defining an interface for managing type-length-value entries in a slab
//! of bytes, to be used with Solana accounts.

#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

extern crate self as spl_discriminator;

/// Exports the discriminator module
pub mod discriminator;

// Export for downstream
pub use discriminator::{Discriminator, SplDiscriminator};
pub use spl_discriminator_derive::SplDiscriminator;
pub use spl_discriminator_syn::*;

#[cfg(test)]
mod tests {
    use super::*;

    use crate::discriminator::Discriminator;

    use solana_program::hash;

    #[allow(dead_code)]
    #[derive(SplDiscriminator)]
    #[discriminator_namespace("some_discriminator_namespace")]
    pub struct MyInstruction1 {
        arg1: String,
        arg2: u8,
    }

    #[allow(dead_code)]
    #[derive(SplDiscriminator)]
    #[discriminator_namespace("yet_another_discriminator_namespace")]
    pub struct MyInstruction2 {
        arg1: u64,
    }

    #[allow(dead_code)]
    #[derive(SplDiscriminator)]
    #[discriminator_namespace("global:my_instruction_3")]
    pub enum MyInstruction3 {
        One,
        Two,
        Three,
    }

    fn assert_discriminator<T: spl_discriminator::discriminator::SplDiscriminator>(
        namespace: &str,
    ) {
        let preimage = hash::hashv(&[namespace.as_bytes()]);
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&preimage.to_bytes()[..8]);
        let discriminator = Discriminator::new(bytes);
        assert_eq!(T::SPL_DISCRIMINATOR, discriminator);
        assert_eq!(T::SPL_DISCRIMINATOR_SLICE, discriminator.as_slice());
    }

    #[test]
    fn test_compiles() {
        assert_discriminator::<MyInstruction1>("some_discriminator");
        assert_discriminator::<MyInstruction2>("yet_another_d");
        assert_discriminator::<MyInstruction3>("global:my_instruction_3");
    }
}
