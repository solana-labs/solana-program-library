use {
    crate::errors::TokenProofExtractionError,
    solana_zk_sdk::encryption::pod::{
        elgamal::PodElGamalCiphertext,
        grouped_elgamal::{
            PodGroupedElGamalCiphertext2Handles, PodGroupedElGamalCiphertext3Handles,
        },
        pedersen::PodPedersenCommitment,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct PodTransferAmountCiphertext(pub(crate) PodGroupedElGamalCiphertext3Handles);

impl PodTransferAmountCiphertext {
    pub fn extract_commitment(&self) -> PodPedersenCommitment {
        self.0.extract_commitment()
    }

    pub fn try_extract_ciphertext(
        &self,
        index: usize,
    ) -> Result<PodElGamalCiphertext, TokenProofExtractionError> {
        self.0
            .try_extract_ciphertext(index)
            .map_err(|_| TokenProofExtractionError::CiphertextExtraction)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct PodFeeCiphertext(pub(crate) PodGroupedElGamalCiphertext2Handles);

impl PodFeeCiphertext {
    pub fn extract_commitment(&self) -> PodPedersenCommitment {
        self.0.extract_commitment()
    }

    pub fn try_extract_ciphertext(
        &self,
        index: usize,
    ) -> Result<PodElGamalCiphertext, TokenProofExtractionError> {
        self.0
            .try_extract_ciphertext(index)
            .map_err(|_| TokenProofExtractionError::CiphertextExtraction)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct PodBurnAmountCiphertext(pub(crate) PodGroupedElGamalCiphertext3Handles);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct PodMintAmountCiphertext(pub(crate) PodGroupedElGamalCiphertext3Handles);
