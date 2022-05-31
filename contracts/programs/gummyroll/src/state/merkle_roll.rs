use crate::state::{
    change_log::{ChangeLog, Path},
    node::{Node, EMPTY},
};
use anchor_lang::{
    prelude::*,
    solana_program::{keccak::hashv, log::sol_log_compute_units},
};
use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
use concurrent_merkle_roll::utils::{empty_node, fill_in_proof, recompute, ZeroCopy};
use std::convert::AsRef;

#[derive(BorshDeserialize, BorshSerialize)]
#[repr(C)]
pub struct MerkleRollHeader {
    pub max_buffer_size: u32,
    pub max_depth: u32,
    pub authority: Pubkey,
    pub append_authority: Pubkey,
}

impl MerkleRollHeader {
    pub fn initialize(
        &mut self,
        max_depth: u32,
        max_buffer_size: u32,
        authority: &Pubkey,
        append_authority: &Pubkey,
    ) {
        // Check header is empty
        assert_eq!(self.max_buffer_size, 0);
        assert_eq!(self.max_depth, 0);
        self.max_buffer_size = max_buffer_size;
        self.max_depth = max_depth;
        self.authority = *authority;
        self.append_authority = *append_authority;
    }
}
