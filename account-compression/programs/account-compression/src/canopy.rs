//! Canopy is way to cache the upper `N` levels of a SPL ConcurrentMerkleTree.
//!
//! By caching the upper `N` levels of a depth `D` SPL ConcurrentMerkleTree,
//! proofs can be truncated to the first `D - N` nodes. This helps reduce the size of account
//! compression transactions, and makes it possible to
//! modify trees up to depth 31, which store more than 1 billion leaves.
//!
//! Note: this means that creating a tree of depth > 24 without a canopy will be impossible to modify
//! on-chain until TransactionV2 is launched.
//!
//! To initialize a canopy on a ConcurrentMerkleTree account, you must initialize
//! the ConcurrentMerkleTree account with additional bytes. The number of additional bytes
//! needed is `(pow(2, N+1)-1) * 32`, where `N` is the number of levels of the merkle tree
//! you want the canopy to cache.
//!
//! The canopy will be updated everytime the concurrent merkle tree is modified. No additional work
//! needed.

use crate::error::AccountCompressionError;
use crate::events::ChangeLogEvent;
use anchor_lang::prelude::*;
use bytemuck::{cast_slice, cast_slice_mut};
use spl_concurrent_merkle_tree::node::{empty_node_cached, Node, EMPTY};
use std::mem::size_of;

#[inline(always)]
pub fn check_canopy_bytes(canopy_bytes: &[u8]) -> Result<()> {
    if canopy_bytes.len() % size_of::<Node>() != 0 {
        msg!(
            "Canopy byte length {} is not a multiple of {}",
            canopy_bytes.len(),
            size_of::<Node>()
        );
        err!(AccountCompressionError::CanopyLengthMismatch)
    } else {
        Ok(())
    }
}

#[inline(always)]
fn get_cached_path_length(canopy: &[Node], max_depth: u32) -> Result<u32> {
    // The offset of 2 is applied because the canopy is a full binary tree without the root node
    // Size: (2^n - 2) -> Size + 2 must be a power of 2
    let closest_power_of_2 = (canopy.len() + 2) as u32;
    // This expression will return true if `closest_power_of_2` is actually a power of 2
    if closest_power_of_2 & (closest_power_of_2 - 1) == 0 {
        // (1 << max_depth) returns the number of leaves in the full merkle tree
        // (1 << (max_depth + 1)) - 1 returns the number of nodes in the full tree
        // The canopy size cannot exceed the size of the tree
        if closest_power_of_2 > (1 << (max_depth + 1)) {
            msg!(
                "Canopy size is too large. Size: {}. Max size: {}",
                closest_power_of_2 - 2,
                (1 << (max_depth + 1)) - 2
            );
            return err!(AccountCompressionError::CanopyLengthMismatch);
        }
    } else {
        msg!(
            "Canopy length {} is not 2 less than a power of 2",
            canopy.len()
        );
        return err!(AccountCompressionError::CanopyLengthMismatch);
    }
    // 1 is subtracted from the trailing zeros because the root is not stored in the canopy
    Ok(closest_power_of_2.trailing_zeros() - 1)
}

pub fn update_canopy(
    canopy_bytes: &mut [u8],
    max_depth: u32,
    change_log: Option<&ChangeLogEvent>,
) -> Result<()> {
    check_canopy_bytes(canopy_bytes)?;
    let canopy = cast_slice_mut::<u8, Node>(canopy_bytes);
    let path_len = get_cached_path_length(canopy, max_depth)?;
    if let Some(cl_event) = change_log {
        match &*cl_event {
            ChangeLogEvent::V1(cl) => {
                // Update the canopy from the newest change log
                for path_node in cl.path.iter().rev().skip(1).take(path_len as usize) {
                    // node_idx - 2 maps to the canopy index
                    canopy[(path_node.index - 2) as usize] = path_node.node;
                }
            }
        }
    }
    Ok(())
}

pub fn fill_in_proof_from_canopy(
    canopy_bytes: &[u8],
    max_depth: u32,
    index: u32,
    proof: &mut Vec<Node>,
) -> Result<()> {
    // 30 is hard coded as it is the current max depth that SPL Compression supports
    let mut empty_node_cache = Box::new([EMPTY; 30]);
    check_canopy_bytes(canopy_bytes)?;
    let canopy = cast_slice::<u8, Node>(canopy_bytes);
    let path_len = get_cached_path_length(canopy, max_depth)?;

    // We want to compute the node index (w.r.t. the canopy) where the current path
    // intersects the leaves of the canopy
    let mut node_idx = ((1 << max_depth) + index) >> (max_depth - path_len);
    let mut inferred_nodes = vec![];
    while node_idx > 1 {
        // node_idx - 2 maps to the canopy index
        let shifted_index = node_idx as usize - 2;
        let cached_idx = if shifted_index % 2 == 0 {
            shifted_index + 1
        } else {
            shifted_index - 1
        };
        if canopy[cached_idx] == EMPTY {
            let level = max_depth - (31 - node_idx.leading_zeros());
            let empty_node = empty_node_cached::<30>(level, &mut empty_node_cache);
            inferred_nodes.push(empty_node);
        } else {
            inferred_nodes.push(canopy[cached_idx]);
        }
        node_idx >>= 1;
    }
    // We only want to add inferred canopy nodes such that the proof length
    // is equal to the tree depth. If the lengh of proof + lengh of canopy nodes is
    // less than the tree depth, the instruction will fail.
    let overlap = (proof.len() + inferred_nodes.len()).saturating_sub(max_depth as usize);
    proof.extend(inferred_nodes.iter().skip(overlap));
    Ok(())
}
