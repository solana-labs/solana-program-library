use anchor_lang::{
    prelude::*,
    solana_program::{msg, program_error::ProgramError},
};
use bytemuck::PodCastError;
use spl_concurrent_merkle_tree::error::ConcurrentMerkleTreeError;
use std::any::type_name;
use std::mem::size_of;

/// Errors related to misconfiguration or misuse of the Merkle tree
#[error_code]
pub enum AccountCompressionError {
    /// This error is currently not used.
    #[msg("Incorrect leaf length. Expected vec of 32 bytes")]
    IncorrectLeafLength,

    /// A modification to the tree was invalid and a changelog was not emitted.
    /// The proof may be invalid or out-of-date, or the provided leaf hash was invalid.
    #[msg("Concurrent merkle tree error")]
    ConcurrentMerkleTreeError,

    /// An issue was detected with loading the provided account data for this ConcurrentMerkleTree.
    #[msg("Issue zero copying concurrent merkle tree data")]
    ZeroCopyError,

    /// See [ConcurrentMerkleTreeHeader](/spl_account_compression/state/struct.ConcurrentMerkleTreeHeader.html) for valid configuration options.
    #[msg("An unsupported max depth or max buffer size constant was provided")]
    ConcurrentMerkleTreeConstantsError,

    /// When using Canopy, the stored byte length should a multiple of the node's byte length (32 bytes)
    #[msg("Expected a different byte length for the merkle tree canopy")]
    CanopyLengthMismatch,

    /// Incorrect authority account provided
    #[msg("Provided authority does not match expected tree authority")]
    IncorrectAuthority,

    /// Incorrect account owner
    #[msg("Account is owned by a different program, expected it to be owned by this program")]
    IncorrectAccountOwner,

    /// Incorrect account type
    #[msg("Account provided has incorrect account type")]
    IncorrectAccountType,

    /// Tree information cannot be processed because the provided leaf_index
    /// is out of bounds of tree's maximum leaf capacity
    #[msg("Leaf index of concurrent merkle tree is out of bounds")]
    LeafIndexOutOfBounds,
}

impl From<&ConcurrentMerkleTreeError> for AccountCompressionError {
    fn from(_error: &ConcurrentMerkleTreeError) -> Self {
        AccountCompressionError::ConcurrentMerkleTreeError
    }
}

pub fn error_msg<T>(data_len: usize) -> impl Fn(PodCastError) -> ProgramError {
    move |_: PodCastError| -> ProgramError {
        msg!(
            "Failed to load {}. Size is {}, expected {}",
            type_name::<T>(),
            data_len,
            size_of::<T>(),
        );
        ProgramError::InvalidAccountData
    }
}
