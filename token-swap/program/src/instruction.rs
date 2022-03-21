//! Instruction types

#![allow(clippy::too_many_arguments)]

use crate::curve::{base::SwapCurve, fees::Fees};
use crate::error::SwapError;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar,
};
use std::convert::TryInto;
use std::mem::size_of;

#[cfg(feature = "fuzz")]
use arbitrary::Arbitrary;

/// Initialize instruction data
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Initialize {
    /// nonce used to create valid program address
    pub nonce: u8,
    /// all swap fees
    pub fees: Fees,
    /// swap curve info for pool, including CurveType and anything
    /// else that may be required
    pub swap_curve: SwapCurve,
    /// nonce used to create valid program address for the pool
    pub pool_nonce: u8,
}

/// Swap instruction data
#[cfg_attr(feature = "fuzz", derive(Arbitrary))]
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct Swap {
    /// SOURCE amount to transfer, output to DESTINATION is based on the exchange rate
    pub amount_in: u64,
    /// Minimum amount of DESTINATION token to output, prevents excessive slippage
    pub minimum_amount_out: u64,
    /// Flags defining swap behavior; see SwapFlags
    pub flags: u8,
}

/// DepositAllTokenTypes instruction data
#[cfg_attr(feature = "fuzz", derive(Arbitrary))]
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct DepositAllTokenTypes {
    /// Pool token amount to transfer. token_a and token_b amount are set by
    /// the current exchange rate and size of the pool
    pub pool_token_amount: u64,
    /// Maximum token A amount to deposit, prevents excessive slippage
    pub maximum_token_a_amount: u64,
    /// Maximum token B amount to deposit, prevents excessive slippage
    pub maximum_token_b_amount: u64,
}

/// WithdrawAllTokenTypes instruction data
#[cfg_attr(feature = "fuzz", derive(Arbitrary))]
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct WithdrawAllTokenTypes {
    /// Amount of pool tokens to burn. User receives an output of token a
    /// and b based on the percentage of the pool tokens that are returned.
    pub pool_token_amount: u64,
    /// Minimum amount of token A to receive, prevents excessive slippage
    pub minimum_token_a_amount: u64,
    /// Minimum amount of token B to receive, prevents excessive slippage
    pub minimum_token_b_amount: u64,
}

/// Deposit one token type, exact amount in instruction data
#[cfg_attr(feature = "fuzz", derive(Arbitrary))]
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct DepositSingleTokenTypeExactAmountIn {
    /// Token amount to deposit
    pub source_token_amount: u64,
    /// Pool token amount to receive in exchange. The amount is set by
    /// the current exchange rate and size of the pool
    pub minimum_pool_token_amount: u64,
}

/// WithdrawSingleTokenTypeExactAmountOut instruction data
#[cfg_attr(feature = "fuzz", derive(Arbitrary))]
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct WithdrawSingleTokenTypeExactAmountOut {
    /// Amount of token A or B to receive
    pub destination_token_amount: u64,
    /// Maximum amount of pool tokens to burn. User receives an output of token A
    /// or B based on the percentage of the pool tokens that are returned.
    pub maximum_pool_token_amount: u64,
}

/// DeregisterPool instruction data
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct DeregisterPool {
    /// The pubkey of the pool to deregister
    pub pool_index: u64,
}

/// Constants defining the bit flags to use for the Swap flags field
pub mod swap_flags {
    /// sets no flags
    pub const NONE: u8 = 0x00;
    /// sets whether a wsol output will be unwrapped or token account closed (if empty)
    pub const CLOSE_OUTPUT: u8 = 0x01;
    /// sets whether a token account input will be closed (if empty)
    pub const CLOSE_INPUT: u8 = 0x02;
    /// For routed swaps, swap 2 - sets whether a wsol output will be unwrapped or token account closed (if empty)
    pub const CLOSE_OUTPUT_2: u8 = 0x04;
    /// For routed swaps, swap 2 - sets whether a token account input will be closed (if empty)
    pub const CLOSE_INPUT_2: u8 = 0x08;
    /// default value produces legacy behavior
    pub fn default() -> u8 {
        CLOSE_OUTPUT | CLOSE_INPUT
    }
    /// default value produces legacy behavior
    pub fn default_routed() -> u8 {
        CLOSE_INPUT | CLOSE_OUTPUT_2 | CLOSE_INPUT_2
    }
}

