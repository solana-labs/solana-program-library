use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};

use crate::error::AccountCompressionError;

#[derive(Debug, Copy, Clone, PartialEq, BorshDeserialize, BorshSerialize)]
#[repr(u8)]
pub enum CompressionAccountType {
    /// Uninitialized
    Uninitialized,

    /// SPL ConcurrentMerkleTree data structure, may include a Canopy
    ConcurrentMerkleTree,
}

impl std::fmt::Display for CompressionAccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", &self)
    }
}

/// Initialization parameters for an SPL ConcurrentMerkleTree.
///
/// Only the following permutations are valid:
///
/// | max_depth | max_buffer_size       |
/// | --------- | --------------------- |
/// | 14        | (64, 256, 1024, 2048) |           
/// | 20        | (64, 256, 1024, 2048) |           
/// | 24        | (64, 256, 512, 1024, 2048) |           
/// | 26        | (64, 256, 512, 1024, 2048) |           
/// | 30        | (512, 1024, 2048) |           
///
#[derive(BorshDeserialize, BorshSerialize)]
#[repr(C)]
pub struct ConcurrentMerkleTreeHeader {
    /// Account type
    pub account_type: CompressionAccountType,

    /// Needs padding for the account to be 8-byte aligned
    /// 8-byte alignment is necessary to zero-copy the SPL ConcurrentMerkleTree
    pub _padding: [u8; 7],

    /// Buffer of changelogs stored on-chain.
    /// Must be a power of 2; see above table for valid combinations.
    pub max_buffer_size: u32,

    /// Depth of the SPL ConcurrentMerkleTree to store.
    /// Tree capacity can be calculated as power(2, max_depth).
    /// See above table for valid options.
    pub max_depth: u32,

    /// Authority that validates the content of the trees.
    /// Typically a program, e.g., the Bubblegum contract validates that leaves are valid NFTs.
    pub authority: Pubkey,

    /// Slot corresponding to when the Merkle tree was created.
    /// Provides a lower-bound on what slot to start (re-)building a tree from.
    pub creation_slot: u64,
}

impl ConcurrentMerkleTreeHeader {
    pub fn initialize(
        &mut self,
        max_depth: u32,
        max_buffer_size: u32,
        authority: &Pubkey,
        creation_slot: u64,
    ) {
        // Check header is empty
        assert_eq!(self.max_buffer_size, 0);
        assert_eq!(self.max_depth, 0);
        self.account_type = CompressionAccountType::ConcurrentMerkleTree;
        self.max_buffer_size = max_buffer_size;
        self.max_depth = max_depth;
        self.authority = *authority;
        self.creation_slot = creation_slot;
    }

    pub fn assert_valid(&self) -> Result<()> {
        require_eq!(
            self.account_type,
            CompressionAccountType::ConcurrentMerkleTree,
            AccountCompressionError::IncorrectAccountType,
        );
        Ok(())
    }

    pub fn assert_valid_authority(&self, expected_authority: &Pubkey) -> Result<()> {
        self.assert_valid()?;
        require_eq!(
            self.authority,
            *expected_authority,
            AccountCompressionError::IncorrectAuthority,
        );
        Ok(())
    }
}
