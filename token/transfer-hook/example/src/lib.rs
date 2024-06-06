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

/// Place the mint id that you want to target with your transfer hook program.
/// Any other mint will fail to initialize, protecting the transfer hook program
/// from rogue mints trying to get access to accounts.
///
/// There are many situations where it's reasonable to support multiple mints
/// with one transfer-hook program, but because it's easy to make something
/// unsafe, this simple example implementation only allows for one mint.
#[cfg(feature = "forbid-additional-mints")]
pub mod mint {
    solana_program::declare_id!("Mint111111111111111111111111111111111111111");
}
