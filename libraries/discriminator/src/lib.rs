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
    #[discriminator_hash_input("global:my_instruction_with_lifetime")]
    pub struct MyInstruction3<'a> {
        data: &'a [u8],
    }

    #[allow(dead_code)]
    #[derive(SplDiscriminate)]
    #[discriminator_hash_input("global:my_instruction_with_one_generic")]
    pub struct MyInstruction4<T> {
        data: T,
    }

    #[allow(dead_code)]
    #[derive(SplDiscriminate)]
    #[discriminator_hash_input("global:my_instruction_with_one_generic_and_lifetime")]
    pub struct MyInstruction5<'b, T> {
        data: &'b [T],
    }

    #[allow(dead_code)]
    #[derive(SplDiscriminate)]
    #[discriminator_hash_input("global:my_instruction_with_multiple_generics_and_lifetime")]
    pub struct MyInstruction6<'c, U, V> {
        data1: &'c [U],
        data2: &'c [V],
    }

    #[allow(dead_code)]
    #[derive(SplDiscriminate)]
    #[discriminator_hash_input(
        "global:my_instruction_with_multiple_generics_and_lifetime_and_where"
    )]
    pub struct MyInstruction7<'c, U, V>
    where
        U: Clone + Copy,
        V: Clone + Copy,
    {
        data1: &'c [U],
        data2: &'c [V],
    }

    fn assert_discriminator<T: spl_discriminator::discriminator::SplDiscriminate>(
        hash_input: &str,
        case_num: u8,
    ) {
        let discriminator = build_discriminator(hash_input);
        assert_eq!(
            T::SPL_DISCRIMINATOR,
            discriminator,
            "Discriminator mismatch: case: {}",
            case_num
        );
        assert_eq!(
            T::SPL_DISCRIMINATOR_SLICE,
            discriminator.as_slice(),
            "Discriminator mismatch: case: {}",
            case_num
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

        assert_discriminator::<MyInstruction1>("my_first_instruction", 1);
        assert_discriminator::<MyInstruction2>("global:my_second_instruction", 2);
        assert_discriminator::<MyInstruction3<'_>>("global:my_instruction_with_lifetime", 3);
        assert_discriminator::<MyInstruction4<u8>>("global:my_instruction_with_one_generic", 4);
        assert_discriminator::<MyInstruction5<'_, u8>>(
            "global:my_instruction_with_one_generic_and_lifetime",
            5,
        );
        assert_discriminator::<MyInstruction6<'_, u8, u8>>(
            "global:my_instruction_with_multiple_generics_and_lifetime",
            6,
        );
        assert_discriminator::<MyInstruction7<'_, u8, u8>>(
            "global:my_instruction_with_multiple_generics_and_lifetime_and_where",
            7,
        );
    }
}
