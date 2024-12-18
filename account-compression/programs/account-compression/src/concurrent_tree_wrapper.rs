//! This module provides a wrapper around the `ConcurrentMerkleTree` struct from
//! the `spl_concurrent_merkle_tree` crate. It provides a set of functions that
//! can be called from the Anchor program to interact with the tree.
//! The functions are used to initialize the tree, set a leaf, fill empty or
//! append a leaf, and prove a leaf. As the tree is generic over the depth and
//! buffer size, the functions are implemented using macros that infer the depth
//! and buffer size from the header information stored on-chain. Usage of the
//! macros directly is discouraged, as they have huge match statements with
//! every case taking it's own stack frame. Instead, use the exported functions
//! from this module and reference or Box the arguments to the functions to
//! avoid the stack frame explosion.

pub use crate::error::AccountCompressionError;
/// Exported for Anchor / Solita
pub use spl_concurrent_merkle_tree::{
    concurrent_merkle_tree::{
        ConcurrentMerkleTree, FillEmptyOrAppendArgs, InitializeWithRootArgs, ProveLeafArgs,
        SetLeafArgs,
    },
    error::ConcurrentMerkleTreeError,
    node::Node,
    node::EMPTY,
};
use {
    crate::{
        events::ChangeLogEvent, macros::*, state::ConcurrentMerkleTreeHeader, zero_copy::ZeroCopy,
    },
    anchor_lang::prelude::*,
};

#[inline(never)]
pub fn merkle_tree_initialize_empty(
    header: &ConcurrentMerkleTreeHeader,
    tree_id: Pubkey,
    tree_bytes: &mut [u8],
) -> Result<Box<ChangeLogEvent>> {
    merkle_tree_apply_fn_mut!(header, tree_id, tree_bytes, initialize,)
}

#[inline(never)]
pub fn merkle_tree_initialize_with_root(
    header: &ConcurrentMerkleTreeHeader,
    tree_id: Pubkey,
    tree_bytes: &mut [u8],
    args: &InitializeWithRootArgs,
) -> Result<Box<ChangeLogEvent>> {
    merkle_tree_apply_fn_mut!(header, tree_id, tree_bytes, initialize_with_root, args)
}

#[inline(never)]
pub fn merkle_tree_set_leaf(
    header: &ConcurrentMerkleTreeHeader,
    tree_id: Pubkey,
    tree_bytes: &mut [u8],
    args: &SetLeafArgs,
) -> Result<Box<ChangeLogEvent>> {
    merkle_tree_apply_fn_mut!(header, tree_id, tree_bytes, set_leaf, args)
}

#[inline(never)]
pub fn merkle_tree_fill_empty_or_append(
    header: &ConcurrentMerkleTreeHeader,
    tree_id: Pubkey,
    tree_bytes: &mut [u8],
    args: &FillEmptyOrAppendArgs,
) -> Result<Box<ChangeLogEvent>> {
    merkle_tree_apply_fn_mut!(header, tree_id, tree_bytes, fill_empty_or_append, args)
}

#[inline(never)]
pub fn merkle_tree_prove_leaf(
    header: &ConcurrentMerkleTreeHeader,
    tree_id: Pubkey,
    tree_bytes: &[u8],
    args: &ProveLeafArgs,
) -> Result<Box<ChangeLogEvent>> {
    merkle_tree_apply_fn!(header, tree_id, tree_bytes, prove_leaf, args)
}

#[inline(never)]
pub fn merkle_tree_append_leaf(
    header: &ConcurrentMerkleTreeHeader,
    tree_id: Pubkey,
    tree_bytes: &mut [u8],
    args: &[u8; 32],
) -> Result<Box<ChangeLogEvent>> {
    merkle_tree_apply_fn_mut!(header, tree_id, tree_bytes, append, *args)
}

/// Checks whether the tree in not initialized yet without doing the deserialization. A rough
/// equivalent to deserializing the tree and calling is_initialized() on it without the heavy
/// lifting with macros. An empty account is a zero'd account. The tree is considered empty if the
/// tree_bytes are all 0. A regular non-batch initialized tree is initialized early on when the
/// init_empty_merkle_tree is called. A batch initialized tree stays uninitialized until the
/// init_prepared_tree_with_root is called.
pub fn tree_bytes_uninitialized(tree_bytes: &[u8]) -> bool {
    tree_bytes.iter().all(|&x| x == 0)
}

#[inline(never)]
pub fn assert_tree_is_empty(
    header: &ConcurrentMerkleTreeHeader,
    tree_id: Pubkey,
    tree_bytes: &mut [u8],
) -> Result<()> {
    // If the tree is batch initialized and not finalized yet, we can treat it as empty.
    // Before the tree is finalized, the tree_bytes will be all 0 as only the header will be
    // initialized at that point, so we may skip the deserialization.
    if header.get_is_batch_initialized() && tree_bytes_uninitialized(tree_bytes) {
        return Ok(());
    }
    // check the tree is empty
    merkle_tree_apply_fn_mut!(header, tree_id, tree_bytes, prove_tree_is_empty,)?;
    Ok(())
}
