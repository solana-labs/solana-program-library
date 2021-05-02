# Flash Loan Design

We added a new instruction with the following signature for flash loan:
```rust
pub enum LendingInstruction {
    // ....
    /// Make a flash loan.
    ///   0. `[writable]` Source liquidity (reserve liquidity supply), minted by reserve liquidity mint
    ///   1. `[writable]` Destination liquidity (owned by the flash loan receiver program)
    ///   2. `[writable]` Reserve account.
    ///   3. `[]` Lending market account.
    ///   4. `[]` Derived lending market authority.
    ///   5. `[]` Flash Loan Receiver Program Account, which should have a function `ReceiveFlashLoan` that has a tag of 0.
    ///   6. `[]` Token program id
    ///   7. `[writable]` Flash loan fees receiver, must be the fee account specified at InitReserve.
    ///   8. `[writeable]` Host fee receiver.
    ///   .. `[any]` Additional accounts expected by the flash loan receiver program
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
    ///   0. `[writable]` Source liquidity (matching the destination from above)
    ///   1. `[writable]` Destination liquidity (matching the source from above)
    ///   2. Token program id
    ///    .. Additional accounts from above
    ReceiveFlashLoan {
		// Amount that is loaned to the receiver program
        amount: u64
    }
}

```

Developer can find a sample implementation in `token-lending/program/tests/helpers/flash_loan_receiver.rs`.
