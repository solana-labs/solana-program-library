use anchor_lang::prelude::*;

#[error_code]
pub enum GummyrollError {
    #[msg("Incorrect leaf length. Expected vec of 32 bytes")]
    IncorrectLeafLength,
    #[msg("Concurrent merkle tree error")]
    ConcurrentMerkleTreeError,
    #[msg("Issue zero copying concurrent merkle tree data")]
    ZeroCopyError,
    #[msg("An unsupported max depth or max buffer size constant was provided")]
    MerkleRollConstantsError,
    #[msg("Expected a different byte length for the merkle roll")]
    MerkleRollByteLengthMismatch,
}