/// Instructions supported by the token swap program.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum SwapInstruction {
    ///   Initializes a new swap
    ///
    ///   0. `[signer]` Payer for token swap account creation.
    ///   1. `[writable]` New Token-swap to create PDA - [sorted(mintA, mintB), curve].
    ///   2. `[]` swap authority derived from `create_program_address(&[Token-swap account])`
    ///   3. `[]` token_a Account. Must be non zero, owned by swap authority.
    ///   4. `[]` token_b Account. Must be non zero, owned by swap authority.
    ///   5. `[writable]` Pool Token Mint. Must be empty, owned by swap authority.
    ///   6. `[]` Pool Token Account to deposit trading and withdraw fees.
    ///   Must be empty, not owned by swap authority
    ///   7. `[writable]` Pool Token Account to deposit the initial pool token
    ///   supply.  Must be empty, not owned by swap authority.
    ///   8. '[]` Token program id
    ///   9. '[writable]` Pool registry
    ///   10. '[]` System Program
    ///   11. '[]` Rent
    Initialize(Initialize),

    ///   Swap the tokens in the pool.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` swap authority
    ///   2. `[]` user transfer authority
    ///   3. `[writable]` token_(A|B) SOURCE Account, amount is transferable by user transfer authority,
    ///   4. `[writable]` token_(A|B) Base Account to swap INTO.  Must be the SOURCE token.
    ///   5. `[writable]` token_(A|B) Base Account to swap FROM.  Must be the DESTINATION token.
    ///   6. `[writable]` token_(A|B) DESTINATION Account assigned to USER as the owner.
    ///   7. `[writable]` Pool token mint, to generate trading fees
    ///   8. `[writable]` Pool fee account
    ///   9. `[writable]` refund account to unwrap WSOL to
    ///   10. '[]` Token program id
    Swap(Swap),

    ///   Deposit both types of tokens into the pool.  The output is a "pool"
    ///   token representing ownership in the pool. Inputs are converted to
    ///   the current ratio.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` swap authority
    ///   2. `[]` user transfer authority
    ///   3. `[writable]` token_a user transfer authority can transfer amount,
    ///   4. `[writable]` token_b user transfer authority can transfer amount,
    ///   5. `[writable]` token_a Base Account to deposit into.
    ///   6. `[writable]` token_b Base Account to deposit into.
    ///   7. `[writable]` Pool MINT account, swap authority is the owner.
    ///   8. `[writable]` Pool Account to deposit the generated tokens, user is the owner.
    ///   9. '[]` Token program id
    DepositAllTokenTypes(DepositAllTokenTypes),

    ///   Withdraw both types of tokens from the pool at the current ratio, given
    ///   pool tokens.  The pool tokens are burned in exchange for an equivalent
    ///   amount of token A and B.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` swap authority
    ///   2. `[]` user transfer authority
    ///   3. `[writable]` Pool mint account, swap authority is the owner
    ///   4. `[writable]` SOURCE Pool account, amount is transferable by user transfer authority.
    ///   5. `[writable]` token_a Swap Account to withdraw FROM.
    ///   6. `[writable]` token_b Swap Account to withdraw FROM.
    ///   7. `[writable]` token_a user Account to credit.
    ///   8. `[writable]` token_b user Account to credit.
    ///   9. `[writable]` Fee account, to receive withdrawal fees
    ///   10 '[]` Token program id
    WithdrawAllTokenTypes(WithdrawAllTokenTypes),

    ///   Deposit one type of tokens into the pool.  The output is a "pool" token
    ///   representing ownership into the pool. Input token is converted as if
    ///   a swap and deposit all token types were performed.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` swap authority
    ///   2. `[]` user transfer authority
    ///   3. `[writable]` token_(A|B) SOURCE Account, amount is transferable by user transfer authority,
    ///   4. `[writable]` token_a Swap Account, may deposit INTO.
    ///   5. `[writable]` token_b Swap Account, may deposit INTO.
    ///   6. `[writable]` Pool MINT account, swap authority is the owner.
    ///   7. `[writable]` Pool Account to deposit the generated tokens, user is the owner.
    ///   8. '[]` Token program id
    DepositSingleTokenTypeExactAmountIn(DepositSingleTokenTypeExactAmountIn),

    ///   Withdraw one token type from the pool at the current ratio given the
    ///   exact amount out expected.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` swap authority
    ///   2. `[]` user transfer authority
    ///   3. `[writable]` Pool mint account, swap authority is the owner
    ///   4. `[writable]` SOURCE Pool account, amount is transferable by user transfer authority.
    ///   5. `[writable]` token_a Swap Account to potentially withdraw from.
    ///   6. `[writable]` token_b Swap Account to potentially withdraw from.
    ///   7. `[writable]` token_(A|B) User Account to credit
    ///   8. `[writable]` Fee account, to receive withdrawal fees
    ///   9. '[]` Token program id
    WithdrawSingleTokenTypeExactAmountOut(WithdrawSingleTokenTypeExactAmountOut),

    ///   Initializes the pool registry
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of deployer.
    /// 1. `[writable]` The pool registry account.
    InitializeRegistry(),

    ///   Swap across two pools.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` swap authority
    ///   2. `[]` user transfer authority
    ///   3. `[writable]` token_(A|B) SOURCE Account, amount is transferable by user transfer authority,
    ///   4. `[writable]` token_(A|B) Base Account to swap INTO.  Must be the SOURCE token.
    ///   5. `[writable]` token_(A|B) Base Account to swap FROM.  Must be the MIDDLE token.
    ///   6. `[writable]` token_(A|B) MIDDLE Account assigned to USER as the owner.
    ///   7. `[writable]` Pool token mint, to generate trading fees
    ///   8. `[]` Swap 1 fee account
    ///   9. '[]` Token program id
    ///
    ///   10. `[]` Token-swap 2
    ///   11. `[]` swap authority 2
    ///   12. `[writable]` token_(A|B) Base Account to swap INTO.  Must be the MIDDLE token.
    ///   13. `[writable]` token_(A|B) Base Account to swap FROM.  Must be the DESTINATION token.
    ///   14. `[writable]` token_(A|B) DESTINATION Account assigned to USER as the owner.
    ///   15. `[writable]` Pool token mint, to generate trading fees
    ///   16. `[]` Swap 2 fee account
    ///   17. `[writable]` refund account to unwrap WSOL to
    RoutedSwap(Swap),

    ///   Deregisters a pool from the pool registry
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of deployer.
    /// 1. `[writable]` The pool registry account.
    DeregisterPool(DeregisterPool),

    ///   Repairs a token swap whose fee account is closed
    ///
    /// Accounts expected:
    ///
    /// 0. `[writable]` The token swap account to repair.
    /// 1. `[]` The old fee account this must be closed.
    /// 2. `[]` The new fee account. This must be the ATA for the mint and the owner fee account.
    RepairClosedFeeAccount(),
}

