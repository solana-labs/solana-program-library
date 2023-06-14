//! Crate defining an interface for managing type-length-value entries in a slab
//! of bytes, to be used with Solana accounts.

#![allow(clippy::integer_arithmetic)]
#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

extern crate self as spl_type_length_value;

pub mod discriminator;
pub mod error;
pub mod length;
pub mod pod;
pub mod state;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;
pub use spl_type_length_value_derive::SplTlv;
pub use spl_type_length_value_syn::*;

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::hash;
    use spl_type_length_value::discriminator::{Discriminator, TlvDiscriminator};

    #[allow(dead_code)]
    #[derive(SplTlv)]
    #[tlv_namespace("some_particular_program_instruction")]
    pub struct MyInstruction1 {
        arg1: String,
        arg2: u8,
    }

    #[allow(dead_code)]
    #[derive(SplTlv)]
    #[tlv_namespace("yet_another_program_instruction")]
    pub struct MyInstruction2 {
        arg1: u64,
    }

    #[allow(dead_code)]
    #[derive(SplTlv)]
    #[tlv_namespace("token_program_instruction")]
    pub enum MyInstruction3 {
        MintTo,
        Transfer,
    }

    fn assert_discriminators<T: TlvDiscriminator>(
        namespace: &str,
        generated_bytes: [u8; 8],
        generated_slice: &[u8],
    ) {
        let preimage = hash::hashv(&[namespace.as_bytes()]);
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&preimage.to_bytes()[..8]);
        let discriminator = Discriminator::new(bytes);

        assert_eq!(T::TLV_DISCRIMINATOR, discriminator);
        assert_eq!(generated_bytes, bytes);
        assert_eq!(generated_slice, &bytes);
    }

    #[test]
    fn test_compiles() {
        assert_discriminators::<MyInstruction1>(
            "some_particular_program_instruction",
            MY_INSTRUCTION_1_DISCRIMINATOR,
            MY_INSTRUCTION_1_DISCRIMINATOR_SLICE,
        );
        assert_discriminators::<MyInstruction2>(
            "yet_another_program_instruction",
            MY_INSTRUCTION_2_DISCRIMINATOR,
            MY_INSTRUCTION_2_DISCRIMINATOR_SLICE,
        );
        assert_discriminators::<MyInstruction3>(
            "token_program_instruction",
            MY_INSTRUCTION_3_DISCRIMINATOR,
            MY_INSTRUCTION_3_DISCRIMINATOR_SLICE,
        );
    }
}
