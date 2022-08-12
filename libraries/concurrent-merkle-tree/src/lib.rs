//! # Concurrent Merkle Tree
//! 
//! This crate is a Solana-optimized implementation of the
//! concurrent merkle tree data structure introduced in [this
//! whitepaper](https://drive.google.com/file/d/1BOpa5OFmara50fTvL0VIVYjtg-qzHCVc/view)
//!
//! The core implementation of CMT can be found in [merkle_roll]

/// Descriptive errors
pub mod error;
/// Private macros to enable logging in the Solana runtime
#[macro_use]
pub mod log;
/// Core implementation of the concurrent merkle tree structure
pub mod merkle_roll;
/// Structs to support concurrent merkle tree operations
pub mod state;
/// Hashing utils to support concurrent merkle tree operations
pub mod utils;
