//! A Token Fraction program for the Solana blockchain.

pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
mod utils;
// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("vauLTA73sFPqA8whreUbBsbn3SLJH2vhrW9fP5dmfdC");
