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

/// Maximum depth of the tree, supported by the SPL Compression
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

/// Sets the leaf nodes of the canopy. The leaf nodes are the lowest level of the canopy,
/// representing the leaves of the canopy-tree. The method will update the parent nodes of all the
/// modified subtrees up to the uppermost level of the canopy. The leaf nodes indexing for the
/// start_index is 0-based without regards to the full tree indexes or the node indexes. The
/// start_index is the index of the first leaf node to be updated.
pub fn set_canopy_leaf_nodes(
    canopy_bytes: &mut [u8],
    max_depth: u32,
    start_index: u32,
    nodes: &[Node],
) -> Result<()> {
    check_canopy_bytes(canopy_bytes)?;
    let canopy = cast_slice_mut::<u8, Node>(canopy_bytes);
    let path_len = get_cached_path_length(canopy, max_depth)?;
    if path_len == 0 {
        return err!(AccountCompressionError::CanopyNotAllocated);
    }
    let start_canopy_node = leaf_node_index_to_canopy_index(path_len, start_index)?;
    let start_canopy_idx = start_canopy_node - 2;
    // set the "leaf" nodes of the canopy first - that's the lowest level of the canopy
    for (i, node) in nodes.iter().enumerate() {
        canopy[start_canopy_idx + i] = *node;
    }
    let mut start_canopy_node = start_canopy_node;
    let mut end_canopy_node = start_canopy_node + nodes.len() - 1;
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
                canopy,
                &mut empty_node_cache,
            );
            let right_child = get_value_for_node::<MAX_SUPPORTED_DEPTH>(
                (node << 1) + 1,
                level - 1,
                canopy,
                &mut empty_node_cache,
            );
            canopy[node - 2].copy_from_slice(hashv(&[&left_child, &right_child]).as_ref());
        }
    }
    Ok(())
}

/// Checks the root of the canopy against the expected root.
pub fn check_canopy_root(canopy_bytes: &[u8], expected_root: &Node, max_depth: u32) -> Result<()> {
    check_canopy_bytes(canopy_bytes)?;
    let canopy = cast_slice::<u8, Node>(canopy_bytes);
    if canopy.is_empty() {
        return Ok(()); // Canopy is empty
    }
    let mut empty_node_cache = Box::new([EMPTY; MAX_SUPPORTED_DEPTH]);
    // first two nodes are the children of the root, they have index 2 and 3 respectively
    let left_root_child =
        get_value_for_node::<MAX_SUPPORTED_DEPTH>(2, max_depth - 1, canopy, &mut empty_node_cache);
    let right_root_child =
        get_value_for_node::<MAX_SUPPORTED_DEPTH>(3, max_depth - 1, canopy, &mut empty_node_cache);
    let actual_root = hashv(&[&left_root_child, &right_root_child]).to_bytes();
    if actual_root != *expected_root {
        msg!(
            "Canopy root mismatch. Expected: {:?}, Actual: {:?}",
            expected_root,
            actual_root
        );
        err!(AccountCompressionError::CanopyRootMismatch)
    } else {
        Ok(())
    }
}

/// Checks the canopy doesn't have any nodes to the right of the provided index in the full tree.
/// This is done by iterating through the canopy nodes to the right of the provided index and
/// finding the top-most node that has the node as its left child. The node should be empty. The
/// iteration contains following the previous checked node on the same level until the last node on
/// the level is reached.
pub fn check_canopy_no_nodes_to_right_of_index(
    canopy_bytes: &[u8],
    max_depth: u32,
    index: u32,
) -> Result<()> {
    check_canopy_bytes(canopy_bytes)?;
    check_index(index, max_depth)?;
    let canopy = cast_slice::<u8, Node>(canopy_bytes);
    let path_len = get_cached_path_length(canopy, max_depth)?;

    let mut node_idx = ((1 << max_depth) + index) >> (max_depth - path_len);
    // no need to check the node_idx as it's the leaf containing the index underneath it
    while node_idx & (node_idx + 1) != 0 {
        // check the next node to the right
        node_idx += 1;
        // find the top-most node that has the node as its left-most child
        node_idx >>= node_idx.trailing_zeros();

        let shifted_index = node_idx as usize - 2;
        if canopy[shifted_index] != EMPTY {
            msg!("Canopy node at index {} is not empty", shifted_index);
            return err!(AccountCompressionError::CanopyRightmostLeafMismatch);
        }
    }
    Ok(())
}

