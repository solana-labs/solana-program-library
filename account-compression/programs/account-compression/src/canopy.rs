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
use solana_program::keccak::hashv;
use spl_concurrent_merkle_tree::node::{empty_node_cached, empty_node_cached_mut, Node, EMPTY};
use std::mem::size_of;

// 30 is hard coded as it is the current max depth that SPL Compression supports
const MAX_SUPPORTED_DEPTH: usize = 30;

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
    let mut empty_node_cache = Box::new([EMPTY; MAX_SUPPORTED_DEPTH]);
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
            let empty_node = empty_node_cached::<MAX_SUPPORTED_DEPTH>(level, &mut empty_node_cache);
            inferred_nodes.push(empty_node);
        } else {
            inferred_nodes.push(canopy[cached_idx]);
        }
        node_idx >>= 1;
    }
    // We only want to add inferred canopy nodes such that the proof length
    // is equal to the tree depth. If the length of proof + length of canopy nodes is
    // less than the tree depth, the instruction will fail.
    let overlap = (proof.len() + inferred_nodes.len()).saturating_sub(max_depth as usize);
    proof.extend(inferred_nodes.iter().skip(overlap));
    Ok(())
}

/// Sets the leaf nodes of the canopy. The leaf nodes are the lowest level of the canopy, representing the leaves of the canopy-tree.
/// The method will update the parent nodes of all the modified subtrees up to the uppermost level of the canopy.
/// The leaf nodes indexing is 0-based for the start_index.
pub fn set_canopy_leaf_nodes(
    canopy_bytes: &mut [u8],
    max_depth: u32,
    start_index: u32,
    nodes: &[Node],
) -> Result<()> {
    check_canopy_bytes(canopy_bytes)?;
    let canopy = cast_slice_mut::<u8, Node>(canopy_bytes);
    let path_len = get_cached_path_length(canopy, max_depth)?;

    let start_canopy_node = leaf_node_index_to_canopy_index(path_len, start_index);
    let start_canopy_idx = start_canopy_node - 2;
    // set the "leaf" nodes of the canopy first - that's the lowest level of the canopy
    for (i, node) in nodes.iter().enumerate() {
        canopy[start_canopy_idx + i] = *node;
    }
    let mut start_canopy_node = start_canopy_node;
    let mut end_canopy_node = start_canopy_node + nodes.len() - 1 as usize;
    let mut empty_node_cache = Box::new([EMPTY; MAX_SUPPORTED_DEPTH]);
    let leaf_node_level = max_depth - path_len;
    // traverse up the tree and update the parent nodes in the modified subtree
    for level in leaf_node_level + 1..max_depth {
        start_canopy_node >>= 1;
        end_canopy_node >>= 1;
        for node in start_canopy_node..end_canopy_node + 1 {
            let left_child = get_value_for_node::<MAX_SUPPORTED_DEPTH>(
                node << 1,
                level - 1,
                &canopy,
                &mut empty_node_cache,
            );
            let right_child = get_value_for_node::<MAX_SUPPORTED_DEPTH>(
                (node << 1) + 1,
                level - 1,
                &canopy,
                &mut empty_node_cache,
            );
            canopy[node - 2 as usize].copy_from_slice(hashv(&[&left_child, &right_child]).as_ref());
        }
    }
    Ok(())
}

#[inline(always)]
fn get_value_for_node<const N: usize>(
    node_idx: usize,
    level: u32,
    canopy: &[Node],
    empty_node_cache: &mut [Node; N],
) -> Node {
    if canopy[node_idx - 2] != EMPTY {
        return canopy[node_idx - 2];
    }
    empty_node_cached_mut::<N>(level, empty_node_cache)
}

