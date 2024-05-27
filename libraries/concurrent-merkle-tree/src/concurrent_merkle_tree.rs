use {
    crate::{
        changelog::ChangeLog,
        error::ConcurrentMerkleTreeError,
        hash::{fill_in_proof, hash_to_parent, recompute},
        node::{empty_node, empty_node_cached, Node, EMPTY},
        path::Path,
    },
    bytemuck::{Pod, Zeroable},
    log_compute, solana_logging,
};

/// Enforce constraints on max depth and buffer size
#[inline(always)]
fn check_bounds(max_depth: usize, max_buffer_size: usize) {
    // We cannot allow a tree depth greater than 30 because of the bit math
    // required to update `ChangeLog`s
    assert!(max_depth < 31);
    // This will return true if MAX_BUFFER_SIZE is a power of 2 or if it is 0
    assert!(max_buffer_size & (max_buffer_size - 1) == 0);
}

fn check_leaf_index(leaf_index: u32, max_depth: usize) -> Result<(), ConcurrentMerkleTreeError> {
    if leaf_index >= (1 << max_depth) {
        return Err(ConcurrentMerkleTreeError::LeafIndexOutOfBounds);
    }
    Ok(())
}

/// Conurrent Merkle Tree is a Merkle Tree that allows
/// multiple tree operations targeted for the same tree root to succeed.
///
/// In a normal merkle tree, only the first tree operation will succeed because
/// the following operations will have proofs for the unmodified tree state.
/// ConcurrentMerkleTree avoids this by storing a buffer of modified nodes
/// (`change_logs`) which allows it to implement fast-forwarding of concurrent
/// merkle tree operations.
///
/// As long as the concurrent merkle tree operations
/// have proofs that are valid for a previous state of the tree that can be
/// found in the stored buffer, that tree operation's proof can be
/// fast-forwarded and the tree operation can be applied.
///
/// There are two primitive operations for Concurrent Merkle Trees:
/// [set_leaf](ConcurrentMerkleTree:set_leaf) and
/// [append](ConcurrentMerkleTree::append). Setting a leaf value requires
/// passing a proof to perform that tree operation, but appending does not
/// require a proof.
///
/// An additional key property of ConcurrentMerkleTree is support for
/// [append](ConcurrentMerkleTree::append) operations, which do not require any
/// proofs to be passed. This is accomplished by keeping track of the
/// proof to the rightmost leaf in the tree (`rightmost_proof`).
///
/// The `ConcurrentMerkleTree` is a generic struct that may be interacted with
/// using macros. Those macros may wrap up the construction and both mutable and
/// immutable calls to the `ConcurrentMerkleTree` struct. If the macro contains
/// a big match statement over different sizes of a tree and buffer, it might
/// create a huge stack footprint. This in turn might lead to a stack overflow
/// given the max stack offset of just 4kb. In order to minimize the stack frame
/// size, the arguments for the `ConcurrentMerkleTree` methods that contain the
/// proofs are passed as references to structs.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ConcurrentMerkleTree<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> {
    pub sequence_number: u64,
    /// Index of most recent root & changes
    pub active_index: u64,
    /// Number of active changes we are tracking
    pub buffer_size: u64,
    /// Proof for respective root
    pub change_logs: [ChangeLog<MAX_DEPTH>; MAX_BUFFER_SIZE],
    pub rightmost_proof: Path<MAX_DEPTH>,
}

unsafe impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> Zeroable
    for ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>
{
}
unsafe impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> Pod
    for ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>
{
}

impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> Default
    for ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>
{
    fn default() -> Self {
        Self {
            sequence_number: 0,
            active_index: 0,
            buffer_size: 0,
            change_logs: [ChangeLog::<MAX_DEPTH>::default(); MAX_BUFFER_SIZE],
            rightmost_proof: Path::<MAX_DEPTH>::default(),
        }
    }
}

/// Arguments structure for initializing a tree with a root.
pub struct InitializeWithRootArgs {
    pub root: Node,
    pub rightmost_leaf: Node,
    pub proof_vec: Vec<Node>,
    pub index: u32,
}

/// Arguments structure for setting a leaf in the tree.
pub struct SetLeafArgs {
    pub current_root: Node,
    pub previous_leaf: Node,
    pub new_leaf: Node,
    pub proof_vec: Vec<Node>,
    pub index: u32,
}

/// Arguments structure for filling an empty leaf or appending a new leaf to the
/// tree.
pub struct FillEmptyOrAppendArgs {
    pub current_root: Node,
    pub leaf: Node,
    pub proof_vec: Vec<Node>,
    pub index: u32,
}

