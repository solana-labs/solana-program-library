//! Ciphertext extraction and proof related helper logic
//!
//! This submodule should be removed with the next upgrade to the Solana program

use crate::{
    extension::{
        confidential_transfer::*, confidential_transfer_fee::EncryptedFee,
        transfer_fee::TransferFee,
    },
    solana_program::program_error::ProgramError,
    solana_zk_token_sdk::{
        curve25519::{
            ristretto::{self, PodRistrettoPoint},
            scalar::PodScalar,
        },
        instruction::{
            transfer::{TransferProofContext, TransferWithFeeProofContext},
            BatchedGroupedCiphertext2HandlesValidityProofContext, BatchedRangeProofContext,
            CiphertextCommitmentEqualityProofContext, FeeSigmaProofContext,
        },
        zk_token_elgamal::pod::{
            DecryptHandle, FeeEncryption, GroupedElGamalCiphertext2Handles,
            GroupedElGamalCiphertext3Handles, PedersenCommitment, TransferAmountCiphertext,
        },
    },
};
#[cfg(feature = "serde-traits")]
use {
    crate::serialization::decrypthandle_fromstr,
    serde::{Deserialize, Serialize},
};

/// Extract the commitment component from a grouped ciphertext with 2 handles.
///
/// A grouped ciphertext with 2 handles consists of the following 32-bytes
/// components that are serialized in order:
///   1. The `commitment` component that encodes the fee amount.
///   3. The `decryption handle` component with respect to the destination
///      public key.
///   4. The `decryption handle` component with respect to the withdraw withheld
///      authority public key.
///
/// The fee commitment component consists of the first 32-byte.
pub(crate) fn extract_commitment_from_grouped_ciphertext(
    transfer_amount_ciphertext: &GroupedElGamalCiphertext2Handles,
) -> PedersenCommitment {
    let transfer_amount_ciphertext_bytes = bytemuck::bytes_of(transfer_amount_ciphertext);
    let transfer_amount_commitment_bytes =
        transfer_amount_ciphertext_bytes[..32].try_into().unwrap();
    PedersenCommitment(transfer_amount_commitment_bytes)
}

/// Extract the transfer amount ciphertext encrypted under the source ElGamal
/// public key.
///
/// A transfer amount ciphertext consists of the following 32-byte components
/// that are serialized in order:
///   1. The `commitment` component that encodes the transfer amount.
///   2. The `decryption handle` component with respect to the source public
///      key.
///   3. The `decryption handle` component with respect to the destination
///      public key.
///   4. The `decryption handle` component with respect to the auditor public
///      key.
///
/// An ElGamal ciphertext for the source consists of the `commitment` component
/// and the `decryption handle` component with respect to the source.
pub fn transfer_amount_source_ciphertext(
    transfer_amount_ciphertext: &TransferAmountCiphertext,
) -> ElGamalCiphertext {
    let transfer_amount_ciphertext_bytes = bytemuck::bytes_of(transfer_amount_ciphertext);

    let mut source_ciphertext_bytes = [0u8; 64];
    source_ciphertext_bytes[..32].copy_from_slice(&transfer_amount_ciphertext_bytes[..32]);
    source_ciphertext_bytes[32..].copy_from_slice(&transfer_amount_ciphertext_bytes[32..64]);

    ElGamalCiphertext(source_ciphertext_bytes)
}

/// Extract the transfer amount ciphertext encrypted under the destination
/// ElGamal public key.
///
/// A transfer amount ciphertext consists of the following 32-byte components
/// that are serialized in order:
///   1. The `commitment` component that encodes the transfer amount.
///   2. The `decryption handle` component with respect to the source public
///      key.
///   3. The `decryption handle` component with respect to the destination
///      public key.
///   4. The `decryption handle` component with respect to the auditor public
///      key.
///
/// An ElGamal ciphertext for the destination consists of the `commitment`
/// component and the `decryption handle` component with respect to the
/// destination public key.
#[cfg(feature = "zk-ops")]
pub(crate) fn transfer_amount_destination_ciphertext(
    transfer_amount_ciphertext: &TransferAmountCiphertext,
) -> ElGamalCiphertext {
    let transfer_amount_ciphertext_bytes = bytemuck::bytes_of(transfer_amount_ciphertext);

    let mut destination_ciphertext_bytes = [0u8; 64];
    destination_ciphertext_bytes[..32].copy_from_slice(&transfer_amount_ciphertext_bytes[..32]);
    destination_ciphertext_bytes[32..].copy_from_slice(&transfer_amount_ciphertext_bytes[64..96]);

    ElGamalCiphertext(destination_ciphertext_bytes)
}