impl SwapInstruction {
    /// Unpacks a byte buffer into a [SwapInstruction](enum.SwapInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input.split_first().ok_or(SwapError::InvalidInstruction)?;
        Ok(match tag {
            0 => {
                let (&nonce, rest) = rest.split_first().ok_or(SwapError::InvalidInstruction)?;
                if rest.len() >= Fees::LEN {
                    let (fees, rest) = rest.split_at(Fees::LEN);
                    let fees = Fees::unpack_unchecked(fees)?;
                    let (curve, rest) = rest.split_at(SwapCurve::LEN);
                    let swap_curve = SwapCurve::unpack_unchecked(curve)?;
                    let pool_nonce = rest[0];
                    Self::Initialize(Initialize {
                        nonce,
                        fees,
                        swap_curve,
                        pool_nonce,
                    })
                } else {
                    return Err(SwapError::InvalidInstruction.into());
                }
            }
            1 => {
                let (amount_in, rest) = Self::unpack_u64(rest)?;
                let (minimum_amount_out, rest) = Self::unpack_u64(rest)?;
                let flags = *rest.first().unwrap_or(&swap_flags::default());
                Self::Swap(Swap {
                    amount_in,
                    minimum_amount_out,
                    flags,
                })
            }
            2 => {
                let (pool_token_amount, rest) = Self::unpack_u64(rest)?;
                let (maximum_token_a_amount, rest) = Self::unpack_u64(rest)?;
                let (maximum_token_b_amount, _rest) = Self::unpack_u64(rest)?;
                Self::DepositAllTokenTypes(DepositAllTokenTypes {
                    pool_token_amount,
                    maximum_token_a_amount,
                    maximum_token_b_amount,
                })
            }
            3 => {
                let (pool_token_amount, rest) = Self::unpack_u64(rest)?;
                let (minimum_token_a_amount, rest) = Self::unpack_u64(rest)?;
                let (minimum_token_b_amount, _rest) = Self::unpack_u64(rest)?;
                Self::WithdrawAllTokenTypes(WithdrawAllTokenTypes {
                    pool_token_amount,
                    minimum_token_a_amount,
                    minimum_token_b_amount,
                })
            }
            4 => {
                let (source_token_amount, rest) = Self::unpack_u64(rest)?;
                let (minimum_pool_token_amount, _rest) = Self::unpack_u64(rest)?;
                Self::DepositSingleTokenTypeExactAmountIn(DepositSingleTokenTypeExactAmountIn {
                    source_token_amount,
                    minimum_pool_token_amount,
                })
            }
            5 => {
                let (destination_token_amount, rest) = Self::unpack_u64(rest)?;
                let (maximum_pool_token_amount, _rest) = Self::unpack_u64(rest)?;
                Self::WithdrawSingleTokenTypeExactAmountOut(WithdrawSingleTokenTypeExactAmountOut {
                    destination_token_amount,
                    maximum_pool_token_amount,
                })
            }
            6 => Self::InitializeRegistry {},
            7 => {
                let (amount_in, rest) = Self::unpack_u64(rest)?;
                let (minimum_amount_out, rest) = Self::unpack_u64(rest)?;
                let flags = *rest.first().unwrap_or(&swap_flags::default_routed());
                Self::RoutedSwap(Swap {
                    amount_in,
                    minimum_amount_out,
                    flags,
                })
            }
            8 => {
                let (pool_index, _rest) = Self::unpack_u64(rest)?;
                Self::DeregisterPool(DeregisterPool { pool_index })
            }
            9 => Self::RepairClosedFeeAccount {},
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
            Self::Initialize(Initialize {
                nonce,
                fees,
                swap_curve,
                pool_nonce,
            }) => {
                buf.push(0);
                buf.push(*nonce);
                let mut fees_slice = [0u8; Fees::LEN];
                Pack::pack_into_slice(fees, &mut fees_slice[..]);
                buf.extend_from_slice(&fees_slice);
                let mut swap_curve_slice = [0u8; SwapCurve::LEN];
                Pack::pack_into_slice(swap_curve, &mut swap_curve_slice[..]);
                buf.extend_from_slice(&swap_curve_slice);
                buf.push(*pool_nonce);
            }
            Self::Swap(Swap {
                amount_in,
                minimum_amount_out,
                flags,
            }) => {
                buf.push(1);
                buf.extend_from_slice(&amount_in.to_le_bytes());
                buf.extend_from_slice(&minimum_amount_out.to_le_bytes());
                buf.push(*flags);
            }
            Self::DepositAllTokenTypes(DepositAllTokenTypes {
                pool_token_amount,
                maximum_token_a_amount,
                maximum_token_b_amount,
            }) => {
                buf.push(2);
                buf.extend_from_slice(&pool_token_amount.to_le_bytes());
                buf.extend_from_slice(&maximum_token_a_amount.to_le_bytes());
                buf.extend_from_slice(&maximum_token_b_amount.to_le_bytes());
            }
            Self::WithdrawAllTokenTypes(WithdrawAllTokenTypes {
                pool_token_amount,
                minimum_token_a_amount,
                minimum_token_b_amount,
            }) => {
                buf.push(3);
                buf.extend_from_slice(&pool_token_amount.to_le_bytes());
                buf.extend_from_slice(&minimum_token_a_amount.to_le_bytes());
                buf.extend_from_slice(&minimum_token_b_amount.to_le_bytes());
            }
            Self::DepositSingleTokenTypeExactAmountIn(DepositSingleTokenTypeExactAmountIn {
                source_token_amount,
                minimum_pool_token_amount,
            }) => {
                buf.push(4);
                buf.extend_from_slice(&source_token_amount.to_le_bytes());
                buf.extend_from_slice(&minimum_pool_token_amount.to_le_bytes());
            }
            Self::WithdrawSingleTokenTypeExactAmountOut(
                WithdrawSingleTokenTypeExactAmountOut {
                    destination_token_amount,
                    maximum_pool_token_amount,
                },
            ) => {
                buf.push(5);
                buf.extend_from_slice(&destination_token_amount.to_le_bytes());
                buf.extend_from_slice(&maximum_pool_token_amount.to_le_bytes());
            }
            Self::InitializeRegistry() => {
                buf.push(6);
            }
            Self::RoutedSwap(Swap {
                amount_in,
                minimum_amount_out,
                flags,
            }) => {
                buf.push(7);
                buf.extend_from_slice(&amount_in.to_le_bytes());
                buf.extend_from_slice(&minimum_amount_out.to_le_bytes());
                buf.push(*flags);
            }
            Self::DeregisterPool(DeregisterPool { pool_index }) => {
                buf.push(8);
                buf.extend_from_slice(&pool_index.to_le_bytes());
            }
            Self::RepairClosedFeeAccount() => {
                buf.push(9);
            }
        }
        buf
    }
}

