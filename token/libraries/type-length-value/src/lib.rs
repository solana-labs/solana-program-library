//! Crate defining an interface for managing type-length-value entries in a slab
//! of bytes, to be used with Solana accounts.

#![allow(clippy::integer_arithmetic)]
#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

pub mod discriminator;
pub mod error;
pub mod length;
pub mod pod;
pub mod state;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;
