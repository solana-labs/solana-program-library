//! State transition types

use crate::error::Error;
use crate::instruction::{unpack, Fee};
use solana_sdk::{entrypoint::ProgramResult, program_error::ProgramError, pubkey::Pubkey};
use std::mem::size_of;

/// Initialized program details.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StakePool {
    /// Owner authority
    /// allows for updating the staking authority
    pub owner: Pubkey,
    /// Deposit authority bump seed
    /// for `create_program_address(&[state::StakePool account, "deposit"])`
    pub deposit_bump_seed: u8,
    /// Withdrawal authority bump seed
    /// for `create_program_address(&[state::StakePool account, "withdrawal"])`
    pub withdraw_bump_seed: u8,
    /// Pool Mint
    pub pool_mint: Pubkey,
    /// Owner fee account
    pub owner_fee_account: Pubkey,
    /// Pool token program id
    pub token_program_id: Pubkey,
    /// total stake under management
    pub stake_total: u64,
    /// total pool
    pub pool_total: u64,
    /// Fee applied to deposits
    pub fee: Fee,
}
impl StakePool {
    /// calculate the pool tokens that should be minted
    pub fn calc_pool_deposit_amount(&self, stake_lamports: u64) -> Option<u128> {
        if self.stake_total == 0 {
            return Some(stake_lamports as u128);
        }
        self.calc_pool_withdraw_amount(stake_lamports)
    }
    /// calculate the pool tokens that should be withdrawn
    pub fn calc_pool_withdraw_amount(&self, stake_lamports: u64) -> Option<u128> {
        (stake_lamports as u128)
            .checked_mul(self.pool_total as u128)?
            .checked_div(self.stake_total as u128)
    }
    /// calculate the fee in pool tokens that goes to the owner
    pub fn calc_fee_amount(&self, pool_amount: u128) -> Option<u128> {
        if self.fee.denominator == 0 {
            return Some(0);
        }
        pool_amount
            .checked_mul(self.fee.numerator as u128)?
            .checked_div(self.fee.denominator as u128)
    }
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

impl State {
    /// Length of state data when serialized
    pub const LEN: usize = size_of::<u8>() + size_of::<StakePool>();
    /// Deserializes a byte buffer into a [State](struct.State.html).
    /// TODO efficient unpacking here
    pub fn deserialize(input: &[u8]) -> Result<State, ProgramError> {
        if input.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(match input[0] {
            0 => State::Unallocated,
            1 => {
                // We send whole input here, because unpack skips the first byte
                let swap: &StakePool = unpack(&input)?;
                State::Init(*swap)
            }
            _ => return Err(ProgramError::InvalidAccountData),
        })
    }

    /// Serializes [State](struct.State.html) into a byte buffer.
    /// TODO efficient packing here
    pub fn serialize(&self, output: &mut [u8]) -> ProgramResult {
        if output.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        match self {
            Self::Unallocated => output[0] = 0,
            Self::Init(swap) => {
                if output.len() < size_of::<u8>() + size_of::<StakePool>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                output[0] = 1;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut StakePool) };
                *value = *swap;
            }
        }
        Ok(())
    }
    /// Gets the `StakePool` from `State`
    pub fn stake_pool(&self) -> Result<StakePool, ProgramError> {
        if let State::Init(swap) = &self {
            Ok(*swap)
        } else {
            Err(Error::InvalidState.into())
        }
    }
}
