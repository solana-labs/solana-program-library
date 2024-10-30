//! Program instructions

use num_enum::{IntoPrimitive, TryFromPrimitive};

/// Instructions supported by the Token Wrap program
#[derive(Clone, Debug, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum TokenWrapInstruction {
    /// Create a wrapped token mint
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writeable,signer]` Funding account for mint and backpointer (must
    ///    be a system account)
    /// 1. `[writeable]` Unallocated wrapped mint account to create, address
    ///    must be: `get_wrapped_mint_address(unwrapped_mint_address,
    ///    wrapped_token_program_id)`
    /// 2. `[writeable]` Unallocated wrapped backpointer account to create
    ///    `get_wrapped_mint_backpointer_address(wrapped_mint_address)`
    /// 3. `[]` Existing unwrapped mint
    /// 4. `[]` System program
    /// 5. `[]` SPL Token program for wrapped mint
    ///
    /// Data expected by this instruction:
    ///   * bool: true = idempotent creation, false = non-idempotent creation
    CreateMint,

    /// Wrap tokens
    ///
    /// Move a user's unwrapped tokens into an escrow account and mint the same
    /// number of wrapped tokens into the provided account.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writeable]` Unwrapped token account to wrap
    /// 1. `[writeable]` Escrow of unwrapped tokens, must be owned by:
    ///    `get_wrapped_mint_authority(wrapped_mint_address)`
    /// 2. `[]` Unwrapped token mint
    /// 3. `[writeable]` Wrapped mint, must be initialized, address must be:
    ///    `get_wrapped_mint_address(unwrapped_mint_address,
    ///    wrapped_token_program_id)`
    /// 4. `[writeable]` Recipient wrapped token account
    /// 5. `[]` Escrow mint authority, address must be:
    ///    `get_wrapped_mint_authority(wrapped_mint)`
    /// 6. `[]` SPL Token program for unwrapped mint
    /// 7. `[]` SPL Token program for wrapped mint
    /// 8. `[signer]` Transfer authority on unwrapped token account
    /// 9. ..8+M. `[signer]` (Optional) M multisig signers on unwrapped token
    ///    account
    ///
    /// Data expected by this instruction:
    ///   * little-endian u64 representing the amount to wrap
    Wrap,

    /// Unwrap tokens
    ///
    /// Burn user wrapped tokens and transfer the same amount of unwrapped
    /// tokens from the escrow account to the provided account.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writeable]` Wrapped token account to unwrap
    /// 1. `[writeable]` Wrapped mint, address must be:
    ///    `get_wrapped_mint_address(unwrapped_mint_address,
    ///    wrapped_token_program_id)`
    /// 2. `[writeable]` Escrow of unwrapped tokens, must be owned by:
    ///    `get_wrapped_mint_authority(wrapped_mint_address)`
    /// 3. `[writeable]` Recipient unwrapped tokens
    /// 4. `[]` Unwrapped token mint
    /// 5. `[]` Escrow unwrapped token authority
    ///    `get_wrapped_mint_authority(wrapped_mint)`
    /// 6. `[]` SPL Token program for wrapped mint
    /// 7. `[]` SPL Token program for unwrapped mint
    /// 8. `[signer]` Transfer authority on wrapped token account
    /// 9. ..8+M. `[signer]` (Optional) M multisig signers on wrapped token
    ///    account
    ///
    /// Data expected by this instruction:
    ///   * little-endian u64 representing the amount to unwrap
    Unwrap,
}
