//! Various constraints as required for production environments

#[cfg(feature = "production")]
use std::env;

/// Encodes fee constraints, used in multihost environments where the program
/// may be used by multiple frontends, to ensure that proper fees are being
/// assessed.
pub struct FeeConstraints<'a> {
    /// Owner of the program
    pub owner_key: &'a str,
    /// Fee numerator
    pub trade_fee_numerator: u64,
    /// Fee denominator
    pub trade_fee_denominator: u64,
    /// Owner trade fee numerator
    pub owner_trade_fee_numerator: u64,
    /// Owner trade fee denominator
    pub owner_trade_fee_denominator: u64,
    /// Host fee numerator (e.g. 20 / 100 for host to receive 20% of owner trade fees)
    pub host_fee_numerator: u64,
    /// Host fee denominator
    pub host_fee_denominator: u64,
}

#[cfg(feature = "production")]
const OWNER_KEY: &'static str = env!("SWAP_PROGRAM_OWNER_FEE_ADDRESS");

/// Fee structure defined by program creator in order to enforce certain
/// fees when others use the program.  Adds checks on pool creation and
/// swapping to ensure the correct fees and account owners are passed.
pub const FEE_CONSTRAINTS: Option<FeeConstraints> = {
    #[cfg(feature = "production")]
    {
        Some(FeeConstraints {
            owner_key: OWNER_KEY,
            trade_fee_numerator: 25,
            trade_fee_denominator: 10000,
            owner_trade_fee_numerator: 5,
            owner_trade_fee_denominator: 10000,
            host_fee_numerator: 20,
            host_fee_denominator: 100,
        })
    }
    #[cfg(not(feature = "production"))]
    {
        None
    }
};
