#![deny(missing_docs)]

//! A program for creating pools of Solana stakes managed by a Stake-o-Matic

pub mod error;
pub mod instruction;
pub mod processor;
pub mod stake;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("3T92SNpyV3ipDm1VeR4SZxsUVtRT3sBJMhynJ8hwLc8G");
