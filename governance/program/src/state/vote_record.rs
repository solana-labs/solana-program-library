//! Proposal Vote Record Account

use borsh::maybestd::io::Write;

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::account_info::AccountInfo;
use solana_program::borsh::try_from_slice_unchecked;

use solana_program::program_error::ProgramError;
use solana_program::{program_pack::IsInitialized, pubkey::Pubkey};
use spl_governance_tools::account::{get_account_data, AccountMaxSize};

use crate::error::GovernanceError;

use crate::PROGRAM_AUTHORITY_SEED;

use crate::state::{
    enums::GovernanceAccountType,
    legacy::{VoteRecordV1, VoteWeightV1},
};

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

    /// Declare indifference to proposal
    /// Note: Not supported in the current version
    Abstain,

    /// Veto proposal
    /// Note: Not supported in the current version
    Veto,
}

/// Proposal VoteRecord
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct VoteRecordV2 {
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

    /// Reserved space for versions v2 and onwards
    /// Note: This space won't be available to v1 accounts until runtime supports resizing
    pub reserved_v2: [u8; 8],
}

impl AccountMaxSize for VoteRecordV2 {}

impl IsInitialized for VoteRecordV2 {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::VoteRecordV2
    }
}
impl VoteRecordV2 {
    /// Checks the vote can be relinquished
    pub fn assert_can_relinquish_vote(&self) -> Result<(), ProgramError> {
        if self.is_relinquished {
            return Err(GovernanceError::VoteAlreadyRelinquished.into());
        }

        Ok(())
    }

    /// Serializes account into the target buffer
    pub fn serialize<W: Write>(self, writer: &mut W) -> Result<(), ProgramError> {
        if self.account_type == GovernanceAccountType::VoteRecordV2 {
            BorshSerialize::serialize(&self, writer)?
        } else if self.account_type == GovernanceAccountType::VoteRecordV1 {
            // V1 account can't be resized and we have to translate it back to the original format

            // If reserved_v2 is used it must be individually asses for v1 backward compatibility impact
            if self.reserved_v2 != [0; 8] {
                panic!("Extended data not supported by VoteRecordV1")
            }

            let vote_weight = match &self.vote {
                Vote::Approve(_options) => VoteWeightV1::Yes(self.voter_weight),
                Vote::Deny => VoteWeightV1::No(self.voter_weight),
                Vote::Abstain | Vote::Veto => {
                    panic!("Vote type: {:?} not supported by VoteRecordV1", &self.vote)
                }
            };

            let vote_record_data_v1 = VoteRecordV1 {
                account_type: self.account_type,
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
) -> Result<VoteRecordV2, ProgramError> {
    let account_type: GovernanceAccountType =
        try_from_slice_unchecked(&vote_record_info.data.borrow())?;

    // If the account is V1 version then translate to V2
    if account_type == GovernanceAccountType::VoteRecordV1 {
        let vote_record_data_v1 = get_account_data::<VoteRecordV1>(program_id, vote_record_info)?;

        let (vote, voter_weight) = match vote_record_data_v1.vote_weight {
            VoteWeightV1::Yes(weight) => (
                Vote::Approve(vec![VoteChoice {
                    rank: 0,
                    weight_percentage: 100,
                }]),
                weight,
            ),
            VoteWeightV1::No(weight) => (Vote::Deny, weight),
        };

        return Ok(VoteRecordV2 {
            account_type,
            proposal: vote_record_data_v1.proposal,
            governing_token_owner: vote_record_data_v1.governing_token_owner,
            is_relinquished: vote_record_data_v1.is_relinquished,
            voter_weight,
            vote,
            reserved_v2: [0; 8],
        });
    }

    get_account_data::<VoteRecordV2>(program_id, vote_record_info)
}

/// Deserializes VoteRecord and checks it belongs to the provided Proposal and Governing Token Owner
pub fn get_vote_record_data_for_proposal_and_token_owner(
    program_id: &Pubkey,
    vote_record_info: &AccountInfo,
    proposal: &Pubkey,
    governing_token_owner: &Pubkey,
) -> Result<VoteRecordV2, ProgramError> {
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

    use borsh::BorshSerialize;
    use solana_program::clock::Epoch;

    use super::*;

    #[test]
    fn test_vote_record_v1_to_v2_serialisation_roundtrip() {
        // Arrange

        let vote_record_v1_source = VoteRecordV1 {
            account_type: GovernanceAccountType::VoteRecordV1,
            proposal: Pubkey::new_unique(),
            governing_token_owner: Pubkey::new_unique(),
            is_relinquished: true,
            vote_weight: VoteWeightV1::Yes(120),
        };

        let mut account_data = vec![];
        vote_record_v1_source.serialize(&mut account_data).unwrap();

        let program_id = Pubkey::new_unique();

        let info_key = Pubkey::new_unique();
        let mut lamports = 10u64;

        let account_info = AccountInfo::new(
            &info_key,
            false,
            false,
            &mut lamports,
            &mut account_data[..],
            &program_id,
            false,
            Epoch::default(),
        );

        // Act

        let vote_record_v2 = get_vote_record_data(&program_id, &account_info).unwrap();

        vote_record_v2
            .serialize(&mut &mut **account_info.data.borrow_mut())
            .unwrap();

        // Assert

        let vote_record_v1_target =
            get_account_data::<VoteRecordV1>(&program_id, &account_info).unwrap();

        assert_eq!(vote_record_v1_source, vote_record_v1_target)
    }
}
