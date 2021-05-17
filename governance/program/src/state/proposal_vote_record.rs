//! Proposal Vote Record Account

use solana_program::pubkey::Pubkey;

use super::enums::{GovernanceAccountType, VoteWeight};

/// Proposal Vote Record
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProposalVoteRecord {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Proposal account
    pub proposal: Pubkey,

    /// The user who casted this vote
    /// This is the Governing Token Owner who deposited governing tokens into the Realm
    pub governing_token_owner: Pubkey,

    /// Voter's vote: Yes/No and amount
    pub vote: Option<VoteWeight>,
}
