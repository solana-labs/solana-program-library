#![allow(clippy::integer_arithmetic)]
#![deny(missing_docs)]

//! A program for creating and managing pools of stake delegated to a single validator

pub mod error;
pub mod instruction;
pub mod pda;
pub mod processor;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;
use spl_stake_pool::state::Fee;

/// The SOL deposit fee required for all program-managed pools, 5 basis points
pub const SOL_DEPOSIT_FEE: Fee = Fee {
    numerator: 5,
    denominator: 10_000,
};

solana_program::declare_id!("SVPoo3ud9JDXTNXUAo7o8NgwkDVgcfnwRMoS8r6oP6G");
