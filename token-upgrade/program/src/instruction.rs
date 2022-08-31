//! Program instructions

use num_enum::{IntoPrimitive, TryFromPrimitive};

/// Instructions supported by the TokenUpgrade program
#[derive(Clone, Copy, Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum TokenUpgradeInstruction {
    /// Burns all of the source tokens in the user's account, and transfers the same
    /// amount of tokens from an account owned by a PDA into another account.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable]` Source token account to burn from
    ///   1. `[writeable]` Source token mint
    ///   2. `[writeable]` Bag of destination tokens held by or delegated to program escrow:
    ///       `get_token_upgrade_authority_address(source_mint, destination_mint, program_id)`
    ///   3. `[writeable]` Destination token account to transfer into
    ///   4. `[]` Destination token mint
    ///   5. `[]` Transfer authority (owner or delegate) of destination token bag held in program escrow, must be:
    ///       `get_token_upgrade_authority_address(source_mint, destination_mint, program_id)`
    ///   6. `[]` SPL Token program for source mint
    ///   7. `[]` SPL Token program for destination mint
    ///   8. `[]` Source token account transfer authority (owner or delegate)
    ///   9. ..9+M `[signer]` M multisig signer accounts
    ///
    /// Data expected by this instruction:
    ///   None
    ///
    Exchange,
}
