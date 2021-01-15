//! Instruction types

use crate::{
    error::LendingError,
    state::{ReserveConfig, ReserveFees},
};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar,
};
use std::{convert::TryInto, mem::size_of};

/// Describe how the borrow input amount should be treated
#[derive(Clone, Copy, Debug, PartialEq, FromPrimitive, ToPrimitive)]
pub enum BorrowAmountType {
    /// Treat amount as amount of liquidity to borrow
    LiquidityBorrowAmount,
    /// Treat amount as amount of collateral tokens to deposit
    CollateralDepositAmount,
}

/// Instructions supported by the lending program.
#[derive(Clone, Debug, PartialEq)]
pub enum LendingInstruction {
    /// Initializes a new lending market.
    ///
    ///   0. `[writable]` Lending market account.
    ///   1. `[]` Quote currency SPL Token mint. Must be initialized.
    ///   2. `[]` Rent sysvar
    ///   3. '[]` Token program id
    InitLendingMarket {
        /// Owner authority which can add new reserves
        market_owner: Pubkey,
    },

    /// Initializes a new lending market reserve.
    ///
    ///   0. `[writable]` Source liquidity token account.  $authority can transfer $liquidity_amount
    ///   1. `[writable]` Destination collateral token account - uninitialized
    ///   2. `[writable]` Reserve account.
    ///   3. `[]` Reserve liquidity SPL Token mint
    ///   4. `[writable]` Reserve liquidity supply SPL Token account - uninitialized
    ///   5. `[writable]` Reserve collateral SPL Token mint - uninitialized
    ///   6. `[writable]` Reserve collateral token supply - uninitialized
    ///   7. `[writable]` Reserve collateral fees receiver - uninitialized.
    ///                     Owner will be set to the lending market account.
    ///   8. `[]` Lending market account.
    ///   9. `[signer]` Lending market owner.
    ///   10 `[]` Derived lending market authority.
    ///   11 `[]` User transfer authority ($authority).
    ///   12 `[]` Clock sysvar
    ///   13 `[]` Rent sysvar
    ///   14 '[]` Token program id
    ///   15 `[optional]` Serum DEX market account. Not required for quote currency reserves. Must be initialized and match quote and base currency.
    InitReserve {
        /// Initial amount of liquidity to deposit into the new reserve
        liquidity_amount: u64,
        /// Reserve configuration values
        config: ReserveConfig,
    },

    /// Deposit liquidity into a reserve. The output is a collateral token representing ownership
    /// of the reserve liquidity pool.
    ///
    ///   0. `[writable]` Source liquidity token account. $authority can transfer $liquidity_amount
    ///   1. `[writable]` Destination collateral token account.
    ///   2. `[writable]` Reserve account.
    ///   3. `[writable]` Reserve liquidity supply SPL Token account.
    ///   4. `[writable]` Reserve collateral SPL Token mint.
    ///   5. `[]` Lending market account.
    ///   6. `[]` Derived lending market authority.
    ///   7. `[]` User transfer authority ($authority).
    ///   8. `[]` Clock sysvar
    ///   9. '[]` Token program id
    DepositReserveLiquidity {
        /// Amount to deposit into the reserve
        liquidity_amount: u64,
    },

    /// Withdraw tokens from a reserve. The input is a collateral token representing ownership
    /// of the reserve liquidity pool.
    ///
    ///   0. `[writable]` Source collateral token account. $authority can transfer $collateral_amount
    ///   1. `[writable]` Destination liquidity token account.
    ///   2. `[writable]` Reserve account.
    ///   3. `[writable]` Reserve collateral SPL Token mint.
    ///   4. `[writable]` Reserve liquidity supply SPL Token account.
    ///   5. `[]` Lending market account.
    ///   6. `[]` Derived lending market authority.
    ///   7. `[]` User transfer authority ($authority).
    ///   8. '[]` Token program id
    WithdrawReserveLiquidity {
        /// Amount of collateral to deposit in exchange for liquidity
        collateral_amount: u64,
    },

