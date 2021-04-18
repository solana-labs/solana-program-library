//! Error types

use num_derive::FromPrimitive;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

/// Errors that may be returned by the TokenLending program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum LendingError {
    // 0
    /// Invalid instruction data passed in.
    #[error("Failed to unpack instruction data")]
    InstructionUnpackError,
    /// The account cannot be initialized because it is already in use.
    #[error("Account is already initialized")]
    AlreadyInitialized,
    /// Lamport balance below rent-exempt threshold.
    #[error("Lamport balance below rent-exempt threshold")]
    NotRentExempt,
    /// The program address provided doesn't match the value generated by the program.
    #[error("Market authority is invalid")]
    InvalidMarketAuthority,
    /// Expected a different market owner
    #[error("Market owner is invalid")]
    InvalidMarketOwner,

    // 5
    /// The owner of the input isn't set to the program address generated by the program.
    #[error("Input account owner is not the program address")]
    InvalidAccountOwner,
    /// The owner of the account input isn't set to the correct token program id.
    #[error("Input token account is not owned by the correct token program id")]
    InvalidTokenOwner,
    /// Expected an SPL Token mint
    #[error("Input token mint account is not valid")]
    InvalidTokenMint,
    /// Expected a different SPL Token program
    #[error("Input token program account is not valid")]
    InvalidTokenProgram,
    /// Invalid amount, must be greater than zero
    #[error("Input amount is invalid")]
    InvalidAmount,

    // 10
    /// Invalid config value
    #[error("Input config value is invalid")]
    InvalidConfig,
    /// Invalid config value
    #[error("Input account must be a signer")]
    InvalidSigner,
    /// Invalid account input
    #[error("Invalid account input")]
    InvalidAccountInput,
    /// Math operation overflow
    #[error("Math operation overflow")]
    MathOverflow,
    /// Negative interest rate
    #[error("Interest rate is negative")]
    NegativeInterestRate,

    // 15
    /// Memory is too small
    #[error("Memory is too small")]
    MemoryTooSmall,
    /// The reserve lending market must be the same
    #[error("Reserve mints do not match dex market mints")]
    DexMarketMintMismatch,
    /// The reserve lending market must be the same
    #[error("Reserve lending market mismatch")]
    LendingMarketMismatch,
    /// The obligation token owner must be the same if reusing an obligation
    #[error("Obligation token owner mismatch")]
    ObligationTokenOwnerMismatch,
    /// Insufficient liquidity available
    #[error("Insufficient liquidity available")]
    InsufficientLiquidity,

    // 20
    /// This reserve's collateral cannot be used for borrows
    #[error("Input reserve has collateral disabled")]
    ReserveCollateralDisabled,
    /// Input reserves cannot be the same
    #[error("Input reserves cannot be the same")]
    DuplicateReserve,
    /// Input reserves cannot use the same liquidity mint
    #[error("Input reserves cannot use the same liquidity mint")]
    DuplicateReserveMint,
    /// Obligation amount is empty
    #[error("Obligation amount is empty")]
    ObligationEmpty,
    /// Cannot liquidate healthy obligations
    #[error("Cannot liquidate healthy obligations")]
    HealthyObligation,

    // 25
    /// Borrow amount too small
    #[error("Borrow amount too small")]
    BorrowTooSmall,
    /// Liquidation amount too small
    #[error("Liquidation amount too small to receive collateral")]
    LiquidationTooSmall,
    /// Reserve state stale
    #[error("Reserve state needs to be updated for the current slot")]
    ReserveStale,
    /// Trade simulation error
    #[error("Trade simulation error")]
    TradeSimulationError,
    /// Invalid dex order book side
    #[error("Invalid dex order book side")]
    DexInvalidOrderBookSide,

    // 30
    /// Token initialize mint failed
    #[error("Token initialize mint failed")]
    TokenInitializeMintFailed,
    /// Token initialize account failed
    #[error("Token initialize account failed")]
    TokenInitializeAccountFailed,
    /// Token transfer failed
    #[error("Token transfer failed")]
    TokenTransferFailed,
    /// Token mint to failed
    #[error("Token mint to failed")]
    TokenMintToFailed,
    /// Token burn failed
    #[error("Token burn failed")]
    TokenBurnFailed,

    // 35
    /// Invalid obligation collateral amount
    #[error("Invalid obligation collateral amount")]
    InvalidObligationCollateral,
    /// Obligation collateral is already below required amount
    #[error("Obligation collateral is already below required amount")]
    ObligationCollateralBelowRequired,
    /// Obligation collateral cannot be withdrawn below required amount
    #[error("Obligation collateral cannot be withdrawn below required amount")]
    ObligationCollateralWithdrawBelowRequired,
    /// Error invoking flash loan receiver
    #[error("Error invoking flash loan receiver.")]
    InvokingFlashLoanReceiverFailed,
}

impl From<LendingError> for ProgramError {
    fn from(e: LendingError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for LendingError {
    fn type_of() -> &'static str {
        "Lending Error"
    }
}
