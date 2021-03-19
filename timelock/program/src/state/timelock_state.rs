use super::enums;
use enums::TimelockStateStatus;
use solana_program::pubkey::Pubkey;

/// Transaction slots allowed
pub const TRANSACTION_SLOTS: usize = 4;
/// How many characters are allowed in the description
pub const DESC_SIZE: usize = 200;
/// How many characters are allowed in the name
pub const NAME_SIZE: usize = 32;

/// Timelock state
#[derive(Clone)]
pub struct TimelockState {
    /// Current state of the invoked instruction account
    pub status: TimelockStateStatus,

    /// Total signatory tokens minted, for use comparing to supply remaining during draft period
    pub total_signing_tokens_minted: u64,

    /// Array of pubkeys pointing at TimelockTransactions, up to 10
    pub timelock_transactions: [Pubkey; TRANSACTION_SLOTS],

    /// Link to proposal
    pub desc_link: [u8; DESC_SIZE],

    /// Proposal name
    pub name: [u8; NAME_SIZE],

    /// When the timelock ended voting
    pub voting_ended_at: u64,

    /// When the timelock began voting
    pub voting_began_at: u64,

    /// Executions
    pub executions: u8,

    /// Used slots
    pub used_txn_slots: u8,
}
