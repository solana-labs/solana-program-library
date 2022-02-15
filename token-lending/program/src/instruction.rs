//! Instruction types

use crate::{
    error::LendingError,
    state::{ReserveConfig, ReserveFees},
};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    msg,
    program_error::ProgramError,
    pubkey::{Pubkey, PUBKEY_BYTES},
    sysvar,
};
use std::{convert::TryInto, mem::size_of};

/// Instructions supported by the lending program.
#[derive(Clone, Debug, PartialEq)]
pub enum LendingInstruction {
    // 0
    /// Initializes a new lending market.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Lending market account - uninitialized.
    ///   1. `[]` Rent sysvar.
    ///   2. `[]` Token program id.
    ///   3. `[]` Oracle program id.
    InitLendingMarket {
        /// Owner authority which can add new reserves
        owner: Pubkey,
        /// Currency market prices are quoted in
        /// e.g. "USD" null padded (`*b"USD\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"`) or SPL token mint pubkey
        quote_currency: [u8; 32],
    },

    // 1
    /// Sets the new owner of a lending market.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Lending market account.
    ///   1. `[signer]` Current owner.
    SetLendingMarketOwner {
        /// The new owner
        new_owner: Pubkey,
    },

    // 2
    /// Initializes a new lending market reserve.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Source liquidity token account.
    ///                     $authority can transfer $liquidity_amount.
    ///   1. `[writable]` Destination collateral token account - uninitialized.
    ///   2. `[writable]` Reserve account - uninitialized.
    ///   3. `[]` Reserve liquidity SPL Token mint.
    ///   4. `[writable]` Reserve liquidity supply SPL Token account - uninitialized.
    ///   5. `[writable]` Reserve liquidity fee receiver - uninitialized.
    ///   6. `[writable]` Reserve collateral SPL Token mint - uninitialized.
    ///   7. `[writable]` Reserve collateral token supply - uninitialized.
    ///   8. `[]` Pyth product account.
    ///   9. `[]` Pyth price account.
    ///             This will be used as the reserve liquidity oracle account.
    ///   10 `[]` Lending market account.
    ///   11 `[]` Derived lending market authority.
    ///   12 `[signer]` Lending market owner.
    ///   13 `[signer]` User transfer authority ($authority).
    ///   14 `[]` Clock sysvar.
    ///   15 `[]` Rent sysvar.
    ///   16 `[]` Token program id.
    InitReserve {
        /// Initial amount of liquidity to deposit into the new reserve
        liquidity_amount: u64,
        /// Reserve configuration values
        config: ReserveConfig,
    },

    // 3
    /// Accrue interest and update market price of liquidity on a reserve.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Reserve account.
    ///   1. `[]` Reserve liquidity oracle account.
    ///             Must be the Pyth price account specified at InitReserve.
    ///   2. `[]` Clock sysvar.
    RefreshReserve,

    // 4
    /// Deposit liquidity into a reserve in exchange for collateral. Collateral represents a share
    /// of the reserve liquidity pool.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Source liquidity token account.
    ///                     $authority can transfer $liquidity_amount.
    ///   1. `[writable]` Destination collateral token account.
    ///   2. `[writable]` Reserve account.
    ///   3. `[writable]` Reserve liquidity supply SPL Token account.
    ///   4. `[writable]` Reserve collateral SPL Token mint.
    ///   5. `[]` Lending market account.
    ///   6. `[]` Derived lending market authority.
    ///   7. `[signer]` User transfer authority ($authority).
    ///   8. `[]` Clock sysvar.
    ///   9. `[]` Token program id.
    DepositReserveLiquidity {
        /// Amount of liquidity to deposit in exchange for collateral tokens
        liquidity_amount: u64,
    },

    // 5
    /// Redeem collateral from a reserve in exchange for liquidity.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Source collateral token account.
    ///                     $authority can transfer $collateral_amount.
    ///   1. `[writable]` Destination liquidity token account.
    ///   2. `[writable]` Reserve account.
    ///   3. `[writable]` Reserve collateral SPL Token mint.
    ///   4. `[writable]` Reserve liquidity supply SPL Token account.
    ///   5. `[]` Lending market account.
    ///   6. `[]` Derived lending market authority.
    ///   7. `[signer]` User transfer authority ($authority).
    ///   8. `[]` Clock sysvar.
    ///   9. `[]` Token program id.
    RedeemReserveCollateral {
        /// Amount of collateral tokens to redeem in exchange for liquidity
        collateral_amount: u64,
    },

    // 6
    /// Initializes a new lending market obligation.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Obligation account - uninitialized.
    ///   1. `[]` Lending market account.
    ///   2. `[signer]` Obligation owner.
    ///   3. `[]` Clock sysvar.
    ///   4. `[]` Rent sysvar.
    ///   5. `[]` Token program id.
    InitObligation,

    // 7
    /// Refresh an obligation's accrued interest and collateral and liquidity prices. Requires
    /// refreshed reserves, as all obligation collateral deposit reserves in order, followed by all
    /// liquidity borrow reserves in order.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Obligation account.
    ///   1. `[]` Clock sysvar.
    ///   .. `[]` Collateral deposit reserve accounts - refreshed, all, in order.
    ///   .. `[]` Liquidity borrow reserve accounts - refreshed, all, in order.
    RefreshObligation,

