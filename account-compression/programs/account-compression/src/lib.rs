//! SPL Account Compression is an on-chain program that exposes an interface to manipulating SPL ConcurrentMerkleTrees
//!
//! A buffer of proof-like changelogs is stored on-chain that allow multiple proof-based writes to succeed within the same slot.
//! This is accomplished by fast-forwarding out-of-date (or possibly invalid) proofs based on information stored in the changelogs.
//! See a copy of the whitepaper [here](https://drive.google.com/file/d/1BOpa5OFmara50fTvL0VIVYjtg-qzHCVc/view)
//!
//! To circumvent proof size restrictions stemming from Solana transaction size restrictions,
//! SPL Account Compression also provides the ability to cache the upper most leaves of the
//! concurrent merkle tree. This is called the "canopy", and is stored at the end of the
//! ConcurrentMerkleTreeAccount. More information can be found in the initialization instruction
//! documentation.
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
    prelude::*,
    solana_program::sysvar::{clock::Clock, rent::Rent},
};
use borsh::{BorshDeserialize, BorshSerialize};

pub mod canopy;
pub mod error;
pub mod events;
#[macro_use]
pub mod macros;
mod noop;
pub mod state;
pub mod zero_copy;

pub use crate::noop::{wrap_application_data_v1, Noop};

use crate::canopy::{fill_in_proof_from_canopy, update_canopy};
pub use crate::error::AccountCompressionError;
pub use crate::events::{AccountCompressionEvent, ChangeLogEvent};
use crate::noop::wrap_event;
use crate::state::{
    merkle_tree_get_size, ConcurrentMerkleTreeHeader, CONCURRENT_MERKLE_TREE_HEADER_SIZE_V1,
};
use crate::zero_copy::ZeroCopy;

/// Exported for Anchor / Solita
pub use spl_concurrent_merkle_tree::{
    concurrent_merkle_tree::ConcurrentMerkleTree, error::ConcurrentMerkleTreeError, node::Node,
};

declare_id!("cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK");

/// Context for initializing a new SPL ConcurrentMerkleTree
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(zero)]
    /// CHECK: This account will be zeroed out, and the size will be validated
    pub merkle_tree: UncheckedAccount<'info>,

    /// Authority that controls write-access to the tree
    /// Typically a program, e.g., the Bubblegum contract validates that leaves are valid NFTs.
    pub authority: Signer<'info>,

    /// Program used to emit changelogs as cpi instruction data.
    pub noop: Program<'info, Noop>,
}

/// Context for inserting, appending, or replacing a leaf in the tree
///
/// Modification instructions also require the proof to the leaf to be provided
/// as 32-byte nodes via "remaining accounts".
#[derive(Accounts)]
pub struct Modify<'info> {
    #[account(mut)]
    /// CHECK: This account is validated in the instruction
    pub merkle_tree: UncheckedAccount<'info>,

    /// Authority that controls write-access to the tree
    /// Typically a program, e.g., the Bubblegum contract validates that leaves are valid NFTs.
    pub authority: Signer<'info>,

    /// Program used to emit changelogs as cpi instruction data.
    pub noop: Program<'info, Noop>,
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

    /// Authority that controls write-access to the tree
    /// Typically a program, e.g., the Bubblegum contract validates that leaves are valid NFTs.
    pub authority: Signer<'info>,
}

/// Context for closing a tree
#[derive(Accounts)]
pub struct CloseTree<'info> {
    #[account(mut)]
    /// CHECK: This account is validated in the instruction
    pub merkle_tree: AccountInfo<'info>,

    /// Authority that controls write-access to the tree
    pub authority: Signer<'info>,

    /// CHECK: Recipient of funds after
    #[account(mut)]
    pub recipient: AccountInfo<'info>,
}

