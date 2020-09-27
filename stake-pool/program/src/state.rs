//! State transition types

use crate::error::Error;
use crate::instruction::{unpack, Fee};
use solana_sdk::{
    account_info::next_account_info, account_info::AccountInfo, decode_error::DecodeError,
    entrypoint::ProgramResult, program_error::PrintProgramError, program_error::ProgramError,
    pubkey::Pubkey,
};
use std::mem::size_of;

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
    /// Fee applied to deposits
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

impl State {
    /// Deserializes a byte buffer into a [State](struct.State.html).
    pub fn deserialize(input: &[u8]) -> Result<State, ProgramError> {
        if input.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(match input[0] {
            0 => State::Unallocated,
            1 => {
                let swap: &StakePool = unpack(input)?;
                State::Init(*swap)
            }
            _ => return Err(ProgramError::InvalidAccountData),
        })
    }

    /// Serializes [State](struct.State.html) into a byte buffer.
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
