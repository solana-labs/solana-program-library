//! Instruction types

/// Specifies the financial specifics of a token.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TokenInfo {
    /// Total supply of tokens.
    pub supply: u64,
    /// Number of base 10 digits to the right of the decimal place in the total supply.
    pub decimals: u64,
}

/// Instructions supported by the token program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    /// Creates a new token and deposit all the newly minted tokens in an account.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[writable, signer]` New token to create.
    ///   1.
    ///      * If supply is non-zero: `[writable]` Account to hold all the newly minted tokens.
    ///      * If supply is zero: `[]` Owner of the token.
    ///   2. Optional: `[]` Owner of the token if supply is non-zero, if present then the token allows further minting of tokens.
    NewToken(TokenInfo),
    /// Creates a new account.  The new account can either hold tokens or be a delegate
    /// for another account.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[writable, signer]`  New account being created.
    ///   1. `[]` Owner of the new account.
    ///   2. `[]` Token this account will be associated with.
    ///   3. Optional: `[]` Source account that this account will be a delegate for.
    NewAccount,
    /// Transfers tokens from one account to another either directly or via a delegate.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[signer]` Owner of the source account.
    ///   1. `[writable]` Source/Delegate account.
    ///   2. `[writable]` Destination account.
    ///   3. Optional: `[writable]` Source account if key 1 is a delegate account.
    Transfer(u64),
    /// Approves a delegate.  A delegate account is given the authority to transfer
    /// another accounts tokens without the other account's owner signing the transfer.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[signer]` Owner of the source account.
    ///   1. `[]` Source account.
    ///   2. `[writable]` Delegate account.
    Approve(u64),
    /// Sets a new owner of an account.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[signer]` Current owner of the account.
    ///   1. `[writable]` account to change the owner of.
    ///   2. `[]` New owner of the account.
    SetOwner,
    /// Mints new tokens to an account.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[signer]` Owner of the token.
    ///   1. `[writable]` Token to mint.
    ///   2. `[writable]` Account to mint tokens to.
    MintTo(u64),
    /// Burns tokens by removing them from an account and the total supply.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[signer]` Owner of the account to burn from.
    ///   1. `[writable]` Account to burn from.
    ///   2. `[writable]` Token being burned.
    Burn(u64),
}
