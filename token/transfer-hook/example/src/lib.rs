//! Crate defining an example program for performing a hook on transfer, where
//! the token program calls into a separate program with additional accounts
//! after all other logic, to be sure that a transfer has accomplished all
//! required preconditions.

#![allow(clippy::arithmetic_side_effects)]
#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

// Export current sdk types for downstream users building with a different sdk
// version
pub use solana_program;
