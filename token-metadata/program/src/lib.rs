//! A Token Metadata program for the Solana blockchain.

pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
mod utils;
// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("meta75ZHbozdG3sYzM6PdN7PNK6w9PgsAEEjVYKoAKr");