    /// Borrow tokens from a reserve by depositing collateral tokens. The number of borrowed tokens
    /// is calculated by market price. The debt obligation is tokenized.
    ///
    ///   0. `[writable]` Source collateral token account, minted by deposit reserve collateral mint,
    ///                     $authority can transfer $collateral_amount
    ///   1. `[writable]` Destination liquidity token account, minted by borrow reserve liquidity mint
    ///   2. `[writable]` Deposit reserve account.
    ///   3. `[writable]` Deposit reserve collateral supply SPL Token account
    ///   4. `[writable]` Deposit reserve collateral fee receiver account.
    ///                     Must be the fee account specified at InitReserve.
    ///   5. `[writable]` Borrow reserve account.
    ///   6. `[writable]` Borrow reserve liquidity supply SPL Token account
    ///   7. `[writable]` Obligation
    ///   8. `[writable]` Obligation token mint
    ///   9. `[writable]` Obligation token output
    ///   10 `[]` Obligation token owner
    ///   11 `[]` Lending market account.
    ///   12 `[]` Derived lending market authority.
    ///   13 `[]` User transfer authority ($authority).
    ///   14 `[]` Dex market
    ///   15 `[]` Dex market order book side
    ///   16 `[]` Temporary memory
    ///   17 `[]` Clock sysvar
    ///   18 `[]` Rent sysvar
    ///   19 '[]` Token program id
    ///   20 `[optional, writable]` Deposit reserve collateral host fee receiver account.
    BorrowReserveLiquidity {
        // TODO: slippage constraint
        /// Amount whose usage depends on `amount_type`
        amount: u64,
        /// Describe how the amount should be treated
        amount_type: BorrowAmountType,
    },

    /// Repay loaned tokens to a reserve and receive collateral tokens. The obligation balance
    /// will be recalculated for interest.
    ///
    ///   0. `[writable]` Source liquidity token account, minted by repay reserve liquidity mint
    ///                     $authority can transfer $collateral_amount
    ///   1. `[writable]` Destination collateral token account, minted by withdraw reserve collateral mint
    ///   2. `[writable]` Repay reserve account.
    ///   3. `[writable]` Repay reserve liquidity supply SPL Token account
    ///   4. `[]` Withdraw reserve account.
    ///   5. `[writable]` Withdraw reserve collateral supply SPL Token account
    ///   6. `[writable]` Obligation - initialized
    ///   7. `[writable]` Obligation token mint
    ///   8. `[writable]` Obligation token input, $authority can transfer calculated amount
    ///   9. `[]` Lending market account.
    ///   10 `[]` Derived lending market authority.
    ///   11 `[]` User transfer authority ($authority).
    ///   12 `[]` Clock sysvar
    ///   13 `[]` Token program id
    RepayReserveLiquidity {
        /// Amount of loan to repay
        liquidity_amount: u64,
    },

    /// Purchase collateral tokens at a discount rate if the chosen obligation is unhealthy.
    ///
    ///   0. `[writable]` Source liquidity token account, minted by repay reserve liquidity mint
    ///                     $authority can transfer $collateral_amount
    ///   1. `[writable]` Destination collateral token account, minted by withdraw reserve collateral mint
    ///   2. `[writable]` Repay reserve account.
    ///   3. `[writable]` Repay reserve liquidity supply SPL Token account
    ///   4. `[writable]` Withdraw reserve account.
    ///   5. `[writable]` Withdraw reserve collateral supply SPL Token account
    ///   6. `[writable]` Obligation - initialized
    ///   7. `[]` Lending market account.
    ///   8. `[]` Derived lending market authority.
    ///   9. `[]` User transfer authority ($authority).
    ///   10 `[]` Dex market
    ///   11 `[]` Dex market order book side
    ///   12 `[]` Temporary memory
    ///   13 `[]` Clock sysvar
    ///   14 `[]` Token program id
    LiquidateObligation {
        /// Amount of loan to repay
        liquidity_amount: u64,
    },
}

