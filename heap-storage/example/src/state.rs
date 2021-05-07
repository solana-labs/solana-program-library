//! State transition types
use borsh::{BorshDeserialize, BorshSerialize};

/// Account with data
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct DataAccount {
    /// value
    pub value: u8,
}