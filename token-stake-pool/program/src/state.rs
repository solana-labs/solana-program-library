//! State transition types

use crate::instruction::Fee;
use solana_sdk::pubkey::Pubkey;

/// Initialized program details.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StakePool {
    /// Owner authority
    /// allows for updating the staking authority
    pub owner: Pubkey,
    /// Deposit authority
    /// derived from `create_program_address(&[state::StakePool account, "deposit"])`
    pub deposit: Pubkey,
    /// Withdrawal authority
    /// derived from `create_program_address(&[state::StakePool account, "withdrawal"])`
    pub withdrawal: Pubkey,
    /// Pool Mint 
    pub pool_mint: Pubkey,
    /// Fee applied to withdrawals
    pub fee: Fee,
}

/// Program states.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Unallocated state, may be initialized into another state.
    Unallocated,
    /// Initialized state.
    Init(StakePool),
}
