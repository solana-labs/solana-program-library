use anchor_lang::{
    emit,
    prelude::*,
    solana_program::sysvar::{clock::Clock, rent::Rent},
};
use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::cast_slice_mut;
use concurrent_merkle_tree::{state::EMPTY, utils::empty_node_cached};
use std::mem::size_of;

pub mod error;
pub mod state;
pub mod utils;

use crate::error::GummyrollError;
use crate::state::{ChangeLogEvent, MerkleRollHeader};
use crate::utils::ZeroCopy;
pub use concurrent_merkle_tree::{error::CMTError, merkle_roll::MerkleRoll, state::Node};

declare_id!("GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD");

const MAX_TREE_DEPTH: usize = 30;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(zero)]
    /// CHECK: This account will be zeroed out, and the size will be validated
    pub merkle_roll: UncheckedAccount<'info>,
    pub authority: Signer<'info>,
    /// CHECK: unsafe
    pub append_authority: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Modify<'info> {
    #[account(mut)]
    /// CHECK: This account is validated in the instruction
    pub merkle_roll: UncheckedAccount<'info>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Append<'info> {
    #[account(mut)]
    /// CHECK: This account is validated in the instruction
    pub merkle_roll: UncheckedAccount<'info>,
    pub authority: Signer<'info>,
    pub append_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct VerifyLeaf<'info> {
    /// CHECK: This account is validated in the instruction
    pub merkle_roll: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct TransferAuthority<'info> {
    #[account(mut)]
    /// CHECK: This account is validated in the instruction
    pub merkle_roll: UncheckedAccount<'info>,
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
        err!(GummyrollError::CanopyLengthMismatch)
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
            return err!(GummyrollError::CanopyLengthMismatch);
        }
    } else {
        msg!(
            "Canopy length {} is not 2 less than a power of 2",
            canopy.len()
        );
        return err!(GummyrollError::CanopyLengthMismatch);
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
    // 26 is hard coded as it is the current max depth that Gummyroll supports
    let mut empty_node_cache = Box::new([EMPTY; MAX_TREE_DEPTH]);
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
            let empty_node = empty_node_cached::<MAX_TREE_DEPTH>(level, &mut empty_node_cache);
            canopy[cached_idx] = empty_node;
            inferred_nodes.push(empty_node);
        } else {
            inferred_nodes.push(canopy[cached_idx]);
        }
        node_idx >>= 1;
    }
    proof.extend(inferred_nodes.iter());
    Ok(())
}

/// This macro applies functions on a merkle roll and emits leaf information
/// needed to sync the merkle tree state with off-chain indexers.
macro_rules! merkle_roll_depth_size_apply_fn {
    ($max_depth:literal, $max_size:literal, $id:ident, $bytes:ident, $func:ident, $($arg:tt)*) => {
        match MerkleRoll::<$max_depth, $max_size>::load_mut_bytes($bytes) {
            Ok(merkle_roll) => {
                match merkle_roll.$func($($arg)*) {
                    Ok(_) => {
                        Ok(Box::<ChangeLogEvent>::from((merkle_roll.get_change_log(), $id, merkle_roll.sequence_number)))
                    }
                    Err(err) => {
                        msg!("Error using concurrent merkle tree: {}", err);
                        err!(GummyrollError::ConcurrentMerkleTreeError)
                    }
                }
            }
            Err(err) => {
                msg!("Error zero copying merkle roll: {}", err);
                err!(GummyrollError::ZeroCopyError)
            }
        }
    }
}