#[inline(always)]
fn leaf_node_index_to_canopy_index(path_len: u32, index: u32) -> usize {
    (1 << path_len) + index as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use spl_concurrent_merkle_tree::node::empty_node;

    fn test_leaf_node_index_to_canopy_index_impl(path_len: u32, index: u32, expected: usize) {
        assert_eq!(leaf_node_index_to_canopy_index(path_len, index), expected);
    }

    // todo: 0,0,0?

    #[test]
    fn test_1_level_0_index() {
        test_leaf_node_index_to_canopy_index_impl(1, 0, 2);
    }

    #[test]
    fn test_1_level_1_index() {
        test_leaf_node_index_to_canopy_index_impl(1, 1, 3);
    }

    #[test]
    fn test_2_level_0_index() {
        test_leaf_node_index_to_canopy_index_impl(2, 0, 4);
    }
    #[test]
    fn test_2_level_3_index() {
        test_leaf_node_index_to_canopy_index_impl(2, 3, 7);
    }

    #[test]
    fn test_10_level_0_index() {
        test_leaf_node_index_to_canopy_index_impl(10, 0, 1024);
    }

    #[test]
    fn test_10_level_1023_index() {
        test_leaf_node_index_to_canopy_index_impl(10, 1023, 2047);
    }

    #[test]
    fn test_simple_single_level_canopy_set_canopy_leaf_nodes_with_empty_nodes() {
        let mut canopy_bytes = vec![0_u8; 2 * size_of::<Node>()];
        let nodes = vec![EMPTY; 2];
        set_canopy_leaf_nodes(&mut canopy_bytes, 1, 0, &nodes).unwrap();
        let canopy = cast_slice::<u8, Node>(&canopy_bytes);

        assert_eq!(canopy[0], EMPTY);
        assert_eq!(canopy[1], EMPTY);
    }

    #[test]
    fn test_simple_single_level_canopy_set_canopy_leaf_nodes_non_empty_nodes() {
        let mut canopy_bytes = vec![0_u8; 2 * size_of::<Node>()];
        let nodes = vec![[1_u8; 32], [2_u8; 32]];
        set_canopy_leaf_nodes(&mut canopy_bytes, 1, 0, &nodes).unwrap();
        let canopy = cast_slice::<u8, Node>(&canopy_bytes);

        assert_eq!(canopy[0], [1_u8; 32]);
        assert_eq!(canopy[1], [2_u8; 32]);
    }

    #[test]
    fn test_2levels_canopy_set_canopy_leaf_nodes_first_2_elements_provided() {
        let mut canopy_bytes = vec![0_u8; 6 * size_of::<Node>()];
        let nodes = vec![[1_u8; 32], [2_u8; 32]];
        set_canopy_leaf_nodes(&mut canopy_bytes, 2, 0, &nodes).unwrap();
        let canopy = cast_slice::<u8, Node>(&canopy_bytes);

        assert_eq!(canopy[0], hashv(&[&[1_u8; 32], &[2_u8; 32]]).to_bytes());
        assert_eq!(canopy[1], EMPTY); // is not updated
        assert_eq!(canopy[2], [1_u8; 32]);
        assert_eq!(canopy[3], [2_u8; 32]);
        assert_eq!(canopy[4], EMPTY);
        assert_eq!(canopy[5], EMPTY);
    }

    #[test]
    fn test_2levels_canopy_set_canopy_leaf_nodes_last_2_elements_provided() {
        let mut canopy_bytes = vec![0_u8; 6 * size_of::<Node>()];
        let nodes = vec![[1_u8; 32], [2_u8; 32]];
        set_canopy_leaf_nodes(&mut canopy_bytes, 2, 2, &nodes).unwrap();
        let canopy = cast_slice::<u8, Node>(&canopy_bytes);

        assert_eq!(canopy[0], EMPTY); // is not updated
        assert_eq!(canopy[1], hashv(&[&[1_u8; 32], &[2_u8; 32]]).to_bytes());
        assert_eq!(canopy[2], EMPTY);
        assert_eq!(canopy[3], EMPTY);
        assert_eq!(canopy[4], [1_u8; 32]);
        assert_eq!(canopy[5], [2_u8; 32]);
    }

    #[test]
    fn test_2levels_canopy_set_canopy_leaf_nodes_middle_2_elements_provided() {
        let mut canopy_bytes = vec![0_u8; 6 * size_of::<Node>()];
        let nodes = vec![[1_u8; 32], [2_u8; 32]];
        set_canopy_leaf_nodes(&mut canopy_bytes, 2, 1, &nodes).unwrap();
        let canopy = cast_slice::<u8, Node>(&canopy_bytes);

        assert_eq!(canopy[2], EMPTY);
        assert_eq!(canopy[3], [1_u8; 32]);
        assert_eq!(canopy[4], [2_u8; 32]);
        assert_eq!(canopy[5], EMPTY);
        assert_eq!(canopy[0], hashv(&[&EMPTY, &[1_u8; 32]]).to_bytes());
        assert_eq!(canopy[1], hashv(&[&[2_u8; 32], &EMPTY]).to_bytes());
    }

    #[test]
    fn test_3level_canopy_in_10_level_tree_set_canopy_leaf_nodes_first_2_elements_provided() {
        let mut canopy_bytes = vec![0_u8; 14 * size_of::<Node>()];
        let nodes = vec![[1_u8; 32], [2_u8; 32]];
        set_canopy_leaf_nodes(&mut canopy_bytes, 10, 0, &nodes).unwrap();
        let canopy = cast_slice::<u8, Node>(&canopy_bytes);

        let expected_hash12 = hashv(&[&[1_u8; 32], &[2_u8; 32]]).to_bytes();
        assert_eq!(
            canopy[0],
            hashv(&[&expected_hash12, &empty_node(8)]).to_bytes()
        );
        assert_eq!(canopy[1], EMPTY); // is not updated
        assert_eq!(canopy[2], expected_hash12);
        assert_eq!(canopy[3], EMPTY); // is not updated
        assert_eq!(canopy[4], EMPTY); // is not updated
        assert_eq!(canopy[5], EMPTY); // is not updated
        assert_eq!(canopy[6], [1_u8; 32]);
        assert_eq!(canopy[7], [2_u8; 32]);
    }

    #[test]
    fn test_3level_canopy_in_10_level_tree_set_canopy_leaf_nodes_middle_2_elements_provided() {
        let mut canopy_bytes = vec![0_u8; 14 * size_of::<Node>()];
        let nodes = vec![[1_u8; 32], [2_u8; 32]];
        set_canopy_leaf_nodes(&mut canopy_bytes, 10, 3, &nodes).unwrap();
        let canopy = cast_slice::<u8, Node>(&canopy_bytes);

        let expected_hash_empty_1 = hashv(&[&empty_node(7), &[1_u8; 32]]).to_bytes();
        let expected_hash_2_empty = hashv(&[&[2_u8; 32], &empty_node(7)]).to_bytes();

        assert_eq!(
            canopy[0],
            hashv(&[&empty_node(8), &expected_hash_empty_1]).to_bytes()
        );
        assert_eq!(
            canopy[1],
            hashv(&[&expected_hash_2_empty, &empty_node(8)]).to_bytes()
        );
        assert_eq!(canopy[2], EMPTY); // is not updated
        assert_eq!(canopy[3], expected_hash_empty_1);
        assert_eq!(canopy[4], expected_hash_2_empty);
        assert_eq!(canopy[5], EMPTY); // is not updated
        assert_eq!(canopy[9], [1_u8; 32]);
        assert_eq!(canopy[10], [2_u8; 32]);
    }

    #[test]
    fn test_3level_canopy_empty_set_canopy_leaf_nodes_no_impact() {
        let mut canopy_bytes = vec![0_u8; 14 * size_of::<Node>()];
        let nodes = vec![];
        set_canopy_leaf_nodes(&mut canopy_bytes, 10, 0, &nodes).unwrap();
        assert_eq!(canopy_bytes, vec![0_u8; 14 * size_of::<Node>()]);
    }
}
