//! Legacy Accounts

use crate::state::{
    enums::{
        GovernanceAccountType, InstructionExecutionFlags, InstructionExecutionStatus,
        MintMaxVoteWeightSource, ProposalState, VoteThresholdPercentage,
    },
    proposal_instruction::InstructionData,
};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    clock::{Slot, UnixTimestamp},
    program_pack::IsInitialized,
    pubkey::Pubkey,
};

/// Realm Config instruction args
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct RealmConfigArgsV1 {
    /// Indicates whether council_mint should be used
    /// If yes then council_mint account must also be passed to the instruction
    pub use_council_mint: bool,

    /// Min number of community tokens required to create a governance
    pub min_community_tokens_to_create_governance: u64,

    /// The source used for community mint max vote weight source
    pub community_mint_max_vote_weight_source: MintMaxVoteWeightSource,
}

/// Instructions supported by the Governance program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum GovernanceInstructionV1 {
    /// Creates Governance Realm account which aggregates governances for given Community Mint and optional Council Mint
    CreateRealm {
        #[allow(dead_code)]
        /// UTF-8 encoded Governance Realm name
        name: String,

        #[allow(dead_code)]
        /// Realm config args     
        config_args: RealmConfigArgsV1,
    },

    /// Deposits governing tokens (Community or Council) to Governance Realm and establishes your voter weight to be used for voting within the Realm
    DepositGoverningTokens {
        /// The amount to deposit into the realm
        #[allow(dead_code)]
        amount: u64,
    },
}

/// Realm Config defining Realm parameters.
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct RealmConfigV1 {
    /// Reserved space for future versions
    pub reserved: [u8; 8],

    /// Min number of community tokens required to create a governance
    pub min_community_tokens_to_create_governance: u64,

    /// The source used for community mint max vote weight source
    pub community_mint_max_vote_weight_source: MintMaxVoteWeightSource,

    /// Optional council mint
    pub council_mint: Option<Pubkey>,
}

/// Governance Realm Account
/// Account PDA seeds" ['governance', name]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct RealmV1 {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Community mint
    pub community_mint: Pubkey,

    /// Configuration of the Realm
    pub config: RealmConfigV1,

    /// Reserved space for future versions
    pub reserved: [u8; 8],

    /// Realm authority. The authority must sign transactions which update the realm config
    /// The authority should be transferred to Realm Governance to make the Realm self governed through proposals
    pub authority: Option<Pubkey>,

    /// Governance Realm name
    pub name: String,
}

impl IsInitialized for RealmV1 {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::Realm
    }
}

/// Governance Proposal
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct ProposalV1 {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Governance account the Proposal belongs to
    pub governance: Pubkey,

    /// Indicates which Governing Token is used to vote on the Proposal
    /// Whether the general Community token owners or the Council tokens owners vote on this Proposal
    pub governing_token_mint: Pubkey,

    /// Current proposal state
    pub state: ProposalState,

    /// The TokenOwnerRecord representing the user who created and owns this Proposal
    pub token_owner_record: Pubkey,

    /// The number of signatories assigned to the Proposal
    pub signatories_count: u8,

    /// The number of signatories who already signed
    pub signatories_signed_off_count: u8,

    /// The number of Yes votes
    pub yes_votes_count: u64,

    /// The number of No votes
    pub no_votes_count: u64,

    /// The number of the instructions already executed
    pub instructions_executed_count: u16,

    /// The number of instructions included in the proposal
    pub instructions_count: u16,

    /// The index of the the next instruction to be added
    pub instructions_next_index: u16,

    /// When the Proposal was created and entered Draft state
    pub draft_at: UnixTimestamp,

    /// When Signatories started signing off the Proposal
    pub signing_off_at: Option<UnixTimestamp>,

    /// When the Proposal began voting as UnixTimestamp
    pub voting_at: Option<UnixTimestamp>,

    /// When the Proposal began voting as Slot
    /// Note: The slot is not currently used but the exact slot is going to be required to support snapshot based vote weights
    pub voting_at_slot: Option<Slot>,

    /// When the Proposal ended voting and entered either Succeeded or Defeated
    pub voting_completed_at: Option<UnixTimestamp>,

    /// When the Proposal entered Executing state
    pub executing_at: Option<UnixTimestamp>,

    /// When the Proposal entered final state Completed or Cancelled and was closed
    pub closed_at: Option<UnixTimestamp>,

    /// Instruction execution flag for ordered and transactional instructions
    /// Note: This field is not used in the current version
    pub execution_flags: InstructionExecutionFlags,

    /// The max vote weight for the Governing Token mint at the time Proposal was decided
    /// It's used to show correct vote results for historical proposals in cases when the mint supply or max weight source changed
    /// after vote was completed.
    pub max_vote_weight: Option<u64>,

    /// The vote threshold percentage at the time Proposal was decided
    /// It's used to show correct vote results for historical proposals in cases when the threshold
    /// was changed for governance config after vote was completed.
    pub vote_threshold_percentage: Option<VoteThresholdPercentage>,

    /// Proposal name
    pub name: String,

    /// Link to proposal's description
    pub description_link: String,
}

impl IsInitialized for ProposalV1 {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::ProposalV1
    }
}

/// Proposal instruction V1
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct ProposalInstructionV1 {
    /// Governance Account type
    pub account_type: GovernanceAccountType,

    /// The Proposal the instruction belongs to
    pub proposal: Pubkey,

    /// Unique instruction index within it's parent Proposal
    pub instruction_index: u16,

    /// Minimum waiting time in seconds for the  instruction to be executed once proposal is voted on
    pub hold_up_time: u32,

    /// Instruction to execute
    /// The instruction will be signed by Governance PDA the Proposal belongs to
    // For example for ProgramGovernance the instruction to upgrade program will be signed by ProgramGovernance PDA
    pub instruction: InstructionData,

    /// Executed at flag
    pub executed_at: Option<UnixTimestamp>,

    /// Instruction execution status
    pub execution_status: InstructionExecutionStatus,
}

impl IsInitialized for ProposalInstructionV1 {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::ProposalInstructionV1
    }
}

/// Vote  with number of votes
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum VoteWeightV1 {
    /// Yes vote
    Yes(u64),

    /// No vote
    No(u64),
}

/// Proposal VoteRecord
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct VoteRecordV1 {
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
    pub vote_weight: VoteWeightV1,
}

impl IsInitialized for VoteRecordV1 {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::VoteRecordV1
    }
}