    // 8
    /// Deposit collateral to an obligation. Requires a refreshed reserve.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Source collateral token account.
    ///                     Minted by deposit reserve collateral mint.
    ///                     $authority can transfer $collateral_amount.
    ///   1. `[writable]` Destination deposit reserve collateral supply SPL Token account.
    ///   2. `[]` Deposit reserve account - refreshed.
    ///   3. `[writable]` Obligation account.
    ///   4. `[]` Lending market account.
    ///   5. `[signer]` Obligation owner.
    ///   6. `[signer]` User transfer authority ($authority).
    ///   7. `[]` Clock sysvar.
    ///   8. `[]` Token program id.
    DepositObligationCollateral {
        /// Amount of collateral tokens to deposit
        collateral_amount: u64,
    },

    // 9
    /// Withdraw collateral from an obligation. Requires a refreshed obligation and reserve.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Source withdraw reserve collateral supply SPL Token account.
    ///   1. `[writable]` Destination collateral token account.
    ///                     Minted by withdraw reserve collateral mint.
    ///   2. `[]` Withdraw reserve account - refreshed.
    ///   3. `[writable]` Obligation account - refreshed.
    ///   4. `[]` Lending market account.
    ///   5. `[]` Derived lending market authority.
    ///   6. `[signer]` Obligation owner.
    ///   7. `[]` Clock sysvar.
    ///   8. `[]` Token program id.
    WithdrawObligationCollateral {
        /// Amount of collateral tokens to withdraw - u64::MAX for up to 100% of deposited amount
        collateral_amount: u64,
    },

    // 10
    /// Borrow liquidity from a reserve by depositing collateral tokens. Requires a refreshed
    /// obligation and reserve.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Source borrow reserve liquidity supply SPL Token account.
    ///   1. `[writable]` Destination liquidity token account.
    ///                     Minted by borrow reserve liquidity mint.
    ///   2. `[writable]` Borrow reserve account - refreshed.
    ///   3. `[writable]` Borrow reserve liquidity fee receiver account.
    ///                     Must be the fee account specified at InitReserve.
    ///   4. `[writable]` Obligation account - refreshed.
    ///   5. `[]` Lending market account.
    ///   6. `[]` Derived lending market authority.
    ///   7. `[signer]` Obligation owner.
    ///   8. `[]` Clock sysvar.
    ///   9. `[]` Token program id.
    ///   10 `[optional, writable]` Host fee receiver account.
    BorrowObligationLiquidity {
        /// Amount of liquidity to borrow - u64::MAX for 100% of borrowing power
        liquidity_amount: u64,
        // @TODO: slippage constraint - https://git.io/JmV67
    },

    // 11
    /// Repay borrowed liquidity to a reserve. Requires a refreshed obligation and reserve.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Source liquidity token account.
    ///                     Minted by repay reserve liquidity mint.
    ///                     $authority can transfer $liquidity_amount.
    ///   1. `[writable]` Destination repay reserve liquidity supply SPL Token account.
    ///   2. `[writable]` Repay reserve account - refreshed.
    ///   3. `[writable]` Obligation account - refreshed.
    ///   4. `[]` Lending market account.
    ///   5. `[signer]` User transfer authority ($authority).
    ///   6. `[]` Clock sysvar.
    ///   7. `[]` Token program id.
    RepayObligationLiquidity {
        /// Amount of liquidity to repay - u64::MAX for 100% of borrowed amount
        liquidity_amount: u64,
    },

    // 12
    /// Repay borrowed liquidity to a reserve to receive collateral at a discount from an unhealthy
    /// obligation. Requires a refreshed obligation and reserves.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Source liquidity token account.
    ///                     Minted by repay reserve liquidity mint.
    ///                     $authority can transfer $liquidity_amount.
    ///   1. `[writable]` Destination collateral token account.
    ///                     Minted by withdraw reserve collateral mint.
    ///   2. `[writable]` Repay reserve account - refreshed.
    ///   3. `[writable]` Repay reserve liquidity supply SPL Token account.
    ///   4. `[]` Withdraw reserve account - refreshed.
    ///   5. `[writable]` Withdraw reserve collateral supply SPL Token account.
    ///   6. `[writable]` Obligation account - refreshed.
    ///   7. `[]` Lending market account.
    ///   8. `[]` Derived lending market authority.
    ///   9. `[signer]` User transfer authority ($authority).
    ///   10 `[]` Clock sysvar.
    ///   11 `[]` Token program id.
    LiquidateObligation {
        /// Amount of liquidity to repay - u64::MAX for up to 100% of borrowed amount
        liquidity_amount: u64,
    },

