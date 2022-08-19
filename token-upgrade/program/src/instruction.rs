//! Program instructions

use num_enum::{IntoPrimitive, TryFromPrimitive};

/// Instructions supported by the TokenUpgrade program
#[derive(Clone, Copy, Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum TokenUpgradeInstruction {
    /// Creates an upgrade factory for the given mint.
    ///
    /// Returns an error if the account exists.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable,signer]` Funding account (must be a system account)
    ///   1. `[writeable]` PDA factory account to be created
    ///   2. `[]` Source token mint
    ///   3. `[signer]` Source token mint authority
    ///   4. `[]` Pre-minted token account, owned by `get_factory_token_account_authority_address(factory_address, token_upgrade_program_id)`.
    ///   5. `[]` System program
    ///
    /// Data expected by this instruction:
    ///   None
    ///
    CreateFactory,
    /// Burns all of the source tokens in the user's account, and transfers the same
    /// amount of pre-minted tokens into another account.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable]` Source token account to burn from
    ///   1. `[writeable]` Source token mint
    ///   2. `[writeable]` Pre-minted token account
    ///   3. `[writeable]` Destination token account to transfer into
    ///   4. `[]` Destination token mint
    ///   5. `[]` PDA upgrade factory account for the source mint, given by:
    ///        `get_factory_address(source_mint, program_id)`. Must be initialized.
    ///   6. `[signer]` Source token account transfer authority (owner or delegate)
    ///   7. `[]` PDA token account authority for the factory, given by:
    ///        `get_factory_token_account_authority_address(factory_address, program_id)`
    ///   8. `[]` SPL Token program for source mint
    ///   9. `[]` SPL Token program for destination mint
    ///
    /// Data expected by this instruction:
    ///   None
    ///
    UpgradeTokens,
}
