//! State transition types

use {
    bytemuck::{Pod, Zeroable},
    solana_program::pubkey::Pubkey,
    spl_token_2022::pod::OptionalNonZeroPubkey,
};

/// Upgrade Factory data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct UpgradeFactory {
    /// Mint to be used for all new tokens created via upgrade
    pub destination_mint: Pubkey,
    /// Authority that can set minting authority on the destination mint
    pub set_mint_authority: OptionalNonZeroPubkey,
}
