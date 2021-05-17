//! Realm Account

use solana_program::pubkey::Pubkey;

use super::enums::GovernanceAccountType;

/// Governance Realm Account
/// Account PDA seeds" ['governance', name]
#[repr(C)]
pub struct Realm {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Community mint
    pub community_mint: Pubkey,

    /// Council mint
    pub council_mint: Option<Pubkey>,

    /// Governance Realm name
    pub name: String,
}
