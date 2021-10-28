//! Proposal Vote Record Account

use std::io::Write;

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::account_info::AccountInfo;
use solana_program::borsh::try_from_slice_unchecked;

use solana_program::program_error::ProgramError;
use solana_program::{program_pack::IsInitialized, pubkey::Pubkey};
use spl_governance_tools::account::{get_account_data, AccountMaxSize};

use crate::error::GovernanceError;

use crate::PROGRAM_AUTHORITY_SEED;

use crate::state::enums::GovernanceAccountType;

/// Voter choice for a proposal option
/// In the current version only 1) Single choice and 2) Multiple choices proposals are supported
/// In the future versions we can add support for 1) Quadratic voting, 2) Ranked choice voting and 3) Weighted voting
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct VoteChoice {
    /// The rank given to the choice by voter
    /// Note: The filed is not used in the current version
    pub rank: u8,

    /// The voter's weight percentage given by the voter to the choice
    pub weight_percentage: u8,
}

impl VoteChoice {
    /// Returns the choice weight given the voter's weight
    pub fn get_choice_weight(&self, voter_weight: u64) -> Result<u64, ProgramError> {
        Ok(match self.weight_percentage {
            100 => voter_weight,
            0 => 0,
            _ => return Err(GovernanceError::InvalidVoteChoiceWeightPercentage.into()),
        })
    }
}

/// User's vote
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum Vote {
    /// Vote approving choices
    Approve(Vec<VoteChoice>),

    /// Vote rejecting proposal
    Deny,
}

/// Proposal VoteRecord
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

    /// The weight of the user casting the vote
    pub voter_weight: u64,

    /// Voter's vote
    pub vote: Vote,
}

impl AccountMaxSize for VoteRecord {}

impl IsInitialized for VoteRecord {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::VoteRecordV2
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

    /// Serializes account into the target buffer
    pub fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), ProgramError> {
        if self.account_type == GovernanceAccountType::VoteRecordV2 {
            BorshSerialize::serialize(&self, writer)?
        } else if self.account_type == GovernanceAccountType::VoteRecord {
            // For V1 translate the account back to the original format
            let vote_weight = match self.vote.clone() {
                Vote::Approve(_options) => {
                    spl_governance_v1::state::enums::VoteWeight::Yes(self.voter_weight)
                }
                Vote::Deny => spl_governance_v1::state::enums::VoteWeight::No(self.voter_weight),
            };

            let vote_record_data_v1 = spl_governance_v1::state::vote_record::VoteRecord {
                account_type: spl_governance_v1::state::enums::GovernanceAccountType::VoteRecord,
                proposal: self.proposal,
                governing_token_owner: self.governing_token_owner,
                is_relinquished: self.is_relinquished,
                vote_weight,
            };

            BorshSerialize::serialize(&vote_record_data_v1, writer)?;
        }

        Ok(())
    }
}

/// Deserializes VoteRecord account and checks owner program
pub fn get_vote_record_data(
    program_id: &Pubkey,
    vote_record_info: &AccountInfo,
) -> Result<VoteRecord, ProgramError> {
    let account_type: GovernanceAccountType =
        try_from_slice_unchecked(&vote_record_info.data.borrow())?;

    // If the account is V1 version then translate to V2
    if account_type == GovernanceAccountType::VoteRecord {
        let vote_record_data_v1 = get_account_data::<
            spl_governance_v1::state::vote_record::VoteRecord,
        >(program_id, vote_record_info)?;

        let (vote, voter_weight) = match vote_record_data_v1.vote_weight {
            spl_governance_v1::state::enums::VoteWeight::Yes(weight) => (
                Vote::Approve(vec![VoteChoice {
                    rank: 0,
                    weight_percentage: 100,
                }]),
                weight,
            ),
            spl_governance_v1::state::enums::VoteWeight::No(weight) => (Vote::Deny, weight),
        };

        return Ok(VoteRecord {
            account_type: GovernanceAccountType::VoteRecord,
            proposal: vote_record_data_v1.proposal,
            governing_token_owner: vote_record_data_v1.governing_token_owner,
            is_relinquished: vote_record_data_v1.is_relinquished,
            voter_weight,
            vote,
        });
    }

    get_account_data::<VoteRecord>(program_id, vote_record_info)
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

#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};

    use borsh::BorshSerialize;
    use solana_program::clock::Epoch;

    use super::*;

    #[test]
    fn test_vote_record_v1_to_v2_serialisation_roundtrip() {
        // Arrange

        let vote_record_v1_source = spl_governance_v1::state::vote_record::VoteRecord {
            account_type: spl_governance_v1::state::enums::GovernanceAccountType::VoteRecord,
            proposal: Pubkey::new_unique(),
            governing_token_owner: Pubkey::new_unique(),
            is_relinquished: true,
            vote_weight: spl_governance_v1::state::enums::VoteWeight::Yes(120),
        };

        let mut vote_record_v1_source_vec = vec![];
        vote_record_v1_source
            .serialize(&mut vote_record_v1_source_vec)
            .unwrap();

        let program_id = Pubkey::new_unique();

        let info_key = Pubkey::new_unique();
        let mut lamports = 10u64;

        let mut vote_record_v1_info = AccountInfo::new(
            &info_key,
            false,
            false,
            &mut lamports,
            &mut vote_record_v1_source_vec[..],
            &program_id,
            false,
            Epoch::default(),
        );

        // Act

        let vote_record_v2 = get_vote_record_data(&program_id, &vote_record_v1_info).unwrap();

        let mut vote_record_v1_target_vec = vec![];

        vote_record_v2
            .serialize(&mut vote_record_v1_target_vec)
            .unwrap();

        // Assert

        vote_record_v1_info.data = Rc::new(RefCell::new(&mut vote_record_v1_target_vec));

        let vote_record_v1_target = get_account_data::<
            spl_governance_v1::state::vote_record::VoteRecord,
        >(&program_id, &vote_record_v1_info)
        .unwrap();

        assert_eq!(vote_record_v1_source, vote_record_v1_target)
    }
}
