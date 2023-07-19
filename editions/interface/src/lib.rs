//! Crate defining an interface for token-editions

#![allow(clippy::integer_arithmetic)]
#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

pub mod error;
pub mod instruction;
pub mod state;

// Export current sdk types for downstream users building with a different sdk
// version
// Export borsh for downstream users
pub use {borsh, solana_program};

/// Namespace for all programs implementing token-editions
pub const NAMESPACE: &str = "spl_token_editions_interface";
