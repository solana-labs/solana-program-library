//! Proposal Vote Record Account

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;
use solana_program::{program_pack::IsInitialized, pubkey::Pubkey};

use crate::error::GovernanceError;
use crate::tools::account::get_account_data;
use crate::{tools::account::AccountMaxSize, PROGRAM_AUTHORITY_SEED};

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

    /// Indicates whether the vote was relinquished by voter
    pub is_relinquished: bool,

    /// Voter's vote: Yes/No and amount
    pub vote_weight: VoteWeight,
}

impl AccountMaxSize for VoteRecord {}

impl IsInitialized for VoteRecord {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::VoteRecord
    }
}
impl VoteRecord {
    /// Checks the vote can be relinquished
    pub fn assert_can_relinquish_vote(&self) -> Result<(), ProgramError> {
        if self.is_relinquished {
            return Err(GovernanceError::VoteAlreadyRelinquished.into());
        }

        Ok(())
    }
}

/// Deserializes VoteRecord account and checks owner program
pub fn get_vote_record_data(
    program_id: &Pubkey,
    vote_record_info: &AccountInfo,
) -> Result<VoteRecord, ProgramError> {
    get_account_data::<VoteRecord>(vote_record_info, program_id)
}

/// Deserializes VoteRecord and checks it belongs to the provided Proposal and Governing Token Owner
pub fn get_vote_record_data_for_proposal_and_token_owner(
    program_id: &Pubkey,
    vote_record_info: &AccountInfo,
    proposal: &Pubkey,
    governing_token_owner: &Pubkey,
) -> Result<VoteRecord, ProgramError> {
    let vote_record_data = get_vote_record_data(program_id, vote_record_info)?;

    if vote_record_data.proposal != *proposal {
        return Err(GovernanceError::InvalidProposalForVoterRecord.into());
    }

    if vote_record_data.governing_token_owner != *governing_token_owner {
        return Err(GovernanceError::InvalidGoverningTokenOwnerForVoteRecord.into());
    }

    Ok(vote_record_data)
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
pub fn get_vote_record_address<'a>(
    program_id: &Pubkey,
    proposal: &'a Pubkey,
    token_owner_record: &'a Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_vote_record_address_seeds(proposal, token_owner_record),
        program_id,
    )
    .0
}
