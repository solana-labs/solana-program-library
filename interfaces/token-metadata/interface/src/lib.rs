//! Crate defining an interface for token-metadata

#![allow(clippy::integer_arithmetic)]
#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

pub mod error;
pub mod instruction;
pub mod state;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;
// Export borsh for downstream users
pub use borsh;

/// Namespace for all programs implementing token-metadata
pub const NAMESPACE: &str = "spl_token_metadata_interface";
