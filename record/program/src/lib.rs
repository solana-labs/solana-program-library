//! Record program
#![deny(missing_docs)]

mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

// Export current SDK types for downstream users building with a different SDK version
pub use solana_program;

solana_program::declare_id!("ReciQBw6sQKH9TVVJQDnbnJ5W7FP539tPHjZhRF4E9r");
