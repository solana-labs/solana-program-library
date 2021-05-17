//! Voter Record Account

use solana_program::pubkey::Pubkey;

use super::enums::{GovernanceAccountType, GoverningTokenType};

/// Governance Voter Record
/// Account PDA seeds: ['governance', realm, token_mint, token_owner ]
#[repr(C)]
pub struct VoterRecord {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// The Realm the VoterRecord belongs to
    pub realm: Pubkey,

    /// The type of the Governing Token the VoteRecord is for
    pub token_type: GoverningTokenType,

    /// The owner (either single or multisig) of the deposited governing SPL Tokens
    /// This is who can authorize a withdrawal
    pub token_owner: Pubkey,

    /// The amount of governing tokens deposited into the Realm
    /// This amount is the voter weight used when voting on proposals
    pub token_deposit_amount: u64,

    /// A single account that is allowed to operate governance with the deposited governing tokens
    /// It's delegated to by the token owner
    pub vote_authority: Pubkey,

    /// The number of active votes cast by voter
    pub active_votes_count: u8,

    /// The total number of votes cast by the voter
    pub total_votes_count: u8,
}