#[program]
pub mod spl_account_compression {
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
    ///
    /// For instructions on enabling the canopy, see [canopy].
    pub fn init_empty_merkle_tree(
        ctx: Context<Initialize>,
        max_depth: u32,
        max_buffer_size: u32,
    ) -> Result<()> {
        require_eq!(
            *ctx.accounts.merkle_tree.owner,
            crate::id(),
            AccountCompressionError::IncorrectAccountOwner
        );
        let mut merkle_tree_bytes = ctx.accounts.merkle_tree.try_borrow_mut_data()?;

        let (mut header_bytes, rest) =
            merkle_tree_bytes.split_at_mut(CONCURRENT_MERKLE_TREE_HEADER_SIZE_V1);

        let mut header = ConcurrentMerkleTreeHeader::try_from_slice(header_bytes)?;
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
        let change_log_event = merkle_tree_apply_fn_mut!(header, id, tree_bytes, initialize,)?;
        wrap_event(
            &AccountCompressionEvent::ChangeLog(*change_log_event),
            &ctx.accounts.noop,
        )?;
        update_canopy(canopy_bytes, header.get_max_depth(), None)
    }

    /// Note:
    /// Supporting this instruction open a security vulnerability for indexers.
    /// This instruction has been deemed unusable for publicly indexed compressed NFTs.
    /// Indexing batched data in this way requires indexers to read in the `uri`s onto physical storage
    /// and then into their database. This opens up a DOS attack vector, whereby this instruction is
    /// repeatedly invoked, causing indexers to fail.
    ///
    /// Because this instruction was deemed insecure, this instruction has been removed
    /// until secure usage is available on-chain.
    // pub fn init_merkle_tree_with_root(
    //     ctx: Context<Initialize>,
    //     max_depth: u32,
    //     max_buffer_size: u32,
    //     root: [u8; 32],
    //     leaf: [u8; 32],
    //     index: u32,
    //     _changelog_db_uri: String,
    //     _metadata_db_uri: String,
    // ) -> Result<()> {
    //     require_eq!(
    //         *ctx.accounts.merkle_tree.owner,
    //         crate::id(),
    //         AccountCompressionError::IncorrectAccountOwner
    //     );
    //     let mut merkle_tree_bytes = ctx.accounts.merkle_tree.try_borrow_mut_data()?;

    //     let (mut header_bytes, rest) =
    //         merkle_tree_bytes.split_at_mut(CONCURRENT_MERKLE_TREE_HEADER_SIZE_V1);

    //     let mut header = ConcurrentMerkleTreeHeader::try_from_slice(&header_bytes)?;
    //     header.initialize(
    //         max_depth,
    //         max_buffer_size,
    //         &ctx.accounts.authority.key(),
    //         Clock::get()?.slot,
    //     );
    //     header.serialize(&mut header_bytes)?;
    //     let merkle_tree_size = merkle_tree_get_size(&header)?;
    //     let (tree_bytes, canopy_bytes) = rest.split_at_mut(merkle_tree_size);

    //     // Get rightmost proof from accounts
    //     let mut proof = vec![];
    //     for node in ctx.remaining_accounts.iter() {
    //         proof.push(node.key().to_bytes());
    //     }
    //     fill_in_proof_from_canopy(canopy_bytes, header.max_depth, index, &mut proof)?;
    //     assert_eq!(proof.len(), max_depth as usize);

    //     let id = ctx.accounts.merkle_tree.key();
    //     // A call is made to ConcurrentMerkleTree::initialize_with_root(root, leaf, proof, index)
    //     let change_log = merkle_tree_apply_fn!(
    //         header,
    //         id,
    //         tree_bytes,
    //         initialize_with_root,
    //         root,
    //         leaf,
    //         &proof,
    //         index
    //     )?;
    //     wrap_event(change_log.try_to_vec()?, &ctx.accounts.log_wrapper)?;
    //     update_canopy(canopy_bytes, header.max_depth, Some(change_log))
    // }

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
        require_eq!(
            *ctx.accounts.merkle_tree.owner,
            crate::id(),
            AccountCompressionError::IncorrectAccountOwner
        );
        let mut merkle_tree_bytes = ctx.accounts.merkle_tree.try_borrow_mut_data()?;
        let (header_bytes, rest) =
            merkle_tree_bytes.split_at_mut(CONCURRENT_MERKLE_TREE_HEADER_SIZE_V1);

