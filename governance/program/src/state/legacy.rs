//! Legacy Accounts

use crate::state::{
    enums::{
        GovernanceAccountType, InstructionExecutionFlags, ProposalState,
        TransactionExecutionStatus, VoteThresholdPercentage,
    },
    governance::GovernanceConfig,
    proposal_transaction::InstructionData,
    realm::RealmConfig,
};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    clock::{Slot, UnixTimestamp},
    program_pack::IsInitialized,
    pubkey::Pubkey,
};

/// Governance Realm Account
/// Account PDA seeds" ['governance', name]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct RealmV1 {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Community mint
    pub community_mint: Pubkey,

    /// Configuration of the Realm
    pub config: RealmConfig,

    /// Reserved space for future versions
    pub reserved: [u8; 6],

    /// The number of proposals in voting state in the Realm
    /// Note: This is field introduced in V2 but it took space from reserved
    /// and we have preserve it for V1 serialization roundtrip
    pub voting_proposal_count: u16,

    /// Realm authority. The authority must sign transactions which update the realm config
    /// The authority should be transferred to Realm Governance to make the Realm self governed through proposals
    pub authority: Option<Pubkey>,

    /// Governance Realm name
    pub name: String,
}

impl IsInitialized for RealmV1 {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::RealmV1
    }
}

/// Governance Token Owner Record
/// Account PDA seeds: ['governance', realm, token_mint, token_owner ]
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct TokenOwnerRecordV1 {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// The Realm the TokenOwnerRecord belongs to
    pub realm: Pubkey,

    /// Governing Token Mint the TokenOwnerRecord holds deposit for
    pub governing_token_mint: Pubkey,

    /// The owner (either single or multisig) of the deposited governing SPL Tokens
    /// This is who can authorize a withdrawal of the tokens
    pub governing_token_owner: Pubkey,

    /// The amount of governing tokens deposited into the Realm
    /// This amount is the voter weight used when voting on proposals
    pub governing_token_deposit_amount: u64,

    /// The number of votes cast by TokenOwner but not relinquished yet
    /// Every time a vote is cast this number is increased and it's always decreased when relinquishing a vote regardless of the vote state
    pub unrelinquished_votes_count: u32,

    /// The total number of votes cast by the TokenOwner
    /// If TokenOwner withdraws vote while voting is still in progress total_votes_count is decreased  and the vote doesn't count towards the total
    pub total_votes_count: u32,

    /// The number of outstanding proposals the TokenOwner currently owns
    /// The count is increased when TokenOwner creates a proposal
    /// and decreased  once it's either voted on (Succeeded or Defeated) or Cancelled
    /// By default it's restricted to 1 outstanding Proposal per token owner
    pub outstanding_proposal_count: u8,

    /// Reserved space for future versions
    pub reserved: [u8; 7],

    /// A single account that is allowed to operate governance with the deposited governing tokens
    /// It can be delegated to by the governing_token_owner or current governance_delegate
    pub governance_delegate: Option<Pubkey>,
}

impl IsInitialized for TokenOwnerRecordV1 {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::TokenOwnerRecordV1
    }
}

/// Governance Account
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct GovernanceV1 {
    /// Account type. It can be Uninitialized, Governance, ProgramGovernance, TokenGovernance or MintGovernance
    pub account_type: GovernanceAccountType,

    /// Governance Realm
    pub realm: Pubkey,

    /// Account governed by this Governance and/or PDA identity seed
    /// It can be Program account, Mint account, Token account or any other account
    ///
    /// Note: The account doesn't have to exist. In that case the field is only a PDA seed
    ///
    /// Note: Setting governed_account doesn't give any authority over the governed account
    /// The relevant authorities for specific account types must still be transferred to the Governance PDA
    /// Ex: mint_authority/freeze_authority for a Mint account
    /// or upgrade_authority for a Program account should be transferred to the Governance PDA
    pub governed_account: Pubkey,

    /// Running count of proposals
    pub proposals_count: u32,

    /// Governance config
    pub config: GovernanceConfig,

    /// Reserved space for future versions
    pub reserved: [u8; 6],

    /// The number of proposals in voting state in the Governance
    /// Note: This is field introduced in V2 but it took space from reserved
    /// and we have preserve it for V1 serialization roundtrip
    pub voting_proposal_count: u16,
}

/// Checks if the given account type is one of the Governance V1 account types
pub fn is_governance_v1_account_type(account_type: &GovernanceAccountType) -> bool {
    match account_type {
        GovernanceAccountType::GovernanceV1
        | GovernanceAccountType::ProgramGovernanceV1
        | GovernanceAccountType::MintGovernanceV1
        | GovernanceAccountType::TokenGovernanceV1 => true,
        GovernanceAccountType::Uninitialized
        | GovernanceAccountType::RealmV1
        | GovernanceAccountType::RealmV2
        | GovernanceAccountType::RealmConfig
        | GovernanceAccountType::TokenOwnerRecordV1
        | GovernanceAccountType::TokenOwnerRecordV2
        | GovernanceAccountType::GovernanceV2
        | GovernanceAccountType::ProgramGovernanceV2
        | GovernanceAccountType::MintGovernanceV2
        | GovernanceAccountType::TokenGovernanceV2
        | GovernanceAccountType::ProposalV1
        | GovernanceAccountType::ProposalV2
        | GovernanceAccountType::SignatoryRecordV1
        | GovernanceAccountType::SignatoryRecordV2
        | GovernanceAccountType::ProposalInstructionV1
        | GovernanceAccountType::ProposalTransactionV2
        | GovernanceAccountType::VoteRecordV1
        | GovernanceAccountType::VoteRecordV2
        | GovernanceAccountType::ProgramMetadata => false,
    }
}

impl IsInitialized for GovernanceV1 {
    fn is_initialized(&self) -> bool {
        is_governance_v1_account_type(&self.account_type)
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

/// Account PDA seeds: ['governance', proposal, signatory]
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct SignatoryRecordV1 {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Proposal the signatory is assigned for
    pub proposal: Pubkey,

    /// The account of the signatory who can sign off the proposal
    pub signatory: Pubkey,

    /// Indicates whether the signatory signed off the proposal
    pub signed_off: bool,
}

impl IsInitialized for SignatoryRecordV1 {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::SignatoryRecordV1
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
    pub execution_status: TransactionExecutionStatus,
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
