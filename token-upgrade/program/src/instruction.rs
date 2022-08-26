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
    ///   2. `[writeable]` Destination token upgrade account, owned by:
    ///       `get_token_upgrade_authority_address(source_mint, destination_mint, program_id)`
    ///   3. `[writeable]` Destination token account to transfer into
    ///   4. `[]` Destination token mint
    ///   5. `[signer]` Source token account transfer authority (owner or delegate)
    ///   6. `[]` PDA owner of token upgrade account, given by:
    ///       `get_token_upgrade_authority_address(source_mint, destination_mint, program_id)`
    ///   7. `[]` SPL Token program for source mint
    ///   8. `[]` SPL Token program for destination mint
    ///
    /// Data expected by this instruction:
    ///   None
    ///
    UpgradeTokens,
}
