//! Ciphertext extraction and proof related helper logic
//!
//! This submodule should be removed with the next upgrade to the Solana program

use crate::{
    extension::{confidential_transfer::*, confidential_transfer_fee::EncryptedFee},
    solana_program::program_error::ProgramError,
    solana_zk_token_sdk::{
        instruction::{
            transfer::TransferProofContext, BatchedGroupedCiphertext2HandlesValidityProofContext,
            BatchedRangeProofContext, CiphertextCommitmentEqualityProofContext,
        },
        zk_token_elgamal::pod::{
            DecryptHandle, GroupedElGamalCiphertext2Handles, GroupedElGamalCiphertext3Handles,
            PedersenCommitment, TransferAmountCiphertext,
        },
    },
};

#[cfg(feature = "serde-traits")]
use {
    crate::serialization::decrypthandle_fromstr,
    serde::{Deserialize, Serialize},
};

pub(crate) fn transfer_amount_commitment(
    transfer_amount_ciphertext: &GroupedElGamalCiphertext2Handles,
) -> PedersenCommitment {
    let transfer_amount_ciphertext_bytes = bytemuck::bytes_of(transfer_amount_ciphertext);
    let transfer_amount_commitment_bytes =
        transfer_amount_ciphertext_bytes[..32].try_into().unwrap();
    PedersenCommitment(transfer_amount_commitment_bytes)
}

/// Extract the transfer amount ciphertext encrypted under the source ElGamal public key.
///
/// A transfer amount ciphertext consists of the following 32-byte components that are serialized
/// in order:
///   1. The `commitment` component that encodes the transfer amount.
///   2. The `decryption handle` component with respect to the source public key.
///   3. The `decryption handle` component with respect to the destination public key.
///   4. The `decryption handle` component with respect to the auditor public key.
///
/// An ElGamal ciphertext for the source consists of the `commitment` component and the `decryption
/// handle` component with respect to the source.
pub(crate) fn transfer_amount_source_ciphertext(
    transfer_amount_ciphertext: &TransferAmountCiphertext,
) -> ElGamalCiphertext {
    let transfer_amount_ciphertext_bytes = bytemuck::bytes_of(transfer_amount_ciphertext);

    let mut source_ciphertext_bytes = [0u8; 64];
    source_ciphertext_bytes[..32].copy_from_slice(&transfer_amount_ciphertext_bytes[..32]);
    source_ciphertext_bytes[32..].copy_from_slice(&transfer_amount_ciphertext_bytes[32..64]);

    ElGamalCiphertext(source_ciphertext_bytes)
}

/// Extract the transfer amount ciphertext encrypted under the destination ElGamal public key.
///
/// A transfer amount ciphertext consists of the following 32-byte components that are serialized
/// in order:
///   1. The `commitment` component that encodes the transfer amount.
///   2. The `decryption handle` component with respect to the source public key.
///   3. The `decryption handle` component with respect to the destination public key.
///   4. The `decryption handle` component with respect to the auditor public key.
///
/// An ElGamal ciphertext for the destination consists of the `commitment` component and the
/// `decryption handle` component with respect to the destination public key.
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

/// Extract the fee amount ciphertext encrypted under the destination ElGamal public key.
///
/// A fee encryption amount consists of the following 32-byte components that are serialized in
/// order:
///   1. The `commitment` component that encodes the fee amount.
///   2. The `decryption handle` component with respect to the destination public key.
///   3. The `decryption handle` component with respect to the withdraw withheld authority public
///      key.
///
/// An ElGamal ciphertext for the destination consists of the `commitment` component and the
/// `decryption handle` component with respect to the destination public key.
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

/// Extract the transfer amount ciphertext encrypted under the withdraw withheld authority ElGamal
/// public key.
///
/// A fee encryption amount consists of the following 32-byte components that are serialized in
/// order:
///   1. The `commitment` component that encodes the fee amount.
///   2. The `decryption handle` component with respect to the destination public key.
///   3. The `decryption handle` component with respect to the withdraw withheld authority public
///      key.
///
/// An ElGamal ciphertext for the destination consists of the `commitment` component and the
/// `decryption handle` component with respect to the withdraw withheld authority public key.
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
    /// Create a transfer proof context information needed to process a [Transfer] instruction from
    /// split proof contexts after verifying their consistency.
    pub fn new(
        equality_proof_context: &CiphertextCommitmentEqualityProofContext,
        ciphertext_validity_proof_context: &BatchedGroupedCiphertext2HandlesValidityProofContext,
        range_proof_context: &BatchedRangeProofContext,
        source_decrypt_handles: &SourceDecryptHandles,
    ) -> Result<Self, ProgramError> {
        // The equality proof context consists of the source ElGamal public key, the new source
        // available balance ciphertext, and the new source available commitment. The public key
        // and ciphertext should be returned as parts of `TransferProofContextInfo` and the
        // commitment should be checked with range proof for consistency.
        let CiphertextCommitmentEqualityProofContext {
            pubkey: source_pubkey,
            ciphertext: new_source_ciphertext,
            commitment: new_source_commitment,
        } = equality_proof_context;

        // The ciphertext validity proof context consists of the destination ElGamal public key,
        // auditor ElGamal public key, and the transfer amount ciphertexts. All of these fields
        // should be returned as part of `TransferProofContextInfo`. In addition, the commitments
        // pertaining to the transfer amount ciphertexts should be checked with range proof for
        // consistency.
        let BatchedGroupedCiphertext2HandlesValidityProofContext {
            destination_pubkey,
            auditor_pubkey,
            grouped_ciphertext_lo: transfer_amount_ciphertext_lo,
            grouped_ciphertext_hi: transfer_amount_ciphertext_hi,
        } = ciphertext_validity_proof_context;

        // The range proof context consists of the Pedersen commitments and bit-lengths for which
        // the range proof is proved. The commitments must consist of three commitments pertaining
        // to the low bits of the transfer amount, high bits of the transfer amount, and the new
        // source available balance. These commitments must be checked for `16`, `32`, `80`.
        let BatchedRangeProofContext {
            commitments: range_proof_commitments,
            bit_lengths: range_proof_bit_lengths,
        } = range_proof_context;

        // check that the range proof was created for the correct set of Pedersen commitments
        let transfer_amount_commitment_lo =
            transfer_amount_commitment(transfer_amount_ciphertext_lo);
        let transfer_amount_commitment_hi =
            transfer_amount_commitment(transfer_amount_ciphertext_hi);

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

/// The ElGamal ciphertext decryption handle pertaining to the low and high bits of the transfer
/// amount under the source public key of the transfer.
///
/// The `TransferProofContext` contains decryption handles for the low and high bits of the
/// transfer amount. Howver, these decryption handles were (mistakenly) removed from the split
/// proof contexts as a form of optimization. These components should be added back into these
/// split proofs in `zk-token-sdk`. Until this modifications is made, include `SourceDecryptHandle`
/// in the transfer instruction data.
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct SourceDecryptHandles {
    /// The ElGamal decryption handle pertaining to the low 16 bits of the transfer amount.
    #[cfg_attr(feature = "serde-traits", serde(with = "decrypthandle_fromstr"))]
    pub lo: DecryptHandle,
    /// The ElGamal decryption handle pertaining to the low 32 bits of the transfer amount.
    #[cfg_attr(feature = "serde-traits", serde(with = "decrypthandle_fromstr"))]
    pub hi: DecryptHandle,
}
