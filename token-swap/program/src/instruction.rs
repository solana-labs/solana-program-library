//! Instruction types

#![allow(clippy::too_many_arguments)]

use crate::curve::base::SwapCurve;
use crate::error::SwapError;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};
use std::convert::TryInto;
use std::mem::size_of;

/// Instructions supported by the SwapInfo program.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum SwapInstruction {
    ///   Initializes a new SwapInfo.
    ///
    ///   0. `[writable, signer]` New Token-swap to create.
    ///   1. `[]` $authority derived from `create_program_address(&[Token-swap account])`
    ///   2. `[]` token_a Account. Must be non zero, owned by $authority.
    ///   3. `[]` token_b Account. Must be non zero, owned by $authority.
    ///   4. `[writable]` Pool Token Mint. Must be empty, owned by $authority.
    ///   5. `[]` Pool Token Account to deposit trading and withdraw fees.
    ///   Must be empty, not owned by $authority
    ///   6. `[writable]` Pool Token Account to deposit the initial pool token
    ///   supply.  Must be empty, not owned by $authority.
    ///   7. '[]` Token program id
    Initialize {
        /// nonce used to create valid program address
        nonce: u8,
        /// swap curve info for pool, including CurveType, fees, and anything
        /// else that may be required
        swap_curve: SwapCurve,
    },

    ///   Swap the tokens in the pool.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` $authority
    ///   2. `[writable]` token_(A|B) SOURCE Account, amount is transferable by $authority,
    ///   3. `[writable]` token_(A|B) Base Account to swap INTO.  Must be the SOURCE token.
    ///   4. `[writable]` token_(A|B) Base Account to swap FROM.  Must be the DESTINATION token.
    ///   5. `[writable]` token_(A|B) DESTINATION Account assigned to USER as the owner.
    ///   6. `[writable]` Pool token mint, to generate trading fees
    ///   7. `[writable]` Fee account, to receive trading fees
    ///   8. '[]` Token program id
    ///   9. `[optional, writable]` Host fee account to receive additional trading fees
    Swap {
        /// SOURCE amount to transfer, output to DESTINATION is based on the exchange rate
        amount_in: u64,
        /// Minimum amount of DESTINATION token to output, prevents excessive slippage
        minimum_amount_out: u64,
    },

    ///   Deposit some tokens into the pool.  The output is a "pool" token representing ownership
    ///   into the pool. Inputs are converted to the current ratio.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` $authority
    ///   2. `[writable]` token_a $authority can transfer amount,
    ///   3. `[writable]` token_b $authority can transfer amount,
    ///   4. `[writable]` token_a Base Account to deposit into.
    ///   5. `[writable]` token_b Base Account to deposit into.
    ///   6. `[writable]` Pool MINT account, $authority is the owner.
    ///   7. `[writable]` Pool Account to deposit the generated tokens, user is the owner.
    ///   8. '[]` Token program id
    Deposit {
        /// Pool token amount to transfer. token_a and token_b amount are set by
        /// the current exchange rate and size of the pool
        pool_token_amount: u64,
        /// Maximum token A amount to deposit, prevents excessive slippage
        maximum_token_a_amount: u64,
        /// Maximum token B amount to deposit, prevents excessive slippage
        maximum_token_b_amount: u64,
    },

    ///   Withdraw the token from the pool at the current ratio.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` $authority
    ///   2. `[writable]` Pool mint account, $authority is the owner
    ///   3. `[writable]` SOURCE Pool account, amount is transferable by $authority.
    ///   4. `[writable]` token_a Swap Account to withdraw FROM.
    ///   5. `[writable]` token_b Swap Account to withdraw FROM.
    ///   6. `[writable]` token_a user Account to credit.
    ///   7. `[writable]` token_b user Account to credit.
    ///   8. `[writable]` Fee account, to receive withdrawal fees
    ///   9. '[]` Token program id
    Withdraw {
        /// Amount of pool tokens to burn. User receives an output of token a
        /// and b based on the percentage of the pool tokens that are returned.
        pool_token_amount: u64,
        /// Minimum amount of token A to receive, prevents excessive slippage
        minimum_token_a_amount: u64,
        /// Minimum amount of token B to receive, prevents excessive slippage
        minimum_token_b_amount: u64,
    },
}

