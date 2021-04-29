# Flash Loan Design

We added a new instruction with the following signature for flash loan:
```rust
    // 12
    /// Make a flash loan.
    ///
    ///   0. `[writable]` Destination liquidity token account, minted by reserve liquidity mint.
    ///   1. `[writable]` Reserve account.
    ///   2. `[]` Lending market account.
    ///   3. `[]` Derived lending market authority.
    ///   4. `[]` Temporary memory
    ///   5. `[]` Flash Loan Receiver Program Account, which should have a function (which we will
    ///   call it `ExecuteOperation(amount: u64)` to mimic Aave flash loan) that has tag of 0.
    ///   6. `[]` Flash Loan Receiver Program Derived Account
    ///   7. `[]` Token program id
    ///   8. `[writable]` Host fee receiver.
    ///   9. `[writeable]` Flash loan fees receiver, must match init reserve.
    /// ... a variable number of accounts that is needed for `executeOperation(amount: u64)`.
    ///
    ///   The flash loan receiver program that is to be invoked should contain an instruction with
    ///   tag `0` and accept the total amount that needs to be returned back after its execution
    ///   has completed.
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
    	/// 0. `[writable]` The destination liquidity token account owned by the PDA of the program.
	/// 1. `[]` The program derived account of flash loan receiver.
    	/// 2. `[writable]` The repay token account.
    	/// 3. `[]` The token program Id.
    	/// 4... `[writable]` the account that the FlashLoanReceiver needs to write to.


	ExecuteOperation{
		// Amount that is loaned to the receiver program
        amount: u64
    }
}

```