/// Extract the fee amount ciphertext encrypted under the destination ElGamal
/// public key.
///
/// A fee encryption amount consists of the following 32-byte components that
/// are serialized in order:
///   1. The `commitment` component that encodes the fee amount.
///   2. The `decryption handle` component with respect to the destination
///      public key.
///   3. The `decryption handle` component with respect to the withdraw withheld
///      authority public key.
///
/// An ElGamal ciphertext for the destination consists of the `commitment`
/// component and the `decryption handle` component with respect to the
/// destination public key.
#[cfg(feature = "zk-ops")]
pub(crate) fn fee_amount_destination_ciphertext(
    transfer_amount_ciphertext: &EncryptedFee,
) -> ElGamalCiphertext {
    let transfer_amount_ciphertext_bytes = bytemuck::bytes_of(transfer_amount_ciphertext);

    let mut source_ciphertext_bytes = [0u8; 64];
    source_ciphertext_bytes[..32].copy_from_slice(&transfer_amount_ciphertext_bytes[..32]);
    source_ciphertext_bytes[32..].copy_from_slice(&transfer_amount_ciphertext_bytes[32..64]);

    ElGamalCiphertext(source_ciphertext_bytes)
}

/// Extract the transfer amount ciphertext encrypted under the withdraw withheld
/// authority ElGamal public key.
///
/// A fee encryption amount consists of the following 32-byte components that
/// are serialized in order:
///   1. The `commitment` component that encodes the fee amount.
///   2. The `decryption handle` component with respect to the destination
///      public key.
///   3. The `decryption handle` component with respect to the withdraw withheld
///      authority public key.
///
/// An ElGamal ciphertext for the destination consists of the `commitment`
/// component and the `decryption handle` component with respect to the withdraw
/// withheld authority public key.
#[cfg(feature = "zk-ops")]
pub(crate) fn fee_amount_withdraw_withheld_authority_ciphertext(
    transfer_amount_ciphertext: &EncryptedFee,
) -> ElGamalCiphertext {
    let transfer_amount_ciphertext_bytes = bytemuck::bytes_of(transfer_amount_ciphertext);

    let mut destination_ciphertext_bytes = [0u8; 64];
    destination_ciphertext_bytes[..32].copy_from_slice(&transfer_amount_ciphertext_bytes[..32]);
    destination_ciphertext_bytes[32..].copy_from_slice(&transfer_amount_ciphertext_bytes[64..96]);

    ElGamalCiphertext(destination_ciphertext_bytes)
}

#[cfg(feature = "zk-ops")]
pub(crate) fn transfer_amount_encryption_from_decrypt_handle(
    source_decrypt_handle: &DecryptHandle,
    grouped_ciphertext: &GroupedElGamalCiphertext2Handles,
) -> TransferAmountCiphertext {
    let source_decrypt_handle_bytes = bytemuck::bytes_of(source_decrypt_handle);
    let grouped_ciphertext_bytes = bytemuck::bytes_of(grouped_ciphertext);

    let mut transfer_amount_ciphertext_bytes = [0u8; 128];
    transfer_amount_ciphertext_bytes[..32].copy_from_slice(&grouped_ciphertext_bytes[..32]);
    transfer_amount_ciphertext_bytes[32..64].copy_from_slice(source_decrypt_handle_bytes);
    transfer_amount_ciphertext_bytes[64..128].copy_from_slice(&grouped_ciphertext_bytes[32..96]);

    TransferAmountCiphertext(GroupedElGamalCiphertext3Handles(
        transfer_amount_ciphertext_bytes,
    ))
}

/// The transfer public keys associated with a transfer.
#[cfg(feature = "zk-ops")]
pub struct TransferPubkeysInfo {
    /// Source ElGamal public key
    pub source: ElGamalPubkey,
    /// Destination ElGamal public key
    pub destination: ElGamalPubkey,
    /// Auditor ElGamal public key
    pub auditor: ElGamalPubkey,
}

