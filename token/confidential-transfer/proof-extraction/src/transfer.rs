use {
    crate::{encryption::PodTransferAmountCiphertext, errors::TokenProofExtractionError},
    solana_zk_sdk::{
        encryption::pod::elgamal::{PodElGamalCiphertext, PodElGamalPubkey},
        zk_elgamal_proof_program::proof_data::{
            BatchedGroupedCiphertext3HandlesValidityProofContext, BatchedRangeProofContext,
            CiphertextCommitmentEqualityProofContext,
        },
    },
};

/// The transfer public keys associated with a transfer.
pub struct TransferPubkeys {
    /// Source ElGamal public key
    pub source: PodElGamalPubkey,
    /// Destination ElGamal public key
    pub destination: PodElGamalPubkey,
    /// Auditor ElGamal public key
    pub auditor: PodElGamalPubkey,
}

/// The proof context information needed to process a [Transfer] instruction.
pub struct TransferProofContext {
    /// Ciphertext containing the low 16 bits of the transafer amount
    pub ciphertext_lo: PodTransferAmountCiphertext,
    /// Ciphertext containing the high 32 bits of the transafer amount
    pub ciphertext_hi: PodTransferAmountCiphertext,
    /// The transfer public keys associated with a transfer
    pub transfer_pubkeys: TransferPubkeys,
    /// The new source available balance ciphertext
    pub new_source_ciphertext: PodElGamalCiphertext,
}

impl TransferProofContext {
    pub fn verify_and_extract(
        equality_proof_context: &CiphertextCommitmentEqualityProofContext,
        ciphertext_validity_proof_context: &BatchedGroupedCiphertext3HandlesValidityProofContext,
        range_proof_context: &BatchedRangeProofContext,
    ) -> Result<Self, TokenProofExtractionError> {
        // The equality proof context consists of the source ElGamal public key, the new
        // source available balance ciphertext, and the new source available
        // commitment. The public key and ciphertext should be returned as parts
        // of `TransferProofContextInfo` and the commitment should be checked
        // with range proof for consistency.
        let CiphertextCommitmentEqualityProofContext {
            pubkey: source_pubkey_from_equality_proof,
            ciphertext: new_source_ciphertext,
            commitment: new_source_commitment,
        } = equality_proof_context;

        // The ciphertext validity proof context consists of the destination ElGamal
        // public key, auditor ElGamal public key, and the transfer amount
        // ciphertexts. All of these fields should be returned as part of
        // `TransferProofContextInfo`. In addition, the commitments pertaining
        // to the transfer amount ciphertexts should be checked with range proof for
        // consistency.
        let BatchedGroupedCiphertext3HandlesValidityProofContext {
            first_pubkey: source_pubkey_from_validity_proof,
            second_pubkey: destination_pubkey,
            third_pubkey: auditor_pubkey,
            grouped_ciphertext_lo: transfer_amount_ciphertext_lo,
            grouped_ciphertext_hi: transfer_amount_ciphertext_hi,
        } = ciphertext_validity_proof_context;

        // The range proof context consists of the Pedersen commitments and bit-lengths
        // for which the range proof is proved. The commitments must consist of
        // three commitments pertaining to the new source available balance, the
        // low bits of the transfer amount, and high bits of the transfer
        // amount. These commitments must be checked for bit lengths `64`, `16`,
        // and `32`.
        let BatchedRangeProofContext {
            commitments: range_proof_commitments,
            bit_lengths: range_proof_bit_lengths,
        } = range_proof_context;

        // check that the source pubkey is consistent between equality and ciphertext
        // validity proofs
        if source_pubkey_from_equality_proof != source_pubkey_from_validity_proof {
            return Err(TokenProofExtractionError::ElGamalPubkeyMismatch);
        }

        // check that the range proof was created for the correct set of Pedersen
        // commitments
        let transfer_amount_commitment_lo = transfer_amount_ciphertext_lo.extract_commitment();
        let transfer_amount_commitment_hi = transfer_amount_ciphertext_hi.extract_commitment();

        let expected_commitments = [
            *new_source_commitment,
            transfer_amount_commitment_lo,
            transfer_amount_commitment_hi,
        ];

        if !range_proof_commitments
            .iter()
            .zip(expected_commitments.iter())
            .all(|(proof_commitment, expected_commitment)| proof_commitment == expected_commitment)
        {
            return Err(TokenProofExtractionError::PedersenCommitmentMismatch);
        }

        // check that the range proof was created for the correct number of bits
        const REMAINING_BALANCE_BIT_LENGTH: u8 = 64;
        const TRANSFER_AMOUNT_LO_BIT_LENGTH: u8 = 16;
        const TRANSFER_AMOUNT_HI_BIT_LENGTH: u8 = 32;
        const PADDING_BIT_LENGTH: u8 = 16;
        let expected_bit_lengths = [
            REMAINING_BALANCE_BIT_LENGTH,
            TRANSFER_AMOUNT_LO_BIT_LENGTH,
            TRANSFER_AMOUNT_HI_BIT_LENGTH,
            PADDING_BIT_LENGTH,
        ]
        .iter();

        if !range_proof_bit_lengths
            .iter()
            .zip(expected_bit_lengths)
            .all(|(proof_len, expected_len)| proof_len == expected_len)
        {
            return Err(TokenProofExtractionError::RangeProofLengthMismatch);
        }

        let transfer_pubkeys = TransferPubkeys {
            source: *source_pubkey_from_equality_proof,
            destination: *destination_pubkey,
            auditor: *auditor_pubkey,
        };

        let context_info = TransferProofContext {
            ciphertext_lo: PodTransferAmountCiphertext(*transfer_amount_ciphertext_lo),
            ciphertext_hi: PodTransferAmountCiphertext(*transfer_amount_ciphertext_hi),
            transfer_pubkeys,
            new_source_ciphertext: *new_source_ciphertext,
        };

        Ok(context_info)
    }
}
