#![deny(missing_docs)]
//! Governance VoterWeight Addin program

pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
//pub mod state;
// pub mod tools;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;