/// Arguments structure for proving a leaf in the tree.
pub struct ProveLeafArgs {
    pub current_root: Node,
    pub leaf: Node,
    pub proof_vec: Vec<Node>,
    pub index: u32,
}

impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize>
    ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_initialized(&self) -> bool {
        !(self.buffer_size == 0 && self.sequence_number == 0 && self.active_index == 0)
    }

    /// This is the trustless initialization method that should be used in most
    /// cases.
    pub fn initialize(&mut self) -> Result<Node, ConcurrentMerkleTreeError> {
        check_bounds(MAX_DEPTH, MAX_BUFFER_SIZE);
        if self.is_initialized() {
            return Err(ConcurrentMerkleTreeError::TreeAlreadyInitialized);
        }
        let mut rightmost_proof = Path::default();
        let empty_node_cache = [Node::default(); MAX_DEPTH];
        for (i, node) in rightmost_proof.proof.iter_mut().enumerate() {
            *node = empty_node_cached::<MAX_DEPTH>(i as u32, &empty_node_cache);
        }
        let mut path = [Node::default(); MAX_DEPTH];
        for (i, node) in path.iter_mut().enumerate() {
            *node = empty_node_cached::<MAX_DEPTH>(i as u32, &empty_node_cache);
        }
        self.change_logs[0].root = empty_node(MAX_DEPTH as u32);
        self.change_logs[0].path = path;
        self.sequence_number = 0;
        self.active_index = 0;
        self.buffer_size = 1;
        self.rightmost_proof = rightmost_proof;
        Ok(self.change_logs[0].root)
    }

    /// This is a trustful initialization method that assumes the root contains
    /// the expected leaves.
    ///
    /// At the time of this crate's publishing, there is no supported way to
    /// efficiently verify a pre-initialized root on-chain. Using this
    /// method before having a method for on-chain verification will prevent
    /// other applications from indexing the leaf data stored in this tree.
    pub fn initialize_with_root(
        &mut self,
        args: &InitializeWithRootArgs,
    ) -> Result<Node, ConcurrentMerkleTreeError> {
        check_bounds(MAX_DEPTH, MAX_BUFFER_SIZE);
        check_leaf_index(args.index, MAX_DEPTH)?;

        if self.is_initialized() {
            return Err(ConcurrentMerkleTreeError::TreeAlreadyInitialized);
        }
        let mut proof: [Node; MAX_DEPTH] = [Node::default(); MAX_DEPTH];
        proof.copy_from_slice(&args.proof_vec);
        let rightmost_proof = Path {
            proof,
            index: args.index + 1,
            leaf: args.rightmost_leaf,
            _padding: 0,
        };
        self.change_logs[0].root = args.root;
        self.sequence_number = 1;
        self.active_index = 0;
        self.buffer_size = 1;
        self.rightmost_proof = rightmost_proof;
        if args.root != recompute(args.rightmost_leaf, &proof, args.index) {
            solana_logging!("Proof failed to verify");
            return Err(ConcurrentMerkleTreeError::InvalidProof);
        }
        Ok(args.root)
    }

    /// Errors if one of the leaves of the current merkle tree is non-EMPTY
    pub fn prove_tree_is_empty(&self) -> Result<(), ConcurrentMerkleTreeError> {
        if !self.is_initialized() {
            return Err(ConcurrentMerkleTreeError::TreeNotInitialized);
        }
        let empty_node_cache = [EMPTY; MAX_DEPTH];
        if self.get_root() != empty_node_cached::<MAX_DEPTH>(MAX_DEPTH as u32, &empty_node_cache) {
            return Err(ConcurrentMerkleTreeError::TreeNonEmpty);
        }
        Ok(())
    }

    /// Returns the current root of the merkle tree
    pub fn get_root(&self) -> [u8; 32] {
        self.get_change_log().root
    }

    /// Returns the most recent changelog
    pub fn get_change_log(&self) -> Box<ChangeLog<MAX_DEPTH>> {
        if !self.is_initialized() {
            solana_logging!("Tree is not initialized, returning default change log");
            return Box::<ChangeLog<MAX_DEPTH>>::default();
        }
        Box::new(self.change_logs[self.active_index as usize])
    }

    /// This method will fail if the leaf cannot be proven
    /// to exist in the current tree root.
    ///
    /// This method will attempts to prove the leaf first
    /// using the proof nodes provided. However if this fails,
    /// then a proof will be constructed by inferring a proof
    /// from the changelog buffer.
    ///
    /// Note: this is *not* the same as verifying that a (proof, leaf)
    /// combination is valid for the given root. That functionality
    /// is provided by `check_valid_proof`.
    pub fn prove_leaf(&self, args: &ProveLeafArgs) -> Result<(), ConcurrentMerkleTreeError> {
        check_bounds(MAX_DEPTH, MAX_BUFFER_SIZE);
        check_leaf_index(args.index, MAX_DEPTH)?;
        if !self.is_initialized() {
            return Err(ConcurrentMerkleTreeError::TreeNotInitialized);
        }

        if args.index > self.rightmost_proof.index {
            solana_logging!(
                "Received an index larger than the rightmost index {} > {}",
                args.index,
                self.rightmost_proof.index
            );
            Err(ConcurrentMerkleTreeError::LeafIndexOutOfBounds)
        } else {
            let mut proof: [Node; MAX_DEPTH] = [Node::default(); MAX_DEPTH];
            fill_in_proof::<MAX_DEPTH>(&args.proof_vec, &mut proof);
            let valid_root =
                self.check_valid_leaf(args.current_root, args.leaf, &mut proof, args.index, true)?;
            if !valid_root {
                solana_logging!("Proof failed to verify");
                return Err(ConcurrentMerkleTreeError::InvalidProof);
            }
            Ok(())
        }
    }

    /// Only used to initialize right most path for a completely empty tree.
    #[inline(always)]
    fn initialize_tree_from_append(
        &mut self,
        leaf: Node,
        mut proof: [Node; MAX_DEPTH],
    ) -> Result<Node, ConcurrentMerkleTreeError> {
        let old_root = recompute(EMPTY, &proof, 0);
        if old_root == empty_node(MAX_DEPTH as u32) {
            self.try_apply_proof(old_root, EMPTY, leaf, &mut proof, 0, false)
        } else {
            Err(ConcurrentMerkleTreeError::TreeAlreadyInitialized)
        }
    }

    /// Appending a non-empty Node will always succeed .
    pub fn append(&mut self, mut node: Node) -> Result<Node, ConcurrentMerkleTreeError> {
        check_bounds(MAX_DEPTH, MAX_BUFFER_SIZE);
        if !self.is_initialized() {
            return Err(ConcurrentMerkleTreeError::TreeNotInitialized);
        }
        if node == EMPTY {
            return Err(ConcurrentMerkleTreeError::CannotAppendEmptyNode);
        }
        if self.rightmost_proof.index >= 1 << MAX_DEPTH {
            return Err(ConcurrentMerkleTreeError::TreeFull);
        }
        if self.rightmost_proof.index == 0 {
            return self.initialize_tree_from_append(node, self.rightmost_proof.proof);
        }
        let leaf = node;
        let intersection = self.rightmost_proof.index.trailing_zeros() as usize;
        let mut change_list = [EMPTY; MAX_DEPTH];
        let mut intersection_node = self.rightmost_proof.leaf;
        let empty_node_cache = [Node::default(); MAX_DEPTH];

        for (i, cl_item) in change_list.iter_mut().enumerate().take(MAX_DEPTH) {
            *cl_item = node;
            match i {
                i if i < intersection => {
                    // Compute proof to the appended node from empty nodes
                    let sibling = empty_node_cached::<MAX_DEPTH>(i as u32, &empty_node_cache);
                    hash_to_parent(
                        &mut intersection_node,
                        &self.rightmost_proof.proof[i],
                        ((self.rightmost_proof.index - 1) >> i) & 1 == 0,
                    );
                    hash_to_parent(&mut node, &sibling, true);
                    self.rightmost_proof.proof[i] = sibling;
                }
                i if i == intersection => {
                    // Compute the where the new node intersects the main tree
                    hash_to_parent(&mut node, &intersection_node, false);
                    self.rightmost_proof.proof[intersection] = intersection_node;
                }
                _ => {
                    // Update the change list path up to the root
                    hash_to_parent(
                        &mut node,
                        &self.rightmost_proof.proof[i],
                        ((self.rightmost_proof.index - 1) >> i) & 1 == 0,
                    );
                }
            }
        }

        self.update_internal_counters();
        self.change_logs[self.active_index as usize] =
            ChangeLog::<MAX_DEPTH>::new(node, change_list, self.rightmost_proof.index);
        self.rightmost_proof.index += 1;
        self.rightmost_proof.leaf = leaf;
        Ok(node)
    }

    /// Convenience function for `set_leaf`
    ///
    /// This method will `set_leaf` if the leaf at `index` is an empty node,
    /// otherwise it will `append` the new leaf.
    pub fn fill_empty_or_append(
        &mut self,
        args: &FillEmptyOrAppendArgs,
    ) -> Result<Node, ConcurrentMerkleTreeError> {
        check_bounds(MAX_DEPTH, MAX_BUFFER_SIZE);
        check_leaf_index(args.index, MAX_DEPTH)?;
        if !self.is_initialized() {
            return Err(ConcurrentMerkleTreeError::TreeNotInitialized);
        }

        let mut proof: [Node; MAX_DEPTH] = [Node::default(); MAX_DEPTH];
        fill_in_proof::<MAX_DEPTH>(&args.proof_vec, &mut proof);

        log_compute!();
        match self.try_apply_proof(
            args.current_root,
            EMPTY,
            args.leaf,
            &mut proof,
            args.index,
            false,
        ) {
            Ok(new_root) => Ok(new_root),
            Err(error) => match error {
                ConcurrentMerkleTreeError::LeafContentsModified => self.append(args.leaf),
                _ => Err(error),
            },
        }
    }

    /// This method will update the leaf at `index`.
    ///
    /// However if the proof cannot be verified, this method will fail.
    pub fn set_leaf(&mut self, args: &SetLeafArgs) -> Result<Node, ConcurrentMerkleTreeError> {
        check_bounds(MAX_DEPTH, MAX_BUFFER_SIZE);
        check_leaf_index(args.index, MAX_DEPTH)?;
        if !self.is_initialized() {
            return Err(ConcurrentMerkleTreeError::TreeNotInitialized);
        }

        if args.index > self.rightmost_proof.index {
            Err(ConcurrentMerkleTreeError::LeafIndexOutOfBounds)
        } else {
            let mut proof: [Node; MAX_DEPTH] = [Node::default(); MAX_DEPTH];
            fill_in_proof::<MAX_DEPTH>(&args.proof_vec, &mut proof);

            log_compute!();
            self.try_apply_proof(
                args.current_root,
                args.previous_leaf,
                args.new_leaf,
                &mut proof,
                args.index,
                true,
            )
        }
    }

    /// Returns the Current Seq of the tree, the seq is the monotonic counter of
    /// the tree operations that is incremented every time a mutable
    /// operation is performed on the tree.
    pub fn get_seq(&self) -> u64 {
        self.sequence_number
    }

    /// Modifies the `proof` for leaf at `leaf_index`
    /// in place by fast-forwarding the given `proof` through the
    /// `changelog`s, starting at index `changelog_buffer_index`
    /// Returns false if the leaf was updated in the change log
    #[inline(always)]
    fn fast_forward_proof(
        &self,
        leaf: &mut Node,
        proof: &mut [Node; MAX_DEPTH],
        leaf_index: u32,
        mut changelog_buffer_index: u64,
        use_full_buffer: bool,
    ) -> bool {
        solana_logging!(
            "Fast-forwarding proof, starting index {}",
            changelog_buffer_index
        );
        let mask: usize = MAX_BUFFER_SIZE - 1;

        let mut updated_leaf = *leaf;
        log_compute!();
        // Modifies proof by iterating through the change log
        loop {
            // If use_full_buffer is false, this loop will terminate if the initial value of
            // changelog_buffer_index is the active index
            if !use_full_buffer && changelog_buffer_index == self.active_index {
                break;
            }
            changelog_buffer_index = (changelog_buffer_index + 1) & mask as u64;
            self.change_logs[changelog_buffer_index as usize].update_proof_or_leaf(
                leaf_index,
                proof,
                &mut updated_leaf,
            );
            // If use_full_buffer is true, this loop will do 1 full pass of the change logs
            if use_full_buffer && changelog_buffer_index == self.active_index {
                break;
            }
        }
        log_compute!();
        let proof_leaf_unchanged = updated_leaf == *leaf;
        *leaf = updated_leaf;
        proof_leaf_unchanged
    }

    #[inline(always)]
    fn find_root_in_changelog(&self, current_root: Node) -> Option<u64> {
        let mask: usize = MAX_BUFFER_SIZE - 1;
        for i in 0..self.buffer_size {
            let j = self.active_index.wrapping_sub(i) & mask as u64;
            if self.change_logs[j as usize].root == current_root {
                return Some(j);
            }
        }
        None
    }

    #[inline(always)]
    fn check_valid_leaf(
        &self,
        current_root: Node,
        leaf: Node,
        proof: &mut [Node; MAX_DEPTH],
        leaf_index: u32,
        allow_inferred_proof: bool,
    ) -> Result<bool, ConcurrentMerkleTreeError> {
        let mask: usize = MAX_BUFFER_SIZE - 1;
        let (changelog_index, use_full_buffer) = match self.find_root_in_changelog(current_root) {
            Some(matching_changelog_index) => (matching_changelog_index, false),
            None => {
                if allow_inferred_proof {
                    solana_logging!("Failed to find root in change log -> replaying full buffer");
                    (
                        self.active_index.wrapping_sub(self.buffer_size - 1) & mask as u64,
                        true,
                    )
                } else {
                    return Err(ConcurrentMerkleTreeError::RootNotFound);
                }
            }
        };
        let mut updatable_leaf_node = leaf;
        let proof_leaf_unchanged = self.fast_forward_proof(
            &mut updatable_leaf_node,
            proof,
            leaf_index,
            changelog_index,
            use_full_buffer,
        );
        if !proof_leaf_unchanged {
            return Err(ConcurrentMerkleTreeError::LeafContentsModified);
        }
        Ok(self.check_valid_proof(updatable_leaf_node, proof, leaf_index))
    }

    /// Checks that the proof provided is valid for the current root.
    pub fn check_valid_proof(
        &self,
        leaf: Node,
        proof: &[Node; MAX_DEPTH],
        leaf_index: u32,
    ) -> bool {
        if !self.is_initialized() {
            solana_logging!("Tree is not initialized, returning false");
            return false;
        }
        if check_leaf_index(leaf_index, MAX_DEPTH).is_err() {
            solana_logging!("Leaf index out of bounds for max_depth");
            return false;
        }
        recompute(leaf, proof, leaf_index) == self.get_root()
    }

    /// Note: Enabling `allow_inferred_proof` will fast forward the given proof
    /// from the beginning of the buffer in the case that the supplied root is
    /// not in the buffer.
    #[inline(always)]
    fn try_apply_proof(
        &mut self,
        current_root: Node,
        leaf: Node,
        new_leaf: Node,
        proof: &mut [Node; MAX_DEPTH],
        leaf_index: u32,
        allow_inferred_proof: bool,
    ) -> Result<Node, ConcurrentMerkleTreeError> {
        solana_logging!("Active Index: {}", self.active_index);
        solana_logging!("Rightmost Index: {}", self.rightmost_proof.index);
        solana_logging!("Buffer Size: {}", self.buffer_size);
        solana_logging!("Leaf Index: {}", leaf_index);
        let valid_root =
            self.check_valid_leaf(current_root, leaf, proof, leaf_index, allow_inferred_proof)?;
        if !valid_root {
            return Err(ConcurrentMerkleTreeError::InvalidProof);
        }
        self.update_internal_counters();
        Ok(self.update_buffers_from_proof(new_leaf, proof, leaf_index))
    }

    /// Implements circular addition for changelog buffer index
    fn update_internal_counters(&mut self) {
        let mask: usize = MAX_BUFFER_SIZE - 1;
        self.active_index += 1;
        self.active_index &= mask as u64;
        if self.buffer_size < MAX_BUFFER_SIZE as u64 {
            self.buffer_size += 1;
        }
        self.sequence_number = self.sequence_number.saturating_add(1);
    }

    /// Creates a new root from a proof that is valid for the root at
    /// `self.active_index`
    fn update_buffers_from_proof(&mut self, start: Node, proof: &[Node], index: u32) -> Node {
        let change_log = &mut self.change_logs[self.active_index as usize];
        // Also updates change_log's current root
        let root = change_log.replace_and_recompute_path(index, start, proof);
        // Update rightmost path if possible
        if self.rightmost_proof.index < (1 << MAX_DEPTH) {
            if index < self.rightmost_proof.index {
                change_log.update_proof_or_leaf(
                    self.rightmost_proof.index - 1,
                    &mut self.rightmost_proof.proof,
                    &mut self.rightmost_proof.leaf,
                );
            } else {
                assert!(index == self.rightmost_proof.index);
                solana_logging!("Appending rightmost leaf");
                self.rightmost_proof.proof.copy_from_slice(proof);
                self.rightmost_proof.index = index + 1;
                self.rightmost_proof.leaf = change_log.get_leaf();
            }
        }
        root
    }
}
