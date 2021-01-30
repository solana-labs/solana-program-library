use super::enums;
use enums::TimelockStateStatus;
use solana_program::pubkey::Pubkey;

use super::TRANSACTION_SLOTS;

/// Timelock state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelockState {
    /// Current state of the invoked instruction account
    pub status: TimelockStateStatus,

    /// Total voting tokens minted, for use comparing to supply remaining during consensus
    pub total_voting_tokens_minted: u64,

    /// Array of pubkeys pointing at TimelockTransactions, up to 10
    pub timelock_transactions: [Pubkey; TRANSACTION_SLOTS],
}
