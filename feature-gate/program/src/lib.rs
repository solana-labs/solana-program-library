//! Feature Gate program

#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;

// Export current SDK types for downstream users building with a different SDK
// version
pub use solana_program;

solana_program::declare_id!("Feature111111111111111111111111111111111111");
