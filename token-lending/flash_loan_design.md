# Flash Loan Design

We added a new instruction with the following signature for flash loan:
```rust
pub enum LendingInstruction {
    // ....
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
    ///   3. `[]` Lending market account.
    ///   4. `[]` Derived lending market authority.
    ///   5. `[]` Flash loan receiver program account.
    ///             Must implement an instruction that has tag of 0 and a signature of `(repay_amount: u64)`
    ///             This instruction must return the amount to the source liquidity account.
    ///   6. `[]` Token program id.
    ///   7. `[writable]` Flash loan fee receiver account.
    ///                     Must match the reserve liquidity fee receiver.
    ///   8. `[writable]` Host fee receiver.
    ///   .. `[any]` Additional accounts expected by the receiving program's `ReceiveFlashLoan` instruction.
    FlashLoan {
        /// The amount that is to be borrowed
        amount: u64,
    },
}
```

In the implementation, we do the following in order:

1. Perform safety checks and calculate fees
2. Transfer `amount` from the source liquidity account to the destination liquidity account
2. Call the `ReceiveFlashLoan` function (the flash loan receiver program is required to have this function with tag `0`).
   The additional account required for `ReceiveFlashLoan` is given from the 10th account of the `FlashLoan` instruction, i.e. after host fee receiver.
3. Check that the returned amount with the fee is in the reserve account after the completion of `ReceiveFlashLoan` function.

The flash loan receiver program should have a `ReceiveFlashLoan` instruction which executes the user-defined operation and return the funds to the reserve in the end.

```rust
pub enum FlashLoanReceiverInstruction {
	
    /// Receive a flash loan and perform user-defined operation and finally return the fund back.
    ///
    /// Accounts expected:
    ///
    ///   0. `[writable]` Source liquidity (matching the destination from above).
    ///   1. `[writable]` Destination liquidity (matching the source from above).
    ///   2. `[]` Token program id
    ///   .. `[any]` Additional accounts provided to the lending program's `FlashLoan` instruction above.
    ReceiveFlashLoan {
		// Amount that is loaned to the receiver program
        amount: u64
    }
}

```

You can view a sample implementation [here](https://github.com/solana-labs/solana-program-library/tree/master/token-lending/program/tests/helpers/flash_loan_receiver.rs).
