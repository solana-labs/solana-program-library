use {
    crate::{
        encryption::{PodFeeCiphertext, PodTransferAmountCiphertext},
        errors::TokenProofExtractionError,
    },
    bytemuck::bytes_of,
    solana_curve25519::{
        ristretto::{self, PodRistrettoPoint},
        scalar::PodScalar,
    },
    solana_zk_sdk::{
        encryption::pod::{
            elgamal::{PodElGamalCiphertext, PodElGamalPubkey},
            pedersen::PodPedersenCommitment,
        },
        zk_elgamal_proof_program::proof_data::{
            BatchedGroupedCiphertext2HandlesValidityProofContext,
            BatchedGroupedCiphertext3HandlesValidityProofContext, BatchedRangeProofContext,
            CiphertextCommitmentEqualityProofContext, PercentageWithCapProofContext,
        },
    },
};

const MAX_FEE_BASIS_POINTS: u64 = 10_000;
const REMAINING_BALANCE_BIT_LENGTH: u8 = 64;
const TRANSFER_AMOUNT_LO_BIT_LENGTH: u8 = 16;
const TRANSFER_AMOUNT_HI_BIT_LENGTH: u8 = 32;
const DELTA_BIT_LENGTH: u8 = 48;
const FEE_AMOUNT_LO_BIT_LENGTH: u8 = 16;
const FEE_AMOUNT_HI_BIT_LENGTH: u8 = 32;

/// The transfer public keys associated with a transfer with fee.
pub struct TransferWithFeePubkeys {
    /// Source ElGamal public key
    pub source: PodElGamalPubkey,
    /// Destination ElGamal public key
    pub destination: PodElGamalPubkey,
    /// Auditor ElGamal public key
    pub auditor: PodElGamalPubkey,
    /// Withdraw withheld authority public key
    pub withdraw_withheld_authority: PodElGamalPubkey,
}

/// The proof context information needed to process a [Transfer] instruction
/// with fee.
pub struct TransferWithFeeProofContext {
    /// Group encryption of the low 16 bits of the transfer amount
    pub ciphertext_lo: PodTransferAmountCiphertext,
    /// Group encryption of the high 48 bits of the transfer amount
    pub ciphertext_hi: PodTransferAmountCiphertext,
    /// The public encryption keys associated with the transfer: source, dest,
    /// auditor, and withdraw withheld authority
    pub transfer_with_fee_pubkeys: TransferWithFeePubkeys,
    /// The final spendable ciphertext after the transfer,
    pub new_source_ciphertext: PodElGamalCiphertext,
    /// The transfer fee encryption of the low 16 bits of the transfer fee
    /// amount
    pub fee_ciphertext_lo: PodFeeCiphertext,
    /// The transfer fee encryption of the hi 32 bits of the transfer fee amount
    pub fee_ciphertext_hi: PodFeeCiphertext,
}

