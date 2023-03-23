use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};

use spl_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree;
use std::mem::size_of;

use crate::error::AccountCompressionError;

pub const CONCURRENT_MERKLE_TREE_HEADER_SIZE_V1: usize = 2 + 54;

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
#[repr(C)]
#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct ConcurrentMerkleTreeHeader {
    /// Account type
    pub account_type: CompressionAccountType,
    /// Versioned header
    pub header: ConcurrentMerkleTreeHeaderData,
}

#[repr(C)]
#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct ConcurrentMerkleTreeHeaderDataV1 {
    /// Buffer of changelogs stored on-chain.
    /// Must be a power of 2; see above table for valid combinations.
    max_buffer_size: u32,

    /// Depth of the SPL ConcurrentMerkleTree to store.
    /// Tree capacity can be calculated as power(2, max_depth).
    /// See above table for valid options.
    max_depth: u32,

    /// Authority that validates the content of the trees.
    /// Typically a program, e.g., the Bubblegum contract validates that leaves are valid NFTs.
    authority: Pubkey,

    /// Slot corresponding to when the Merkle tree was created.
    /// Provides a lower-bound on what slot to start (re-)building a tree from.
    creation_slot: u64,

    /// Needs padding for the account to be 8-byte aligned
    /// 8-byte alignment is necessary to zero-copy the SPL ConcurrentMerkleTree
    _padding: [u8; 6],
}

#[repr(C)]
#[derive(AnchorDeserialize, AnchorSerialize)]
pub enum ConcurrentMerkleTreeHeaderData {
    V1(ConcurrentMerkleTreeHeaderDataV1),
}

impl ConcurrentMerkleTreeHeader {
    pub fn initialize(
        &mut self,
        max_depth: u32,
        max_buffer_size: u32,
        authority: &Pubkey,
        creation_slot: u64,
    ) {
        self.account_type = CompressionAccountType::ConcurrentMerkleTree;

        match self.header {
            ConcurrentMerkleTreeHeaderData::V1(ref mut header) => {
                // Double check header is empty after deserialization from zero'd bytes
                assert_eq!(header.max_buffer_size, 0);
                assert_eq!(header.max_depth, 0);
                header.max_buffer_size = max_buffer_size;
                header.max_depth = max_depth;
                header.authority = *authority;
                header.creation_slot = creation_slot;
            }
        }
    }

    pub fn get_max_depth(&self) -> u32 {
        match &self.header {
            ConcurrentMerkleTreeHeaderData::V1(header) => header.max_depth,
        }
    }

    pub fn get_max_buffer_size(&self) -> u32 {
        match &self.header {
            ConcurrentMerkleTreeHeaderData::V1(header) => header.max_buffer_size,
        }
    }

    pub fn get_creation_slot(&self) -> u64 {
        match &self.header {
            ConcurrentMerkleTreeHeaderData::V1(header) => header.creation_slot,
        }
    }

    pub fn set_new_authority(&mut self, new_authority: &Pubkey) {
        match self.header {
            ConcurrentMerkleTreeHeaderData::V1(ref mut header) => {
                header.authority = new_authority.clone();
                msg!("Authority transferred to: {:?}", header.authority);
            }
        }
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
        match &self.header {
            ConcurrentMerkleTreeHeaderData::V1(header) => {
                require_eq!(
                    header.authority,
                    *expected_authority,
                    AccountCompressionError::IncorrectAuthority,
                );
            }
        }
        Ok(())
    }

    pub fn assert_valid_leaf_index(&self, leaf_index: u32) -> Result<()> {
        if leaf_index >= (1 << self.get_max_depth()) {
            return Err(AccountCompressionError::LeafIndexOutOfBounds.into());
        }
        Ok(())
    }
}

pub fn merkle_tree_get_size(header: &ConcurrentMerkleTreeHeader) -> Result<usize> {
    // Note: max_buffer_size MUST be a power of 2
    match (header.get_max_depth(), header.get_max_buffer_size()) {
        (3, 8) => Ok(size_of::<ConcurrentMerkleTree<3, 8>>()),
        (5, 8) => Ok(size_of::<ConcurrentMerkleTree<5, 8>>()),
        (14, 64) => Ok(size_of::<ConcurrentMerkleTree<14, 64>>()),
        (14, 256) => Ok(size_of::<ConcurrentMerkleTree<14, 256>>()),
        (14, 1024) => Ok(size_of::<ConcurrentMerkleTree<14, 1024>>()),
        (14, 2048) => Ok(size_of::<ConcurrentMerkleTree<14, 2048>>()),
        (15, 64) => Ok(size_of::<ConcurrentMerkleTree<15, 64>>()),
        (16, 64) => Ok(size_of::<ConcurrentMerkleTree<16, 64>>()),
        (17, 64) => Ok(size_of::<ConcurrentMerkleTree<17, 64>>()),
        (18, 64) => Ok(size_of::<ConcurrentMerkleTree<18, 64>>()),
        (19, 64) => Ok(size_of::<ConcurrentMerkleTree<19, 64>>()),
        (20, 64) => Ok(size_of::<ConcurrentMerkleTree<20, 64>>()),
        (20, 256) => Ok(size_of::<ConcurrentMerkleTree<20, 256>>()),
        (20, 1024) => Ok(size_of::<ConcurrentMerkleTree<20, 1024>>()),
        (20, 2048) => Ok(size_of::<ConcurrentMerkleTree<20, 2048>>()),
        (24, 64) => Ok(size_of::<ConcurrentMerkleTree<24, 64>>()),
        (24, 256) => Ok(size_of::<ConcurrentMerkleTree<24, 256>>()),
        (24, 512) => Ok(size_of::<ConcurrentMerkleTree<24, 512>>()),
        (24, 1024) => Ok(size_of::<ConcurrentMerkleTree<24, 1024>>()),
        (24, 2048) => Ok(size_of::<ConcurrentMerkleTree<24, 2048>>()),
        (26, 512) => Ok(size_of::<ConcurrentMerkleTree<26, 512>>()),
        (26, 1024) => Ok(size_of::<ConcurrentMerkleTree<26, 1024>>()),
        (26, 2048) => Ok(size_of::<ConcurrentMerkleTree<26, 2048>>()),
        (30, 512) => Ok(size_of::<ConcurrentMerkleTree<30, 512>>()),
        (30, 1024) => Ok(size_of::<ConcurrentMerkleTree<30, 1024>>()),
        (30, 2048) => Ok(size_of::<ConcurrentMerkleTree<30, 2048>>()),
        _ => {
            msg!(
                "Failed to get size of max depth {} and max buffer size {}",
                header.get_max_depth(),
                header.get_max_buffer_size()
            );
            err!(AccountCompressionError::ConcurrentMerkleTreeConstantsError)
        }
    }
}