    // 13
    /// Make a flash loan.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Source liquidity token account.
    ///                     Minted by reserve liquidity mint.
    ///                     Must match the reserve liquidity supply.
    ///   1. `[writable]` Destination liquidity token account.
    ///                     Minted by reserve liquidity mint.
    ///   2. `[writable]` Reserve account.
    ///   3. `[writable]` Flash loan fee receiver account.
    ///                     Must match the reserve liquidity fee receiver.
    ///   4. `[writable]` Host fee receiver.
    ///   5. `[]` Lending market account.
    ///   6. `[]` Derived lending market authority.
    ///   7. `[]` Token program id.
    ///   8. `[]` Flash loan receiver program id.
    ///             Must implement an instruction that has tag of 0 and a signature of `(amount: u64)`
    ///             This instruction must return the amount to the source liquidity account.
    ///   .. `[any]` Additional accounts expected by the receiving program's `ReceiveFlashLoan` instruction.
    ///
    ///   The flash loan receiver program that is to be invoked should contain an instruction with
    ///   tag `0` and accept the total amount (including fee) that needs to be returned back after
    ///   its execution has completed.
    ///
    ///   Flash loan receiver should have an instruction with the following signature:
    ///
    ///   0. `[writable]` Source liquidity (matching the destination from above).
    ///   1. `[writable]` Destination liquidity (matching the source from above).
    ///   2. `[]` Token program id
    ///   .. `[any]` Additional accounts provided to the lending program's `FlashLoan` instruction above.
    ///   ReceiveFlashLoan {
    ///       // Amount that must be repaid by the receiver program
    ///       amount: u64
    ///   }
    FlashLoan {
        /// The amount that is to be borrowed - u64::MAX for up to 100% of available liquidity
        amount: u64,
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
                let (owner, rest) = Self::unpack_pubkey(rest)?;
                let (quote_currency, _rest) = Self::unpack_bytes32(rest)?;
                Self::InitLendingMarket {
                    owner,
                    quote_currency: *quote_currency,
                }
            }
            1 => {
                let (new_owner, _rest) = Self::unpack_pubkey(rest)?;
                Self::SetLendingMarketOwner { new_owner }
            }
            2 => {
                let (liquidity_amount, rest) = Self::unpack_u64(rest)?;
                let (optimal_utilization_rate, rest) = Self::unpack_u8(rest)?;
                let (loan_to_value_ratio, rest) = Self::unpack_u8(rest)?;
                let (liquidation_bonus, rest) = Self::unpack_u8(rest)?;
                let (liquidation_threshold, rest) = Self::unpack_u8(rest)?;
                let (min_borrow_rate, rest) = Self::unpack_u8(rest)?;
                let (optimal_borrow_rate, rest) = Self::unpack_u8(rest)?;
                let (max_borrow_rate, rest) = Self::unpack_u8(rest)?;
                let (borrow_fee_wad, rest) = Self::unpack_u64(rest)?;
                let (flash_loan_fee_wad, rest) = Self::unpack_u64(rest)?;
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
                            flash_loan_fee_wad,
                            host_fee_percentage,
                        },
                    },
                }
            }
            3 => Self::RefreshReserve,
            4 => {
                let (liquidity_amount, _rest) = Self::unpack_u64(rest)?;
                Self::DepositReserveLiquidity { liquidity_amount }
            }
            5 => {
                let (collateral_amount, _rest) = Self::unpack_u64(rest)?;
                Self::RedeemReserveCollateral { collateral_amount }
            }
            6 => Self::InitObligation,
            7 => Self::RefreshObligation,
            8 => {
                let (collateral_amount, _rest) = Self::unpack_u64(rest)?;
                Self::DepositObligationCollateral { collateral_amount }
            }
            9 => {
                let (collateral_amount, _rest) = Self::unpack_u64(rest)?;
                Self::WithdrawObligationCollateral { collateral_amount }
            }
            10 => {
                let (liquidity_amount, _rest) = Self::unpack_u64(rest)?;
                Self::BorrowObligationLiquidity { liquidity_amount }
            }
            11 => {
                let (liquidity_amount, _rest) = Self::unpack_u64(rest)?;
                Self::RepayObligationLiquidity { liquidity_amount }
            }
            12 => {
                let (liquidity_amount, _rest) = Self::unpack_u64(rest)?;
                Self::LiquidateObligation { liquidity_amount }
            }
            13 => {
                let (amount, _rest) = Self::unpack_u64(rest)?;
                Self::FlashLoan { amount }
            }
            _ => {
                msg!("Instruction cannot be unpacked");
                return Err(LendingError::InstructionUnpackError.into());
            }
        })
    }

    fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
        if input.len() < 8 {
            msg!("u64 cannot be unpacked");
            return Err(LendingError::InstructionUnpackError.into());
        }
        let (bytes, rest) = input.split_at(8);
        let value = bytes
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(LendingError::InstructionUnpackError)?;
        Ok((value, rest))
    }

    fn unpack_u8(input: &[u8]) -> Result<(u8, &[u8]), ProgramError> {
        if input.is_empty() {
            msg!("u8 cannot be unpacked");
            return Err(LendingError::InstructionUnpackError.into());
        }
        let (bytes, rest) = input.split_at(1);
        let value = bytes
            .get(..1)
            .and_then(|slice| slice.try_into().ok())
            .map(u8::from_le_bytes)
            .ok_or(LendingError::InstructionUnpackError)?;
        Ok((value, rest))
    }

    fn unpack_bytes32(input: &[u8]) -> Result<(&[u8; 32], &[u8]), ProgramError> {
        if input.len() < 32 {
            msg!("32 bytes cannot be unpacked");
            return Err(LendingError::InstructionUnpackError.into());
        }
        let (bytes, rest) = input.split_at(32);
        Ok((
            bytes
                .try_into()
                .map_err(|_| LendingError::InstructionUnpackError)?,
            rest,
        ))
    }

    fn unpack_pubkey(input: &[u8]) -> Result<(Pubkey, &[u8]), ProgramError> {
        if input.len() < PUBKEY_BYTES {
            msg!("Pubkey cannot be unpacked");
            return Err(LendingError::InstructionUnpackError.into());
        }
        let (key, rest) = input.split_at(PUBKEY_BYTES);
        let pk = Pubkey::new(key);
        Ok((pk, rest))
    }

    /// Packs a [LendingInstruction](enum.LendingInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match *self {
            Self::InitLendingMarket {
                owner,
                quote_currency,
            } => {
                buf.push(0);
                buf.extend_from_slice(owner.as_ref());
                buf.extend_from_slice(quote_currency.as_ref());
            }
            Self::SetLendingMarketOwner { new_owner } => {
                buf.push(1);
                buf.extend_from_slice(new_owner.as_ref());
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
                                flash_loan_fee_wad,
                                host_fee_percentage,
                            },
                    },
            } => {
                buf.push(2);
                buf.extend_from_slice(&liquidity_amount.to_le_bytes());
                buf.extend_from_slice(&optimal_utilization_rate.to_le_bytes());
                buf.extend_from_slice(&loan_to_value_ratio.to_le_bytes());
                buf.extend_from_slice(&liquidation_bonus.to_le_bytes());
                buf.extend_from_slice(&liquidation_threshold.to_le_bytes());
                buf.extend_from_slice(&min_borrow_rate.to_le_bytes());
                buf.extend_from_slice(&optimal_borrow_rate.to_le_bytes());
                buf.extend_from_slice(&max_borrow_rate.to_le_bytes());
                buf.extend_from_slice(&borrow_fee_wad.to_le_bytes());
                buf.extend_from_slice(&flash_loan_fee_wad.to_le_bytes());
                buf.extend_from_slice(&host_fee_percentage.to_le_bytes());
            }
            Self::RefreshReserve => {
                buf.push(3);
            }
            Self::DepositReserveLiquidity { liquidity_amount } => {
                buf.push(4);
                buf.extend_from_slice(&liquidity_amount.to_le_bytes());
            }
            Self::RedeemReserveCollateral { collateral_amount } => {
                buf.push(5);
                buf.extend_from_slice(&collateral_amount.to_le_bytes());
            }
            Self::InitObligation => {
                buf.push(6);
            }
            Self::RefreshObligation => {
                buf.push(7);
            }
            Self::DepositObligationCollateral { collateral_amount } => {
                buf.push(8);
                buf.extend_from_slice(&collateral_amount.to_le_bytes());
            }
            Self::WithdrawObligationCollateral { collateral_amount } => {
                buf.push(9);
                buf.extend_from_slice(&collateral_amount.to_le_bytes());
            }
            Self::BorrowObligationLiquidity { liquidity_amount } => {
                buf.push(10);
                buf.extend_from_slice(&liquidity_amount.to_le_bytes());
            }
            Self::RepayObligationLiquidity { liquidity_amount } => {
                buf.push(11);
                buf.extend_from_slice(&liquidity_amount.to_le_bytes());
            }
            Self::LiquidateObligation { liquidity_amount } => {
                buf.push(12);
                buf.extend_from_slice(&liquidity_amount.to_le_bytes());
            }
            Self::FlashLoan { amount } => {
                buf.push(13);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
        }
        buf
    }
}

