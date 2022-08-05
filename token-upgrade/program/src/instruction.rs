//! Program instructions

use {
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    spl_token_2022::pod::OptionalNonZeroPubkey,
};

/// Instructions supported by the TokenUpgrade program
#[derive(Clone, Copy, Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum TokenUpgradeInstruction {
    /// Creates an upgrade factory for the given mint address. The destination
    /// token mint authority must be set to
    /// `get_factory_mint_authority_address(source_mint, token_upgrade_program_id)`.
    ///
    /// Returns an error if the account exists.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable,signer]` Funding account (must be a system account)
    ///   1. `[writeable]` PDA factory account to be created
    ///   2. `[]` Source token mint
    ///   3. `[signer]` Source token mint authority
    ///   4. `[]` Destination token mint
    ///   5. `[]` System program
    ///
    /// Data expected by this instruction:
    ///   `CreateFactoryInstructionData`
    ///
    CreateFactory,
    /// Burns all of the source tokens in the user's account, and mints the same
    /// amount of destination tokens into another account.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable]` Source token account to burn from
    ///   1. `[writeable]` Source token mint
    ///   2. `[writeable]` Destination token account to mint into
    ///   3. `[writeable]` Destination token mint
    ///   4. `[]` PDA upgrade factory account for the source mint, given by:
    ///        `get_factory_address(source_mint, program_id)`. Must be initialized.
    ///   5. `[signer]` Source token account transfer authority (owner or delegate)
    ///   6. `[]` PDA mint authority for the destination mint, given by:
    ///        `get_factory_mint_authority_address(source_mint, program_id)`
    ///   7. `[]` SPL Token program for source mint
    ///   8. `[]` SPL Token program for destination mint
    ///
    /// Data expected by this instruction:
    ///   None
    ///
    UpgradeTokens,
    /// Sets the reclaim mint authority on the destination mint, specified in
    /// the factory.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable]` Destination token mint
    ///   1. `[]` PDA upgrade factory account for the source mint, given by:
    ///        `get_factory_address(source_mint, program_id)`. Must be initialized.
    ///   2. `[]` PDA mint authority for the destination mint, given by:
    ///        `get_factory_mint_authority_address(source_mint, program_id)`
    ///   3. `[signer]` The reclaim mint authority on the factory
    ///   4. `[]` SPL Token program for destination mint
    ///
    /// Data expected by this instruction:
    ///   `SetDestinationMintAuthorityInstructionData`
    ///
    SetDestinationMintAuthority,
}

/// Data expected by `TokenUpgradeInstruction::CreateFactory`
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(transparent)]
pub struct CreateFactoryInstructionData {
    /// Authority that can reset the destination mint authority
    pub set_mint_authority: OptionalNonZeroPubkey,
}

/// Data expected by `TokenUpgradeInstruction::SetDestinationMintAuthority`
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(transparent)]
pub struct SetDestinationMintAuthorityInstructionData {
    /// New destination mint authority
    pub new_destination_mint_authority: OptionalNonZeroPubkey,
}
