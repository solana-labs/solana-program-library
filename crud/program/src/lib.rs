//! CRUD program
#![deny(missing_docs)]

mod entrypoint;
pub mod instruction;
pub mod processor;
pub mod state;

// Export current SDK types for downstream users building with a different SDK version
pub use solana_program;
use solana_program::{program_pack::Pack, pubkey::Pubkey};

solana_program::declare_id!("Crud11111111111111111111111");
