//! Instruction types

#![allow(clippy::too_many_arguments)]

use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use std::mem::size_of;

/// fee rate as a ratio
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub Fee {
    /// denominator of the fee ratio
    pub denominator: u64,
    /// numerator of the fee ratio
    pub numerator: u64,
}

pub Init {
    fee: Fee,
    recv_nonce: u8,
    withdraw_none: u8,
}

/// Instructions supported by the SwapInfo program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum StakePoolInstruction {
    ///   Initializes a new StakePool.
    ///
    ///   0. `[w, s]` New StakePool to create.
    ///   1. `[]` Owner
    ///   2. `[]` pool token Mint. Must be non zero, owned by $authority.
    Initialize(Init),

    ///   Deposit some stake into the pool.  The output is a "pool" token representing ownership
    ///   into the pool. Inputs are converted to the current ratio.
    ///
    ///   0. `[]` StakePool
    ///   1. `[]` receive $authority
    ///   2. `[]` withdraw  $authority
    ///   3. `[w]` Stake, receive $authority is set as the withdrawal key
    ///   4. `[w]` Pool MINT account, $authority is the owner.
    ///   5. `[w]` Pool Account to deposit the generated tokens, user is the owner.
    Deposit,

    ///   Withdraw the token from the pool at the current ratio.
    ///   The amount withdrawn is the MIN(u64, stake size)
    ///   
    ///   0. `[]` StakePool
    ///   1. `[]` withdraw  $authority
    ///   2. `[w]` SOURCE Pool account, amount is transferable by $authority
    ///   3. `[w]` Pool MINT account, $authority is the owner
    ///   4. `[w]` Stake SOURCE owned by the withdraw $authority  
    ///   5. `[w]` Stake destination, uninitialized, for owner fees
    ///   6. `[w]` Stake destination, uninitialized, for the user stake
    ///   userdata: amount to withdraw
    Withdraw(u64),

    ///   Update the staking pubkey for a stake
    ///
    ///   0. `[w]` StakePool
    ///   1. `[s]` Owner
    ///   2. `[]` withdraw $authority
    ///   3. '[]` Staking pubkey.
    ///   4. `[w]` Stake to update the staking pubkey
    UpdateStakingAuthority,

    ///   Update owner
    ///
    ///   0. `[w]` StakePool
    ///   1. `[s]` Owner
    ///   2. '[]` New owner pubkey.
    UpdateOwner,

}

impl SwapInstruction {
    /// Deserializes a byte buffer into an [SwapInstruction](enum.SwapInstruction.html).
    pub fn deserialize(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(match input[0] {
            0 => {
                let fee: &Fee = unpack(input)?;
                Self::Initialize(*fee)
            }
            1 => {
                Self::Deposit
            }
            2 => {
                Self::Withdraw
            }
            2 => {
                Self::UpdateStakingAuthority
            }
            _ => return Err(ProgramError::InvalidAccountData),
        })
    }

    /// Serializes an [SwapInstruction](enum.SwapInstruction.html) into a byte buffer.
    pub fn serialize(self: &Self) -> Result<Vec<u8>, ProgramError> {
        let mut output = vec![0u8; size_of::<SwapInstruction>()];
        match self {
            Self::Initialize(fees) => {
                output[0] = 0;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut Fee) };
                *value = *fees;
            }
            Self::Deposit => {
                output[0] = 1;
            }
            Self::Withdraw => {
                output[0] = 2;
            }
            Self::UpdateStaking => {
                output[0] = 3;
            }
        }
        Ok(output)
    }
}

/// Creates an 'initialize' instruction.
pub fn initialize(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    swap_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    token_a_pubkey: &Pubkey,
    token_b_pubkey: &Pubkey,
    pool_pubkey: &Pubkey,
    user_output_pubkey: &Pubkey,
    fee: Fee,
) -> Result<Instruction, ProgramError> {
    let data = SwapInstruction::Initialize(fee).serialize()?;

    let accounts = vec![
        AccountMeta::new(*swap_pubkey, true),
        AccountMeta::new(*authority_pubkey, false),
        AccountMeta::new(*token_a_pubkey, false),
        AccountMeta::new(*token_b_pubkey, false),
        AccountMeta::new(*pool_pubkey, false),
        AccountMeta::new(*user_output_pubkey, false),
        AccountMeta::new(*token_program_id, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Unpacks a reference from a bytes buffer.
pub fn unpack<T>(input: &[u8]) -> Result<&T, ProgramError> {
    if input.len() < size_of::<u8>() + size_of::<T>() {
        return Err(ProgramError::InvalidAccountData);
    }
    #[allow(clippy::cast_ptr_alignment)]
    let val: &T = unsafe { &*(&input[1] as *const u8 as *const T) };
    Ok(val)
}