/// Creates an 'initialize' instruction.
pub fn initialize(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    payer_pubkey: &Pubkey,
    swap_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    token_a_pubkey: &Pubkey,
    token_b_pubkey: &Pubkey,
    pool_pubkey: &Pubkey,
    fee_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    nonce: u8,
    fees: Fees,
    swap_curve: SwapCurve,
    pool_registry_pubkey: &Pubkey,
    pool_nonce: u8,
) -> Result<Instruction, ProgramError> {
    let init_data = SwapInstruction::Initialize(Initialize {
        nonce,
        fees,
        swap_curve,
        pool_nonce,
    });
    let data = init_data.pack();

    let accounts = vec![
        AccountMeta::new(*payer_pubkey, true),
        AccountMeta::new(*swap_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, false),
        AccountMeta::new_readonly(*token_a_pubkey, false),
        AccountMeta::new_readonly(*token_b_pubkey, false),
        AccountMeta::new(*pool_pubkey, false),
        AccountMeta::new_readonly(*fee_pubkey, false),
        AccountMeta::new(*destination_pubkey, false),
        AccountMeta::new_readonly(*token_program_id, false),
        AccountMeta::new(*pool_registry_pubkey, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates a 'deposit_all_token_types' instruction.
pub fn deposit_all_token_types(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    swap_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    user_transfer_authority_pubkey: &Pubkey,
    deposit_token_a_pubkey: &Pubkey,
    deposit_token_b_pubkey: &Pubkey,
    swap_token_a_pubkey: &Pubkey,
    swap_token_b_pubkey: &Pubkey,
    pool_mint_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    instruction: DepositAllTokenTypes,
) -> Result<Instruction, ProgramError> {
    let data = SwapInstruction::DepositAllTokenTypes(instruction).pack();

    let accounts = vec![
        AccountMeta::new_readonly(*swap_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, false),
        AccountMeta::new_readonly(*user_transfer_authority_pubkey, true),
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

/// Creates a 'withdraw_all_token_types' instruction.
pub fn withdraw_all_token_types(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    swap_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    user_transfer_authority_pubkey: &Pubkey,
    pool_mint_pubkey: &Pubkey,
    fee_account_pubkey: &Pubkey,
    source_pubkey: &Pubkey,
    swap_token_a_pubkey: &Pubkey,
    swap_token_b_pubkey: &Pubkey,
    destination_token_a_pubkey: &Pubkey,
    destination_token_b_pubkey: &Pubkey,
    instruction: WithdrawAllTokenTypes,
) -> Result<Instruction, ProgramError> {
    let data = SwapInstruction::WithdrawAllTokenTypes(instruction).pack();

    let accounts = vec![
        AccountMeta::new_readonly(*swap_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, false),
        AccountMeta::new_readonly(*user_transfer_authority_pubkey, true),
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

/// Creates a 'deposit_single_token_type_exact_amount_in' instruction.
pub fn deposit_single_token_type_exact_amount_in(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    swap_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    user_transfer_authority_pubkey: &Pubkey,
    source_token_pubkey: &Pubkey,
    swap_token_a_pubkey: &Pubkey,
    swap_token_b_pubkey: &Pubkey,
    pool_mint_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    instruction: DepositSingleTokenTypeExactAmountIn,
) -> Result<Instruction, ProgramError> {
    let data = SwapInstruction::DepositSingleTokenTypeExactAmountIn(instruction).pack();

    let accounts = vec![
        AccountMeta::new_readonly(*swap_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, false),
        AccountMeta::new_readonly(*user_transfer_authority_pubkey, true),
        AccountMeta::new(*source_token_pubkey, false),
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

/// Creates a 'withdraw_single_token_type_exact_amount_out' instruction.
pub fn withdraw_single_token_type_exact_amount_out(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    swap_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    user_transfer_authority_pubkey: &Pubkey,
    pool_mint_pubkey: &Pubkey,
    fee_account_pubkey: &Pubkey,
    pool_token_source_pubkey: &Pubkey,
    swap_token_a_pubkey: &Pubkey,
    swap_token_b_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    instruction: WithdrawSingleTokenTypeExactAmountOut,
) -> Result<Instruction, ProgramError> {
    let data = SwapInstruction::WithdrawSingleTokenTypeExactAmountOut(instruction).pack();

    let accounts = vec![
        AccountMeta::new_readonly(*swap_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, false),
        AccountMeta::new_readonly(*user_transfer_authority_pubkey, true),
        AccountMeta::new(*pool_mint_pubkey, false),
        AccountMeta::new(*pool_token_source_pubkey, false),
        AccountMeta::new(*swap_token_a_pubkey, false),
        AccountMeta::new(*swap_token_b_pubkey, false),
        AccountMeta::new(*destination_pubkey, false),
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
    user_transfer_authority_pubkey: &Pubkey,
    source_pubkey: &Pubkey,
    swap_source_pubkey: &Pubkey,
    swap_destination_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    pool_mint_pubkey: &Pubkey,
    pool_fee_pubkey: &Pubkey,
    host_fee_pubkey: Option<&Pubkey>,
    //for unwrapping sol
    refund_pubkey: &Pubkey,
    instruction: Swap,
) -> Result<Instruction, ProgramError> {
    let data = SwapInstruction::Swap(instruction).pack();

    let mut accounts = vec![
        AccountMeta::new_readonly(*swap_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, false),
        AccountMeta::new_readonly(*user_transfer_authority_pubkey, true),
        AccountMeta::new(*source_pubkey, false),
        AccountMeta::new(*swap_source_pubkey, false),
        AccountMeta::new(*swap_destination_pubkey, false),
        AccountMeta::new(*destination_pubkey, false),
        AccountMeta::new(*pool_mint_pubkey, false),
        AccountMeta::new(*pool_fee_pubkey, false),
        AccountMeta::new(*refund_pubkey, false),
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

/// Creates a 'routedswap' instruction.
pub fn routed_swap(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    //swap 1
    swap_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    user_transfer_authority_pubkey: &Pubkey,
    source_pubkey: &Pubkey,
    swap_source_pubkey: &Pubkey,
    swap_destination_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    pool_mint_pubkey: &Pubkey,
    pool_fee_pubkey: &Pubkey,
    //swap 2
    swap_pubkey2: &Pubkey,
    authority_pubkey2: &Pubkey,
    swap_source_pubkey2: &Pubkey,
    swap_destination_pubkey2: &Pubkey,
    destination_pubkey2: &Pubkey,
    pool_mint_pubkey2: &Pubkey,
    pool_fee_pubkey2: &Pubkey,
    //for unwrap and cleanup
    refund_pubkey: &Pubkey,

    instruction: Swap,
) -> Result<Instruction, ProgramError> {
    let data = SwapInstruction::RoutedSwap(instruction).pack();

    let accounts = vec![
        AccountMeta::new_readonly(*swap_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, false),
        AccountMeta::new_readonly(*user_transfer_authority_pubkey, true),
        AccountMeta::new(*source_pubkey, false),
        AccountMeta::new(*swap_source_pubkey, false),
        AccountMeta::new(*swap_destination_pubkey, false),
        AccountMeta::new(*destination_pubkey, false),
        AccountMeta::new(*pool_mint_pubkey, false),
        AccountMeta::new(*pool_fee_pubkey, false),
        AccountMeta::new_readonly(*token_program_id, false),
        AccountMeta::new_readonly(*swap_pubkey2, false),
        AccountMeta::new_readonly(*authority_pubkey2, false),
        AccountMeta::new(*swap_source_pubkey2, false),
        AccountMeta::new(*swap_destination_pubkey2, false),
        AccountMeta::new(*destination_pubkey2, false),
        AccountMeta::new(*pool_mint_pubkey2, false),
        AccountMeta::new(*pool_fee_pubkey2, false),
        AccountMeta::new(*refund_pubkey, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates an 'initialize_registry' instruction.
pub fn initialize_registry(
    program_id: &Pubkey,
    payer: &Pubkey,
    pool_registry_pubkey: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let init_data = SwapInstruction::InitializeRegistry();
    let data = init_data.pack();

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*pool_registry_pubkey, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates an 'deregister_pool' instruction.
pub fn deregister_pool(
    program_id: &Pubkey,
    payer: &Pubkey,
    pool_registry_pubkey: &Pubkey,
    pool_index: u64,
) -> Result<Instruction, ProgramError> {
    let init_data = SwapInstruction::DeregisterPool(DeregisterPool { pool_index });
    let data = init_data.pack();

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*pool_registry_pubkey, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates an 'repair_closed_fee_account' instruction.
pub fn repair_closed_fee_account(
    program_id: &Pubkey,
    token_swap: &Pubkey,
    old_fee_account: &Pubkey,
    new_fee_account: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let init_data = SwapInstruction::RepairClosedFeeAccount();
    let data = init_data.pack();

    let accounts = vec![
        AccountMeta::new(*token_swap, false),
        AccountMeta::new(*old_fee_account, false),
        AccountMeta::new(*new_fee_account, false),
    ];

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
    use crate::curve::{base::CurveType, stable::StableCurve};
    use std::sync::Arc;

    #[test]
    fn pack_initialize() {
        let trade_fee_numerator: u64 = 1;
        let trade_fee_denominator: u64 = 4;
        let owner_trade_fee_numerator: u64 = 2;
        let owner_trade_fee_denominator: u64 = 5;
        let owner_withdraw_fee_numerator: u64 = 1;
        let owner_withdraw_fee_denominator: u64 = 3;
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };
        let nonce: u8 = 255;
        let amp: u64 = 1;
        let curve_type = CurveType::Stable;
        let calculator = Arc::new(StableCurve { amp });
        let swap_curve = SwapCurve {
            curve_type,
            calculator,
        };
        let pool_nonce: u8 = 250;
        let check = SwapInstruction::Initialize(Initialize {
            nonce,
            fees,
            swap_curve,
            pool_nonce,
        });
        let packed = check.pack();
        let mut expect = vec![0u8, nonce];
        expect.extend_from_slice(&trade_fee_numerator.to_le_bytes());
        expect.extend_from_slice(&trade_fee_denominator.to_le_bytes());
        expect.extend_from_slice(&owner_trade_fee_numerator.to_le_bytes());
        expect.extend_from_slice(&owner_trade_fee_denominator.to_le_bytes());
        expect.extend_from_slice(&owner_withdraw_fee_numerator.to_le_bytes());
        expect.extend_from_slice(&owner_withdraw_fee_denominator.to_le_bytes());
        expect.push(curve_type as u8);
        expect.extend_from_slice(&amp.to_le_bytes());
        expect.extend_from_slice(&[0u8; 24]);
        expect.push(pool_nonce as u8);
        assert_eq!(packed, expect);
        let unpacked = SwapInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }

    #[test]
    fn pack_swap() {
        let amount_in: u64 = 2;
        let minimum_amount_out: u64 = 10;
        let check = SwapInstruction::Swap(Swap {
            amount_in,
            minimum_amount_out,
            flags: swap_flags::default(),
        });
        let packed = check.pack();
        let mut expect = vec![1];
        expect.extend_from_slice(&amount_in.to_le_bytes());
        expect.extend_from_slice(&minimum_amount_out.to_le_bytes());
        expect.extend_from_slice(&[swap_flags::default()]);
        assert_eq!(packed, expect);
        let unpacked = SwapInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }

    #[test]
    fn pack_deposit() {
        let pool_token_amount: u64 = 5;
        let maximum_token_a_amount: u64 = 10;
        let maximum_token_b_amount: u64 = 20;
        let check = SwapInstruction::DepositAllTokenTypes(DepositAllTokenTypes {
            pool_token_amount,
            maximum_token_a_amount,
            maximum_token_b_amount,
        });
        let packed = check.pack();
        let mut expect = vec![2];
        expect.extend_from_slice(&pool_token_amount.to_le_bytes());
        expect.extend_from_slice(&maximum_token_a_amount.to_le_bytes());
        expect.extend_from_slice(&maximum_token_b_amount.to_le_bytes());
        assert_eq!(packed, expect);
        let unpacked = SwapInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }

    #[test]
    fn pack_withdraw() {
        let pool_token_amount: u64 = 1212438012089;
        let minimum_token_a_amount: u64 = 102198761982612;
        let minimum_token_b_amount: u64 = 2011239855213;
        let check = SwapInstruction::WithdrawAllTokenTypes(WithdrawAllTokenTypes {
            pool_token_amount,
            minimum_token_a_amount,
            minimum_token_b_amount,
        });
        let packed = check.pack();
        let mut expect = vec![3];
        expect.extend_from_slice(&pool_token_amount.to_le_bytes());
        expect.extend_from_slice(&minimum_token_a_amount.to_le_bytes());
        expect.extend_from_slice(&minimum_token_b_amount.to_le_bytes());
        assert_eq!(packed, expect);
        let unpacked = SwapInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }

    #[test]
    fn pack_deposit_one_exact_in() {
        let source_token_amount: u64 = 10;
        let minimum_pool_token_amount: u64 = 5;
        let check = SwapInstruction::DepositSingleTokenTypeExactAmountIn(
            DepositSingleTokenTypeExactAmountIn {
                source_token_amount,
                minimum_pool_token_amount,
            },
        );
        let packed = check.pack();
        let mut expect = vec![4];
        expect.extend_from_slice(&source_token_amount.to_le_bytes());
        expect.extend_from_slice(&minimum_pool_token_amount.to_le_bytes());
        assert_eq!(packed, expect);
        let unpacked = SwapInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }

    #[test]
    fn pack_withdraw_one_exact_out() {
        let destination_token_amount: u64 = 102198761982612;
        let maximum_pool_token_amount: u64 = 1212438012089;
        let check = SwapInstruction::WithdrawSingleTokenTypeExactAmountOut(
            WithdrawSingleTokenTypeExactAmountOut {
                destination_token_amount,
                maximum_pool_token_amount,
            },
        );
        let packed = check.pack();
        let mut expect = vec![5];
        expect.extend_from_slice(&destination_token_amount.to_le_bytes());
        expect.extend_from_slice(&maximum_pool_token_amount.to_le_bytes());
        assert_eq!(packed, expect);
        let unpacked = SwapInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }
}
