use anchor_lang::{
    emit,
    prelude::*,
    solana_program::{entrypoint::ProgramResult, program_error::ProgramError, sysvar::rent::Rent},
};
use borsh::{BorshDeserialize, BorshSerialize};
use std::mem::size_of;

pub mod error;
pub mod state;
pub mod utils;

use crate::error::GummyrollError;
use crate::state::{ChangeLogEvent, MerkleRollHeader, Node};
use crate::utils::ZeroCopy;
use concurrent_merkle_tree::{merkle_roll::MerkleRoll, state::Node as TreeNode};

declare_id!("GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD");

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

macro_rules! merkle_roll_depth_size_apply_fn {
    ($max_depth:literal, $max_size:literal, $emit_msg:ident, $id:ident, $bytes:ident, $func:ident, $($arg:tt)*) => {
        if size_of::<MerkleRoll::<$max_depth, $max_size>>() != $bytes.len() {
            msg!("{} {}", size_of::<MerkleRoll::<$max_depth, $max_size>>(), $bytes.len());
            msg!("Received account of invalid length");
            let expected_bytes = size_of::<MerkleRoll::<$max_depth, $max_size>>();
            let bytes_received = $bytes.len();
            msg!("Expected: {}, received: {}", expected_bytes, bytes_received);
            err!(GummyrollError::MerkleRollByteLengthMismatch)
        } else {
            match MerkleRoll::<$max_depth, $max_size>::load_mut_bytes($bytes) {
                Ok(merkle_roll) => {
                    match merkle_roll.$func($($arg)*) {
                        Ok(_) => {
                            if $emit_msg {
                                emit!(*Box::<ChangeLogEvent>::from((merkle_roll.get_change_log(), $id, merkle_roll.sequence_number)));
                            }
                            Ok(())
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
}

macro_rules! merkle_roll_apply_fn {
    ($header:ident, $emit_msg:ident, $id:ident, $bytes:ident, $func:ident, $($arg:tt)*) => {
        // Note: max_buffer_size MUST be a power of 2
        match ($header.max_depth, $header.max_buffer_size) {
            (3, 8) => merkle_roll_depth_size_apply_fn!(3, 8, $emit_msg, $id, $bytes, $func, $($arg)*),
            (14, 64) => merkle_roll_depth_size_apply_fn!(14, 64, $emit_msg, $id, $bytes, $func, $($arg)*),
            (14, 256) => merkle_roll_depth_size_apply_fn!(14, 256, $emit_msg, $id, $bytes, $func, $($arg)*),
            (14, 1024) => merkle_roll_depth_size_apply_fn!(14, 1024, $emit_msg, $id, $bytes, $func, $($arg)*),
            (14, 2048) => merkle_roll_depth_size_apply_fn!(14, 2048, $emit_msg, $id, $bytes, $func, $($arg)*),
            (16, 64) => merkle_roll_depth_size_apply_fn!(16, 64, $emit_msg, $id, $bytes, $func, $($arg)*),
            (16, 256) => merkle_roll_depth_size_apply_fn!(16, 256, $emit_msg, $id, $bytes, $func, $($arg)*),
            (16, 1024) => merkle_roll_depth_size_apply_fn!(16, 1024, $emit_msg, $id, $bytes, $func, $($arg)*),
            (16, 2048) => merkle_roll_depth_size_apply_fn!(16, 2048, $emit_msg, $id, $bytes, $func, $($arg)*),
            (18, 64) => merkle_roll_depth_size_apply_fn!(18, 64, $emit_msg, $id, $bytes, $func, $($arg)*),
            (18, 256) => merkle_roll_depth_size_apply_fn!(18, 256, $emit_msg, $id, $bytes, $func, $($arg)*),
            (18, 1024) => merkle_roll_depth_size_apply_fn!(18, 1024, $emit_msg, $id, $bytes, $func, $($arg)*),
            (18, 2048) => merkle_roll_depth_size_apply_fn!(18, 2048, $emit_msg, $id, $bytes, $func, $($arg)*),
            (20, 64) => merkle_roll_depth_size_apply_fn!(20, 64, $emit_msg, $id, $bytes, $func, $($arg)*),
            (20, 256) => merkle_roll_depth_size_apply_fn!(20, 256, $emit_msg, $id, $bytes, $func, $($arg)*),
            (20, 1024) => merkle_roll_depth_size_apply_fn!(20, 1024, $emit_msg, $id, $bytes, $func, $($arg)*),
            (20, 2048) => merkle_roll_depth_size_apply_fn!(20, 2048, $emit_msg, $id, $bytes, $func, $($arg)*),
            (22, 64) => merkle_roll_depth_size_apply_fn!(22, 64, $emit_msg, $id, $bytes, $func, $($arg)*),
            (22, 256) => merkle_roll_depth_size_apply_fn!(22, 256, $emit_msg, $id, $bytes, $func, $($arg)*),
            (22, 1024) => merkle_roll_depth_size_apply_fn!(22, 1024, $emit_msg, $id, $bytes, $func, $($arg)*),
            (22, 2048) => merkle_roll_depth_size_apply_fn!(22, 2048, $emit_msg, $id, $bytes, $func, $($arg)*),
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

    pub fn init_empty_gummyroll(
        ctx: Context<Initialize>,
        max_depth: u32,
        max_buffer_size: u32,
    ) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;

        let (mut header_bytes, roll_bytes) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let mut header = Box::new(MerkleRollHeader::try_from_slice(&header_bytes)?);
        header.initialize(
            max_depth,
            max_buffer_size,
            &ctx.accounts.authority.key(),
            &ctx.accounts.append_authority.key(),
        );
        header.serialize(&mut header_bytes)?;
        let id = ctx.accounts.merkle_roll.key();
        merkle_roll_apply_fn!(header, true, id, roll_bytes, initialize,)
    }

    pub fn init_gummyroll_with_root(
        ctx: Context<Initialize>,
        max_depth: u32,
        max_buffer_size: u32,
        root: Node,
        leaf: Node,
        index: u32,
        changelog_db_uri: String,
        metadata_db_uri: String,
    ) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;

        let (mut header_bytes, roll_bytes) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let mut header = Box::new(MerkleRollHeader::try_from_slice(&header_bytes)?);
        header.initialize(
            max_depth,
            max_buffer_size,
            &ctx.accounts.authority.key(),
            &ctx.accounts.append_authority.key(),
        );
        header.serialize(&mut header_bytes)?;

        // Get rightmost proof from accounts
        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(TreeNode::new(node.key().to_bytes()));
        }
        assert_eq!(proof.len(), max_depth as usize);

        let id = ctx.accounts.merkle_roll.key();
        // A call is made to MerkleRoll::initialize_with_root(root, leaf, proof, index)
        merkle_roll_apply_fn!(
            header,
            true,
            id,
            roll_bytes,
            initialize_with_root,
            root.into(),
            leaf.into(),
            proof,
            index
        )
    }

    pub fn replace_leaf(
        ctx: Context<Modify>,
        root: Node,
        previous_leaf: Node,
        new_leaf: Node,
        index: u32,
    ) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, roll_bytes) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let header = Box::new(MerkleRollHeader::try_from_slice(header_bytes)?);
        assert_eq!(header.authority, ctx.accounts.authority.key());

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(TreeNode::new(node.key().to_bytes()));
        }

        let id = ctx.accounts.merkle_roll.key();
        // A call is made to MerkleRoll::set_leaf(root, previous_leaf, new_leaf, proof, index)
        merkle_roll_apply_fn!(
            header,
            true,
            id,
            roll_bytes,
            set_leaf,
            root.into(),
            previous_leaf.into(),
            new_leaf.into(),
            proof,
            index
        )
    }

    pub fn append(ctx: Context<Append>, leaf: Node) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, roll_bytes) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let header = Box::new(MerkleRollHeader::try_from_slice(header_bytes)?);
        assert_eq!(header.authority, ctx.accounts.authority.key());
        assert_eq!(header.append_authority, ctx.accounts.append_authority.key());

        let id = ctx.accounts.merkle_roll.key();
        merkle_roll_apply_fn!(header, true, id, roll_bytes, append, leaf.into())
    }

    pub fn insert_or_append(
        ctx: Context<Modify>,
        root: Node,
        leaf: Node,
        index: u32,
    ) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, roll_bytes) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let header = Box::new(MerkleRollHeader::try_from_slice(header_bytes)?);
        assert_eq!(header.authority, ctx.accounts.authority.key());

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(TreeNode::new(node.key().to_bytes()));
        }

        let id = ctx.accounts.merkle_roll.key();
        // A call is made to MerkleRoll::fill_empty_or_append
        merkle_roll_apply_fn!(
            header,
            true,
            id,
            roll_bytes,
            fill_empty_or_append,
            root.into(),
            leaf.into(),
            proof,
            index
        )
    }
}
