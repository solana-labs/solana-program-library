use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};

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
        self.max_buffer_size = max_buffer_size;
        self.max_depth = max_depth;
        self.authority = *authority;
        self.creation_slot = creation_slot;
    }
}
