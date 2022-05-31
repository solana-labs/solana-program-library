use anchor_lang::{
    emit,
    prelude::*,
    solana_program::{entrypoint::ProgramResult, program_error::ProgramError, sysvar::rent::Rent},
};
use borsh::{BorshDeserialize, BorshSerialize};
use std::mem::size_of;

pub mod state;
pub mod utils;

use crate::state::{
    merkle_roll::{MerkleRoll, MerkleRollHeader},
    node::Node,
};

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
            None
        } else {
            match MerkleRoll::<$max_depth, $max_size>::load_mut_bytes($bytes) {
                Ok(merkle_roll) => {
                    match merkle_roll.$func($($arg)*) {
                        Some(x) => {
                            if $emit_msg {
                                emit!(*merkle_roll.get_change_log().to_event($id, merkle_roll.sequence_number));
                            }
                            Some(x)
                        }
                        None => None,
                    }
                }
                Err(e) => {
                    msg!("Error zero copying merkle roll {}", e);
                    None
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
                None
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
    ) -> ProgramResult {
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
        match merkle_roll_apply_fn!(header, true, id, roll_bytes, initialize,) {
            Some(_) => Ok(()),
            None => Err(ProgramError::InvalidInstructionData),
        }
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
    ) -> ProgramResult {
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
            proof.push(Node::new(node.key().to_bytes()));
        }
        assert_eq!(proof.len(), max_depth as usize);

        let id = ctx.accounts.merkle_roll.key();
        // A call is made to MerkleRoll::initialize_with_root(root, leaf, proof, index)
        match merkle_roll_apply_fn!(
            header,
            true,
            id,
            roll_bytes,
            initialize_with_root,
            root,
            leaf,
            proof,
            index
        ) {
            Some(_) => Ok(()),
            None => Err(ProgramError::InvalidInstructionData),
        }
    }

    pub fn replace_leaf(
        ctx: Context<Modify>,
        root: Node,
        previous_leaf: Node,
        new_leaf: Node,
        index: u32,
    ) -> ProgramResult {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, roll_bytes) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let header = Box::new(MerkleRollHeader::try_from_slice(header_bytes)?);
        assert_eq!(header.authority, ctx.accounts.authority.key());

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(Node::new(node.key().to_bytes()));
        }

        let id = ctx.accounts.merkle_roll.key();
        // A call is made to MerkleRoll::set_leaf(root, previous_leaf, new_leaf, proof, index)
        match merkle_roll_apply_fn!(
            header,
            true,
            id,
            roll_bytes,
            set_leaf,
            root,
            previous_leaf,
            new_leaf,
            proof,
            index
        ) {
            Some(_) => Ok(()),
            None => Err(ProgramError::InvalidInstructionData),
        }
    }

    pub fn append(ctx: Context<Append>, leaf: Node) -> ProgramResult {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, roll_bytes) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let header = Box::new(MerkleRollHeader::try_from_slice(header_bytes)?);
        assert_eq!(header.authority, ctx.accounts.authority.key());
        assert_eq!(header.append_authority, ctx.accounts.append_authority.key());

        let id = ctx.accounts.merkle_roll.key();
        match merkle_roll_apply_fn!(header, true, id, roll_bytes, append, leaf) {
            Some(_) => Ok(()),
            None => Err(ProgramError::InvalidInstructionData),
        }
    }

    pub fn insert_or_append(
        ctx: Context<Modify>,
        root: Node,
        leaf: Node,
        index: u32,
    ) -> ProgramResult {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, roll_bytes) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let header = Box::new(MerkleRollHeader::try_from_slice(header_bytes)?);
        assert_eq!(header.authority, ctx.accounts.authority.key());

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(Node::new(node.key().to_bytes()));
        }

        let id = ctx.accounts.merkle_roll.key();
        // A call is made to MerkleRoll::fill_empty_or_append
        match merkle_roll_apply_fn!(
            header,
            true,
            id,
            roll_bytes,
            fill_empty_or_append,
            root,
            leaf,
            proof,
            index
        ) {
            Some(_) => Ok(()),
            None => Err(ProgramError::InvalidInstructionData),
        }
    }
}
