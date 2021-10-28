//! Legacy Accounts

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{clock::UnixTimestamp, program_pack::IsInitialized, pubkey::Pubkey};

use super::{
    enums::{GovernanceAccountType, InstructionExecutionStatus},
    proposal_instruction::InstructionData,
};

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