#[inline(always)]
fn check_index(index: u32, at_depth: u32) -> Result<()> {
    if at_depth > MAX_SUPPORTED_DEPTH as u32 {
        return err!(AccountCompressionError::ConcurrentMerkleTreeConstantsError);
    }
    if at_depth == 0 {
        return err!(AccountCompressionError::ConcurrentMerkleTreeConstantsError);
    }
    if index >= (1 << at_depth) {
        return err!(AccountCompressionError::LeafIndexOutOfBounds);
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
fn leaf_node_index_to_canopy_index(path_len: u32, index: u32) -> Result<usize> {
    check_index(index, path_len)?;
    Ok((1 << path_len) + index as usize)
}

#[cfg(test)]
mod tests {
    use {super::*, spl_concurrent_merkle_tree::node::empty_node};

    fn success_leaf_node_index_to_canopy_index(path_len: u32, index: u32, expected: usize) {
        assert_eq!(
            leaf_node_index_to_canopy_index(path_len, index).unwrap(),
            expected
        );
    }

    #[test]
    fn test_zero_length_tree() {
        assert_eq!(
            leaf_node_index_to_canopy_index(0, 0).unwrap_err(),
            AccountCompressionError::ConcurrentMerkleTreeConstantsError.into()
        );
    }

    #[test]
    fn test_1_level_0_index() {
        success_leaf_node_index_to_canopy_index(1, 0, 2);
    }

    #[test]
    fn test_1_level_1_index() {
        success_leaf_node_index_to_canopy_index(1, 1, 3);
    }

    #[test]
    fn test_2_level_0_index() {
        success_leaf_node_index_to_canopy_index(2, 0, 4);
    }
    #[test]
    fn test_2_level_3_index() {
        success_leaf_node_index_to_canopy_index(2, 3, 7);
    }

    #[test]
    fn test_10_level_0_index() {
        success_leaf_node_index_to_canopy_index(10, 0, 1024);
    }

    #[test]
    fn test_10_level_1023_index() {
        success_leaf_node_index_to_canopy_index(10, 1023, 2047);
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

    #[test]
    fn test_success_check_canopy_root() {
        let mut canopy_bytes = vec![0_u8; 2 * size_of::<Node>()];
        let expected_root = hashv(&[&[1_u8; 32], &[2_u8; 32]]).to_bytes();
        let nodes = vec![[1_u8; 32], [2_u8; 32]];
        set_canopy_leaf_nodes(&mut canopy_bytes, 1, 0, &nodes).unwrap();
        check_canopy_root(&canopy_bytes, &expected_root, 30).unwrap();
    }

    #[test]
    fn test_success_check_canopy_root_with_empty_right_branch() {
        let mut canopy_bytes = vec![0_u8; 2 * size_of::<Node>()];
        let mut empty_node_cache = Box::new([EMPTY; MAX_SUPPORTED_DEPTH]);
        let top_level = (MAX_SUPPORTED_DEPTH - 1) as u32;
        let right_branch =
            empty_node_cached_mut::<MAX_SUPPORTED_DEPTH>(top_level, &mut empty_node_cache);
        let expected_root = hashv(&[&[1_u8; 32], &right_branch]).to_bytes();
        let nodes = vec![[1_u8; 32], EMPTY];
        set_canopy_leaf_nodes(&mut canopy_bytes, MAX_SUPPORTED_DEPTH as u32, 0, &nodes).unwrap();
        check_canopy_root(&canopy_bytes, &expected_root, 30).unwrap();
    }

    #[test]
    fn test_failure_check_canopy_root() {
        let mut canopy_bytes = vec![0_u8; 2 * size_of::<Node>()];
        let expected_root = hashv(&[&[1_u8; 32], &[2_u8; 32]]).to_bytes();
        let nodes = vec![[1_u8; 32], [2_u8; 32]];
        set_canopy_leaf_nodes(&mut canopy_bytes, 1, 0, &nodes).unwrap();
        let mut expected_root = expected_root;
        expected_root[0] = 0;
        assert_eq!(
            check_canopy_root(&canopy_bytes, &expected_root, 30).unwrap_err(),
            AccountCompressionError::CanopyRootMismatch.into()
        );
    }

    #[test]
    fn test_success_check_canopy_no_nodes_to_right_of_index_empty_tree_first_index() {
        let canopy_bytes = vec![0_u8; 6 * size_of::<Node>()];
        check_canopy_no_nodes_to_right_of_index(&canopy_bytes, 20, 0).unwrap();
    }

    #[test]
    fn test_success_check_canopy_no_nodes_to_right_of_index_empty_tree_last_index() {
        let canopy_bytes = vec![0_u8; 6 * size_of::<Node>()];
        check_canopy_no_nodes_to_right_of_index(&canopy_bytes, 20, (1 << 20) - 1).unwrap();
    }

    #[test]
    fn test_success_check_canopy_no_nodes_to_right_of_index_empty_canopy_only_tree_first_index() {
        let canopy_bytes = vec![0_u8; 6 * size_of::<Node>()];
        check_canopy_no_nodes_to_right_of_index(&canopy_bytes, 2, 0).unwrap();
    }

    #[test]
    fn test_success_check_canopy_no_nodes_to_right_of_index_empty_canopy_only_tree_last_index() {
        let canopy_bytes = vec![0_u8; 6 * size_of::<Node>()];
        check_canopy_no_nodes_to_right_of_index(&canopy_bytes, 2, (1 << 2) - 1).unwrap();
    }

    #[test]
    fn test_failure_check_canopy_no_nodes_to_right_of_index_empty_tree_index_out_of_range() {
        let canopy_bytes = vec![0_u8; 6 * size_of::<Node>()];
        assert_eq!(
            check_canopy_no_nodes_to_right_of_index(&canopy_bytes, 2, 1 << 20).unwrap_err(),
            AccountCompressionError::LeafIndexOutOfBounds.into()
        );
    }

    #[test]
    fn test_failure_check_canopy_no_nodes_to_right_of_index_full_tree_index_out_of_range() {
        let canopy_bytes = vec![1_u8; 6 * size_of::<Node>()];
        assert_eq!(
            check_canopy_no_nodes_to_right_of_index(&canopy_bytes, 2, 1 << 21).unwrap_err(),
            AccountCompressionError::LeafIndexOutOfBounds.into()
        );
    }

    #[test]
    fn test_success_check_canopy_no_nodes_to_right_of_index_full_tree_last_index() {
        let canopy_bytes = vec![1_u8; 6 * size_of::<Node>()];
        check_canopy_no_nodes_to_right_of_index(&canopy_bytes, 20, (1 << 20) - 1).unwrap();
    }

    #[test]
    fn test_success_check_canopy_no_nodes_to_right_of_index_full_tree_first_child_of_last_canopy_node_leaf(
    ) {
        let canopy_bytes = vec![1_u8; 6 * size_of::<Node>()];
        check_canopy_no_nodes_to_right_of_index(&canopy_bytes, 20, 3 << (20 - 2)).unwrap();
    }

    #[test]
    fn test_failure_check_canopy_no_nodes_to_right_of_index_full_tree_last_child_of_second_to_last_canopy_node_leaf(
    ) {
        let canopy_bytes = vec![1_u8; 6 * size_of::<Node>()];
        assert_eq!(
            check_canopy_no_nodes_to_right_of_index(&canopy_bytes, 20, (3 << (20 - 2)) - 1)
                .unwrap_err(),
            AccountCompressionError::CanopyRightmostLeafMismatch.into()
        );
    }

    #[test]
    fn test_success_check_canopy_no_nodes_to_right_of_index_last_child_of_second_to_last_canopy_node_leaf(
    ) {
        let mut canopy_bytes = vec![1_u8; 6 * size_of::<Node>()];
        canopy_bytes[5 * size_of::<Node>()..].fill(0);
        check_canopy_no_nodes_to_right_of_index(&canopy_bytes, 20, (3 << (20 - 2)) - 1).unwrap();
    }

    #[test]
    fn test_succes_check_canopy_no_nodes_to_right_of_index_no_canopy() {
        let canopy_bytes = vec![];
        check_canopy_no_nodes_to_right_of_index(&canopy_bytes, 20, 0).unwrap();
    }
}
