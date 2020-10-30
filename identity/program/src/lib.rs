#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! An Identity program for Solana

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

#[cfg(not(feature = "exclude_entrypoint"))]
mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("1dent1ty11111111111111111111111111111111111");
