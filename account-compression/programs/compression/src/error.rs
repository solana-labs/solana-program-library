use anchor_lang::prelude::*;
use spl_concurrent_merkle_tree::error::ConcurrentMerkleTreeError;

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

    /// An issue was detected with loading the provided account data for this Gummyroll tree.
    #[msg("Issue zero copying concurrent merkle tree data")]
    ZeroCopyError,

    /// See [ConcurrentMerkleTreeHeader](/gummyroll/state/struct.ConcurrentMerkleTreeHeader.html) for valid configuration options.
    #[msg("An unsupported max depth or max buffer size constant was provided")]
    ConcurrentMerkleTreeConstantsError,

    /// When using Canopy, the stored byte length should a multiple of the node's byte length (32 bytes)
    #[msg("Expected a different byte length for the merkle roll canopy")]
    CanopyLengthMismatch,
}

impl From<&ConcurrentMerkleTreeError> for AccountCompressionError {
    fn from(_error: &ConcurrentMerkleTreeError) -> Self {
        AccountCompressionError::ConcurrentMerkleTreeError
    }
}
