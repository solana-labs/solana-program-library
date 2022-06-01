use crate::{
    error::CMTError,
    state::{ChangeLog, Node, Path, EMPTY},
    utils::{empty_node, fill_in_proof, recompute},
};
use bytemuck::{Pod, Zeroable};
pub(crate) use log_compute;
pub(crate) use solana_logging;

use solana_program::keccak::hashv;

#[cfg(feature = "sol-log")]
use solana_program::{log::sol_log_compute_units, msg};

/// Tracks updates to off-chain Merkle tree
///
/// Allows for concurrent writes to same merkle tree so long as proof
/// was generated for a that has had at most MAX_SIZE updates since the tx was submitted
#[derive(Copy, Clone)]
pub struct MerkleRoll<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> {
    pub sequence_number: u128,
    /// Index of most recent root & changes
    pub active_index: u64,
    /// Number of active changes we are tracking
    pub buffer_size: u64,
    /// Proof for respective root
    pub change_logs: [ChangeLog<MAX_DEPTH>; MAX_BUFFER_SIZE],
    pub rightmost_proof: Path<MAX_DEPTH>,
}

unsafe impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> Zeroable
    for MerkleRoll<MAX_DEPTH, MAX_BUFFER_SIZE>
{
}
unsafe impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> Pod
    for MerkleRoll<MAX_DEPTH, MAX_BUFFER_SIZE>
{
}

impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> MerkleRoll<MAX_DEPTH, MAX_BUFFER_SIZE> {
    pub fn new() -> Self {
        Self {
            sequence_number: 0,
            active_index: 0,
            buffer_size: 0,
            change_logs: [ChangeLog::<MAX_DEPTH>::default(); MAX_BUFFER_SIZE],
            rightmost_proof: Path::<MAX_DEPTH>::default(),
        }
    }

    pub fn initialize(&mut self) -> Result<Node, CMTError> {
        let mut rightmost_proof = Path::default();
        for (i, node) in rightmost_proof.proof.iter_mut().enumerate() {
            *node = empty_node(i as u32);
        }
        let mut path = [Node::default(); MAX_DEPTH];
        for (i, node) in path.iter_mut().enumerate() {
            *node = empty_node(i as u32);
        }
        self.change_logs[0].root = empty_node(MAX_DEPTH as u32);
        self.change_logs[0].path = path;
        self.sequence_number = 0;
        self.active_index = 0;
        self.buffer_size = 1;
        self.rightmost_proof = rightmost_proof;
        Ok(self.change_logs[0].root)
    }

    pub fn initialize_with_root(
        &mut self,
        root: Node,
        rightmost_leaf: Node,
        proof_vec: Vec<Node>,
        index: u32,
    ) -> Result<Node, CMTError> {
        let mut proof: [Node; MAX_DEPTH] = [Node::default(); MAX_DEPTH];
        proof.copy_from_slice(&proof_vec[..]);
        let rightmost_proof = Path {
            proof,
            index: index + 1,
            leaf: rightmost_leaf,
            _padding: 0,
        };
        self.change_logs[0].root = root;
        self.sequence_number = 1;
        self.active_index = 0;
        self.buffer_size = 1;
        self.rightmost_proof = rightmost_proof;
        assert_eq!(root, recompute(rightmost_leaf, &proof, index,));
        Ok(root)
    }

    pub fn get_change_log(&self) -> Box<ChangeLog<MAX_DEPTH>> {
        Box::new(self.change_logs[self.active_index as usize])
    }

    /// Only used to initialize right most path for a completely empty tree
    #[inline(always)]
    fn initialize_tree(
        &mut self,
        leaf: Node,
        mut proof: [Node; MAX_DEPTH],
    ) -> Result<Node, CMTError> {
        let old_root = recompute(EMPTY, &proof, 0);
        if old_root == empty_node(MAX_DEPTH as u32) {
            self.apply_and_record_proof(old_root, EMPTY, leaf, &mut proof, 0, false, false)
        } else {
            return Err(CMTError::TreeAlreadyInitialized);
        }
    }

    pub fn prove_leaf(
        &mut self,
        current_root: Node,
        leaf: Node,
        proof_vec: Vec<Node>,
        leaf_index: u32,
    ) -> Result<Node, CMTError> {
        if leaf_index > self.rightmost_proof.index {
            solana_logging!(
                "Received an index larger than the rightmost index {} > {}",
                leaf_index,
                self.rightmost_proof.index
            );
            return Err(CMTError::LeafIndexOutOfBounds);
        } else {
            let mut proof: [Node; MAX_DEPTH] = [Node::default(); MAX_DEPTH];
            fill_in_proof::<MAX_DEPTH>(proof_vec, &mut proof);

            // It's important to identify the root index
            // to remove possibility of incorrectly failing
            // due to a leaf collision that happened before the
            // root of the given proof
            match self.find_root_in_changelog(current_root) {
                Some(matching_changelog_index) => {
                    if !self.fast_forward_proof(
                        leaf,
                        &mut proof,
                        leaf_index,
                        matching_changelog_index,
                        false,
                    ) {
                        solana_logging!(
                            "Leaf was updated since proof was issued. Failing to verify"
                        );
                        return Err(CMTError::LeafAlreadyUpdated);
                    }
                }
                None => {
                    if !self.fast_forward_proof(leaf, &mut proof, leaf_index, 0, true) {
                        solana_logging!(
                            "Leaf was updated since proof was issued. Failing to verify"
                        );
                        return Err(CMTError::LeafAlreadyUpdated);
                    }
                }
            }

            if recompute(leaf, &proof, leaf_index) != self.get_change_log().root {
                solana_logging!("Proof failed to verify");
                return Err(CMTError::InvalidProof);
            }

            Ok(Node::default())
        }
    }

    /// Basic operation that always succeeds
    pub fn append(&mut self, mut node: Node) -> Result<Node, CMTError> {
        if node == EMPTY {
            return Err(CMTError::CannotAppendEmptyNode);
        }
        if self.rightmost_proof.index >= 1 << MAX_DEPTH {
            return Err(CMTError::TreeFull);
        }
        if self.rightmost_proof.index == 0 {
            return self.initialize_tree(node, self.rightmost_proof.proof);
        }
        let leaf = node.clone();
        let intersection = self.rightmost_proof.index.trailing_zeros() as usize;
        let mut change_list = [EMPTY; MAX_DEPTH];
        let mut intersection_node = self.rightmost_proof.leaf;

        // Compute proof to the appended node from empty nodes
        for i in 0..intersection {
            change_list[i] = node;
            let hash = hashv(&[node.as_ref(), empty_node(i as u32).as_ref()]);
            node.copy_from_slice(hash.as_ref());
            let rightmost_hash = if ((self.rightmost_proof.index - 1) >> i) & 1 == 1 {
                hashv(&[
                    self.rightmost_proof.proof[i].as_ref(),
                    intersection_node.as_ref(),
                ])
            } else {
                hashv(&[
                    intersection_node.as_ref(),
                    self.rightmost_proof.proof[i].as_ref(),
                ])
            };
            intersection_node.copy_from_slice(rightmost_hash.as_ref());
            self.rightmost_proof.proof[i] = empty_node(i as u32);
        }

        // Compute the where the new node intersects the main tree
        change_list[intersection] = node;
        let hash = hashv(&[intersection_node.as_ref(), node.as_ref()]);
        node.copy_from_slice(hash.as_ref());
        self.rightmost_proof.proof[intersection] = intersection_node;

        // Update the change list path up to the root
        for i in intersection + 1..MAX_DEPTH {
            change_list[i] = node;
            let hash = if (self.rightmost_proof.index >> i) & 1 == 1 {
                hashv(&[self.rightmost_proof.proof[i].as_ref(), node.as_ref()])
            } else {
                hashv(&[node.as_ref(), self.rightmost_proof.proof[i].as_ref()])
            };
            node.copy_from_slice(hash.as_ref());
        }

        self.increment_active_index();
        self.change_logs[self.active_index as usize] = ChangeLog::<MAX_DEPTH> {
            root: node,
            path: change_list,
            index: self.rightmost_proof.index,
            _padding: 0,
        };
        self.rightmost_proof.index = self.rightmost_proof.index + 1;
        self.rightmost_proof.leaf = leaf;
        self.sequence_number = self.sequence_number.saturating_add(1);
        Ok(node)
    }

    /// Convenience function for `set_leaf`
    /// On write conflict:
    /// Will append
    pub fn fill_empty_or_append(
        &mut self,
        current_root: Node,
        leaf: Node,
        proof_vec: Vec<Node>,
        index: u32,
    ) -> Result<Node, CMTError> {
        let mut proof: [Node; MAX_DEPTH] = [Node::default(); MAX_DEPTH];
        fill_in_proof::<MAX_DEPTH>(proof_vec, &mut proof);
        log_compute!();
        let root =
            self.apply_and_record_proof(current_root, EMPTY, leaf, &mut proof, index, true, false);
        log_compute!();
        root
    }

    /// On write conflict:
    /// Will fail by returning None
    pub fn set_leaf(
        &mut self,
        current_root: Node,
        previous_leaf: Node,
        new_leaf: Node,
        proof_vec: Vec<Node>,
        index: u32,
    ) -> Result<Node, CMTError> {
        if index > self.rightmost_proof.index {
            return Err(CMTError::LeafIndexOutOfBounds);
        } else {
            let mut proof: [Node; MAX_DEPTH] = [Node::default(); MAX_DEPTH];
            fill_in_proof::<MAX_DEPTH>(proof_vec, &mut proof);

            log_compute!();
            let root = self.apply_and_record_proof(
                current_root,
                previous_leaf,
                new_leaf,
                &mut proof,
                index,
                false,
                false,
            );
            log_compute!();
            root
        }
    }

    #[inline]
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

    /// Modifies the `proof` for leaf at `leaf_index`
    /// in place by fast-forwarding the given `proof` through the
    /// `changelog`s, starting at index `changelog_buffer_index`
    /// Returns false if the leaf was updated in the change log
    fn fast_forward_proof(
        &mut self,
        leaf: Node,
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
        let padding: usize = 32 - MAX_DEPTH;

        let mut updated_leaf = leaf;
        log_compute!();
        // Modifies proof by iterating through the change log
        loop {
            // If use_full_buffer is false, this loop will terminate if the initial value of changelog_buffer_index is the active index
            if !use_full_buffer && changelog_buffer_index == self.active_index {
                break;
            }
            changelog_buffer_index += 1;
            changelog_buffer_index &= mask as u64;
            if leaf_index != self.change_logs[changelog_buffer_index as usize].index {
                let common_path_len = ((leaf_index
                    ^ self.change_logs[changelog_buffer_index as usize].index)
                    << padding)
                    .leading_zeros() as usize;
                let critbit_index = (MAX_DEPTH - 1) - common_path_len;
                proof[critbit_index] =
                    self.change_logs[changelog_buffer_index as usize].path[critbit_index];
            } else {
                updated_leaf = self.change_logs[changelog_buffer_index as usize].get_leaf();
            }
            // If use_full_buffer is true, this loop will do 1 full pass of the change logs
            if use_full_buffer && changelog_buffer_index == self.active_index {
                break;
            }
        }
        log_compute!();
        updated_leaf == leaf
    }

    /// Update root & record changelog
    /// --------
    /// Fast-forwards submitted proof to be valid for the root at `self.current_index`
    ///
    /// Enabling `use_full_buffer` will skip root matching and just fast forward the given proof
    /// from the beginning of the buffer.
    /// Note: `use_full_buffer` significantly reduces security
    fn apply_and_record_proof(
        &mut self,
        current_root: Node,
        leaf: Node,
        new_leaf: Node,
        proof: &mut [Node; MAX_DEPTH],
        leaf_index: u32,
        append_on_conflict: bool,
        use_full_buffer: bool,
    ) -> Result<Node, CMTError> {
        solana_logging!("Active Index: {}", self.active_index);
        solana_logging!("Rightmost Index: {}", self.rightmost_proof.index);
        solana_logging!("Buffer Size: {}", self.buffer_size);
        solana_logging!("Leaf Index: {}", leaf_index);

        let mask: usize = MAX_BUFFER_SIZE - 1;
        let changelog_buffer_index: u64;
        if use_full_buffer {
            changelog_buffer_index = self.active_index.wrapping_sub(self.buffer_size) & mask as u64
        } else {
            match self.find_root_in_changelog(current_root) {
                Some(matching_changelog_index) => {
                    changelog_buffer_index = matching_changelog_index;
                }
                None => return Err(CMTError::RootNotFound),
            }
        };

        let valid_fast_forward = self.fast_forward_proof(
            new_leaf,
            proof,
            leaf_index,
            changelog_buffer_index,
            use_full_buffer,
        );

        let valid_root = recompute(leaf, proof, leaf_index) == self.get_change_log().root;
        if !valid_fast_forward || leaf_index > self.rightmost_proof.index {
            // If the supplied root was not found in the queue, the instruction should fail if the leaf index changes
            // NOTE: previously we checked if the FF'd proof with the value of the overwritten leaf
            //      could be hashed to match the current root
            //      However, this was removed for simplicity because it did not add to the security model
            //      of insert_or_append functionality.
            if !use_full_buffer && leaf == EMPTY && append_on_conflict {
                return self.append(new_leaf);
            } else {
                return Err(CMTError::LeafAlreadyUpdated);
            }
        }
        if valid_root {
            self.increment_active_index();
            self.sequence_number = self.sequence_number.saturating_add(1);
            Ok(self.apply_changes(new_leaf, proof, leaf_index))
        } else {
            return Err(CMTError::InvalidProof);
        }
    }

    fn increment_active_index(&mut self) {
        let mask: usize = MAX_BUFFER_SIZE - 1;

        self.active_index += 1;
        self.active_index &= mask as u64;
        if self.buffer_size < MAX_BUFFER_SIZE as u64 {
            self.buffer_size += 1;
        }
    }

    /// Creates a new root from a proof that is valid for the root at `self.active_index`
    fn apply_changes(&mut self, start: Node, proof: &[Node], index: u32) -> Node {
        let padding: usize = 32 - MAX_DEPTH;
        let change_log = &mut self.change_logs[self.active_index as usize];
        change_log.index = index;

        // Also updates change_log's current root
        let root = change_log.recompute_path(start, proof);

        // Update rightmost path if possible
        if self.rightmost_proof.index < (1 << MAX_DEPTH) {
            if index < self.rightmost_proof.index as u32 {
                if index != self.rightmost_proof.index - 1 {
                    let common_path_len = ((index ^ (self.rightmost_proof.index - 1) as u32)
                        << padding)
                        .leading_zeros() as usize;
                    let critbit_index = (MAX_DEPTH - 1) - common_path_len;
                    self.rightmost_proof.proof[critbit_index] = change_log.path[critbit_index];
                }
            } else {
                assert!(index == self.rightmost_proof.index);
                solana_logging!("Appending rightmost leaf");
                self.rightmost_proof.proof.copy_from_slice(&proof);
                self.rightmost_proof.index = index + 1;
                self.rightmost_proof.leaf = change_log.get_leaf();
            }
        }
        root
    }
}
