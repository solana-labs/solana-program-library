use {
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    solana_program::pubkey::Pubkey,
    spl_governance::state::{enums::GovernanceAccountType, governance::GovernanceV2},
};

/// Legacy Governance account as of spl-gov V1
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct LegacyGovernanceV1 {
    /// Account type.
    pub account_type: GovernanceAccountType,

    /// Governance Realm
    pub realm: Pubkey,

    /// Governance seed
    pub governance_seed: Pubkey,

    /// Running count of proposals
    pub proposals_count: u32,

    /// Governance config
    pub config: LegacyGovernanceConfigV1,

    /// Reserved space for future versions
    pub reserved: [u8; 8],
}

/// Legacy GovernanceConfig as of spl-gov V1
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct LegacyGovernanceConfigV1 {
    /// The type of the vote threshold used for voting
    pub vote_threshold_percentage: VoteThresholdPercentage,

    /// Minimum number of community tokens a governance token owner must possess
    /// to be able to create a proposal
    pub min_community_tokens_to_create_proposal: u64,

    /// The wait time in seconds before transactions can be executed after
    /// proposal is successfully voted on
    pub transactions_hold_up_time: u32,

    /// Time limit in seconds for proposal to be open for voting
    pub max_voting_time: u32,

    /// The source of vote weight for voters
    /// Note: In the current version only token deposits are accepted as vote
    /// weight
    pub vote_weight_source: VoteWeightSource,

    /// The time period in seconds within which a Proposal can be still
    /// cancelled after being voted on Once cool off time expires Proposal
    /// can't be cancelled any longer and becomes a law Note: This field is
    /// not implemented in the current version
    pub proposal_cool_off_time: u32,

    /// Minimum number of council tokens a governance token owner must possess
    /// to be able to create a proposal
    pub min_council_tokens_to_create_proposal: u64,
}

/// Legacy VoteWeightSource as of spl-gov V1
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum VoteWeightSource {
    /// Governing token deposits into the Realm are used as voter weights
    Deposit,
    /// Governing token account snapshots as of the time a proposal entered
    /// voting state are used as voter weights Note: Snapshot source is not
    /// supported in the current version Support for account snapshots are
    /// required in solana and/or arweave as a prerequisite
    Snapshot,
}

/// Legacy VoteThresholdPercentage as of spl-gov V1
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum VoteThresholdPercentage {
    /// Voting threshold of Yes votes in % required to tip the vote
    /// It's the percentage of tokens out of the entire pool of governance
    /// tokens eligible to vote Note: If the threshold is below or equal to
    /// 50% then an even split of votes ex: 50:50 or 40:40 is always resolved as
    /// Defeated In other words a '+1 vote' tie breaker is always required
    /// to have a successful vote
    YesVote(u8),

    /// The minimum number of votes in % out of the entire pool of governance
    /// tokens eligible to vote which must be cast for the vote to be valid
    /// Once the quorum is achieved a simple majority (50%+1) of Yes votes is
    /// required for the vote to succeed Note: Quorum is not implemented in
    /// the current version
    Quorum(u8),
}

impl From<GovernanceV2> for LegacyGovernanceV1 {
    fn from(governance_v2: GovernanceV2) -> Self {
        let account_type = match governance_v2.account_type {
            GovernanceAccountType::GovernanceV2 => GovernanceAccountType::GovernanceV1,
            GovernanceAccountType::ProgramGovernanceV2 => {
                GovernanceAccountType::ProgramGovernanceV1
            }
            GovernanceAccountType::MintGovernanceV2 => GovernanceAccountType::MintGovernanceV1,
            GovernanceAccountType::TokenGovernanceV2 => GovernanceAccountType::TokenGovernanceV1,
            _ => panic!("Invalid Governance account type"),
        };

        let yes_vote_threshold = match governance_v2.config.community_vote_threshold {
            spl_governance::state::enums::VoteThreshold::YesVotePercentage(yes_vote_percentage) => {
                yes_vote_percentage
            }
            _ => panic!("Invalid vote threshold"),
        };

        LegacyGovernanceV1 {
            account_type,
            realm: governance_v2.realm,
            governance_seed: governance_v2.governance_seed,
            proposals_count: 0,
            config: LegacyGovernanceConfigV1 {
                vote_threshold_percentage: VoteThresholdPercentage::YesVote(yes_vote_threshold),
                min_community_tokens_to_create_proposal: governance_v2
                    .config
                    .min_community_weight_to_create_proposal,
                transactions_hold_up_time: governance_v2.config.transactions_hold_up_time,
                max_voting_time: governance_v2.config.voting_base_time,
                vote_weight_source: VoteWeightSource::Deposit,
                proposal_cool_off_time: 0,
                min_council_tokens_to_create_proposal: governance_v2
                    .config
                    .min_council_weight_to_create_proposal,
            },
            reserved: [0; 8],
        }
    }
}
