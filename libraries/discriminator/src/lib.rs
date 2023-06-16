//! Crate defining a discriminator type, which creates a set of bytes
//! meant to be unique for instructions or struct types

#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

extern crate self as spl_discriminator;

/// Exports the discriminator module
pub mod discriminator;

// Export for downstream
pub use {
    discriminator::{Discriminator, SplDiscriminator},
    spl_discriminator_derive::SplDiscriminator,
    spl_discriminator_syn::*,
};

#[cfg(test)]
mod tests {
    use {super::*, crate::discriminator::Discriminator, solana_program::hash};

    #[allow(dead_code)]
    #[derive(SplDiscriminator)]
    #[discriminator_hash_input("some_discriminator_hash_input")]
    pub struct MyInstruction1 {
        arg1: String,
        arg2: u8,
    }

    #[allow(dead_code)]
    #[derive(SplDiscriminator)]
    #[discriminator_hash_input("yet_another_discriminator_hash_input")]
    pub struct MyInstruction2 {
        arg1: u64,
    }

    #[allow(dead_code)]
    #[derive(SplDiscriminator)]
    #[discriminator_hash_input("global:my_instruction_3")]
    pub enum MyInstruction3 {
        One,
        Two,
        Three,
    }

    fn assert_discriminator<T: spl_discriminator::discriminator::SplDiscriminator>(
        hash_input: &str,
    ) {
        let preimage = hash::hashv(&[hash_input.as_bytes()]);
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&preimage.to_bytes()[..8]);
        let discriminator = Discriminator::new(bytes);
        assert_eq!(T::SPL_DISCRIMINATOR, discriminator);
        assert_eq!(T::SPL_DISCRIMINATOR_SLICE, discriminator.as_slice());
    }

    #[test]
    fn test_compiles() {
        assert_discriminator::<MyInstruction1>("some_discriminator_hash_input");
        assert_discriminator::<MyInstruction2>("yet_another_discriminator_hash_input");
        assert_discriminator::<MyInstruction3>("global:my_instruction_3");
    }
}
