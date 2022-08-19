//! State transition types

use {
    bytemuck::{Pod, Zeroable},
    solana_program::pubkey::Pubkey,
};

/// Upgrade Factory data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct UpgradeFactory {
    /// Pre-minted bag of new tokens for users seeking to upgrade
    pub pre_minted_token_account: Pubkey,
}
