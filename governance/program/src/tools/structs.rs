//! General purpose structs utilities

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

/// Reserved 110 bytes
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct Reserved110 {
    /// Reserved 64 bytes
    pub reserved64: [u8; 64],
    /// Reserved 32 bytes
    pub reserved32: [u8; 32],
    /// Reserved 4 bytes
    pub reserved14: [u8; 14],
}

impl Default for Reserved110 {
    fn default() -> Self {
        Self {
            reserved64: [0; 64],
            reserved32: [0; 32],
            reserved14: [0; 14],
        }
    }
}

/// Reserved 120 bytes
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct Reserved120 {
    /// Reserved 64 bytes
    pub reserved64: [u8; 64],
    /// Reserved 32 bytes
    pub reserved32: [u8; 32],
    /// Reserved 4 bytes
    pub reserved24: [u8; 24],
}

impl Default for Reserved120 {
    fn default() -> Self {
        Self {
            reserved64: [0; 64],
            reserved32: [0; 32],
            reserved24: [0; 24],
        }
    }
}