impl SwapInstruction {
    /// Unpacks a byte buffer into a [SwapInstruction](enum.SwapInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input.split_first().ok_or(SwapError::InvalidInstruction)?;
        Ok(match tag {
            0 => {
                let (&nonce, rest) = rest.split_first().ok_or(SwapError::InvalidInstruction)?;
                let swap_curve = SwapCurve::unpack_unchecked(rest)?;
                Self::Initialize { nonce, swap_curve }
            }
            1 => {
                let (amount_in, rest) = Self::unpack_u64(rest)?;
                let (minimum_amount_out, _rest) = Self::unpack_u64(rest)?;
                Self::Swap {
                    amount_in,
                    minimum_amount_out,
                }
            }
            2 => {
                let (pool_token_amount, rest) = Self::unpack_u64(rest)?;
                let (maximum_token_a_amount, rest) = Self::unpack_u64(rest)?;
                let (maximum_token_b_amount, _rest) = Self::unpack_u64(rest)?;
                Self::Deposit {
                    pool_token_amount,
                    maximum_token_a_amount,
                    maximum_token_b_amount,
                }
            }
            3 => {
                let (pool_token_amount, rest) = Self::unpack_u64(rest)?;
                let (minimum_token_a_amount, rest) = Self::unpack_u64(rest)?;
                let (minimum_token_b_amount, _rest) = Self::unpack_u64(rest)?;
                Self::Withdraw {
                    pool_token_amount,
                    minimum_token_a_amount,
                    minimum_token_b_amount,
                }
            }
            _ => return Err(SwapError::InvalidInstruction.into()),
        })
    }

    fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
        if input.len() >= 8 {
            let (amount, rest) = input.split_at(8);
            let amount = amount
                .get(..8)
                .and_then(|slice| slice.try_into().ok())
                .map(u64::from_le_bytes)
                .ok_or(SwapError::InvalidInstruction)?;
            Ok((amount, rest))
        } else {
            Err(SwapError::InvalidInstruction.into())
        }
    }

    /// Packs a [SwapInstruction](enum.SwapInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match &*self {
            Self::Initialize { nonce, swap_curve } => {
                buf.push(0);
                buf.push(*nonce);
                let mut swap_curve_slice = [0u8; SwapCurve::LEN];
                Pack::pack_into_slice(swap_curve, &mut swap_curve_slice[..]);
                buf.extend_from_slice(&swap_curve_slice);
            }
            Self::Swap {
                amount_in,
                minimum_amount_out,
            } => {
                buf.push(1);
                buf.extend_from_slice(&amount_in.to_le_bytes());
                buf.extend_from_slice(&minimum_amount_out.to_le_bytes());
            }
            Self::Deposit {
                pool_token_amount,
                maximum_token_a_amount,
                maximum_token_b_amount,
            } => {
                buf.push(2);
                buf.extend_from_slice(&pool_token_amount.to_le_bytes());
                buf.extend_from_slice(&maximum_token_a_amount.to_le_bytes());
                buf.extend_from_slice(&maximum_token_b_amount.to_le_bytes());
            }
            Self::Withdraw {
                pool_token_amount,
                minimum_token_a_amount,
                minimum_token_b_amount,
            } => {
                buf.push(3);
                buf.extend_from_slice(&pool_token_amount.to_le_bytes());
                buf.extend_from_slice(&minimum_token_a_amount.to_le_bytes());
                buf.extend_from_slice(&minimum_token_b_amount.to_le_bytes());
            }
        }
        buf
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
    fee_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    nonce: u8,
    swap_curve: SwapCurve,
) -> Result<Instruction, ProgramError> {
    let init_data = SwapInstruction::Initialize { nonce, swap_curve };
    let data = init_data.pack();

    let accounts = vec![
        AccountMeta::new(*swap_pubkey, true),
        AccountMeta::new_readonly(*authority_pubkey, false),
        AccountMeta::new_readonly(*token_a_pubkey, false),
        AccountMeta::new_readonly(*token_b_pubkey, false),
        AccountMeta::new(*pool_pubkey, false),
        AccountMeta::new_readonly(*fee_pubkey, false),
        AccountMeta::new(*destination_pubkey, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates a 'deposit' instruction.
pub fn deposit(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    swap_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    deposit_token_a_pubkey: &Pubkey,
    deposit_token_b_pubkey: &Pubkey,
    swap_token_a_pubkey: &Pubkey,
    swap_token_b_pubkey: &Pubkey,
    pool_mint_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    pool_token_amount: u64,
    maximum_token_a_amount: u64,
    maximum_token_b_amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = SwapInstruction::Deposit {
        pool_token_amount,
        maximum_token_a_amount,
        maximum_token_b_amount,
    }
    .pack();

    let accounts = vec![
        AccountMeta::new_readonly(*swap_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, false),
        AccountMeta::new(*deposit_token_a_pubkey, false),
        AccountMeta::new(*deposit_token_b_pubkey, false),
        AccountMeta::new(*swap_token_a_pubkey, false),
        AccountMeta::new(*swap_token_b_pubkey, false),
        AccountMeta::new(*pool_mint_pubkey, false),
        AccountMeta::new(*destination_pubkey, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates a 'withdraw' instruction.
pub fn withdraw(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    swap_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    pool_mint_pubkey: &Pubkey,
    fee_account_pubkey: &Pubkey,
    source_pubkey: &Pubkey,
    swap_token_a_pubkey: &Pubkey,
    swap_token_b_pubkey: &Pubkey,
    destination_token_a_pubkey: &Pubkey,
    destination_token_b_pubkey: &Pubkey,
    pool_token_amount: u64,
    minimum_token_a_amount: u64,
    minimum_token_b_amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = SwapInstruction::Withdraw {
        pool_token_amount,
        minimum_token_a_amount,
        minimum_token_b_amount,
    }
    .pack();

    let accounts = vec![
        AccountMeta::new_readonly(*swap_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, false),
        AccountMeta::new(*pool_mint_pubkey, false),
        AccountMeta::new(*source_pubkey, false),
        AccountMeta::new(*swap_token_a_pubkey, false),
        AccountMeta::new(*swap_token_b_pubkey, false),
        AccountMeta::new(*destination_token_a_pubkey, false),
        AccountMeta::new(*destination_token_b_pubkey, false),
        AccountMeta::new(*fee_account_pubkey, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates a 'swap' instruction.
pub fn swap(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    swap_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    source_pubkey: &Pubkey,
    swap_source_pubkey: &Pubkey,
    swap_destination_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    pool_mint_pubkey: &Pubkey,
    pool_fee_pubkey: &Pubkey,
    host_fee_pubkey: Option<&Pubkey>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<Instruction, ProgramError> {
    let data = SwapInstruction::Swap {
        amount_in,
        minimum_amount_out,
    }
    .pack();

    let mut accounts = vec![
        AccountMeta::new_readonly(*swap_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, false),
        AccountMeta::new(*source_pubkey, false),
        AccountMeta::new(*swap_source_pubkey, false),
        AccountMeta::new(*swap_destination_pubkey, false),
        AccountMeta::new(*destination_pubkey, false),
        AccountMeta::new(*pool_mint_pubkey, false),
        AccountMeta::new(*pool_fee_pubkey, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    if let Some(host_fee_pubkey) = host_fee_pubkey {
        accounts.push(AccountMeta::new(*host_fee_pubkey, false));
    }

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Unpacks a reference from a bytes buffer.
/// TODO actually pack / unpack instead of relying on normal memory layout.
pub fn unpack<T>(input: &[u8]) -> Result<&T, ProgramError> {
    if input.len() < size_of::<u8>() + size_of::<T>() {
        return Err(ProgramError::InvalidAccountData);
    }
    #[allow(clippy::cast_ptr_alignment)]
    let val: &T = unsafe { &*(&input[1] as *const u8 as *const T) };
    Ok(val)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::curve::{base::CurveType, flat::FlatCurve};

    #[test]
    fn test_instruction_packing() {
        let trade_fee_numerator: u64 = 1;
        let trade_fee_denominator: u64 = 4;
        let owner_trade_fee_numerator: u64 = 2;
        let owner_trade_fee_denominator: u64 = 5;
        let owner_withdraw_fee_numerator: u64 = 1;
        let owner_withdraw_fee_denominator: u64 = 3;
        let host_fee_numerator: u64 = 5;
        let host_fee_denominator: u64 = 20;
        let nonce: u8 = 255;
        let curve_type = CurveType::Flat;
        let calculator = Box::new(FlatCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        });
        let swap_curve = SwapCurve {
            curve_type,
            calculator,
        };
        let check = SwapInstruction::Initialize { nonce, swap_curve };
        let packed = check.pack();
        let mut expect = vec![];
        expect.push(0u8);
        expect.push(nonce);
        expect.push(curve_type as u8);
        expect.extend_from_slice(&trade_fee_numerator.to_le_bytes());
        expect.extend_from_slice(&trade_fee_denominator.to_le_bytes());
        expect.extend_from_slice(&owner_trade_fee_numerator.to_le_bytes());
        expect.extend_from_slice(&owner_trade_fee_denominator.to_le_bytes());
        expect.extend_from_slice(&owner_withdraw_fee_numerator.to_le_bytes());
        expect.extend_from_slice(&owner_withdraw_fee_denominator.to_le_bytes());
        expect.extend_from_slice(&host_fee_numerator.to_le_bytes());
        expect.extend_from_slice(&host_fee_denominator.to_le_bytes());
        assert_eq!(packed, expect);
        let unpacked = SwapInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let amount_in: u64 = 2;
        let minimum_amount_out: u64 = 10;
        let check = SwapInstruction::Swap {
            amount_in,
            minimum_amount_out,
        };
        let packed = check.pack();
        let mut expect = vec![1];
        expect.extend_from_slice(&amount_in.to_le_bytes());
        expect.extend_from_slice(&minimum_amount_out.to_le_bytes());
        assert_eq!(packed, expect);
        let unpacked = SwapInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let pool_token_amount: u64 = 5;
        let maximum_token_a_amount: u64 = 10;
        let maximum_token_b_amount: u64 = 20;
        let check = SwapInstruction::Deposit {
            pool_token_amount,
            maximum_token_a_amount,
            maximum_token_b_amount,
        };
        let packed = check.pack();
        let mut expect = vec![2];
        expect.extend_from_slice(&pool_token_amount.to_le_bytes());
        expect.extend_from_slice(&maximum_token_a_amount.to_le_bytes());
        expect.extend_from_slice(&maximum_token_b_amount.to_le_bytes());
        assert_eq!(packed, expect);
        let unpacked = SwapInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let pool_token_amount: u64 = 1212438012089;
        let minimum_token_a_amount: u64 = 102198761982612;
        let minimum_token_b_amount: u64 = 2011239855213;
        let check = SwapInstruction::Withdraw {
            pool_token_amount,
            minimum_token_a_amount,
            minimum_token_b_amount,
        };
        let packed = check.pack();
        let mut expect = vec![3];
        expect.extend_from_slice(&pool_token_amount.to_le_bytes());
        expect.extend_from_slice(&minimum_token_a_amount.to_le_bytes());
        expect.extend_from_slice(&minimum_token_b_amount.to_le_bytes());
        assert_eq!(packed, expect);
        let unpacked = SwapInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }
}