/// Creates an 'InitLendingMarket' instruction.
pub fn init_lending_market(
    program_id: Pubkey,
    owner: Pubkey,
    quote_currency: [u8; 32],
    lending_market_pubkey: Pubkey,
    oracle_program_id: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(lending_market_pubkey, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(oracle_program_id, false),
        ],
        data: LendingInstruction::InitLendingMarket {
            owner,
            quote_currency,
        }
        .pack(),
    }
}

/// Creates a 'SetLendingMarketOwner' instruction.
pub fn set_lending_market_owner(
    program_id: Pubkey,
    lending_market_pubkey: Pubkey,
    lending_market_owner: Pubkey,
    new_owner: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(lending_market_pubkey, false),
            AccountMeta::new_readonly(lending_market_owner, true),
        ],
        data: LendingInstruction::SetLendingMarketOwner { new_owner }.pack(),
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
    reserve_liquidity_fee_receiver_pubkey: Pubkey,
    reserve_collateral_mint_pubkey: Pubkey,
    reserve_collateral_supply_pubkey: Pubkey,
    pyth_product_pubkey: Pubkey,
    pyth_price_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    lending_market_owner_pubkey: Pubkey,
    user_transfer_authority_pubkey: Pubkey,
) -> Instruction {
    let (lending_market_authority_pubkey, _bump_seed) = Pubkey::find_program_address(
        &[&lending_market_pubkey.to_bytes()[..PUBKEY_BYTES]],
        &program_id,
    );
    let accounts = vec![
        AccountMeta::new(source_liquidity_pubkey, false),
        AccountMeta::new(destination_collateral_pubkey, false),
        AccountMeta::new(reserve_pubkey, false),
        AccountMeta::new_readonly(reserve_liquidity_mint_pubkey, false),
        AccountMeta::new(reserve_liquidity_supply_pubkey, false),
        AccountMeta::new(reserve_liquidity_fee_receiver_pubkey, false),
        AccountMeta::new(reserve_collateral_mint_pubkey, false),
        AccountMeta::new(reserve_collateral_supply_pubkey, false),
        AccountMeta::new_readonly(pyth_product_pubkey, false),
        AccountMeta::new_readonly(pyth_price_pubkey, false),
        AccountMeta::new_readonly(lending_market_pubkey, false),
        AccountMeta::new_readonly(lending_market_authority_pubkey, false),
        AccountMeta::new_readonly(lending_market_owner_pubkey, true),
        AccountMeta::new_readonly(user_transfer_authority_pubkey, true),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
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

/// Creates a `RefreshReserve` instruction
pub fn refresh_reserve(
    program_id: Pubkey,
    reserve_pubkey: Pubkey,
    reserve_liquidity_oracle_pubkey: Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(reserve_pubkey, false),
        AccountMeta::new_readonly(reserve_liquidity_oracle_pubkey, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];
    Instruction {
        program_id,
        accounts,
        data: LendingInstruction::RefreshReserve.pack(),
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
    user_transfer_authority_pubkey: Pubkey,
) -> Instruction {
    let (lending_market_authority_pubkey, _bump_seed) = Pubkey::find_program_address(
        &[&lending_market_pubkey.to_bytes()[..PUBKEY_BYTES]],
        &program_id,
    );
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

/// Creates a 'RedeemReserveCollateral' instruction.
#[allow(clippy::too_many_arguments)]
pub fn redeem_reserve_collateral(
    program_id: Pubkey,
    collateral_amount: u64,
    source_collateral_pubkey: Pubkey,
    destination_liquidity_pubkey: Pubkey,
    reserve_pubkey: Pubkey,
    reserve_collateral_mint_pubkey: Pubkey,
    reserve_liquidity_supply_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    user_transfer_authority_pubkey: Pubkey,
) -> Instruction {
    let (lending_market_authority_pubkey, _bump_seed) = Pubkey::find_program_address(
        &[&lending_market_pubkey.to_bytes()[..PUBKEY_BYTES]],
        &program_id,
    );
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
        data: LendingInstruction::RedeemReserveCollateral { collateral_amount }.pack(),
    }
}

/// Creates an 'InitObligation' instruction.
#[allow(clippy::too_many_arguments)]
pub fn init_obligation(
    program_id: Pubkey,
    obligation_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    obligation_owner_pubkey: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(obligation_pubkey, false),
            AccountMeta::new_readonly(lending_market_pubkey, false),
            AccountMeta::new_readonly(obligation_owner_pubkey, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::InitObligation.pack(),
    }
}

/// Creates a 'RefreshObligation' instruction.
#[allow(clippy::too_many_arguments)]
pub fn refresh_obligation(
    program_id: Pubkey,
    obligation_pubkey: Pubkey,
    reserve_pubkeys: Vec<Pubkey>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(obligation_pubkey, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];
    accounts.extend(
        reserve_pubkeys
            .into_iter()
            .map(|pubkey| AccountMeta::new_readonly(pubkey, false)),
    );
    Instruction {
        program_id,
        accounts,
        data: LendingInstruction::RefreshObligation.pack(),
    }
}

/// Creates a 'DepositObligationCollateral' instruction.
#[allow(clippy::too_many_arguments)]
pub fn deposit_obligation_collateral(
    program_id: Pubkey,
    collateral_amount: u64,
    source_collateral_pubkey: Pubkey,
    destination_collateral_pubkey: Pubkey,
    deposit_reserve_pubkey: Pubkey,
    obligation_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    obligation_owner_pubkey: Pubkey,
    user_transfer_authority_pubkey: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(source_collateral_pubkey, false),
            AccountMeta::new(destination_collateral_pubkey, false),
            AccountMeta::new_readonly(deposit_reserve_pubkey, false),
            AccountMeta::new(obligation_pubkey, false),
            AccountMeta::new_readonly(lending_market_pubkey, false),
            AccountMeta::new_readonly(obligation_owner_pubkey, true),
            AccountMeta::new_readonly(user_transfer_authority_pubkey, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::DepositObligationCollateral { collateral_amount }.pack(),
    }
}

/// Creates a 'WithdrawObligationCollateral' instruction.
#[allow(clippy::too_many_arguments)]
pub fn withdraw_obligation_collateral(
    program_id: Pubkey,
    collateral_amount: u64,
    source_collateral_pubkey: Pubkey,
    destination_collateral_pubkey: Pubkey,
    withdraw_reserve_pubkey: Pubkey,
    obligation_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    obligation_owner_pubkey: Pubkey,
) -> Instruction {
    let (lending_market_authority_pubkey, _bump_seed) = Pubkey::find_program_address(
        &[&lending_market_pubkey.to_bytes()[..PUBKEY_BYTES]],
        &program_id,
    );
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(source_collateral_pubkey, false),
            AccountMeta::new(destination_collateral_pubkey, false),
            AccountMeta::new_readonly(withdraw_reserve_pubkey, false),
            AccountMeta::new(obligation_pubkey, false),
            AccountMeta::new_readonly(lending_market_pubkey, false),
            AccountMeta::new_readonly(lending_market_authority_pubkey, false),
            AccountMeta::new_readonly(obligation_owner_pubkey, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::WithdrawObligationCollateral { collateral_amount }.pack(),
    }
}

/// Creates a 'BorrowObligationLiquidity' instruction.
#[allow(clippy::too_many_arguments)]
pub fn borrow_obligation_liquidity(
    program_id: Pubkey,
    liquidity_amount: u64,
    source_liquidity_pubkey: Pubkey,
    destination_liquidity_pubkey: Pubkey,
    borrow_reserve_pubkey: Pubkey,
    borrow_reserve_liquidity_fee_receiver_pubkey: Pubkey,
    obligation_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    obligation_owner_pubkey: Pubkey,
    host_fee_receiver_pubkey: Option<Pubkey>,
) -> Instruction {
    let (lending_market_authority_pubkey, _bump_seed) = Pubkey::find_program_address(
        &[&lending_market_pubkey.to_bytes()[..PUBKEY_BYTES]],
        &program_id,
    );
    let mut accounts = vec![
        AccountMeta::new(source_liquidity_pubkey, false),
        AccountMeta::new(destination_liquidity_pubkey, false),
        AccountMeta::new(borrow_reserve_pubkey, false),
        AccountMeta::new(borrow_reserve_liquidity_fee_receiver_pubkey, false),
        AccountMeta::new(obligation_pubkey, false),
        AccountMeta::new_readonly(lending_market_pubkey, false),
        AccountMeta::new_readonly(lending_market_authority_pubkey, false),
        AccountMeta::new_readonly(obligation_owner_pubkey, true),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    if let Some(host_fee_receiver_pubkey) = host_fee_receiver_pubkey {
        accounts.push(AccountMeta::new(host_fee_receiver_pubkey, false));
    }
    Instruction {
        program_id,
        accounts,
        data: LendingInstruction::BorrowObligationLiquidity { liquidity_amount }.pack(),
    }
}

/// Creates a `RepayObligationLiquidity` instruction
#[allow(clippy::too_many_arguments)]
pub fn repay_obligation_liquidity(
    program_id: Pubkey,
    liquidity_amount: u64,
    source_liquidity_pubkey: Pubkey,
    destination_liquidity_pubkey: Pubkey,
    repay_reserve_pubkey: Pubkey,
    obligation_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    user_transfer_authority_pubkey: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(source_liquidity_pubkey, false),
            AccountMeta::new(destination_liquidity_pubkey, false),
            AccountMeta::new(repay_reserve_pubkey, false),
            AccountMeta::new(obligation_pubkey, false),
            AccountMeta::new_readonly(lending_market_pubkey, false),
            AccountMeta::new_readonly(user_transfer_authority_pubkey, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::RepayObligationLiquidity { liquidity_amount }.pack(),
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
    user_transfer_authority_pubkey: Pubkey,
) -> Instruction {
    let (lending_market_authority_pubkey, _bump_seed) = Pubkey::find_program_address(
        &[&lending_market_pubkey.to_bytes()[..PUBKEY_BYTES]],
        &program_id,
    );
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
            AccountMeta::new_readonly(lending_market_pubkey, false),
            AccountMeta::new_readonly(lending_market_authority_pubkey, false),
            AccountMeta::new_readonly(user_transfer_authority_pubkey, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::LiquidateObligation { liquidity_amount }.pack(),
    }
}

/// Creates a `FlashLoan` instruction.
#[allow(clippy::too_many_arguments)]
pub fn flash_loan(
    program_id: Pubkey,
    amount: u64,
    source_liquidity_pubkey: Pubkey,
    destination_liquidity_pubkey: Pubkey,
    reserve_pubkey: Pubkey,
    reserve_liquidity_fee_receiver_pubkey: Pubkey,
    host_fee_receiver_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    flash_loan_receiver_program_id: Pubkey,
    flash_loan_receiver_program_accounts: Vec<AccountMeta>,
) -> Instruction {
    let (lending_market_authority_pubkey, _bump_seed) = Pubkey::find_program_address(
        &[&lending_market_pubkey.to_bytes()[..PUBKEY_BYTES]],
        &program_id,
    );
    let mut accounts = vec![
        AccountMeta::new(source_liquidity_pubkey, false),
        AccountMeta::new(destination_liquidity_pubkey, false),
        AccountMeta::new(reserve_pubkey, false),
        AccountMeta::new(reserve_liquidity_fee_receiver_pubkey, false),
        AccountMeta::new(host_fee_receiver_pubkey, false),
        AccountMeta::new_readonly(lending_market_pubkey, false),
        AccountMeta::new_readonly(lending_market_authority_pubkey, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(flash_loan_receiver_program_id, false),
    ];
    accounts.extend(flash_loan_receiver_program_accounts);
    Instruction {
        program_id,
        accounts,
        data: LendingInstruction::FlashLoan { amount }.pack(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_lending_market() {
        let program_id = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let lending_market_pubkey = Pubkey::new_unique();
        let oracle_program_id = Pubkey::new_unique();
        let currency = "USD";
        let mut quote_currency = [0u8; 32];
        quote_currency[0..currency.len()].clone_from_slice(currency.as_bytes());
        let instruction = init_lending_market(
            program_id,
            owner,
            quote_currency,
            lending_market_pubkey,
            oracle_program_id,
        );
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 4);
        assert_eq!(
            instruction.data,
            LendingInstruction::InitLendingMarket {
                owner,
                quote_currency,
            }
            .pack()
        );
    }

    #[test]
    fn test_set_lending_market_owner() {
        let program_id = Pubkey::new_unique();
        let lending_market_pubkey = Pubkey::new_unique();
        let lending_market_owner = Pubkey::new_unique();
        let new_owner = Pubkey::new_unique();
        let instruction = set_lending_market_owner(
            program_id,
            lending_market_pubkey,
            lending_market_owner,
            new_owner,
        );
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 2);
        assert_eq!(
            instruction.data,
            LendingInstruction::SetLendingMarketOwner { new_owner }.pack()
        );
    }

    #[test]
    fn test_init_reserve() {
        let program_id = Pubkey::new_unique();
        let liquidity_amount = u64::MAX;
        let config = ReserveConfig {
            optimal_utilization_rate: 50,
            loan_to_value_ratio: 1,
            liquidation_bonus: 10,
            liquidation_threshold: 5,
            min_borrow_rate: 2,
            optimal_borrow_rate: 4,
            max_borrow_rate: 10,
            fees: ReserveFees {
                borrow_fee_wad: 1,
                flash_loan_fee_wad: 3,
                host_fee_percentage: 1,
            },
        };
        let source_liquidity_pubkey = Pubkey::new_unique();
        let destination_collateral_pubkey = Pubkey::new_unique();
        let reserve_pubkey = Pubkey::new_unique();
        let reserve_liquidity_mint_pubkey = Pubkey::new_unique();
        let reserve_liquidity_supply_pubkey = Pubkey::new_unique();
        let reserve_liquidity_fee_receiver_pubkey = Pubkey::new_unique();
        let reserve_collateral_mint_pubkey = Pubkey::new_unique();
        let reserve_collateral_supply_pubkey = Pubkey::new_unique();
        let pyth_product_pubkey = Pubkey::new_unique();
        let pyth_price_pubkey = Pubkey::new_unique();
        let lending_market_pubkey = Pubkey::new_unique();
        let lending_market_owner_pubkey = Pubkey::new_unique();
        let user_transfer_authority_pubkey = Pubkey::new_unique();
        let instruction = init_reserve(
            program_id,
            liquidity_amount,
            config,
            source_liquidity_pubkey,
            destination_collateral_pubkey,
            reserve_pubkey,
            reserve_liquidity_mint_pubkey,
            reserve_liquidity_supply_pubkey,
            reserve_liquidity_fee_receiver_pubkey,
            reserve_collateral_mint_pubkey,
            reserve_collateral_supply_pubkey,
            pyth_product_pubkey,
            pyth_price_pubkey,
            lending_market_pubkey,
            lending_market_owner_pubkey,
            user_transfer_authority_pubkey,
        );
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 17);
        assert_eq!(
            instruction.data,
            LendingInstruction::InitReserve {
                liquidity_amount,
                config,
            }
            .pack()
        );
    }

    #[test]
    fn test_refresh_reserve() {
        let program_id = Pubkey::new_unique();
        let reserve_pubkey = Pubkey::new_unique();
        let reserve_liquidity_oracle_pubkey = Pubkey::new_unique();
        let instruction =
            refresh_reserve(program_id, reserve_pubkey, reserve_liquidity_oracle_pubkey);
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 3);
        assert_eq!(instruction.data, LendingInstruction::RefreshReserve.pack());
    }

    #[test]
    fn test_deposit_reserve_liquidity() {
        let program_id = Pubkey::new_unique();
        let liquidity_amount = u64::MAX;
        let source_liquidity_pubkey = Pubkey::new_unique();
        let destination_collateral_pubkey = Pubkey::new_unique();
        let reserve_pubkey = Pubkey::new_unique();
        let reserve_liquidity_supply_pubkey = Pubkey::new_unique();
        let reserve_collateral_mint_pubkey = Pubkey::new_unique();
        let lending_market_pubkey = Pubkey::new_unique();
        let user_transfer_authority_pubkey = Pubkey::new_unique();
        let instruction = deposit_reserve_liquidity(
            program_id,
            liquidity_amount,
            source_liquidity_pubkey,
            destination_collateral_pubkey,
            reserve_pubkey,
            reserve_liquidity_supply_pubkey,
            reserve_collateral_mint_pubkey,
            lending_market_pubkey,
            user_transfer_authority_pubkey,
        );
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 10);
        assert_eq!(
            instruction.data,
            LendingInstruction::DepositReserveLiquidity { liquidity_amount }.pack()
        );
    }

    #[test]
    fn test_redeem_reserve_collateral() {
        let program_id = Pubkey::new_unique();
        let collateral_amount = u64::MAX;
        let source_collateral_pubkey = Pubkey::new_unique();
        let destination_liquidity_pubkey = Pubkey::new_unique();
        let reserve_pubkey = Pubkey::new_unique();
        let reserve_collateral_mint_pubkey = Pubkey::new_unique();
        let reserve_liquidity_supply_pubkey = Pubkey::new_unique();
        let lending_market_pubkey = Pubkey::new_unique();
        let user_transfer_authority_pubkey = Pubkey::new_unique();
        let instruction = redeem_reserve_collateral(
            program_id,
            collateral_amount,
            source_collateral_pubkey,
            destination_liquidity_pubkey,
            reserve_pubkey,
            reserve_collateral_mint_pubkey,
            reserve_liquidity_supply_pubkey,
            lending_market_pubkey,
            user_transfer_authority_pubkey,
        );
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 10);
        assert_eq!(
            instruction.data,
            LendingInstruction::RedeemReserveCollateral { collateral_amount }.pack()
        );
    }

    #[test]
    fn test_init_obligation() {
        let program_id = Pubkey::new_unique();
        let obligation_pubkey = Pubkey::new_unique();
        let lending_market_pubkey = Pubkey::new_unique();
        let obligation_owner_pubkey = Pubkey::new_unique();
        let instruction = init_obligation(
            program_id,
            obligation_pubkey,
            lending_market_pubkey,
            obligation_owner_pubkey,
        );
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 6);
        assert_eq!(instruction.data, LendingInstruction::InitObligation.pack());
    }

    #[test]
    fn test_refresh_obligation() {
        let program_id = Pubkey::new_unique();
        let obligation_pubkey = Pubkey::new_unique();
        let reserve_pubkeys = vec![Pubkey::new_unique()];
        let instruction = refresh_obligation(program_id, obligation_pubkey, reserve_pubkeys);
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 3);
        assert_eq!(
            instruction.data,
            LendingInstruction::RefreshObligation.pack()
        );
    }

    #[test]
    fn test_deposit_obligation_collateral() {
        let program_id = Pubkey::new_unique();
        let collateral_amount = u64::MAX;
        let source_collateral_pubkey = Pubkey::new_unique();
        let destination_collateral_pubkey = Pubkey::new_unique();
        let deposit_reserve_pubkey = Pubkey::new_unique();
        let obligation_pubkey = Pubkey::new_unique();
        let lending_market_pubkey = Pubkey::new_unique();
        let obligation_owner_pubkey = Pubkey::new_unique();
        let user_transfer_authority_pubkey = Pubkey::new_unique();
        let instruction = deposit_obligation_collateral(
            program_id,
            collateral_amount,
            source_collateral_pubkey,
            destination_collateral_pubkey,
            deposit_reserve_pubkey,
            obligation_pubkey,
            lending_market_pubkey,
            obligation_owner_pubkey,
            user_transfer_authority_pubkey,
        );
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 9);
        assert_eq!(
            instruction.data,
            LendingInstruction::DepositObligationCollateral { collateral_amount }.pack()
        );
    }

    #[test]
    fn test_withdraw_obligation_collateral() {
        let program_id = Pubkey::new_unique();
        let collateral_amount = u64::MAX;
        let source_collateral_pubkey = Pubkey::new_unique();
        let destination_collateral_pubkey = Pubkey::new_unique();
        let withdraw_reserve_pubkey = Pubkey::new_unique();
        let obligation_pubkey = Pubkey::new_unique();
        let lending_market_pubkey = Pubkey::new_unique();
        let obligation_owner_pubkey = Pubkey::new_unique();
        let instruction = withdraw_obligation_collateral(
            program_id,
            collateral_amount,
            source_collateral_pubkey,
            destination_collateral_pubkey,
            withdraw_reserve_pubkey,
            obligation_pubkey,
            lending_market_pubkey,
            obligation_owner_pubkey,
        );
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 9);
        assert_eq!(
            instruction.data,
            LendingInstruction::WithdrawObligationCollateral { collateral_amount }.pack()
        );
    }

    #[test]
    fn test_borrow_obligation_liquidity() {
        let program_id = Pubkey::new_unique();
        let liquidity_amount = u64::MAX;
        let source_liquidity_pubkey = Pubkey::new_unique();
        let destination_liquidity_pubkey = Pubkey::new_unique();
        let borrow_reserve_pubkey = Pubkey::new_unique();
        let borrow_reserve_liquidity_fee_receiver_pubkey = Pubkey::new_unique();
        let obligation_pubkey = Pubkey::new_unique();
        let lending_market_pubkey = Pubkey::new_unique();
        let obligation_owner_pubkey = Pubkey::new_unique();
        let host_fee_receiver_pubkey = Some(Pubkey::new_unique());
        let instruction = borrow_obligation_liquidity(
            program_id,
            liquidity_amount,
            source_liquidity_pubkey,
            destination_liquidity_pubkey,
            borrow_reserve_pubkey,
            borrow_reserve_liquidity_fee_receiver_pubkey,
            obligation_pubkey,
            lending_market_pubkey,
            obligation_owner_pubkey,
            host_fee_receiver_pubkey,
        );
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 11);
        assert_eq!(
            instruction.data,
            LendingInstruction::BorrowObligationLiquidity { liquidity_amount }.pack()
        );
    }

    #[test]
    fn test_repay_obligation_liquidity() {
        let program_id = Pubkey::new_unique();
        let liquidity_amount = u64::MAX;
        let source_liquidity_pubkey = Pubkey::new_unique();
        let destination_liquidity_pubkey = Pubkey::new_unique();
        let repay_reserve_pubkey = Pubkey::new_unique();
        let obligation_pubkey = Pubkey::new_unique();
        let lending_market_pubkey = Pubkey::new_unique();
        let user_transfer_authority_pubkey = Pubkey::new_unique();
        let instruction = repay_obligation_liquidity(
            program_id,
            liquidity_amount,
            source_liquidity_pubkey,
            destination_liquidity_pubkey,
            repay_reserve_pubkey,
            obligation_pubkey,
            lending_market_pubkey,
            user_transfer_authority_pubkey,
        );
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 8);
        assert_eq!(
            instruction.data,
            LendingInstruction::RepayObligationLiquidity { liquidity_amount }.pack()
        );
    }

    #[test]
    fn test_liquidate_obligation() {
        let program_id = Pubkey::new_unique();
        let liquidity_amount = u64::MAX;
        let source_liquidity_pubkey = Pubkey::new_unique();
        let destination_collateral_pubkey = Pubkey::new_unique();
        let repay_reserve_pubkey = Pubkey::new_unique();
        let repay_reserve_liquidity_supply_pubkey = Pubkey::new_unique();
        let withdraw_reserve_pubkey = Pubkey::new_unique();
        let withdraw_reserve_collateral_supply_pubkey = Pubkey::new_unique();
        let obligation_pubkey = Pubkey::new_unique();
        let lending_market_pubkey = Pubkey::new_unique();
        let user_transfer_authority_pubkey = Pubkey::new_unique();
        let instruction = liquidate_obligation(
            program_id,
            liquidity_amount,
            source_liquidity_pubkey,
            destination_collateral_pubkey,
            repay_reserve_pubkey,
            repay_reserve_liquidity_supply_pubkey,
            withdraw_reserve_pubkey,
            withdraw_reserve_collateral_supply_pubkey,
            obligation_pubkey,
            lending_market_pubkey,
            user_transfer_authority_pubkey,
        );
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 12);
        assert_eq!(
            instruction.data,
            LendingInstruction::LiquidateObligation { liquidity_amount }.pack()
        );
    }

    #[test]
    fn test_flash_loan() {
        let program_id = Pubkey::new_unique();
        let amount = u64::MAX;
        let source_liquidity_pubkey = Pubkey::new_unique();
        let destination_liquidity_pubkey = Pubkey::new_unique();
        let repay_reserve_pubkey = Pubkey::new_unique();
        let reserve_liquidity_fee_receiver_pubkey = Pubkey::new_unique();
        let host_fee_receiver_pubkey = Pubkey::new_unique();
        let lending_market_pubkey = Pubkey::new_unique();
        let flash_loan_receiver_program_id = Pubkey::new_unique();
        let account_meta = AccountMeta {
            pubkey: Pubkey::new_unique(),
            is_signer: true,
            is_writable: false,
        };
        let flash_loan_receiver_program_accounts = vec![account_meta];
        let instruction = flash_loan(
            program_id,
            amount,
            source_liquidity_pubkey,
            destination_liquidity_pubkey,
            repay_reserve_pubkey,
            reserve_liquidity_fee_receiver_pubkey,
            host_fee_receiver_pubkey,
            lending_market_pubkey,
            flash_loan_receiver_program_id,
            flash_loan_receiver_program_accounts,
        );
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 10);
        assert_eq!(
            instruction.data,
            LendingInstruction::FlashLoan { amount }.pack()
        );
    }
}
