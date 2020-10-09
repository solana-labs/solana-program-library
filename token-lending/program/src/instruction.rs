//! Instruction types

/// Instructions supported by the lending program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum LendingInstruction {
    /// Initializes a new lending pool.
    InitPool,
    // InitReserve,
    // Deposit,
    // Withdraw,
    // Borrow,
    // Repay,
    // Liquidate,
}
