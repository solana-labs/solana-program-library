//! Slashing program
#![deny(missing_docs)]

mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

// Export current SDK types for downstream users building with a different SDK
// version
pub use solana_program;

solana_program::declare_id!("8sT74BE7sanh4iT84EyVUL8b77cVruLHXGjvTyJ4GwCe");
