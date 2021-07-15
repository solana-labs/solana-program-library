#![deny(missing_docs)]

//! Heap Solana program

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("Heap6FfdWMT2bcoQQ9hN4F2syu7qhRHzNuCPPQqV12hs");
