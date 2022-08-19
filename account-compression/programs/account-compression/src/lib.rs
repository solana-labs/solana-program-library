//! SPL Compression is an on-chain program that exposes an interface to manipulating SPL ConcurrentMerkleTrees
//!
//! A buffer of proof-like changelogs is stored on-chain that allow multiple proof-based writes to succeed within the same slot.
//! This is accomplished by fast-forwarding out-of-date (or possibly invalid) proofs based on information stored in the changelogs.
//! See a copy of the whitepaper [here](https://drive.google.com/file/d/1BOpa5OFmara50fTvL0VIVYjtg-qzHCVc/view)
//!
//! While SPL ConcurrentMerkleTrees can generically store arbitrary information,
//! one exemplified use-case is the [Bubblegum](https://github.com/metaplex-foundation/metaplex-program-library/tree/master/bubblegum) contract,
//! which uses SPL-Compression to store encoded information about NFTs.
//! The use of SPL-Compression within Bubblegum allows for:
//! - up to 1 billion NFTs to be stored in a single account on-chain (>10,000x decrease in on-chain cost)
//! - up to 2048 concurrent updates per slot
//!
//! Operationally, SPL ConcurrentMerkleTrees **must** be supplemented by off-chain indexers to cache information
//! about leafs and to power an API that can supply up-to-date proofs to allow updates to the tree.
//! All modifications to SPL ConcurrentMerkleTrees are settled on the Solana ledger via instructions against the SPL Compression contract.
//! A production-ready indexer (Plerkle) can be found in the [Metaplex program library](https://github.com/metaplex-foundation/digital-asset-validator-plugin)

use anchor_lang::{
    emit,
    prelude::*,
    solana_program::sysvar::{clock::Clock, rent::Rent},
};
use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::cast_slice_mut;
use spl_concurrent_merkle_tree::node::{empty_node_cached, EMPTY};
use std::mem::size_of;

pub mod data_wrapper;
pub mod error;
pub mod events;
pub mod state;
pub mod zero_copy;

use crate::data_wrapper::{wrap_event, Wrapper};
use crate::error::AccountCompressionError;
use crate::events::ChangeLogEvent;
use crate::state::ConcurrentMerkleTreeHeader;
use crate::zero_copy::ZeroCopy;

/// Exported for Anchor / Solita
pub use spl_concurrent_merkle_tree::{
    concurrent_merkle_tree::ConcurrentMerkleTree, error::ConcurrentMerkleTreeError, node::Node,
};

declare_id!("GRoLLzvxpxxu2PGNJMMeZPyMxjAUH9pKqxGXV9DGiceU");

/// Context for initializing a new SPL ConcurrentMerkleTree
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(zero)]
    /// CHECK: This account will be zeroed out, and the size will be validated
    pub merkle_tree: UncheckedAccount<'info>,

    /// Authority that validates the content of the trees.
    /// Typically a program, e.g., the Bubblegum contract validates that leaves are valid NFTs.
    pub authority: Signer<'info>,

    /// Program used to emit changelogs as instruction data.
    /// See `WRAPYChf58WFCnyjXKJHtrPgzKXgHp6MD9aVDqJBbGh`
    pub log_wrapper: Program<'info, Wrapper>,
}

/// Context for inserting, appending, or replacing a leaf in the tree
#[derive(Accounts)]
pub struct Modify<'info> {
    #[account(mut)]
    /// CHECK: This account is validated in the instruction
    pub merkle_tree: UncheckedAccount<'info>,

    /// Authority that validates the content of the trees.
    /// Typically a program, e.g., the Bubblegum contract validates that leaves are valid NFTs.
    pub authority: Signer<'info>,

    /// Program used to emit changelogs as instruction data.
    /// See `WRAPYChf58WFCnyjXKJHtrPgzKXgHp6MD9aVDqJBbGh`
    pub log_wrapper: Program<'info, Wrapper>,
}

/// Context for validating a provided proof against the SPL ConcurrentMerkleTree.
/// Throws an error if provided proof is invalid.
#[derive(Accounts)]
pub struct VerifyLeaf<'info> {
    /// CHECK: This account is validated in the instruction
    pub merkle_tree: UncheckedAccount<'info>,
}

/// Context for transferring `authority`
#[derive(Accounts)]
pub struct TransferAuthority<'info> {
    #[account(mut)]
    /// CHECK: This account is validated in the instruction
    pub merkle_tree: UncheckedAccount<'info>,

    /// Authority that validates the content of the trees.
    /// Typically a program, e.g., the Bubblegum contract validates that leaves are valid NFTs.
    pub authority: Signer<'info>,
}