impl TransferWithFeeProofContext {
    pub fn verify_and_extract(
        equality_proof_context: &CiphertextCommitmentEqualityProofContext,
        transfer_amount_ciphertext_validity_proof_context: &BatchedGroupedCiphertext3HandlesValidityProofContext,
        fee_sigma_proof_context: &PercentageWithCapProofContext,
        fee_ciphertext_validity_proof_context: &BatchedGroupedCiphertext2HandlesValidityProofContext,
        range_proof_context: &BatchedRangeProofContext,
        expected_fee_rate_basis_points: u16,
        expected_maximum_fee: u64,
    ) -> Result<Self, TokenProofExtractionError> {
        // The equality proof context consists of the source ElGamal public key, the new
        // source available balance ciphertext, and the new source available
        // commitment. The public key and ciphertext should be returned as part
        // of `TransferWithFeeProofContextInfo` and the commitment should be
        // checked with range proof for consistency.
        let CiphertextCommitmentEqualityProofContext {
            pubkey: source_pubkey_from_equality_proof,
            ciphertext: new_source_ciphertext,
            commitment: new_source_commitment,
        } = equality_proof_context;

        // The transfer amount ciphertext validity proof context consists of the
        // destination ElGamal public key, auditor ElGamal public key, and the
        // transfer amount ciphertexts. All of these fields should be returned
        // as part of `TransferWithFeeProofContextInfo`. In addition, the
        // commitments pertaining to the transfer amount ciphertexts should be
        // checked with range proof for consistency.
        let BatchedGroupedCiphertext3HandlesValidityProofContext {
            first_pubkey: source_pubkey_from_validity_proof,
            second_pubkey: destination_pubkey,
            third_pubkey: auditor_pubkey,
            grouped_ciphertext_lo: transfer_amount_ciphertext_lo,
            grouped_ciphertext_hi: transfer_amount_ciphertext_hi,
        } = transfer_amount_ciphertext_validity_proof_context;

        // The fee sigma proof context consists of the fee commitment, delta commitment,
        // claimed commitment, and max fee. The fee and claimed commitment
        // should be checked with range proof for consistency. The delta
        // commitment should be checked whether it is properly generated with
        // respect to the fee parameters. The max fee should be checked for
        // consistency with the fee parameters.
        let PercentageWithCapProofContext {
            percentage_commitment: fee_commitment,
            delta_commitment,
            claimed_commitment,
            max_value: proof_maximum_fee,
        } = fee_sigma_proof_context;

        let proof_maximum_fee: u64 = (*proof_maximum_fee).into();
        if expected_maximum_fee != proof_maximum_fee {
            return Err(TokenProofExtractionError::FeeParametersMismatch);
        }

        // The transfer fee ciphertext validity proof context consists of the
        // destination ElGamal public key, withdraw withheld authority ElGamal
        // public key, and the transfer fee ciphertexts. The rest of the fields
        // should be return as part of `TransferWithFeeProofContextInfo`. In
        // addition, the destination public key should be checked for
        // consistency with the destination public key contained in the transfer amount
        // ciphertext validity proof, and the commitments pertaining to the transfer fee
        // amount ciphertexts should be checked with range proof for
        // consistency.
        let BatchedGroupedCiphertext2HandlesValidityProofContext {
            first_pubkey: destination_pubkey_from_transfer_fee_validity_proof,
            second_pubkey: withdraw_withheld_authority_pubkey,
            grouped_ciphertext_lo: fee_ciphertext_lo,
            grouped_ciphertext_hi: fee_ciphertext_hi,
        } = fee_ciphertext_validity_proof_context;

        if destination_pubkey != destination_pubkey_from_transfer_fee_validity_proof {
            return Err(TokenProofExtractionError::ElGamalPubkeyMismatch);
        }

        // The range proof context consists of the Pedersen commitments and bit-lengths
        // for which the range proof is proved. The commitments must consist of
        // seven commitments pertaining to
        // - the new source available balance (64 bits)
        // - the low bits of the transfer amount (16 bits)
        // - the high bits of the transfer amount (32 bits)
        // - the delta amount for the fee (48 bits)
        // - the complement of the delta amount for the fee (48 bits)
        // - the low bits of the fee amount (16 bits)
        // - the high bits of the fee amount (32 bits)
        let BatchedRangeProofContext {
            commitments: range_proof_commitments,
            bit_lengths: range_proof_bit_lengths,
        } = range_proof_context;

        // check that the range proof was created for the correct set of Pedersen
        // commitments
        let transfer_amount_commitment_lo = transfer_amount_ciphertext_lo.extract_commitment();
        let transfer_amount_commitment_hi = transfer_amount_ciphertext_hi.extract_commitment();

        let fee_commitment_lo = fee_ciphertext_lo.extract_commitment();
        let fee_commitment_hi = fee_ciphertext_hi.extract_commitment();

        let max_fee_basis_points_scalar = u64_to_scalar(MAX_FEE_BASIS_POINTS);
        let max_fee_basis_points_commitment =
            ristretto::multiply_ristretto(&max_fee_basis_points_scalar, &G)
                .ok_or(TokenProofExtractionError::CurveArithmetic)?;
        let claimed_complement_commitment = ristretto::subtract_ristretto(
            &max_fee_basis_points_commitment,
            &commitment_to_ristretto(claimed_commitment),
        )
        .ok_or(TokenProofExtractionError::CurveArithmetic)?;

        let expected_commitments = [
            bytes_of(new_source_commitment),
            bytes_of(&transfer_amount_commitment_lo),
            bytes_of(&transfer_amount_commitment_hi),
            bytes_of(claimed_commitment),
            bytes_of(&claimed_complement_commitment),
            bytes_of(&fee_commitment_lo),
            bytes_of(&fee_commitment_hi),
        ];

        if !range_proof_commitments
            .iter()
            .zip(expected_commitments.into_iter())
            .all(|(proof_commitment, expected_commitment)| {
                bytes_of(proof_commitment) == expected_commitment
            })
        {
            return Err(TokenProofExtractionError::PedersenCommitmentMismatch);
        }

        // check that the range proof was created for the correct number of bits
        let expected_bit_lengths = [
            REMAINING_BALANCE_BIT_LENGTH,
            TRANSFER_AMOUNT_LO_BIT_LENGTH,
            TRANSFER_AMOUNT_HI_BIT_LENGTH,
            DELTA_BIT_LENGTH,
            DELTA_BIT_LENGTH,
            FEE_AMOUNT_LO_BIT_LENGTH,
            FEE_AMOUNT_HI_BIT_LENGTH,
        ]
        .iter();

        if !range_proof_bit_lengths
            .iter()
            .zip(expected_bit_lengths)
            .all(|(proof_len, expected_len)| proof_len == expected_len)
        {
            return Err(TokenProofExtractionError::RangeProofLengthMismatch);
        }

        // check consistency between fee sigma and fee ciphertext validity proofs
        let sigma_proof_fee_commitment_point: PodRistrettoPoint =
            commitment_to_ristretto(fee_commitment);
        let validity_proof_fee_point = combine_lo_hi_pedersen_points(
            &commitment_to_ristretto(&fee_commitment_lo),
            &commitment_to_ristretto(&fee_commitment_hi),
        )
        .ok_or(TokenProofExtractionError::CurveArithmetic)?;

        if source_pubkey_from_equality_proof != source_pubkey_from_validity_proof {
            return Err(TokenProofExtractionError::ElGamalPubkeyMismatch);
        }

        if validity_proof_fee_point != sigma_proof_fee_commitment_point {
            return Err(TokenProofExtractionError::FeeParametersMismatch);
        }

        verify_delta_commitment(
            &transfer_amount_commitment_lo,
            &transfer_amount_commitment_hi,
            fee_commitment,
            delta_commitment,
            expected_fee_rate_basis_points,
        )?;

        // create transfer with fee proof context info and return
        let transfer_with_fee_pubkeys = TransferWithFeePubkeys {
            source: *source_pubkey_from_equality_proof,
            destination: *destination_pubkey,
            auditor: *auditor_pubkey,
            withdraw_withheld_authority: *withdraw_withheld_authority_pubkey,
        };

        Ok(Self {
            ciphertext_lo: PodTransferAmountCiphertext(*transfer_amount_ciphertext_lo),
            ciphertext_hi: PodTransferAmountCiphertext(*transfer_amount_ciphertext_hi),
            transfer_with_fee_pubkeys,
            new_source_ciphertext: *new_source_ciphertext,
            fee_ciphertext_lo: PodFeeCiphertext(*fee_ciphertext_lo),
            fee_ciphertext_hi: PodFeeCiphertext(*fee_ciphertext_hi),
        })
    }
}

