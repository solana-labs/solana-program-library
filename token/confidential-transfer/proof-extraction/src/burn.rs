use {
    crate::{encryption::PodBurnAmountCiphertext, errors::TokenProofExtractionError},
    solana_zk_sdk::{
        encryption::pod::elgamal::{PodElGamalCiphertext, PodElGamalPubkey},
        zk_elgamal_proof_program::proof_data::{
            BatchedGroupedCiphertext3HandlesValidityProofContext, BatchedRangeProofContext,
            CiphertextCommitmentEqualityProofContext,
        },
    },
};

/// The public keys associated with a confidential burn
pub struct BurnPubkeys {
    pub source: PodElGamalPubkey,
    pub auditor: PodElGamalPubkey,
    pub supply: PodElGamalPubkey,
}

/// The proof context information needed to process a confidential burn
/// instruction
pub struct BurnProofContext {
    pub burn_amount_ciphertext_lo: PodBurnAmountCiphertext,
    pub burn_amount_ciphertext_hi: PodBurnAmountCiphertext,
    pub burn_pubkeys: BurnPubkeys,
    pub remaining_balance_ciphertext: PodElGamalCiphertext,
}

impl BurnProofContext {
    pub fn verify_and_extract(
        equality_proof_context: &CiphertextCommitmentEqualityProofContext,
        ciphertext_validity_proof_context: &BatchedGroupedCiphertext3HandlesValidityProofContext,
        range_proof_context: &BatchedRangeProofContext,
    ) -> Result<Self, TokenProofExtractionError> {
        // The equality proof context consists of the source ElGamal public key, the new
        // source available balance ciphertext, and the new source avaialble
        // balance commitment. The public key should be checked with ciphertext
        // validity proof context for consistency and the commitment should be
        // checked with range proof for consistency. The public key and
        // the cihpertext should be returned as part of `BurnProofContext`.
        let CiphertextCommitmentEqualityProofContext {
            pubkey: source_elgamal_pubkey_from_equality_proof,
            ciphertext: remaining_balance_ciphertext,
            commitment: remaining_balance_commitment,
        } = equality_proof_context;

        // The ciphertext validity proof context consists of the source ElGamal public
        // key, the auditor ElGamal public key, and the grouped ElGamal
        // ciphertexts for the low and high bits of the burn amount. The source
        // ElGamal public key should be checked with equality
        // proof for consistency and the rest of the data should be returned as part of
        // `BurnProofContext`.
        let BatchedGroupedCiphertext3HandlesValidityProofContext {
            first_pubkey: source_elgamal_pubkey_from_validity_proof,
            second_pubkey: auditor_elgamal_pubkey,
            third_pubkey: supply_elgamal_pubkey,
            grouped_ciphertext_lo: burn_amount_ciphertext_lo,
            grouped_ciphertext_hi: burn_amount_ciphertext_hi,
        } = ciphertext_validity_proof_context;

        // The range proof context consists of the Pedersen commitments and bit-lengths
        // for which the range proof is proved. The commitments must consist of
        // three commitments pertaining to the new source available balance, the
        // low bits of the burn amount, and high bits of the burn
        // amount. These commitments must be checked for bit lengths `64`, `16`,
        // and `32`.
        let BatchedRangeProofContext {
            commitments: range_proof_commitments,
            bit_lengths: range_proof_bit_lengths,
        } = range_proof_context;

        // check that the source pubkey is consistent between equality and ciphertext
        // validity proofs
        if source_elgamal_pubkey_from_equality_proof != source_elgamal_pubkey_from_validity_proof {
            return Err(TokenProofExtractionError::ElGamalPubkeyMismatch);
        }

        // check that the range proof was created for the correct set of Pedersen
        // commitments
        let burn_amount_commitment_lo = burn_amount_ciphertext_lo.extract_commitment();
        let burn_amount_commitment_hi = burn_amount_ciphertext_hi.extract_commitment();

        let expected_commitments = [
            *remaining_balance_commitment,
            burn_amount_commitment_lo,
            burn_amount_commitment_hi,
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
        const BURN_AMOUNT_LO_BIT_LENGTH: u8 = 16;
        const BURN_AMOUNT_HI_BIT_LENGTH: u8 = 32;
        const PADDING_BIT_LENGTH: u8 = 16;
        let expected_bit_lengths = [
            REMAINING_BALANCE_BIT_LENGTH,
            BURN_AMOUNT_LO_BIT_LENGTH,
            BURN_AMOUNT_HI_BIT_LENGTH,
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

        let burn_pubkeys = BurnPubkeys {
            source: *source_elgamal_pubkey_from_equality_proof,
            auditor: *auditor_elgamal_pubkey,
            supply: *supply_elgamal_pubkey,
        };

        Ok(BurnProofContext {
            burn_amount_ciphertext_lo: PodBurnAmountCiphertext(*burn_amount_ciphertext_lo),
            burn_amount_ciphertext_hi: PodBurnAmountCiphertext(*burn_amount_ciphertext_hi),
            burn_pubkeys,
            remaining_balance_ciphertext: *remaining_balance_ciphertext,
        })
    }
}
