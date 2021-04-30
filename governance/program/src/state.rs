//! Program accounts

use solana_program::pubkey::Pubkey;

/// Max length of a governance name
pub const MAX_GOVERNANCE_NAME_LENGTH: usize = 32;

/// Max length of a proposal description link
pub const MAX_PROPOSAL_DESCRIPTION_LINK_LENGTH: usize = 200;

/// Max length of a proposal name
pub const MAX_PROPOSAL_NAME_LENGTH: usize = 32;

/// Max number of transactions allowed for a proposal
pub const MAX_TRANSACTIONS: usize = 5;

/// Defines all Governance accounts types
#[derive(Clone, Debug, PartialEq)]
pub enum GovernanceAccountType {
    /// 0 - Default uninitialized account state
    Uninitialized,

    /// 1 - Governance account
    Governance,

    /// 2 - Proposal account for Governance account. A single Governance account can have multiple Proposal accounts
    Proposal,

    /// 3 - Proposal voting state account. Every Proposal account has exactly one ProposalState account
    ProposalState,

    /// 4 - Vote record account for a given Proposal.  Proposal can have 0..n voting records
    VoteRecord,

    /// 5 Custom Single Signer Transaction account which holds instructions to execute for Proposal
    CustomSingleSignerTransaction,
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

/// Governance Account
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Governance {
    /// Account type
    pub account_type: GovernanceAccountType,

    /// Optional governance name
    pub name: [u8; MAX_GOVERNANCE_NAME_LENGTH],

    /// Voting threshold in % required to tip the vote
    /// It's the percentage of tokens out of the entire pool of governance tokens eligible to vote
    pub vote_threshold: u8,

    /// Minimum % of tokens for a governance token owner to be able to create a proposal
    /// It's the percentage of tokens out of the entire pool of governance tokens eligible to vote
    pub token_threshold_to_create_proposal: u8,

    /// Minimum waiting time in slots for a transaction to be executed after proposal is voted on
    pub min_transaction_hold_up_time: u64,

    /// Governance mint
    pub governance_mint: Pubkey,

    /// Council mint
    pub council_mint: Option<Pubkey>,

    /// Program ID that is governed by this Governance
    pub program: Pubkey,

    /// Time limit in slots for proposal to be open for voting
    pub max_voting_time: u64,

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

    /// Source Token Holding account
    pub source_holding: Pubkey,

    /// Source Mint - either governance or council mint from Governance
    pub source_mint: Pubkey,
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
    pub total_signing_tokens_minted: u64,

    /// Link to proposal's description
    pub description_link: [u8; MAX_PROPOSAL_DESCRIPTION_LINK_LENGTH],

    /// Proposal name
    pub name: [u8; MAX_PROPOSAL_NAME_LENGTH],

    /// When the Proposal ended voting - this will also be when the set was defeated or began executing naturally.
    pub voting_ended_at: u64,

    /// When the Proposal began voting
    pub voting_began_at: u64,

    /// when the Proposal entered draft state
    pub created_at: u64,

    /// when the Proposal entered completed state, also when execution ended naturally.
    pub completed_at: u64,

    /// when the Proposal entered deleted state
    pub deleted_at: u64,

    /// The number of the transactions already executed
    pub number_of_executed_transactions: u8,

    /// The number of transactions included in the proposal
    pub number_of_transactions: u8,

    /// Array of pubkeys pointing at Proposal Transactions, up to 5
    pub transactions: [Pubkey; MAX_TRANSACTIONS],
}

/// What state a Proposal is in
#[derive(Clone, Debug, PartialEq)]
pub enum ProposalStateStatus {
    /// Draft
    Draft,

    /// Taking votes
    Voting,

    /// Voting ended with success
    Succeeded,

    /// Votes completed and after cool oof period and transactions are being executed
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
pub struct GovernanceVoteRecord {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Proposal account
    pub proposal: Pubkey,

    /// The user who casted this vote
    pub voter: Pubkey,

    /// Voter's vote Yes/No and amount
    pub vote: Option<Vote>,
}

/// Account for a transaction with a single instruction signed by a single signer
#[derive(Clone)]
pub struct CustomSingleSignerTransaction {
    /// Governance Account type
    pub account_type: GovernanceAccountType,

    /// Minimum waiting time in slots for a transaction to be executed after proposal is voted on
    pub hold_up_time: u64,

    /// Instruction data
    pub instruction: Vec<u8>,

    /// Executed flag
    pub executed: u8,
}