impl LendingInstruction {
    /// Unpacks a byte buffer into a [LendingInstruction](enum.LendingInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input
            .split_first()
            .ok_or(LendingError::InstructionUnpackError)?;
        Ok(match tag {
            0 => {
                let (market_owner, _rest) = Self::unpack_pubkey(rest)?;
                Self::InitLendingMarket { market_owner }
            }
            1 => {
                let (liquidity_amount, rest) = Self::unpack_u64(rest)?;
                let (optimal_utilization_rate, rest) = Self::unpack_u8(rest)?;
                let (loan_to_value_ratio, rest) = Self::unpack_u8(rest)?;
                let (liquidation_bonus, rest) = Self::unpack_u8(rest)?;
                let (liquidation_threshold, rest) = Self::unpack_u8(rest)?;
                let (min_borrow_rate, rest) = Self::unpack_u8(rest)?;
                let (optimal_borrow_rate, rest) = Self::unpack_u8(rest)?;
                let (max_borrow_rate, rest) = Self::unpack_u8(rest)?;
                let (borrow_fee_wad, rest) = Self::unpack_u64(rest)?;
                let (host_fee_percentage, _rest) = Self::unpack_u8(rest)?;
                Self::InitReserve {
                    liquidity_amount,
                    config: ReserveConfig {
                        optimal_utilization_rate,
                        loan_to_value_ratio,
                        liquidation_bonus,
                        liquidation_threshold,
                        min_borrow_rate,
                        optimal_borrow_rate,
                        max_borrow_rate,
                        fees: ReserveFees {
                            borrow_fee_wad,
                            host_fee_percentage,
                        },
                    },
                }
            }
            2 => {
                let (liquidity_amount, _rest) = Self::unpack_u64(rest)?;
                Self::DepositReserveLiquidity { liquidity_amount }
            }
            3 => {
                let (collateral_amount, _rest) = Self::unpack_u64(rest)?;
                Self::WithdrawReserveLiquidity { collateral_amount }
            }
            4 => {
                let (amount, rest) = Self::unpack_u64(rest)?;
                let (amount_type, _rest) = Self::unpack_u8(rest)?;
                let amount_type = BorrowAmountType::from_u8(amount_type)
                    .ok_or(LendingError::InstructionUnpackError)?;
                Self::BorrowReserveLiquidity {
                    amount,
                    amount_type,
                }
            }
            5 => {
                let (liquidity_amount, _rest) = Self::unpack_u64(rest)?;
                Self::RepayReserveLiquidity { liquidity_amount }
            }
            6 => {
                let (liquidity_amount, _rest) = Self::unpack_u64(rest)?;
                Self::LiquidateObligation { liquidity_amount }
            }
            _ => return Err(LendingError::InstructionUnpackError.into()),
        })
    }

    fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
        if input.len() >= 8 {
            let (amount, rest) = input.split_at(8);
            let amount = amount
                .get(..8)
                .and_then(|slice| slice.try_into().ok())
                .map(u64::from_le_bytes)
                .ok_or(LendingError::InstructionUnpackError)?;
            Ok((amount, rest))
        } else {
            Err(LendingError::InstructionUnpackError.into())
        }
    }

    fn unpack_u8(input: &[u8]) -> Result<(u8, &[u8]), ProgramError> {
        if !input.is_empty() {
            let (amount, rest) = input.split_at(1);
            let amount = amount
                .get(..1)
                .and_then(|slice| slice.try_into().ok())
                .map(u8::from_le_bytes)
                .ok_or(LendingError::InstructionUnpackError)?;
            Ok((amount, rest))
        } else {
            Err(LendingError::InstructionUnpackError.into())
        }
    }

    fn unpack_pubkey(input: &[u8]) -> Result<(Pubkey, &[u8]), ProgramError> {
        if input.len() >= 32 {
            let (key, rest) = input.split_at(32);
            let pk = Pubkey::new(key);
            Ok((pk, rest))
        } else {
            Err(LendingError::InstructionUnpackError.into())
        }
    }

    /// Packs a [LendingInstruction](enum.LendingInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match *self {
            Self::InitLendingMarket { market_owner } => {
                buf.push(0);
                buf.extend_from_slice(market_owner.as_ref());
            }
            Self::InitReserve {
                liquidity_amount,
                config:
                    ReserveConfig {
                        optimal_utilization_rate,
                        loan_to_value_ratio,
                        liquidation_bonus,
                        liquidation_threshold,
                        min_borrow_rate,
                        optimal_borrow_rate,
                        max_borrow_rate,
                        fees:
                            ReserveFees {
                                borrow_fee_wad,
                                host_fee_percentage,
                            },
                    },
            } => {
                buf.push(1);
                buf.extend_from_slice(&liquidity_amount.to_le_bytes());
                buf.extend_from_slice(&optimal_utilization_rate.to_le_bytes());
                buf.extend_from_slice(&loan_to_value_ratio.to_le_bytes());
                buf.extend_from_slice(&liquidation_bonus.to_le_bytes());
                buf.extend_from_slice(&liquidation_threshold.to_le_bytes());
                buf.extend_from_slice(&min_borrow_rate.to_le_bytes());
                buf.extend_from_slice(&optimal_borrow_rate.to_le_bytes());
                buf.extend_from_slice(&max_borrow_rate.to_le_bytes());
                buf.extend_from_slice(&borrow_fee_wad.to_le_bytes());
                buf.extend_from_slice(&host_fee_percentage.to_le_bytes());
            }
            Self::DepositReserveLiquidity { liquidity_amount } => {
                buf.push(2);
                buf.extend_from_slice(&liquidity_amount.to_le_bytes());
            }
            Self::WithdrawReserveLiquidity { collateral_amount } => {
                buf.push(3);
                buf.extend_from_slice(&collateral_amount.to_le_bytes());
            }
            Self::BorrowReserveLiquidity {
                amount,
                amount_type,
            } => {
                buf.push(4);
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.extend_from_slice(&amount_type.to_u8().unwrap().to_le_bytes());
            }
            Self::RepayReserveLiquidity { liquidity_amount } => {
                buf.push(5);
                buf.extend_from_slice(&liquidity_amount.to_le_bytes());
            }
            Self::LiquidateObligation { liquidity_amount } => {
                buf.push(6);
                buf.extend_from_slice(&liquidity_amount.to_le_bytes());
            }
        }
        buf
    }
}

