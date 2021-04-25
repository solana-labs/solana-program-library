use solana_program::program_error::ProgramError;
use std::convert::TryInto;

use crate::error::PerpetualSwapError;
pub enum PerpetualSwapInstruction {
    /// Accounts expected:
    /// 0. `[w, signer]` New PerpetrualSwap to create.
    /// 1. `[]` swap authority derived from `create_program_address(&[Token-swap account])`
    /// 2. `[]` long margin acount
    /// 3. `[]` long user acount
    /// 4. `[]` short margin acount
    /// 5. `[]` short user acount
    /// 6. `[w]` Pool Token Mint. Must be empty, owned by swap authority.
    /// 7. `[w]` Pool Token Account to deposit trading and withdraw fees.
    /// Must be empty, not owned by swap authority
    /// 8. `[w]` Pool Token Account to deposit the initial pool token
    /// supply.  Must be empty, not owned by swap authority.
    /// 9. '[]` Token program id
    InitializePerpetualSwap {
        nonce: u8,
        funding_rate: f64,
        minimum_margin: u64,
        liquidation_threshold: f64,
    },

    /// Accounts expected:
    /// 0. `[w]` PerpetualSwap
    /// 1. `[]` swap authority
    /// 2. `[]` user transfer authority
    /// 3. `[w, s]` The account of the person depositing to the margin account
    /// 4. `[w]` The margin account
    /// 5. `[]` The token program
    InitializeSide { amount_to_deposit: u64 },

    /// Accounts expected:
    /// 0. `[]` PerpetualSwap
    /// 1. `[]` swap authority
    /// 2. `[]` user transfer authority
    /// 3. `[w, s]` The account of the person depositing to the margin account
    /// 4. `[w]` The margin account
    /// 5. `[]` The token program
    DepositToMargin { amount_to_deposit: u64 },

    /// Accounts expected:
    /// 0. `[]` PerpetualSwap
    /// 1. `[]` swap authority
    /// 2. `[]` user transfer authority
    /// 3. `[w]` The account of the person withrawing from the margin account
    /// 4. `[w, s]` The margin account
    /// 5. `[]` The token program
    WithdrawFromMargin { amount_to_withdraw: u64 },

    /// Accounts expected:
    /// 0. `[]` PerpetualSwap
    /// 1. `[]` swap authority
    /// 2. `[]` user transfer authority
    /// 3. `[w]` The margin account of the long party who is selling
    /// 4. `[w]` The user account of the long party who is selling
    /// 5. `[w]` The account of the party who is buying
    /// 6. `[]` The token program
    TransferLong { amount: u64 },

    /// Accounts expected:
    /// 0. `[w]` PerpetualSwap
    /// 1. `[]` swap authority
    /// 2. `[]` user transfer authority
    /// 3. `[w]` The account of the short party who is buying
    /// 4. `[w]` The account of the party who is selling
    /// 5. `[w]` The new margin account of the party who is selling
    /// 7. `[]` The token program
    TransferShort { amount: u64 },

    /// Accounts expected:
    /// 0. `[]` PerpetualSwap
    /// 1. `[]` swap authority
    /// 2. `[]` user transfer authority
    /// 3. `[w]` The account of the party to be liquidated
    /// 4. `[w]` The account of the counterparty
    /// 5. `[w]` The margin account of the party to be liquidated
    /// 6. `[w]` The insurance fund
    /// 8. `[]` The token program
    TryToLiquidate {},

    /// Accounts expected:
    /// 0. `[w]` PerpetualSwap (w because reference time needs to be updated)
    /// 1. `[]` swap authority
    /// 2. `[]` user transfer authority
    /// 3. `[w]` The account of the party who is long
    /// 4. `[w]` The account of the party who is short
    /// 3. `[]` The token program
    TransferFunds {},

    /// Accounts expected:
    /// 0. `[]` PerpetualSwap
    /// 1. `[]` swap authority
    /// 2. `[]` user transfer authority
    /// 3. `[]` The token program
    UpdateIndexPrice {
        price: f64, // Placeholder instruction, will delete
    },

    /// Accounts expected:
    /// 0. `[]` PerpetualSwap
    /// 1. `[]` swap authority
    /// 2. `[]` user transfer authority
    /// 3. `[]` The token program
    UpdateMarkPrice {
        price: f64, // Placeholder instruction, will delete
    },
}

impl PerpetualSwapInstruction {
    /// Unpacks a byte buffer into a [EscrowInstruction](enum.EscrowInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input
            .split_first()
            .ok_or(PerpetualSwapError::InvalidInstruction)?;

        Ok(match tag {
            0 => {
                let (&nonce, rest) = rest
                    .split_first()
                    .ok_or(PerpetualSwapError::InvalidInstruction)?;
                let (funding_rate, rest) = Self::unpack_f64(rest)?;
                let (minimum_margin, rest) = Self::unpack_u64(rest)?;
                let (liquidation_threshold, _rest) = Self::unpack_f64(rest)?;
                Self::InitializePerpetualSwap {
                    nonce,
                    funding_rate,
                    minimum_margin,
                    liquidation_threshold,
                }
            }
            1 => {
                let (amount_to_deposit, _rest) = Self::unpack_u64(rest)?;
                Self::InitializeSide { amount_to_deposit }
            }
            2 => {
                let (amount_to_deposit, _rest) = Self::unpack_u64(rest)?;
                Self::DepositToMargin { amount_to_deposit }
            }
            3 => {
                let (amount_to_withdraw, _rest) = Self::unpack_u64(rest)?;
                Self::WithdrawFromMargin { amount_to_withdraw }
            }
            4 => Self::TryToLiquidate {},
            5 => Self::TransferFunds {},
            6 => {
                let (price, _rest) = Self::unpack_f64(rest)?;
                Self::UpdateIndexPrice { price }
            }
            7 => {
                let (price, _rest) = Self::unpack_f64(rest)?;
                Self::UpdateMarkPrice { price }
            }
            _ => return Err(PerpetualSwapError::InvalidInstruction.into()),
        })
    }

    fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
        if input.len() >= 8 {
            let (amount, rest) = input.split_at(8);
            let amount = amount
                .get(..8)
                .and_then(|slice| slice.try_into().ok())
                .map(u64::from_le_bytes)
                .ok_or(PerpetualSwapError::InvalidInstruction)?;
            Ok((amount, rest))
        } else {
            Err(PerpetualSwapError::InvalidInstruction.into())
        }
    }

    fn unpack_f64(input: &[u8]) -> Result<(f64, &[u8]), ProgramError> {
        if input.len() >= 8 {
            let (amount, rest) = input.split_at(8);
            let amount = amount
                .get(..8)
                .and_then(|slice| slice.try_into().ok())
                .map(f64::from_le_bytes)
                .ok_or(PerpetualSwapError::InvalidInstruction)?;
            Ok((amount, rest))
        } else {
            Err(PerpetualSwapError::InvalidInstruction.into())
        }
    }
}