/// This applies a given function on a merkle roll by
/// allowing the compiler to infer the size of the tree based
/// upon the header information stored on-chain
macro_rules! merkle_roll_get_size {
    ($header:ident) => {
        // Note: max_buffer_size MUST be a power of 2
        match ($header.max_depth, $header.max_buffer_size) {
            (3, 8) => Ok(size_of::<MerkleRoll<3, 8>>()),
            (5, 8) => Ok(size_of::<MerkleRoll<5, 8>>()),
            (14, 64) => Ok(size_of::<MerkleRoll<14, 64>>()),
            (14, 256) => Ok(size_of::<MerkleRoll<14, 256>>()),
            (14, 1024) => Ok(size_of::<MerkleRoll<14, 1024>>()),
            (14, 2048) => Ok(size_of::<MerkleRoll<14, 2048>>()),
            (20, 64) => Ok(size_of::<MerkleRoll<20, 64>>()),
            (20, 256) => Ok(size_of::<MerkleRoll<20, 256>>()),
            (20, 1024) => Ok(size_of::<MerkleRoll<20, 1024>>()),
            (20, 2048) => Ok(size_of::<MerkleRoll<20, 2048>>()),
            (24, 64) => Ok(size_of::<MerkleRoll<24, 64>>()),
            (24, 256) => Ok(size_of::<MerkleRoll<24, 256>>()),
            (24, 512) => Ok(size_of::<MerkleRoll<24, 512>>()),
            (24, 1024) => Ok(size_of::<MerkleRoll<24, 1024>>()),
            (24, 2048) => Ok(size_of::<MerkleRoll<24, 2048>>()),
            (26, 512) => Ok(size_of::<MerkleRoll<26, 512>>()),
            (26, 1024) => Ok(size_of::<MerkleRoll<26, 1024>>()),
            (26, 2048) => Ok(size_of::<MerkleRoll<26, 2048>>()),
            (30, 512) => Ok(size_of::<MerkleRoll<30, 512>>()),
            (30, 1024) => Ok(size_of::<MerkleRoll<30, 1024>>()),
            (30, 2048) => Ok(size_of::<MerkleRoll<30, 2048>>()),
            _ => {
                msg!(
                    "Failed to get size of max depth {} and max buffer size {}",
                    $header.max_depth,
                    $header.max_buffer_size
                );
                err!(GummyrollError::MerkleRollConstantsError)
            }
        }
    };
}

/// This applies a given function on a merkle roll by
/// allowing the compiler to infer the size of the tree based
/// upon the header information stored on-chain
macro_rules! merkle_roll_apply_fn {
    ($header:ident, $id:ident, $bytes:ident, $func:ident, $($arg:tt)*) => {
        // Note: max_buffer_size MUST be a power of 2
        match ($header.max_depth, $header.max_buffer_size) {
            (3, 8) => merkle_roll_depth_size_apply_fn!(3, 8, $id, $bytes, $func, $($arg)*),
            (5, 8) => merkle_roll_depth_size_apply_fn!(5, 8, $id, $bytes, $func, $($arg)*),
            (14, 64) => merkle_roll_depth_size_apply_fn!(14, 64, $id, $bytes, $func, $($arg)*),
            (14, 256) => merkle_roll_depth_size_apply_fn!(14, 256, $id, $bytes, $func, $($arg)*),
            (14, 1024) => merkle_roll_depth_size_apply_fn!(14, 1024, $id, $bytes, $func, $($arg)*),
            (14, 2048) => merkle_roll_depth_size_apply_fn!(14, 2048, $id, $bytes, $func, $($arg)*),
            (20, 64) => merkle_roll_depth_size_apply_fn!(20, 64, $id, $bytes, $func, $($arg)*),
            (20, 256) => merkle_roll_depth_size_apply_fn!(20, 256, $id, $bytes, $func, $($arg)*),
            (20, 1024) => merkle_roll_depth_size_apply_fn!(20, 1024, $id, $bytes, $func, $($arg)*),
            (20, 2048) => merkle_roll_depth_size_apply_fn!(20, 2048, $id, $bytes, $func, $($arg)*),
            (24, 64) => merkle_roll_depth_size_apply_fn!(24, 64, $id, $bytes, $func, $($arg)*),
            (24, 256) => merkle_roll_depth_size_apply_fn!(24, 256, $id, $bytes, $func, $($arg)*),
            (24, 512) => merkle_roll_depth_size_apply_fn!(24, 512, $id, $bytes, $func, $($arg)*),
            (24, 1024) => merkle_roll_depth_size_apply_fn!(24, 1024, $id, $bytes, $func, $($arg)*),
            (24, 2048) => merkle_roll_depth_size_apply_fn!(24, 2048, $id, $bytes, $func, $($arg)*),
            (26, 512) => merkle_roll_depth_size_apply_fn!(26, 512, $id, $bytes, $func, $($arg)*),
            (26, 1024) => merkle_roll_depth_size_apply_fn!(26, 1024, $id, $bytes, $func, $($arg)*),
            (26, 2048) => merkle_roll_depth_size_apply_fn!(26, 2048, $id, $bytes, $func, $($arg)*),
            (30, 512) => merkle_roll_depth_size_apply_fn!(30, 512, $id, $bytes, $func, $($arg)*),
            (30, 1024) => merkle_roll_depth_size_apply_fn!(30, 1024, $id, $bytes, $func, $($arg)*),
            (30, 2048) => merkle_roll_depth_size_apply_fn!(30, 2048, $id, $bytes, $func, $($arg)*),
            _ => {
                msg!("Failed to apply {} on merkle roll with max depth {} and max buffer size {}", stringify!($func), $header.max_depth, $header.max_buffer_size);
                err!(GummyrollError::MerkleRollConstantsError)
            }
        }
    };
}

