//! Crate defining a discriminator type, which creates a set of bytes
//! meant to be unique for instructions or struct types

#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

extern crate self as spl_discriminator;

/// Exports the discriminator module
pub mod discriminator;

// Export for downstream
pub use {
    discriminator::{ArrayDiscriminator, SplDiscriminate},
    spl_discriminator_derive::SplDiscriminate,
};

#[cfg(test)]
mod tests {
    use {super::*, crate::discriminator::ArrayDiscriminator};

    #[allow(dead_code)]
    #[derive(SplDiscriminate)]
    #[discriminator_hash_input("my_first_instruction")]
    pub struct MyInstruction1 {
        arg1: String,
        arg2: u8,
    }

    #[allow(dead_code)]
    #[derive(SplDiscriminate)]
    #[discriminator_hash_input("global:my_second_instruction")]
    pub enum MyInstruction2 {
        One,
        Two,
        Three,
    }

    #[allow(dead_code)]
    #[derive(SplDiscriminate)]
    #[discriminator_hash_input("my_crate_public_instruction")]
    pub(crate) struct MyInstruction3 {
        arg1: String,
    }

    #[allow(dead_code)]
    #[derive(SplDiscriminate)]
    #[discriminator_hash_input("my_private_instruction")]
    struct MyInstruction4 {
        arg1: u8,
    }

    fn assert_discriminator<T: spl_discriminator::discriminator::SplDiscriminate>(
        hash_input: &str,
    ) {
        let discriminator = build_discriminator(hash_input);
        assert_eq!(
            T::SPL_DISCRIMINATOR,
            discriminator,
            "Discriminator mismatch: case: {}",
            hash_input
        );
        assert_eq!(
            T::SPL_DISCRIMINATOR_SLICE,
            discriminator.as_slice(),
            "Discriminator mismatch: case: {}",
            hash_input
        );
    }

    fn build_discriminator(hash_input: &str) -> ArrayDiscriminator {
        let preimage = solana_program::hash::hashv(&[hash_input.as_bytes()]);
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&preimage.to_bytes()[..8]);
        ArrayDiscriminator::new(bytes)
    }

    #[test]
    fn test_discrminators() {
        let runtime_discrim = ArrayDiscriminator::new_with_hash_input("my_runtime_hash_input");
        assert_eq!(
            runtime_discrim,
            build_discriminator("my_runtime_hash_input"),
        );

        assert_discriminator::<MyInstruction1>("my_first_instruction");
        assert_discriminator::<MyInstruction2>("global:my_second_instruction");
        assert_discriminator::<MyInstruction3>("my_crate_public_instruction");
        assert_discriminator::<MyInstruction4>("my_private_instruction");
    }
}

#[cfg(all(test, feature = "borsh"))]
mod borsh_test {
    use super::*;

    #[test]
    fn borsh_test() {
        let my_discrim = ArrayDiscriminator::new_with_hash_input("my_discrim");
        let mut buffer = [0u8; 8];
        my_discrim.serialize(&mut buffer[..]).unwrap();
        let my_discrim_again = ArrayDiscriminator::try_from_slice(&buffer).unwrap();
        assert_eq!(my_discrim, my_discrim_again);
        assert_eq!(buf, my_discrim.into());
    }
}