        let header = ConcurrentMerkleTreeHeader::try_from_slice(header_bytes)?;
        header.assert_valid_authority(&ctx.accounts.authority.key())?;
        header.assert_valid_leaf_index(index)?;

        let merkle_tree_size = merkle_tree_get_size(&header)?;
        let (tree_bytes, canopy_bytes) = rest.split_at_mut(merkle_tree_size);

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(node.key().to_bytes());
        }
        fill_in_proof_from_canopy(canopy_bytes, header.get_max_depth(), index, &mut proof)?;
        let id = ctx.accounts.merkle_tree.key();
        // A call is made to ConcurrentMerkleTree::set_leaf(root, previous_leaf, new_leaf, proof, index)
        let change_log_event = merkle_tree_apply_fn_mut!(
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
        update_canopy(
            canopy_bytes,
            header.get_max_depth(),
            Some(&change_log_event),
        )?;
        wrap_event(
            &AccountCompressionEvent::ChangeLog(*change_log_event),
            &ctx.accounts.noop,
        )
    }

    /// Transfers `authority`.
    /// Requires `authority` to sign
    pub fn transfer_authority(
        ctx: Context<TransferAuthority>,
        new_authority: Pubkey,
    ) -> Result<()> {
        require_eq!(
            *ctx.accounts.merkle_tree.owner,
            crate::id(),
            AccountCompressionError::IncorrectAccountOwner
        );
        let mut merkle_tree_bytes = ctx.accounts.merkle_tree.try_borrow_mut_data()?;
        let (mut header_bytes, _) =
            merkle_tree_bytes.split_at_mut(CONCURRENT_MERKLE_TREE_HEADER_SIZE_V1);

        let mut header = ConcurrentMerkleTreeHeader::try_from_slice(header_bytes)?;
        header.assert_valid_authority(&ctx.accounts.authority.key())?;

        header.set_new_authority(&new_authority);
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
        require_eq!(
            *ctx.accounts.merkle_tree.owner,
            crate::id(),
            AccountCompressionError::IncorrectAccountOwner
        );
        let merkle_tree_bytes = ctx.accounts.merkle_tree.try_borrow_data()?;
        let (header_bytes, rest) =
            merkle_tree_bytes.split_at(CONCURRENT_MERKLE_TREE_HEADER_SIZE_V1);

        let header = ConcurrentMerkleTreeHeader::try_from_slice(header_bytes)?;
        header.assert_valid()?;
        header.assert_valid_leaf_index(index)?;

        let merkle_tree_size = merkle_tree_get_size(&header)?;
        let (tree_bytes, canopy_bytes) = rest.split_at(merkle_tree_size);

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(node.key().to_bytes());
        }
        fill_in_proof_from_canopy(canopy_bytes, header.get_max_depth(), index, &mut proof)?;
        let id = ctx.accounts.merkle_tree.key();

        merkle_tree_apply_fn!(header, id, tree_bytes, prove_leaf, root, leaf, &proof, index)?;
        Ok(())
    }

    /// This instruction allows the tree's `authority` to append a new leaf to the tree
    /// without having to supply a proof.
    ///
    /// Learn more about SPL
    /// ConcurrentMerkleTree
    /// [here](https://github.com/solana-labs/solana-program-library/tree/master/libraries/concurrent-merkle-tree)
    pub fn append(ctx: Context<Modify>, leaf: [u8; 32]) -> Result<()> {
        require_eq!(
            *ctx.accounts.merkle_tree.owner,
            crate::id(),
            AccountCompressionError::IncorrectAccountOwner
        );
        let mut merkle_tree_bytes = ctx.accounts.merkle_tree.try_borrow_mut_data()?;
        let (header_bytes, rest) =
            merkle_tree_bytes.split_at_mut(CONCURRENT_MERKLE_TREE_HEADER_SIZE_V1);

        let header = ConcurrentMerkleTreeHeader::try_from_slice(header_bytes)?;
        header.assert_valid_authority(&ctx.accounts.authority.key())?;

        let id = ctx.accounts.merkle_tree.key();
        let merkle_tree_size = merkle_tree_get_size(&header)?;
        let (tree_bytes, canopy_bytes) = rest.split_at_mut(merkle_tree_size);
        let change_log_event = merkle_tree_apply_fn_mut!(header, id, tree_bytes, append, leaf)?;
        update_canopy(
            canopy_bytes,
            header.get_max_depth(),
            Some(&change_log_event),
        )?;
        wrap_event(
            &AccountCompressionEvent::ChangeLog(*change_log_event),
            &ctx.accounts.noop,
        )
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
        require_eq!(
            *ctx.accounts.merkle_tree.owner,
            crate::id(),
            AccountCompressionError::IncorrectAccountOwner
        );
        let mut merkle_tree_bytes = ctx.accounts.merkle_tree.try_borrow_mut_data()?;
        let (header_bytes, rest) =
            merkle_tree_bytes.split_at_mut(CONCURRENT_MERKLE_TREE_HEADER_SIZE_V1);

        let header = ConcurrentMerkleTreeHeader::try_from_slice(header_bytes)?;
        header.assert_valid_authority(&ctx.accounts.authority.key())?;
        header.assert_valid_leaf_index(index)?;

        let merkle_tree_size = merkle_tree_get_size(&header)?;
        let (tree_bytes, canopy_bytes) = rest.split_at_mut(merkle_tree_size);

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(node.key().to_bytes());
        }
        fill_in_proof_from_canopy(canopy_bytes, header.get_max_depth(), index, &mut proof)?;
        // A call is made to ConcurrentMerkleTree::fill_empty_or_append
        let id = ctx.accounts.merkle_tree.key();
        let change_log_event = merkle_tree_apply_fn_mut!(
            header,
            id,
            tree_bytes,
            fill_empty_or_append,
            root,
            leaf,
            &proof,
            index,
        )?;
        update_canopy(
            canopy_bytes,
            header.get_max_depth(),
            Some(&change_log_event),
        )?;
        wrap_event(
            &AccountCompressionEvent::ChangeLog(*change_log_event),
            &ctx.accounts.noop,
        )
    }

    pub fn close_empty_tree(ctx: Context<CloseTree>) -> Result<()> {
        require_eq!(
            *ctx.accounts.merkle_tree.owner,
            crate::id(),
            AccountCompressionError::IncorrectAccountOwner
        );
        let mut merkle_tree_bytes = ctx.accounts.merkle_tree.try_borrow_mut_data()?;
        let (header_bytes, rest) =
            merkle_tree_bytes.split_at_mut(CONCURRENT_MERKLE_TREE_HEADER_SIZE_V1);

        let header = ConcurrentMerkleTreeHeader::try_from_slice(header_bytes)?;
        header.assert_valid_authority(&ctx.accounts.authority.key())?;

        let merkle_tree_size = merkle_tree_get_size(&header)?;
        let (tree_bytes, canopy_bytes) = rest.split_at_mut(merkle_tree_size);

        let id = ctx.accounts.merkle_tree.key();
        merkle_tree_apply_fn_mut!(header, id, tree_bytes, prove_tree_is_empty,)?;

        // Close merkle tree account
        // 1. Move lamports
        let dest_starting_lamports = ctx.accounts.recipient.lamports();
        **ctx.accounts.recipient.lamports.borrow_mut() = dest_starting_lamports
            .checked_add(ctx.accounts.merkle_tree.lamports())
            .unwrap();
        **ctx.accounts.merkle_tree.lamports.borrow_mut() = 0;

        // 2. Set all CMT account bytes to 0
        header_bytes.fill(0);
        tree_bytes.fill(0);
        canopy_bytes.fill(0);

        Ok(())
    }
}