#[program]
pub mod gummyroll {
    use super::*;

    /// Creates a new merkle tree with maximum leaf capacity of power(2, max_depth)
    /// and a minimum concurrency limit of max_buffer_size.
    ///
    /// Concurrency limit represents the # of replace instructions that can be successfully
    /// executed with proofs dated for the same root. For example, a maximum buffer size of 1024
    /// means that a minimum of 1024 replaces can be executed before a new proof must be
    /// generated for the next replace instruction.
    ///
    /// Concurrency limit should be determined by empirically testing the demand for
    /// state built on top of gummyroll.
    pub fn init_empty_gummyroll(
        ctx: Context<Initialize>,
        max_depth: u32,
        max_buffer_size: u32,
    ) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;

        let (mut header_bytes, rest) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let mut header = Box::new(MerkleRollHeader::try_from_slice(&header_bytes)?);
        header.initialize(
            max_depth,
            max_buffer_size,
            &ctx.accounts.authority.key(),
            &ctx.accounts.append_authority.key(),
            Clock::get()?.slot,
        );
        header.serialize(&mut header_bytes)?;
        let merkle_roll_size = merkle_roll_get_size!(header)?;
        let (roll_bytes, canopy_bytes) = rest.split_at_mut(merkle_roll_size);
        let id = ctx.accounts.merkle_roll.key();
        let change_log = merkle_roll_apply_fn!(header, id, roll_bytes, initialize,)?;
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
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;

        let (mut header_bytes, rest) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let mut header = Box::new(MerkleRollHeader::try_from_slice(&header_bytes)?);
        header.initialize(
            max_depth,
            max_buffer_size,
            &ctx.accounts.authority.key(),
            &ctx.accounts.append_authority.key(),
            Clock::get()?.slot,
        );
        header.serialize(&mut header_bytes)?;
        let merkle_roll_size = merkle_roll_get_size!(header)?;
        let (roll_bytes, canopy_bytes) = rest.split_at_mut(merkle_roll_size);

