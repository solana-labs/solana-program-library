#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! An implementation of Brave's THEMIS for the Solana blockchain

pub mod entrypoint;
pub mod errors;
pub mod instruction;
//pub mod option;
//pub mod pack;
//pub mod processor;
pub mod state;
pub mod utils;

// Export current solana-sdk types for downstream users who may also be building with a different
// solana-sdk version
pub use solana_sdk;

solana_sdk::declare_id!("Themis1111111111111111111111111111111111111");
