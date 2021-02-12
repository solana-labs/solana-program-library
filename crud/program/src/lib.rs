//! CRUD program
#![deny(missing_docs)]

mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

// Export current SDK types for downstream users building with a different SDK version
pub use solana_program;

solana_program::declare_id!("CRUD3SXMKeggKN2FZePp8WLK2RpQvhp1LANGf6a5MLdu");