/// The proof context information needed to process a [Transfer] instruction.
#[cfg(feature = "zk-ops")]
pub struct TransferProofContextInfo {
    /// Ciphertext containing the low 16 bits of the transafer amount
    pub ciphertext_lo: TransferAmountCiphertext,
    /// Ciphertext containing the high 32 bits of the transafer amount
    pub ciphertext_hi: TransferAmountCiphertext,
    /// The transfer public keys associated with a transfer
    pub transfer_pubkeys: TransferPubkeysInfo,
    /// The new source available balance ciphertext
    pub new_source_ciphertext: ElGamalCiphertext,
}

#[cfg(feature = "zk-ops")]
impl From<TransferProofContext> for TransferProofContextInfo {
    fn from(context: TransferProofContext) -> Self {
        let transfer_pubkeys = TransferPubkeysInfo {
            source: context.transfer_pubkeys.source,
            destination: context.transfer_pubkeys.destination,
            auditor: context.transfer_pubkeys.auditor,
        };

        TransferProofContextInfo {
            ciphertext_lo: context.ciphertext_lo,
            ciphertext_hi: context.ciphertext_hi,
            transfer_pubkeys,
            new_source_ciphertext: context.new_source_ciphertext,
        }
    }
}

#[cfg(feature = "zk-ops")]
impl TransferProofContextInfo {
    /// Create a transfer proof context information needed to process a
    /// [Transfer] instruction from split proof contexts after verifying
    /// their consistency.
    pub fn verify_and_extract(
        equality_proof_context: &CiphertextCommitmentEqualityProofContext,
        ciphertext_validity_proof_context: &BatchedGroupedCiphertext2HandlesValidityProofContext,
        range_proof_context: &BatchedRangeProofContext,
        source_decrypt_handles: &SourceDecryptHandles,
    ) -> Result<Self, ProgramError> {
        // The equality proof context consists of the source ElGamal public key, the new
        // source available balance ciphertext, and the new source available
        // commitment. The public key and ciphertext should be returned as parts
        // of `TransferProofContextInfo` and the commitment should be checked
        // with range proof for consistency.
        let CiphertextCommitmentEqualityProofContext {
            pubkey: source_pubkey,
            ciphertext: new_source_ciphertext,
            commitment: new_source_commitment,
        } = equality_proof_context;

        // The ciphertext validity proof context consists of the destination ElGamal
        // public key, auditor ElGamal public key, and the transfer amount
        // ciphertexts. All of these fields should be returned as part of
        // `TransferProofContextInfo`. In addition, the commitments pertaining
        // to the transfer amount ciphertexts should be checked with range proof for
        // consistency.
        let BatchedGroupedCiphertext2HandlesValidityProofContext {
            destination_pubkey,
            auditor_pubkey,
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

        // check that the range proof was created for the correct set of Pedersen
        // commitments
        let transfer_amount_commitment_lo =
            extract_commitment_from_grouped_ciphertext(transfer_amount_ciphertext_lo);
        let transfer_amount_commitment_hi =
            extract_commitment_from_grouped_ciphertext(transfer_amount_ciphertext_hi);

        let expected_commitments = [
            *new_source_commitment,
            transfer_amount_commitment_lo,
            transfer_amount_commitment_hi,
            // the fourth dummy commitment can be any commitment
        ];

        if !range_proof_commitments
            .iter()
            .zip(expected_commitments.iter())
            .all(|(proof_commitment, expected_commitment)| proof_commitment == expected_commitment)
        {
            return Err(ProgramError::InvalidInstructionData);
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
            return Err(ProgramError::InvalidInstructionData);
        }

        let transfer_pubkeys = TransferPubkeysInfo {
            source: *source_pubkey,
            destination: *destination_pubkey,
            auditor: *auditor_pubkey,
        };

        let transfer_amount_ciphertext_lo = transfer_amount_encryption_from_decrypt_handle(
            &source_decrypt_handles.lo,
            transfer_amount_ciphertext_lo,
        );

        let transfer_amount_ciphertext_hi = transfer_amount_encryption_from_decrypt_handle(
            &source_decrypt_handles.hi,
            transfer_amount_ciphertext_hi,
        );

        Ok(Self {
            ciphertext_lo: transfer_amount_ciphertext_lo,
            ciphertext_hi: transfer_amount_ciphertext_hi,
            transfer_pubkeys,
            new_source_ciphertext: *new_source_ciphertext,
        })
    }
}

/// The transfer public keys associated with a transfer with fee.
#[cfg(feature = "zk-ops")]
pub struct TransferWithFeePubkeysInfo {
    /// Source ElGamal public key
    pub source: ElGamalPubkey,
    /// Destination ElGamal public key
    pub destination: ElGamalPubkey,
    /// Auditor ElGamal public key
    pub auditor: ElGamalPubkey,
    /// Withdraw withheld authority public key
    pub withdraw_withheld_authority: ElGamalPubkey,
}

/// The proof context information needed to process a [Transfer] instruction
/// with fee.
#[cfg(feature = "zk-ops")]
pub struct TransferWithFeeProofContextInfo {
    /// Group encryption of the low 16 bits of the transfer amount
    pub ciphertext_lo: TransferAmountCiphertext,
    /// Group encryption of the high 48 bits of the transfer amount
    pub ciphertext_hi: TransferAmountCiphertext,
    /// The public encryption keys associated with the transfer: source, dest,
    /// auditor, and withdraw withheld authority
    pub transfer_with_fee_pubkeys: TransferWithFeePubkeysInfo,
    /// The final spendable ciphertext after the transfer,
    pub new_source_ciphertext: ElGamalCiphertext,
    /// The transfer fee encryption of the low 16 bits of the transfer fee
    /// amount
    pub fee_ciphertext_lo: EncryptedFee,
    /// The transfer fee encryption of the hi 32 bits of the transfer fee amount
    pub fee_ciphertext_hi: EncryptedFee,
}

#[cfg(feature = "zk-ops")]
impl From<TransferWithFeeProofContext> for TransferWithFeeProofContextInfo {
    fn from(context: TransferWithFeeProofContext) -> Self {
        let transfer_with_fee_pubkeys = TransferWithFeePubkeysInfo {
            source: context.transfer_with_fee_pubkeys.source,
            destination: context.transfer_with_fee_pubkeys.destination,
            auditor: context.transfer_with_fee_pubkeys.auditor,
            withdraw_withheld_authority: context
                .transfer_with_fee_pubkeys
                .withdraw_withheld_authority,
        };

        TransferWithFeeProofContextInfo {
            ciphertext_lo: context.ciphertext_lo,
            ciphertext_hi: context.ciphertext_hi,
            transfer_with_fee_pubkeys,
            new_source_ciphertext: context.new_source_ciphertext,
            fee_ciphertext_lo: context.fee_ciphertext_lo,
            fee_ciphertext_hi: context.fee_ciphertext_hi,
        }
    }
}

#[cfg(feature = "zk-ops")]
impl TransferWithFeeProofContextInfo {
    /// Create a transfer proof context information needed to process a
    /// [Transfer] instruction from split proof contexts after verifying
    /// their consistency.
    pub fn verify_and_extract(
        equality_proof_context: &CiphertextCommitmentEqualityProofContext,
        transfer_amount_ciphertext_validity_proof_context: &BatchedGroupedCiphertext2HandlesValidityProofContext,
        fee_sigma_proof_context: &FeeSigmaProofContext,
        fee_ciphertext_validity_proof_context: &BatchedGroupedCiphertext2HandlesValidityProofContext,
        range_proof_context: &BatchedRangeProofContext,
        source_decrypt_handles: &SourceDecryptHandles,
        fee_parameters: &TransferFee,
    ) -> Result<Self, ProgramError> {
        // The equality proof context consists of the source ElGamal public key, the new
        // source available balance ciphertext, and the new source available
        // commitment. The public key and ciphertext should be returned as part
        // of `TransferWithFeeProofContextInfo` and the commitment should be
        // checked with range proof for consistency.
        let CiphertextCommitmentEqualityProofContext {
            pubkey: source_pubkey,
            ciphertext: new_source_ciphertext,
            commitment: new_source_commitment,
        } = equality_proof_context;

        // The transfer amount ciphertext validity proof context consists of the
        // destination ElGamal public key, auditor ElGamal public key, and the
        // transfer amount ciphertexts. All of these fields should be returned
        // as part of `TransferWithFeeProofContextInfo`. In addition, the
        // commitments pertaining to the transfer amount ciphertexts should be
        // checked with range proof for consistency.
        let BatchedGroupedCiphertext2HandlesValidityProofContext {
            destination_pubkey,
            auditor_pubkey,
            grouped_ciphertext_lo: transfer_amount_ciphertext_lo,
            grouped_ciphertext_hi: transfer_amount_ciphertext_hi,
        } = transfer_amount_ciphertext_validity_proof_context;

        // The fee sigma proof context consists of the fee commitment, delta commitment,
        // claimed commitment, and max fee. The fee and claimed commitment
        // should be checked with range proof for consistency. The delta
        // commitment should be checked whether it is properly generated with
        // respect to the fee parameters. The max fee should be checked for
        // consistency with the fee parameters.
        let FeeSigmaProofContext {
            fee_commitment,
            delta_commitment,
            claimed_commitment,
            max_fee,
        } = fee_sigma_proof_context;

        let expected_maximum_fee: u64 = fee_parameters.maximum_fee.into();
        let proof_maximum_fee: u64 = (*max_fee).into();
        if expected_maximum_fee != proof_maximum_fee {
            return Err(ProgramError::InvalidInstructionData);
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
            destination_pubkey: destination_pubkey_from_transfer_fee_validity_proof,
            auditor_pubkey: withdraw_withheld_authority_pubkey,
            grouped_ciphertext_lo: fee_ciphertext_lo,
            grouped_ciphertext_hi: fee_ciphertext_hi,
        } = fee_ciphertext_validity_proof_context;

        if destination_pubkey != destination_pubkey_from_transfer_fee_validity_proof {
            return Err(ProgramError::InvalidInstructionData);
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
        let transfer_amount_commitment_lo =
            extract_commitment_from_grouped_ciphertext(transfer_amount_ciphertext_lo);
        let transfer_amount_commitment_hi =
            extract_commitment_from_grouped_ciphertext(transfer_amount_ciphertext_hi);

        let fee_commitment_lo = extract_commitment_from_grouped_ciphertext(fee_ciphertext_lo);
        let fee_commitment_hi = extract_commitment_from_grouped_ciphertext(fee_ciphertext_hi);

        const MAX_FEE_BASIS_POINTS: u64 = 10_000;
        let max_fee_basis_points_scalar = u64_to_scalar(MAX_FEE_BASIS_POINTS);
        let max_fee_basis_points_commitment =
            ristretto::multiply_ristretto(&max_fee_basis_points_scalar, &G)
                .ok_or(TokenError::CiphertextArithmeticFailed)?;
        let claimed_complement_commitment = ristretto::subtract_ristretto(
            &max_fee_basis_points_commitment,
            &(*claimed_commitment).into(),
        )
        .ok_or(TokenError::CiphertextArithmeticFailed)?;

        let expected_commitments = [
            *new_source_commitment,
            transfer_amount_commitment_lo,
            transfer_amount_commitment_hi,
            *claimed_commitment,
            claimed_complement_commitment.into(),
            fee_commitment_lo,
            fee_commitment_hi,
        ];

        if !range_proof_commitments
            .iter()
            .zip(expected_commitments.iter())
            .all(|(proof_commitment, expected_commitment)| proof_commitment == expected_commitment)
        {
            return Err(ProgramError::InvalidInstructionData);
        }

        // check that the range proof was created for the correct number of bits
        const REMAINING_BALANCE_BIT_LENGTH: u8 = 64;
        const TRANSFER_AMOUNT_LO_BIT_LENGTH: u8 = 16;
        const TRANSFER_AMOUNT_HI_BIT_LENGTH: u8 = 32;
        const DELTA_BIT_LENGTH: u8 = 48;
        const FEE_AMOUNT_LO_BIT_LENGTH: u8 = 16;
        const FEE_AMOUNT_HI_BIT_LENGTH: u8 = 32;

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
            return Err(ProgramError::InvalidInstructionData);
        }

        // check consistency between fee sigma and fee ciphertext validity proofs
        let sigma_proof_fee_commitment_point: PodRistrettoPoint = (*fee_commitment).into();
        let validity_proof_fee_point =
            combine_lo_hi_pedersen_points(&fee_commitment_lo.into(), &fee_commitment_hi.into())
                .ok_or(TokenError::CiphertextArithmeticFailed)?;
        if validity_proof_fee_point != sigma_proof_fee_commitment_point {
            return Err(ProgramError::InvalidInstructionData);
        }

        verify_delta_commitment(
            &transfer_amount_commitment_lo,
            &transfer_amount_commitment_hi,
            fee_commitment,
            delta_commitment,
            fee_parameters.transfer_fee_basis_points.into(),
        )?;

        // create transfer with fee proof context info and return
        let transfer_with_fee_pubkeys = TransferWithFeePubkeysInfo {
            source: *source_pubkey,
            destination: *destination_pubkey,
            auditor: *auditor_pubkey,
            withdraw_withheld_authority: *withdraw_withheld_authority_pubkey,
        };

        let transfer_amount_ciphertext_lo = transfer_amount_encryption_from_decrypt_handle(
            &source_decrypt_handles.lo,
            transfer_amount_ciphertext_lo,
        );

        let transfer_amount_ciphertext_hi = transfer_amount_encryption_from_decrypt_handle(
            &source_decrypt_handles.hi,
            transfer_amount_ciphertext_hi,
        );

        Ok(Self {
            ciphertext_lo: transfer_amount_ciphertext_lo,
            ciphertext_hi: transfer_amount_ciphertext_hi,
            transfer_with_fee_pubkeys,
            new_source_ciphertext: *new_source_ciphertext,
            fee_ciphertext_lo: FeeEncryption(*fee_ciphertext_lo),
            fee_ciphertext_hi: FeeEncryption(*fee_ciphertext_hi),
        })
    }
}

/// The ElGamal ciphertext decryption handle pertaining to the low and high bits
/// of the transfer amount under the source public key of the transfer.
///
/// The `TransferProofContext` contains decryption handles for the low and high
/// bits of the transfer amount. Howver, these decryption handles were
/// (mistakenly) removed from the split proof contexts as a form of
/// optimization. These components should be added back into these split proofs
/// in `zk-token-sdk`. Until this modifications is made, include
/// `SourceDecryptHandle` in the transfer instruction data.
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct SourceDecryptHandles {
    /// The ElGamal decryption handle pertaining to the low 16 bits of the
    /// transfer amount.
    #[cfg_attr(feature = "serde-traits", serde(with = "decrypthandle_fromstr"))]
    pub lo: DecryptHandle,
    /// The ElGamal decryption handle pertaining to the low 32 bits of the
    /// transfer amount.
    #[cfg_attr(feature = "serde-traits", serde(with = "decrypthandle_fromstr"))]
    pub hi: DecryptHandle,
}

/// Ristretto generator point for curve25519
const G: PodRistrettoPoint = PodRistrettoPoint([
    226, 242, 174, 10, 106, 188, 78, 113, 168, 132, 169, 97, 197, 0, 81, 95, 88, 227, 11, 106, 165,
    130, 221, 141, 182, 166, 89, 69, 224, 141, 45, 118,
]);

/// Convert a `u16` amount into a curve25519 scalar
fn u16_to_scalar(amount: u16) -> PodScalar {
    let mut bytes = [0u8; 32];
    bytes[..2].copy_from_slice(&amount.to_le_bytes());
    PodScalar(bytes)
}

/// Convert a `u64` amount into a curve25519 scalar
fn u64_to_scalar(amount: u64) -> PodScalar {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&amount.to_le_bytes());
    PodScalar(bytes)
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
    transfer_amount_commitment_lo: &PedersenCommitment,
    transfer_amount_commitment_hi: &PedersenCommitment,
    fee_commitment: &PedersenCommitment,
    proof_delta_commitment: &PedersenCommitment,
    transfer_fee_basis_points: u16,
) -> Result<(), ProgramError> {
    let transfer_amount_point = combine_lo_hi_pedersen_points(
        &(*transfer_amount_commitment_lo).into(),
        &(*transfer_amount_commitment_hi).into(),
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;
    let transfer_fee_basis_points_scalar = u16_to_scalar(transfer_fee_basis_points);
    let scaled_transfer_amount_point =
        ristretto::multiply_ristretto(&transfer_fee_basis_points_scalar, &transfer_amount_point)
            .ok_or(TokenError::CiphertextArithmeticFailed)?;

    const MAX_FEE_BASIS_POINTS: u64 = 10_000;
    let max_fee_basis_points_scalar = u64_to_scalar(MAX_FEE_BASIS_POINTS);
    let fee_point: PodRistrettoPoint = (*fee_commitment).into();
    let scaled_fee_point = ristretto::multiply_ristretto(&max_fee_basis_points_scalar, &fee_point)
        .ok_or(TokenError::CiphertextArithmeticFailed)?;

    let expected_delta_commitment_point =
        ristretto::subtract_ristretto(&scaled_fee_point, &scaled_transfer_amount_point)
            .ok_or(TokenError::CiphertextArithmeticFailed)?;

    let proof_delta_commitment_point = (*proof_delta_commitment).into();
    if expected_delta_commitment_point != proof_delta_commitment_point {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(())
}