        // Get rightmost proof from accounts
        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(node.key().to_bytes());
        }
        fill_in_proof_from_canopy(canopy_bytes, header.max_depth, index, &mut proof)?;
        assert_eq!(proof.len(), max_depth as usize);

        let id = ctx.accounts.merkle_roll.key();
        // A call is made to MerkleRoll::initialize_with_root(root, leaf, proof, index)
        let change_log = merkle_roll_apply_fn!(
            header,
            id,
            roll_bytes,
            initialize_with_root,
            root,
            leaf,
            &proof,
            index
        )?;
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
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, rest) = merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let header = Box::new(MerkleRollHeader::try_from_slice(header_bytes)?);
        assert_eq!(header.authority, ctx.accounts.authority.key());
        let merkle_roll_size = merkle_roll_get_size!(header)?;
        let (roll_bytes, canopy_bytes) = rest.split_at_mut(merkle_roll_size);

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(node.key().to_bytes());
        }
        fill_in_proof_from_canopy(canopy_bytes, header.max_depth, index, &mut proof)?;
        let id = ctx.accounts.merkle_roll.key();
        // A call is made to MerkleRoll::set_leaf(root, previous_leaf, new_leaf, proof, index)
        let change_log = merkle_roll_apply_fn!(
            header,
            id,
            roll_bytes,
            set_leaf,
            root,
            previous_leaf,
            new_leaf,
            &proof,
            index,
        )?;
        emit!(*change_log);
        update_canopy(canopy_bytes, header.max_depth, Some(change_log))
    }

    /// Transfers authority or append authority
    /// requires `authority` to sign
    pub fn transfer_authority(
        ctx: Context<TransferAuthority>,
        new_authority: Option<Pubkey>,
        new_append_authority: Option<Pubkey>,
    ) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (mut header_bytes, _) = merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let mut header = Box::new(MerkleRollHeader::try_from_slice(header_bytes)?);
        assert_eq!(header.authority, ctx.accounts.authority.key());

        match new_authority {
            Some(new_auth) => {
                header.authority = new_auth;
                msg!("Authority transferred to: {:?}", header.authority);
            }
            _ => {}
        }
        match new_append_authority {
            Some(new_append_auth) => {
                header.append_authority = new_append_auth;
                msg!(
                    "Append authority transferred to: {:?}",
                    header.append_authority
                );
            }
            _ => {}
        }
        header.serialize(&mut header_bytes)?;

        Ok(())
    }

    /// If proof is invalid, error is thrown
    pub fn verify_leaf(
        ctx: Context<VerifyLeaf>,
        root: [u8; 32],
        leaf: [u8; 32],
        index: u32,
    ) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, rest) = merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());
        let header = Box::new(MerkleRollHeader::try_from_slice(header_bytes)?);
        let merkle_roll_size = merkle_roll_get_size!(header)?;
        let (roll_bytes, canopy_bytes) = rest.split_at_mut(merkle_roll_size);

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(node.key().to_bytes());
        }
        fill_in_proof_from_canopy(canopy_bytes, header.max_depth, index, &mut proof)?;
        let id = ctx.accounts.merkle_roll.key();

        merkle_roll_apply_fn!(header, id, roll_bytes, prove_leaf, root, leaf, &proof, index)?;
        Ok(())
    }

    /// This instruction allows the tree's mint_authority to append a new leaf to the tree
    /// without having to supply a valid proof.
    ///
    /// This is accomplished by using the rightmost_proof of the merkle roll to construct a
    /// valid proof, and then updating the rightmost_proof for the next leaf if possible.
    pub fn append(ctx: Context<Append>, leaf: [u8; 32]) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, rest) = merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let header = Box::new(MerkleRollHeader::try_from_slice(header_bytes)?);
        assert_eq!(header.authority, ctx.accounts.authority.key());
        assert_eq!(header.append_authority, ctx.accounts.append_authority.key());

        let id = ctx.accounts.merkle_roll.key();
        let merkle_roll_size = merkle_roll_get_size!(header)?;
        let (roll_bytes, canopy_bytes) = rest.split_at_mut(merkle_roll_size);
        let change_log = merkle_roll_apply_fn!(header, id, roll_bytes, append, leaf)?;
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
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, rest) = merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());
        let header = Box::new(MerkleRollHeader::try_from_slice(header_bytes)?);
        assert_eq!(header.authority, ctx.accounts.authority.key());
        let merkle_roll_size = merkle_roll_get_size!(header)?;
        let (roll_bytes, canopy_bytes) = rest.split_at_mut(merkle_roll_size);

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(node.key().to_bytes());
        }
        fill_in_proof_from_canopy(canopy_bytes, header.max_depth, index, &mut proof)?;
        // A call is made to MerkleRoll::fill_empty_or_append
        let id = ctx.accounts.merkle_roll.key();
        let change_log = merkle_roll_apply_fn!(
            header,
            id,
            roll_bytes,
            fill_empty_or_append,
            root,
            leaf,
            &proof,
            index,
        )?;
        emit!(*change_log);
        update_canopy(canopy_bytes, header.max_depth, Some(change_log))
    }
}
