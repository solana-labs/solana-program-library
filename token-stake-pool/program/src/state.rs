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
    /// Deposit authority nonce
    /// for `create_program_address(&[state::StakePool account, "deposit"])`
    pub deposit_nonce: u8,
    /// Withdrawal authority nonce
    /// for `create_program_address(&[state::StakePool account, "withdrawal"])`
    pub withdraw_nonce: u8,
    /// Pool Mint 
    pub pool_mint: Pubkey,
    /// Owner fee account
    pub owner_fee_account: Pubkey,
    /// total stake under management
    pub stake_total: u64,
    /// total pool
    pub pool_total: u64,
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