/// Ristretto generator point for curve25519
const G: PodRistrettoPoint = PodRistrettoPoint([
    226, 242, 174, 10, 106, 188, 78, 113, 168, 132, 169, 97, 197, 0, 81, 95, 88, 227, 11, 106, 165,
    130, 221, 141, 182, 166, 89, 69, 224, 141, 45, 118,
]);

/// Convert a `u64` amount into a curve25519 scalar
fn u64_to_scalar(amount: u64) -> PodScalar {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&amount.to_le_bytes());
    PodScalar(bytes)
}

/// Convert a `u16` amount into a curve25519 scalar
fn u16_to_scalar(amount: u16) -> PodScalar {
    let mut bytes = [0u8; 32];
    bytes[..2].copy_from_slice(&amount.to_le_bytes());
    PodScalar(bytes)
}

fn commitment_to_ristretto(commitment: &PodPedersenCommitment) -> PodRistrettoPoint {
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(bytes_of(commitment));
    PodRistrettoPoint(bytes)
}

/// Combine lo and hi Pedersen commitment points
fn combine_lo_hi_pedersen_points(
    point_lo: &PodRistrettoPoint,
    point_hi: &PodRistrettoPoint,
) -> Option<PodRistrettoPoint> {
    const SCALING_CONSTANT: u64 = 65536;
    let scaling_constant_scalar = u64_to_scalar(SCALING_CONSTANT);
    let scaled_point_hi = ristretto::multiply_ristretto(&scaling_constant_scalar, point_hi)?;
    ristretto::add_ristretto(point_lo, &scaled_point_hi)
}

/// Compute fee delta commitment
fn verify_delta_commitment(
    transfer_amount_commitment_lo: &PodPedersenCommitment,
    transfer_amount_commitment_hi: &PodPedersenCommitment,
    fee_commitment: &PodPedersenCommitment,
    proof_delta_commitment: &PodPedersenCommitment,
    transfer_fee_basis_points: u16,
) -> Result<(), TokenProofExtractionError> {
    let transfer_amount_point = combine_lo_hi_pedersen_points(
        &commitment_to_ristretto(transfer_amount_commitment_lo),
        &commitment_to_ristretto(transfer_amount_commitment_hi),
    )
    .ok_or(TokenProofExtractionError::CurveArithmetic)?;
    let transfer_fee_basis_points_scalar = u16_to_scalar(transfer_fee_basis_points);
    let scaled_transfer_amount_point =
        ristretto::multiply_ristretto(&transfer_fee_basis_points_scalar, &transfer_amount_point)
            .ok_or(TokenProofExtractionError::CurveArithmetic)?;

    let max_fee_basis_points_scalar = u64_to_scalar(MAX_FEE_BASIS_POINTS);
    let fee_point: PodRistrettoPoint = commitment_to_ristretto(fee_commitment);
    let scaled_fee_point = ristretto::multiply_ristretto(&max_fee_basis_points_scalar, &fee_point)
        .ok_or(TokenProofExtractionError::CurveArithmetic)?;

    let expected_delta_commitment_point =
        ristretto::subtract_ristretto(&scaled_fee_point, &scaled_transfer_amount_point)
            .ok_or(TokenProofExtractionError::CurveArithmetic)?;

    let proof_delta_commitment_point = commitment_to_ristretto(proof_delta_commitment);
    if expected_delta_commitment_point != proof_delta_commitment_point {
        return Err(TokenProofExtractionError::CurveArithmetic);
    }
    Ok(())
}
