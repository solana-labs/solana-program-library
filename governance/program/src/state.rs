//! Program accounts

use solana_program::{epoch_schedule::Slot, instruction::Instruction, pubkey::Pubkey};

/// Max number of instructions allowed for a proposal
pub const MAX_INSTRUCTIONS: usize = 5;

/// Defines all Governance accounts types
#[derive(Clone, Debug, PartialEq)]
pub enum GovernanceAccountType {
    /// Default uninitialized account state
    Uninitialized,

    /// Top level aggregation for governances within Governance Token (and optional Council Token).
    GovernanceRealm,

    /// Voter record for each voter within a Realm.
    VoterRecord,

    /// Program Governance account.
    ProgramGovernance,

    /// Proposal account for Governance account. A single Governance account can have multiple Proposal accounts.
    Proposal,

    /// Proposal voting state account. Every Proposal account has exactly one ProposalState account.
    ProposalState,

    /// Vote record account for a given Proposal.  Proposal can have 0..n voting records.
    ProposalVoteRecord,

    /// Custom Single Signer Instruction account which holds an instruction to execute for Proposal.
    CustomSingleSignerInstruction,
}

impl Default for GovernanceAccountType {
    fn default() -> Self {
        GovernanceAccountType::Uninitialized
    }
}

/// Vote  with number of votes
#[derive(Clone, Debug, PartialEq)]
pub enum Vote {
    /// Yes vote
    Yes(u64),

    /// No vote
    No(u64),
}
/// Governance Realm Account
pub struct GovernanceRealm {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Governance mint
    pub governance_mint: Pubkey,

    /// Council mint
    pub council_mint: Option<Pubkey>,

    /// Governance Realm name
    pub name: String,
}

/// Governance Voter Record Account
pub struct VoterRecord {
    /// Governance account type.
    pub account_type: GovernanceAccountType,

    /// Amount of deposited Governance Tokens and voting weight on Proposals voted by Governance Token holders.
    pub governance_token_amount: u64,

    /// Amount of deposited Council Tokens and voting weight on Proposal voted by Council Token holders.
    pub council_token_amount: u64,

    /// Number of outstanding active votes.
    pub active_votes_count: u8,
}

/// Governance Account
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProgramGovernance {
    /// Account type
    pub account_type: GovernanceAccountType,

    /// Voting threshold in % required to tip the vote
    /// It's the percentage of tokens out of the entire pool of governance tokens eligible to vote
    pub vote_threshold: u8,

    /// Minimum % of tokens for a governance token owner to be able to create a proposal
    /// It's the percentage of tokens out of the entire pool of governance tokens eligible to vote
    pub token_threshold_to_create_proposal: u8,

    /// Minimum waiting time in slots for an instruction to be executed after proposal is voted on
    pub min_instruction_hold_up_time: Slot,

    /// Program ID that is governed by this Governance
    pub program: Pubkey,

    /// Time limit in slots for proposal to be open for voting
    pub max_voting_time: Slot,

    /// Running count of proposals
    pub proposal_count: u32,
}

/// Governance Proposal
#[derive(Clone)]
pub struct Proposal {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Governance account the Proposal belongs to
    pub governance: Pubkey,

    /// Proposal State account
    pub state: Pubkey,

    /// Mint that creates signatory tokens of this Proposal
    /// If there are outstanding signatory tokens, then cannot leave draft state. Signatories must burn tokens (ie agree
    /// to move instruction to voting state) and bring mint to net 0 tokens outstanding. Each signatory gets 1 (serves as flag)
    pub signatory_mint: Pubkey,

    /// Admin ownership mint. One token is minted, can be used to grant admin status to a new person.
    pub admin_mint: Pubkey,
}

/// Proposal state
#[derive(Clone)]
pub struct ProposalState {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Proposal account
    pub proposal: Pubkey,

    /// Current status of the proposal
    pub status: ProposalStateStatus,

    /// Total signatory tokens minted, for use comparing to supply remaining during draft period
    pub total_signatory_tokens_minted: u64,

    /// Link to proposal's description
    pub description_link: String,

    /// Proposal name
    pub name: String,

    /// When the Proposal ended voting - this will also be when the set was defeated or began executing naturally.
    pub voting_ended_at: Option<Slot>,

    /// When the Proposal began voting
    pub voting_began_at: Option<Slot>,

    /// when the Proposal entered draft state
    pub created_at: Option<Slot>,

    /// when the Proposal entered completed state, also when execution ended naturally.
    pub completed_at: Option<Slot>,

    /// when the Proposal entered deleted state
    pub deleted_at: Option<Slot>,

    /// The number of the instructions already executed
    pub number_of_executed_instructions: u8,

    /// The number of instructions included in the proposal
    pub number_of_instructions: u8,

    /// Array of pubkeys pointing at Proposal instructions, up to 5
    pub instruction: Vec<Pubkey>,
}

/// What state a Proposal is in
#[derive(Clone, Debug, PartialEq)]
pub enum ProposalStateStatus {
    /// Draft - Proposal enters Draft state when it's created
    Draft,

    /// Signing - The Proposal is being signed by Signatories. Proposal enters the state when first Signatory Sings and leaves it when last Signatory signs
    Signing,

    /// Taking votes
    Voting,

    /// Voting ended with success
    Succeeded,

    /// Voting completed and now instructions are being execute. Proposal enter this state when first instruction is executed and leaves when the last instruction is executed
    Executing,

    /// Completed
    Completed,

    /// Cancelled
    Cancelled,

    /// Defeated
    Defeated,
}

impl Default for ProposalStateStatus {
    fn default() -> Self {
        ProposalStateStatus::Draft
    }
}

/// Governance Vote Record
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProposalVoteRecord {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Proposal account
    pub proposal: Pubkey,

    /// The user who casted this vote
    pub voter: Pubkey,

    /// Voter's vote Yes/No and amount
    pub vote: Option<Vote>,
}

/// Account for an instruction to be executed for Proposal
#[derive(Clone)]
pub struct CustomSingleSignerInstruction {
    /// Governance Account type
    pub account_type: GovernanceAccountType,

    /// Minimum waiting time in slots for the  instruction to be executed once proposal is voted on
    pub hold_up_time: Slot,

    /// Instruction to execute
    pub instruction: Instruction,

    /// Executed flag
    pub executed: bool,
}