#[inline(always)]
fn check_canopy_bytes(canopy_bytes: &mut [u8]) -> Result<()> {
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
fn get_cached_path_length(canopy: &mut [Node], max_depth: u32) -> Result<u32> {
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

fn update_canopy(
    canopy_bytes: &mut [u8],
    max_depth: u32,
    change_log: Option<Box<ChangeLogEvent>>,
) -> Result<()> {
    check_canopy_bytes(canopy_bytes)?;
    let canopy = cast_slice_mut::<u8, Node>(canopy_bytes);
    let path_len = get_cached_path_length(canopy, max_depth)?;
    if let Some(cl) = change_log {
        // Update the canopy from the newest change log
        for path_node in cl.path.iter().rev().skip(1).take(path_len as usize) {
            // node_idx - 2 maps to the canopy index
            canopy[(path_node.index - 2) as usize] = path_node.node;
        }
    }
    Ok(())
}

fn fill_in_proof_from_canopy(
    canopy_bytes: &mut [u8],
    max_depth: u32,
    index: u32,
    proof: &mut Vec<Node>,
) -> Result<()> {
    // 30 is hard coded as it is the current max depth that SPL Compression supports
    let mut empty_node_cache = Box::new([EMPTY; 30]);
    check_canopy_bytes(canopy_bytes)?;
    let canopy = cast_slice_mut::<u8, Node>(canopy_bytes);
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
            canopy[cached_idx] = empty_node;
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

/// This macro applies functions on a merkle roll and emits leaf information
/// needed to sync the merkle tree state with off-chain indexers.
macro_rules! merkle_tree_depth_size_apply_fn {
    ($max_depth:literal, $max_size:literal, $id:ident, $bytes:ident, $func:ident, $($arg:tt)*) => {
        match ConcurrentMerkleTree::<$max_depth, $max_size>::load_mut_bytes($bytes) {
            Ok(merkle_tree) => {
                match merkle_tree.$func($($arg)*) {
                    Ok(_) => {
                        Ok(Box::<ChangeLogEvent>::from((merkle_tree.get_change_log(), $id, merkle_tree.sequence_number)))
                    }
                    Err(err) => {
                        msg!("Error using concurrent merkle tree: {}", err);
                        err!(AccountCompressionError::ConcurrentMerkleTreeError)
                    }
                }
            }
            Err(err) => {
                msg!("Error zero copying merkle roll: {}", err);
                err!(AccountCompressionError::ZeroCopyError)
            }
        }
    }
}

/// This applies a given function on a merkle roll by
/// allowing the compiler to infer the size of the tree based
/// upon the header information stored on-chain
fn merkle_tree_get_size(header: &ConcurrentMerkleTreeHeader) -> Result<usize> {
    // Note: max_buffer_size MUST be a power of 2
    match (header.max_depth, header.max_buffer_size) {
        (3, 8) => Ok(size_of::<ConcurrentMerkleTree<3, 8>>()),
        (5, 8) => Ok(size_of::<ConcurrentMerkleTree<5, 8>>()),
        (14, 64) => Ok(size_of::<ConcurrentMerkleTree<14, 64>>()),
        (14, 256) => Ok(size_of::<ConcurrentMerkleTree<14, 256>>()),
        (14, 1024) => Ok(size_of::<ConcurrentMerkleTree<14, 1024>>()),
        (14, 2048) => Ok(size_of::<ConcurrentMerkleTree<14, 2048>>()),
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
                header.max_depth,
                header.max_buffer_size
            );
            err!(AccountCompressionError::ConcurrentMerkleTreeConstantsError)
        }
    }
}

/// This applies a given function on a merkle roll by
/// allowing the compiler to infer the size of the tree based
/// upon the header information stored on-chain
macro_rules! merkle_tree_apply_fn {
    ($header:ident, $id:ident, $bytes:ident, $func:ident, $($arg:tt)*) => {
        // Note: max_buffer_size MUST be a power of 2
        match ($header.max_depth, $header.max_buffer_size) {
            (3, 8) => merkle_tree_depth_size_apply_fn!(3, 8, $id, $bytes, $func, $($arg)*),
            (5, 8) => merkle_tree_depth_size_apply_fn!(5, 8, $id, $bytes, $func, $($arg)*),
            (14, 64) => merkle_tree_depth_size_apply_fn!(14, 64, $id, $bytes, $func, $($arg)*),
            (14, 256) => merkle_tree_depth_size_apply_fn!(14, 256, $id, $bytes, $func, $($arg)*),
            (14, 1024) => merkle_tree_depth_size_apply_fn!(14, 1024, $id, $bytes, $func, $($arg)*),
            (14, 2048) => merkle_tree_depth_size_apply_fn!(14, 2048, $id, $bytes, $func, $($arg)*),
            (20, 64) => merkle_tree_depth_size_apply_fn!(20, 64, $id, $bytes, $func, $($arg)*),
            (20, 256) => merkle_tree_depth_size_apply_fn!(20, 256, $id, $bytes, $func, $($arg)*),
            (20, 1024) => merkle_tree_depth_size_apply_fn!(20, 1024, $id, $bytes, $func, $($arg)*),
            (20, 2048) => merkle_tree_depth_size_apply_fn!(20, 2048, $id, $bytes, $func, $($arg)*),
            (24, 64) => merkle_tree_depth_size_apply_fn!(24, 64, $id, $bytes, $func, $($arg)*),
            (24, 256) => merkle_tree_depth_size_apply_fn!(24, 256, $id, $bytes, $func, $($arg)*),
            (24, 512) => merkle_tree_depth_size_apply_fn!(24, 512, $id, $bytes, $func, $($arg)*),
            (24, 1024) => merkle_tree_depth_size_apply_fn!(24, 1024, $id, $bytes, $func, $($arg)*),
            (24, 2048) => merkle_tree_depth_size_apply_fn!(24, 2048, $id, $bytes, $func, $($arg)*),
            (26, 512) => merkle_tree_depth_size_apply_fn!(26, 512, $id, $bytes, $func, $($arg)*),
            (26, 1024) => merkle_tree_depth_size_apply_fn!(26, 1024, $id, $bytes, $func, $($arg)*),
            (26, 2048) => merkle_tree_depth_size_apply_fn!(26, 2048, $id, $bytes, $func, $($arg)*),
            (30, 512) => merkle_tree_depth_size_apply_fn!(30, 512, $id, $bytes, $func, $($arg)*),
            (30, 1024) => merkle_tree_depth_size_apply_fn!(30, 1024, $id, $bytes, $func, $($arg)*),
            (30, 2048) => merkle_tree_depth_size_apply_fn!(30, 2048, $id, $bytes, $func, $($arg)*),
            _ => {
                msg!("Failed to apply {} on merkle roll with max depth {} and max buffer size {}", stringify!($func), $header.max_depth, $header.max_buffer_size);
                err!(AccountCompressionError::ConcurrentMerkleTreeConstantsError)
            }
        }
    };
}

#[program]
pub mod spl_compression {
    use super::*;

    /// Creates a new merkle tree with maximum leaf capacity of `power(2, max_depth)`
    /// and a minimum concurrency limit of `max_buffer_size`.
    ///
    /// Concurrency limit represents the # of replace instructions that can be successfully
    /// executed with proofs dated for the same root. For example, a maximum buffer size of 1024
    /// means that a minimum of 1024 replaces can be executed before a new proof must be
    /// generated for the next replace instruction.
    ///
    /// Concurrency limit should be determined by empirically testing the demand for
    /// state built on top of SPL Compression.
    pub fn init_empty_gummyroll(
        ctx: Context<Initialize>,
        max_depth: u32,
        max_buffer_size: u32,
    ) -> Result<()> {
        let mut merkle_tree_bytes = ctx.accounts.merkle_tree.try_borrow_mut_data()?;

        let (mut header_bytes, rest) =
            merkle_tree_bytes.split_at_mut(size_of::<ConcurrentMerkleTreeHeader>());

        let mut header = ConcurrentMerkleTreeHeader::try_from_slice(&header_bytes)?;
        header.initialize(
            max_depth,
            max_buffer_size,
            &ctx.accounts.authority.key(),
            Clock::get()?.slot,
        );
        header.serialize(&mut header_bytes)?;
        let merkle_tree_size = merkle_tree_get_size(&header)?;
        let (tree_bytes, canopy_bytes) = rest.split_at_mut(merkle_tree_size);
        let id = ctx.accounts.merkle_tree.key();
        let change_log = merkle_tree_apply_fn!(header, id, tree_bytes, initialize,)?;
        wrap_event(change_log.try_to_vec()?, &ctx.accounts.log_wrapper)?;
        emit!(*change_log);
        update_canopy(canopy_bytes, header.max_depth, None)
    }

    /// Note:
    /// Supporting this instruction open a security vulnerability for indexers.
    /// This instruction has been deemed unusable for publicly indexed compressed NFTs.
    /// Indexing batched data in this way requires indexers to read in the `uri`s onto physical storage
    /// and then into their database. This opens up a DOS attack vector, whereby this instruction is
    /// repeatedly invoked, causing indexers to fail.
    pub fn init_gummyroll_with_root(
        ctx: Context<Initialize>,
        max_depth: u32,
        max_buffer_size: u32,
        root: [u8; 32],
        leaf: [u8; 32],
        index: u32,
        _changelog_db_uri: String,
        _metadata_db_uri: String,
    ) -> Result<()> {
        let mut merkle_tree_bytes = ctx.accounts.merkle_tree.try_borrow_mut_data()?;

        let (mut header_bytes, rest) =
            merkle_tree_bytes.split_at_mut(size_of::<ConcurrentMerkleTreeHeader>());

        let mut header = ConcurrentMerkleTreeHeader::try_from_slice(&header_bytes)?;
        header.initialize(
            max_depth,
            max_buffer_size,
            &ctx.accounts.authority.key(),
            Clock::get()?.slot,
        );
        header.serialize(&mut header_bytes)?;
        let merkle_tree_size = merkle_tree_get_size(&header)?;
        let (tree_bytes, canopy_bytes) = rest.split_at_mut(merkle_tree_size);

        // Get rightmost proof from accounts
        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(node.key().to_bytes());
        }
        fill_in_proof_from_canopy(canopy_bytes, header.max_depth, index, &mut proof)?;
        assert_eq!(proof.len(), max_depth as usize);

        let id = ctx.accounts.merkle_tree.key();
        // A call is made to ConcurrentMerkleTree::initialize_with_root(root, leaf, proof, index)
        let change_log = merkle_tree_apply_fn!(
            header,
            id,
            tree_bytes,
            initialize_with_root,
            root,
            leaf,
            &proof,
            index
        )?;
        wrap_event(change_log.try_to_vec()?, &ctx.accounts.log_wrapper)?;
        emit!(*change_log);
        update_canopy(canopy_bytes, header.max_depth, Some(change_log))
    }

    /// Executes an instruction that overwrites a leaf node.
    /// Composing programs should check that the data hashed into previous_leaf
    /// matches the authority information necessary to execute this instruction.
    pub fn replace_leaf(
        ctx: Context<Modify>,
        root: [u8; 32],
        previous_leaf: [u8; 32],
        new_leaf: [u8; 32],
        index: u32,
    ) -> Result<()> {
        let mut merkle_tree_bytes = ctx.accounts.merkle_tree.try_borrow_mut_data()?;
        let (header_bytes, rest) =
            merkle_tree_bytes.split_at_mut(size_of::<ConcurrentMerkleTreeHeader>());

        let header = ConcurrentMerkleTreeHeader::try_from_slice(header_bytes)?;
        require_eq!(
            header.authority,
            ctx.accounts.authority.key(),
            AccountCompressionError::IncorrectAuthority
        );

        let merkle_tree_size = merkle_tree_get_size(&header)?;
        let (tree_bytes, canopy_bytes) = rest.split_at_mut(merkle_tree_size);

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(node.key().to_bytes());
        }
        fill_in_proof_from_canopy(canopy_bytes, header.max_depth, index, &mut proof)?;
        let id = ctx.accounts.merkle_tree.key();
        // A call is made to ConcurrentMerkleTree::set_leaf(root, previous_leaf, new_leaf, proof, index)
        let change_log = merkle_tree_apply_fn!(
            header,
            id,
            tree_bytes,
            set_leaf,
            root,
            previous_leaf,
            new_leaf,
            &proof,
            index,
        )?;
        wrap_event(change_log.try_to_vec()?, &ctx.accounts.log_wrapper)?;
        emit!(*change_log);
        update_canopy(canopy_bytes, header.max_depth, Some(change_log))
    }

    /// Transfers `authority`.
    /// Requires `authority` to sign
    pub fn transfer_authority(
        ctx: Context<TransferAuthority>,
        new_authority: Pubkey,
    ) -> Result<()> {
        let mut merkle_tree_bytes = ctx.accounts.merkle_tree.try_borrow_mut_data()?;
        let (mut header_bytes, _) =
            merkle_tree_bytes.split_at_mut(size_of::<ConcurrentMerkleTreeHeader>());

        let mut header = Box::new(ConcurrentMerkleTreeHeader::try_from_slice(header_bytes)?);
        require_eq!(
            header.authority,
            ctx.accounts.authority.key(),
            AccountCompressionError::IncorrectAuthority
        );

        header.authority = new_authority;
        msg!("Authority transferred to: {:?}", header.authority);
        header.serialize(&mut header_bytes)?;

        Ok(())
    }

    /// Verifies a provided proof and leaf.
    /// If invalid, throws an error.
    pub fn verify_leaf(
        ctx: Context<VerifyLeaf>,
        root: [u8; 32],
        leaf: [u8; 32],
        index: u32,
    ) -> Result<()> {
        let mut merkle_tree_bytes = ctx.accounts.merkle_tree.try_borrow_mut_data()?;
        let (header_bytes, rest) =
            merkle_tree_bytes.split_at_mut(size_of::<ConcurrentMerkleTreeHeader>());
        let header = ConcurrentMerkleTreeHeader::try_from_slice(header_bytes)?;
        let merkle_tree_size = merkle_tree_get_size(&header)?;
        let (tree_bytes, canopy_bytes) = rest.split_at_mut(merkle_tree_size);

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(node.key().to_bytes());
        }
        fill_in_proof_from_canopy(canopy_bytes, header.max_depth, index, &mut proof)?;
        let id = ctx.accounts.merkle_tree.key();

        merkle_tree_apply_fn!(header, id, tree_bytes, prove_leaf, root, leaf, &proof, index)?;
        Ok(())
    }

    /// This instruction allows the tree's `authority` to append a new leaf to the tree
    /// without having to supply a valid proof.
    ///
    /// This is accomplished by using the rightmost_proof of the merkle roll to construct a
    /// valid proof, and then updating the rightmost_proof for the next leaf if possible.
    pub fn append(ctx: Context<Modify>, leaf: [u8; 32]) -> Result<()> {
        let mut merkle_tree_bytes = ctx.accounts.merkle_tree.try_borrow_mut_data()?;
        let (header_bytes, rest) =
            merkle_tree_bytes.split_at_mut(size_of::<ConcurrentMerkleTreeHeader>());

        let header = ConcurrentMerkleTreeHeader::try_from_slice(header_bytes)?;
        require_eq!(
            header.authority,
            ctx.accounts.authority.key(),
            AccountCompressionError::IncorrectAuthority
        );

        let id = ctx.accounts.merkle_tree.key();
        let merkle_tree_size = merkle_tree_get_size(&header)?;
        let (tree_bytes, canopy_bytes) = rest.split_at_mut(merkle_tree_size);
        let change_log = merkle_tree_apply_fn!(header, id, tree_bytes, append, leaf)?;
        wrap_event(change_log.try_to_vec()?, &ctx.accounts.log_wrapper)?;
        emit!(*change_log);
        update_canopy(canopy_bytes, header.max_depth, Some(change_log))
    }

    /// This instruction takes a proof, and will attempt to write the given leaf
    /// to the specified index in the tree. If the insert operation fails, the leaf will be `append`-ed
    /// to the tree.
    /// It is up to the indexer to parse the final location of the leaf from the emitted changelog.
    pub fn insert_or_append(
        ctx: Context<Modify>,
        root: [u8; 32],
        leaf: [u8; 32],
        index: u32,
    ) -> Result<()> {
        let mut merkle_tree_bytes = ctx.accounts.merkle_tree.try_borrow_mut_data()?;
        let (header_bytes, rest) =
            merkle_tree_bytes.split_at_mut(size_of::<ConcurrentMerkleTreeHeader>());
        let header = ConcurrentMerkleTreeHeader::try_from_slice(header_bytes)?;
        require_eq!(
            header.authority,
            ctx.accounts.authority.key(),
            AccountCompressionError::IncorrectAuthority
        );

        let merkle_tree_size = merkle_tree_get_size(&header)?;
        let (tree_bytes, canopy_bytes) = rest.split_at_mut(merkle_tree_size);

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(node.key().to_bytes());
        }
        fill_in_proof_from_canopy(canopy_bytes, header.max_depth, index, &mut proof)?;
        // A call is made to ConcurrentMerkleTree::fill_empty_or_append
        let id = ctx.accounts.merkle_tree.key();
        let change_log = merkle_tree_apply_fn!(
            header,
            id,
            tree_bytes,
            fill_empty_or_append,
            root,
            leaf,
            &proof,
            index,
        )?;
        wrap_event(change_log.try_to_vec()?, &ctx.accounts.log_wrapper)?;
        emit!(*change_log);
        update_canopy(canopy_bytes, header.max_depth, Some(change_log))
    }
}
