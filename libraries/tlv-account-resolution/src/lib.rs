//! Crate defining a state interface for offchain account resolution. If a
//! program writes the proper state information into one of their accounts, any
//! offchain and onchain client can fetch any additional required accounts for
//! an instruction.

#![allow(clippy::arithmetic_side_effects)]
#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

pub mod account;
pub mod error;
pub mod pubkey_data;
pub mod seeds;
pub mod state;

// Export current sdk types for downstream users building with a different sdk
// version
pub use solana_program;
