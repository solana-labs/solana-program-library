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
pub mod concurrent_tree_wrapper;
pub mod error;
pub mod events;
#[macro_use]
pub mod macros;
mod noop;
pub mod state;
pub mod zero_copy;

pub use crate::noop::{wrap_application_data_v1, Noop};

use crate::canopy::{
    check_canopy_bytes, check_canopy_no_nodes_to_right_of_index, check_canopy_root,
    fill_in_proof_from_canopy, set_canopy_leaf_nodes, update_canopy,
};
use crate::concurrent_tree_wrapper::*;
pub use crate::error::AccountCompressionError;
pub use crate::events::{AccountCompressionEvent, ChangeLogEvent};
use crate::noop::wrap_event;
use crate::state::{
    merkle_tree_get_size, ConcurrentMerkleTreeHeader, CONCURRENT_MERKLE_TREE_HEADER_SIZE_V1,
};

/// Exported for Anchor / Solita
pub use spl_concurrent_merkle_tree::{
    concurrent_merkle_tree::{ConcurrentMerkleTree, FillEmptyOrAppendArgs},
    error::ConcurrentMerkleTreeError,
    node::Node,
    node::EMPTY,
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

/// Context for modifying a tree: inserting, appending, or replacing a leaf in
/// the existing tree and setting the canopy or finalizing a prepared tree.
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

        let change_log_event = merkle_tree_initialize_empty(&header, id, tree_bytes)?;

        wrap_event(
            &AccountCompressionEvent::ChangeLog(*change_log_event),
            &ctx.accounts.noop,
        )?;
        update_canopy(canopy_bytes, header.get_max_depth(), None)
    }

    /// (Devnet only) In order to initialize a tree with a root, we need to create the tree on-chain first with
    /// the proper authority. The tree might contain a canopy, which is a cache of the uppermost
    /// nodes. The canopy is used to decrease the size of the proof required to update the tree.
    /// If the tree is expected to have a canopy, it needs to be prefilled with the necessary nodes.
    /// There are 2 ways to initialize a merkle tree:
    /// 1. Initialize an empty tree
    /// 2. Initialize a tree with a root and leaf
    /// For the former case, the canopy will be empty which is expected for an empty tree. The
    /// expected flow is `init_empty_merkle_tree`. For the latter case, the canopy should be
    /// filled with the necessary nodes to render the tree usable. Thus we need to prefill the
    /// canopy with the necessary nodes. The expected flow for a tree without canopy is
    /// `prepare_batch_merkle_tree` -> `init_prepared_tree_with_root`. The expected flow for a tree
    /// with canopy is `prepare_batch_merkle_tree` -> `append_canopy_nodes` (multiple times
    /// until all of the canopy is filled) -> `init_prepared_tree_with_root`. This instruction
    /// initializes the tree header while leaving the tree itself uninitialized. This allows
    /// distinguishing between an empty tree and a tree prepare to be initialized with a root.
    pub fn prepare_batch_merkle_tree(
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
        header.initialize_batched(
            max_depth,
            max_buffer_size,
            &ctx.accounts.authority.key(),
            Clock::get()?.slot,
        );
        header.serialize(&mut header_bytes)?;
        let merkle_tree_size = merkle_tree_get_size(&header)?;
        let (_tree_bytes, canopy_bytes) = rest.split_at_mut(merkle_tree_size);
        check_canopy_bytes(canopy_bytes)
    }

    /// (Devnet only) This instruction pre-initializes the canopy with the specified leaf nodes of the canopy.
    /// This is intended to be used after `prepare_batch_merkle_tree` and in conjunction with the
    /// `init_prepared_tree_with_root` instruction that'll finalize the tree initialization.
    /// The canopy is used to cache the uppermost nodes of the tree, which allows for a smaller
    /// proof size when updating the tree. The canopy should be filled with the necessary nodes
    /// before calling `init_prepared_tree_with_root`. You may call this instruction multiple
    /// times to fill the canopy with the necessary nodes. The canopy may be filled with the
    /// nodes in any order. The already filled nodes may be replaced with new nodes before calling
    /// `init_prepared_tree_with_root` if the step was done in error.
    /// The canopy should be filled with all the nodes that are to the left of the rightmost
    /// leaf of the tree before calling `init_prepared_tree_with_root`. The canopy should not
    /// contain any nodes to the right of the rightmost leaf of the tree.
    /// This instruction calculates and fills in all the canopy nodes "above" the provided ones.
    /// The validation of the canopy is done in the `init_prepared_tree_with_root` instruction.
    pub fn append_canopy_nodes(
        ctx: Context<Modify>,
        start_index: u32,
        canopy_nodes: Vec<[u8; 32]>,
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
        header.assert_is_batch_initialized()?;
        // assert the tree is not initialized yet, we don't want to overwrite the canopy of an
        // initialized tree
        let merkle_tree_size = merkle_tree_get_size(&header)?;
        let (tree_bytes, canopy_bytes) = rest.split_at_mut(merkle_tree_size);
        // ensure the tree is not initialized, the hacky way
        require!(
            tree_bytes_uninitialized(tree_bytes),
            AccountCompressionError::TreeAlreadyInitialized
        );
        set_canopy_leaf_nodes(
            canopy_bytes,
            header.get_max_depth(),
            start_index,
            &canopy_nodes,
        )
    }

    /// (Devnet only) Initializes a prepared tree with a root and a rightmost leaf. The rightmost leaf is used to
    /// verify the canopy if the tree has it. Before calling this instruction, the tree should be
    /// prepared with `prepare_batch_merkle_tree` and the canopy should be filled with the necessary
    /// nodes with `append_canopy_nodes` (if the canopy is used). This method should be used for
    /// batch creation of trees. The indexing of such batches should be done off-chain. The
    /// programs calling this instruction should take care of ensuring the indexing is possible.
    /// For example, staking may be required to ensure the tree creator has some responsibility
    /// for what is being indexed. If indexing is not possible, there should be a mechanism to
    /// penalize the tree creator.
    pub fn init_prepared_tree_with_root(
        ctx: Context<Modify>,
        root: [u8; 32],
        rightmost_leaf: [u8; 32],
        rightmost_index: u32,
    ) -> Result<()> {
        require_eq!(
            *ctx.accounts.merkle_tree.owner,
            crate::id(),
            AccountCompressionError::IncorrectAccountOwner
        );
        let mut merkle_tree_bytes = ctx.accounts.merkle_tree.try_borrow_mut_data()?;

        let (header_bytes, rest) =
            merkle_tree_bytes.split_at_mut(CONCURRENT_MERKLE_TREE_HEADER_SIZE_V1);
        // the header should already be initialized with prepare_batch_merkle_tree
        let header = ConcurrentMerkleTreeHeader::try_from_slice(header_bytes)?;
        header.assert_valid_authority(&ctx.accounts.authority.key())?;
        header.assert_is_batch_initialized()?;
        let merkle_tree_size = merkle_tree_get_size(&header)?;
        let (tree_bytes, canopy_bytes) = rest.split_at_mut(merkle_tree_size);
        // check the canopy root matches the tree root
        check_canopy_root(canopy_bytes, &root, header.get_max_depth())?;
        // verify the canopy does not contain any nodes to the right of the rightmost leaf
        check_canopy_no_nodes_to_right_of_index(
            canopy_bytes,
            header.get_max_depth(),
            rightmost_index,
        )?;

        // Get rightmost proof from accounts
        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(node.key().to_bytes());
        }
        fill_in_proof_from_canopy(
            canopy_bytes,
            header.get_max_depth(),
            rightmost_index,
            &mut proof,
        )?;
        assert_eq!(proof.len(), header.get_max_depth() as usize);

        let id = ctx.accounts.merkle_tree.key();
        // A call is made to ConcurrentMerkleTree::initialize_with_root
        let args = &InitializeWithRootArgs {
            root,
            rightmost_leaf,
            proof_vec: proof,
            index: rightmost_index,
        };
        let change_log = merkle_tree_initialize_with_root(&header, id, tree_bytes, args)?;
        update_canopy(canopy_bytes, header.get_max_depth(), Some(&change_log))?;
        wrap_event(
            &AccountCompressionEvent::ChangeLog(*change_log),
            &ctx.accounts.noop,
        )
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
        let args = &SetLeafArgs {
            current_root: root,
            previous_leaf,
            new_leaf,
            proof_vec: proof,
            index,
        };
        let change_log_event = merkle_tree_set_leaf(&header, id, tree_bytes, args)?;

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

        let args = &ProveLeafArgs {
            current_root: root,
            leaf,
            proof_vec: proof,
            index,
        };
        merkle_tree_prove_leaf(&header, id, tree_bytes, args)?;

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
        let change_log_event = merkle_tree_append_leaf(&header, id, tree_bytes, &leaf)?;
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
        let args = &FillEmptyOrAppendArgs {
            current_root: root,
            leaf,
            proof_vec: proof,
            index,
        };
        let change_log_event = merkle_tree_fill_empty_or_append(&header, id, tree_bytes, args)?;

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
        assert_tree_is_empty(&header, id, tree_bytes)?;

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