/// Creates an 'InitLendingMarket' instruction.
pub fn init_lending_market(
    program_id: Pubkey,
    lending_market_pubkey: Pubkey,
    lending_market_owner: Pubkey,
    quote_token_mint: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(lending_market_pubkey, false),
            AccountMeta::new_readonly(quote_token_mint, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::InitLendingMarket {
            market_owner: lending_market_owner,
        }
        .pack(),
    }
}

/// Creates an 'InitReserve' instruction.
#[allow(clippy::too_many_arguments)]
pub fn init_reserve(
    program_id: Pubkey,
    liquidity_amount: u64,
    config: ReserveConfig,
    source_liquidity_pubkey: Pubkey,
    destination_collateral_pubkey: Pubkey,
    reserve_pubkey: Pubkey,
    reserve_liquidity_mint_pubkey: Pubkey,
    reserve_liquidity_supply_pubkey: Pubkey,
    reserve_collateral_mint_pubkey: Pubkey,
    reserve_collateral_supply_pubkey: Pubkey,
    reserve_collateral_fees_receiver_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    lending_market_owner_pubkey: Pubkey,
    user_transfer_authority_pubkey: Pubkey,
    dex_market_pubkey: Option<Pubkey>,
) -> Instruction {
    let (lending_market_authority_pubkey, _bump_seed) =
        Pubkey::find_program_address(&[&lending_market_pubkey.to_bytes()[..32]], &program_id);
    let mut accounts = vec![
        AccountMeta::new(source_liquidity_pubkey, false),
        AccountMeta::new(destination_collateral_pubkey, false),
        AccountMeta::new(reserve_pubkey, false),
        AccountMeta::new_readonly(reserve_liquidity_mint_pubkey, false),
        AccountMeta::new(reserve_liquidity_supply_pubkey, false),
        AccountMeta::new(reserve_collateral_mint_pubkey, false),
        AccountMeta::new(reserve_collateral_supply_pubkey, false),
        AccountMeta::new(reserve_collateral_fees_receiver_pubkey, false),
        AccountMeta::new_readonly(lending_market_pubkey, false),
        AccountMeta::new_readonly(lending_market_owner_pubkey, true),
        AccountMeta::new_readonly(lending_market_authority_pubkey, false),
        AccountMeta::new_readonly(user_transfer_authority_pubkey, true),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    if let Some(dex_market_pubkey) = dex_market_pubkey {
        accounts.push(AccountMeta::new_readonly(dex_market_pubkey, false));
    }

    Instruction {
        program_id,
        accounts,
        data: LendingInstruction::InitReserve {
            liquidity_amount,
            config,
        }
        .pack(),
    }
}

/// Creates a 'DepositReserveLiquidity' instruction.
#[allow(clippy::too_many_arguments)]
pub fn deposit_reserve_liquidity(
    program_id: Pubkey,
    liquidity_amount: u64,
    source_liquidity_pubkey: Pubkey,
    destination_collateral_pubkey: Pubkey,
    reserve_pubkey: Pubkey,
    reserve_liquidity_supply_pubkey: Pubkey,
    reserve_collateral_mint_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    lending_market_authority_pubkey: Pubkey,
    user_transfer_authority_pubkey: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(source_liquidity_pubkey, false),
            AccountMeta::new(destination_collateral_pubkey, false),
            AccountMeta::new(reserve_pubkey, false),
            AccountMeta::new(reserve_liquidity_supply_pubkey, false),
            AccountMeta::new(reserve_collateral_mint_pubkey, false),
            AccountMeta::new_readonly(lending_market_pubkey, false),
            AccountMeta::new_readonly(lending_market_authority_pubkey, false),
            AccountMeta::new_readonly(user_transfer_authority_pubkey, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::DepositReserveLiquidity { liquidity_amount }.pack(),
    }
}

/// Creates a 'WithdrawReserveLiquidity' instruction.
#[allow(clippy::too_many_arguments)]
pub fn withdraw_reserve_liquidity(
    program_id: Pubkey,
    collateral_amount: u64,
    source_collateral_pubkey: Pubkey,
    destination_liquidity_pubkey: Pubkey,
    reserve_pubkey: Pubkey,
    reserve_collateral_mint_pubkey: Pubkey,
    reserve_liquidity_supply_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    lending_market_authority_pubkey: Pubkey,
    user_transfer_authority_pubkey: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(source_collateral_pubkey, false),
            AccountMeta::new(destination_liquidity_pubkey, false),
            AccountMeta::new(reserve_pubkey, false),
            AccountMeta::new(reserve_collateral_mint_pubkey, false),
            AccountMeta::new(reserve_liquidity_supply_pubkey, false),
            AccountMeta::new_readonly(lending_market_pubkey, false),
            AccountMeta::new_readonly(lending_market_authority_pubkey, false),
            AccountMeta::new_readonly(user_transfer_authority_pubkey, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::WithdrawReserveLiquidity { collateral_amount }.pack(),
    }
}

/// Creates a 'BorrowReserveLiquidity' instruction.
#[allow(clippy::too_many_arguments)]
pub fn borrow_reserve_liquidity(
    program_id: Pubkey,
    amount: u64,
    amount_type: BorrowAmountType,
    source_collateral_pubkey: Pubkey,
    destination_liquidity_pubkey: Pubkey,
    deposit_reserve_pubkey: Pubkey,
    deposit_reserve_collateral_supply_pubkey: Pubkey,
    deposit_reserve_collateral_fees_receiver_pubkey: Pubkey,
    borrow_reserve_pubkey: Pubkey,
    borrow_reserve_liquidity_supply_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    lending_market_authority_pubkey: Pubkey,
    user_transfer_authority_pubkey: Pubkey,
    obligation_pubkey: Pubkey,
    obligation_token_mint_pubkey: Pubkey,
    obligation_token_output_pubkey: Pubkey,
    obligation_token_owner_pubkey: Pubkey,
    dex_market_pubkey: Pubkey,
    dex_market_order_book_side_pubkey: Pubkey,
    memory_pubkey: Pubkey,
    deposit_reserve_collateral_host_pubkey: Option<Pubkey>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(source_collateral_pubkey, false),
        AccountMeta::new(destination_liquidity_pubkey, false),
        AccountMeta::new(deposit_reserve_pubkey, false),
        AccountMeta::new(deposit_reserve_collateral_supply_pubkey, false),
        AccountMeta::new(deposit_reserve_collateral_fees_receiver_pubkey, false),
        AccountMeta::new(borrow_reserve_pubkey, false),
        AccountMeta::new(borrow_reserve_liquidity_supply_pubkey, false),
        AccountMeta::new(obligation_pubkey, false),
        AccountMeta::new(obligation_token_mint_pubkey, false),
        AccountMeta::new(obligation_token_output_pubkey, false),
        AccountMeta::new_readonly(obligation_token_owner_pubkey, false),
        AccountMeta::new_readonly(lending_market_pubkey, false),
        AccountMeta::new_readonly(lending_market_authority_pubkey, false),
        AccountMeta::new_readonly(user_transfer_authority_pubkey, true),
        AccountMeta::new_readonly(dex_market_pubkey, false),
        AccountMeta::new_readonly(dex_market_order_book_side_pubkey, false),
        AccountMeta::new_readonly(memory_pubkey, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    if let Some(deposit_reserve_collateral_host_pubkey) = deposit_reserve_collateral_host_pubkey {
        accounts.push(AccountMeta::new(
            deposit_reserve_collateral_host_pubkey,
            false,
        ));
    }
    Instruction {
        program_id,
        accounts,
        data: LendingInstruction::BorrowReserveLiquidity {
            amount,
            amount_type,
        }
        .pack(),
    }
}

/// Creates a `RepayReserveLiquidity` instruction
#[allow(clippy::too_many_arguments)]
pub fn repay_reserve_liquidity(
    program_id: Pubkey,
    liquidity_amount: u64,
    source_liquidity_pubkey: Pubkey,
    destination_collateral_pubkey: Pubkey,
    repay_reserve_pubkey: Pubkey,
    repay_reserve_liquidity_supply_pubkey: Pubkey,
    withdraw_reserve_pubkey: Pubkey,
    withdraw_reserve_collateral_supply_pubkey: Pubkey,
    obligation_pubkey: Pubkey,
    obligation_mint_pubkey: Pubkey,
    obligation_output_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    lending_market_authority_pubkey: Pubkey,
    user_transfer_authority_pubkey: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(source_liquidity_pubkey, false),
            AccountMeta::new(destination_collateral_pubkey, false),
            AccountMeta::new(repay_reserve_pubkey, false),
            AccountMeta::new(repay_reserve_liquidity_supply_pubkey, false),
            AccountMeta::new_readonly(withdraw_reserve_pubkey, false),
            AccountMeta::new(withdraw_reserve_collateral_supply_pubkey, false),
            AccountMeta::new(obligation_pubkey, false),
            AccountMeta::new(obligation_mint_pubkey, false),
            AccountMeta::new(obligation_output_pubkey, false),
            AccountMeta::new_readonly(lending_market_pubkey, false),
            AccountMeta::new_readonly(lending_market_authority_pubkey, false),
            AccountMeta::new_readonly(user_transfer_authority_pubkey, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::RepayReserveLiquidity { liquidity_amount }.pack(),
    }
}

/// Creates a `LiquidateObligation` instruction
#[allow(clippy::too_many_arguments)]
pub fn liquidate_obligation(
    program_id: Pubkey,
    liquidity_amount: u64,
    source_liquidity_pubkey: Pubkey,
    destination_collateral_pubkey: Pubkey,
    repay_reserve_pubkey: Pubkey,
    repay_reserve_liquidity_supply_pubkey: Pubkey,
    withdraw_reserve_pubkey: Pubkey,
    withdraw_reserve_collateral_supply_pubkey: Pubkey,
    obligation_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    lending_market_authority_pubkey: Pubkey,
    user_transfer_authority_pubkey: Pubkey,
    dex_market_pubkey: Pubkey,
    dex_market_order_book_side_pubkey: Pubkey,
    memory_pubkey: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(source_liquidity_pubkey, false),
            AccountMeta::new(destination_collateral_pubkey, false),
            AccountMeta::new(repay_reserve_pubkey, false),
            AccountMeta::new(repay_reserve_liquidity_supply_pubkey, false),
            AccountMeta::new(withdraw_reserve_pubkey, false),
            AccountMeta::new(withdraw_reserve_collateral_supply_pubkey, false),
            AccountMeta::new(obligation_pubkey, false),
            AccountMeta::new_readonly(lending_market_pubkey, false),
            AccountMeta::new_readonly(lending_market_authority_pubkey, false),
            AccountMeta::new_readonly(user_transfer_authority_pubkey, true),
            AccountMeta::new_readonly(dex_market_pubkey, false),
            AccountMeta::new_readonly(dex_market_order_book_side_pubkey, false),
            AccountMeta::new_readonly(memory_pubkey, false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::LiquidateObligation { liquidity_amount }.pack(),
    }
}
