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
pub struct Fee {
    /// denominator of the fee ratio
    pub denominator: u64,
    /// numerator of the fee ratio
    pub numerator: u64,
}

/// Instructions supported by the SwapInfo program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum SwapInstruction {
    ///   Initializes a new SwapInfo.
    ///
    ///   0. `[writable, signer]` New Token-swap to create.
    ///   1. `[]` $authority derived from `create_program_address(&[Token-swap account])`
    ///   2. `[]` token_a Account. Must be non zero, owned by $authority.
    ///   3. `[]` token_b Account. Must be non zero, owned by $authority.
    ///   4. `[writable]` pool Token. Must be empty, owned by $authority.
    ///   5. `[writable]` Pool Account to deposit the generated tokens, user is the owner.
    ///   6. '[]` Token program id
    ///   userdata: fee rate as a ratio
    Initialize(Fee),

    ///   Swap the tokens in the pool.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` $authority
    ///   2. `[writable]` token_(A|B) SOURCE Account, amount is transferable by $authority,
    ///   4. `[writable]` token_(A|B) Base Account to swap INTO.  Must be the SOURCE token.
    ///   5. `[writable]` token_(A|B) Base Account to swap FROM.  Must be the DEST token.
    ///   6. `[writable]` token_(A|B) DEST Account assigned to USER as the owner.
    ///   7. '[]` Token program id
    ///   userdata: SOURCE amount to transfer, output to DEST is based on the exchange rate
    Swap(u64),

    ///   Deposit some tokens into the pool.  The output is a "pool" token representing ownership
    ///   into the pool. Inputs are converted to the current ratio.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` $authority
    ///   2. `[writable]` token_a $authority can transfer amount,
    ///   4. `[writable]` token_b $authority can transfer amount,
    ///   6. `[writable]` token_a Base Account to deposit into.
    ///   7. `[writable]` token_b Base Account to deposit into.
    ///   8. `[writable]` Pool MINT account, $authority is the owner.
    ///   9. `[writable]` Pool Account to deposit the generated tokens, user is the owner.
    ///   10. '[]` Token program id
    ///   userdata: token_a amount to transfer.  token_b amount is set by the current exchange rate.
    Deposit(u64),

    ///   Withdraw the token from the pool at the current ratio.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` $authority
    ///   2. `[writable]` SOURCE Pool account, amount is transferable by $authority.
    ///   5. `[writable]` token_a Account to withdraw FROM.
    ///   6. `[writable]` token_b Account to withdraw FROM.
    ///   7. `[writable]` token_a user Account.
    ///   8. `[writable]` token_b user Account.
    ///   9. '[]` Token program id
    ///   userdata: SOURCE amount of pool tokens to transfer. User receives an output based on the
    ///   percentage of the pool tokens that are returned.
    Withdraw(u64),
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
                let fee: &u64 = unpack(input)?;
                Self::Swap(*fee)
            }
            2 => {
                let fee: &u64 = unpack(input)?;
                Self::Deposit(*fee)
            }
            3 => {
                let fee: &u64 = unpack(input)?;
                Self::Withdraw(*fee)
            }
            _ => return Err(ProgramError::InvalidAccountData),
        })
    }

    /// Serializes an [SwapInstruction](enum.SwapInstruction.html) into a byte buffer.
    pub fn serialize(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output = vec![0u8; size_of::<SwapInstruction>()];
        match self {
            Self::Initialize(fees) => {
                output[0] = 0;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut Fee) };
                *value = *fees;
            }
            Self::Swap(amount) => {
                output[0] = 1;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
            Self::Deposit(amount) => {
                output[0] = 2;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
            Self::Withdraw(amount) => {
                output[0] = 3;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
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
