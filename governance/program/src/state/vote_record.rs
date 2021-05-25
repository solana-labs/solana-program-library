//! Proposal Vote Record Account

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{program_pack::IsInitialized, pubkey::Pubkey};

use crate::{id, tools::account::AccountMaxSize, PROGRAM_AUTHORITY_SEED};

use crate::state::enums::{GovernanceAccountType, VoteWeight};

/// Proposal VoteRecord
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct VoteRecord {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Proposal account
    pub proposal: Pubkey,

    /// The user who casted this vote
    /// This is the Governing Token Owner who deposited governing tokens into the Realm
    pub governing_token_owner: Pubkey,

    /// Voter's vote: Yes/No and amount
    pub vote_weight: VoteWeight,
}

impl AccountMaxSize for VoteRecord {}

impl IsInitialized for VoteRecord {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::VoteRecord
    }
}

/// Returns VoteRecord PDA seeds
pub fn get_vote_record_address_seeds<'a>(
    proposal: &'a Pubkey,
    token_owner_record: &'a Pubkey,
) -> [&'a [u8]; 3] {
    [
        PROGRAM_AUTHORITY_SEED,
        proposal.as_ref(),
        token_owner_record.as_ref(),
    ]
}

/// Returns VoteRecord PDA address
pub fn get_vote_record_address<'a>(proposal: &'a Pubkey, token_owner_record: &'a Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &get_vote_record_address_seeds(proposal, token_owner_record),
        &id(),
    )
    .0
}
