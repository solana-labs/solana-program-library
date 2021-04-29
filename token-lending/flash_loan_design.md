# Flash Loan Design

We added a new instruction with the following signature for flash loan:
```rust
    /// Make a flash loan.
    ///   0. `[writable]` Source liquidity (reserve liquidity supply), minted by reserve liquidity mint
    ///   1. `[writable]` Destination liquidity (owned by the flash loan receiver program)
    ///   2. `[writable]` Reserve account.
    ///   3. `[]` Lending market account.
    ///   4. `[]` Derived lending market authority.
    ///   5. `[]` Flash Loan Receiver Program Account, which should have a function (which we will
    ///   call it `ExecuteOperation(amount: u64)` to mimic Aave flash loan) that has tag of 0.
    ///   6. `[]` Flash Loan Receiver Program Derived Account
    ///   7. `[]` Token program id
    ///   8. `[writable]` Flash loan fees receiver, must be the fee account specified at InitReserve.
    ///   9. `[writeable]` Host fee receiver.
    /// ... a variable number of accounts that is needed for `executeOperation(amount: u64)`.
    FlashLoan {
        /// The amount that is to be borrowed
        amount: u64,
    },
```
In the implementation, we do the following in order (omit the usual account safety check for brevity):
1. Transfer the reserve liquidity to the destination liquidity account owned by the flash loan receiver program if possible (if the request liquidity is too large, or the destination liquidity program is not owned by the flash loan receiver program, then we abort the transaction)
2. Call the `executeOperation` function (the flash loan receiver base is required to have this function with tag 0), and the account required is given from the 8th account of the account required of `FlashLoan` function.
3. Check that the returned amount with the fee is in the reserve account after the completion of `executeOperation` function.

The flash loan receiver program should have the following instruction, that executes the operation before returning the fund. This function is also responsible for returning the fund back to the reserve.

```rust
pub enum FlashLoanReceiverInstruction {
	
/// Execute the operation that is needed after flash loan
    	///
    	/// Accounts expected:
    	///
        ///   0. `[writable]` Source liquidity (matching the destination from above)
        ///   1. `[writable]` Destination liquidity (matching the source from above)
        ///   2. Token program id
        ///    .. Additional accounts from above

	ExecuteOperation{
		// Amount that is loaned to the receiver program
        amount: u64
    }
}

```
