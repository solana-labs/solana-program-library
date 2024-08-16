use thiserror::Error;

#[derive(Error, Clone, Debug, Eq, PartialEq)]
pub enum TokenProofExtractionError {
    #[error("ElGamal pubkey mismatch")]
    ElGamalPubkeyMismatch,
    #[error("Pedersen commitment mismatch")]
    PedersenCommitmentMismatch,
    #[error("Range proof length mismatch")]
    RangeProofLengthMismatch,
    #[error("Fee parameters mismatch")]
    FeeParametersMismatch,
    #[error("Curve arithmetic failed")]
    CurveArithmetic,
    #[error("Ciphertext extraction failed")]
    CiphertextExtraction,
}
