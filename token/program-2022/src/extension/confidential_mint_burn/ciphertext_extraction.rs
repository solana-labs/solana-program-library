#[cfg(feature = "zk-ops")]
use bytemuck::{Pod, Zeroable};
use solana_zk_token_sdk::zk_token_elgamal::pod::{ElGamalCiphertext, ElGamalPubkey};

#[cfg(feature = "zk-ops")]
use crate::{
    extension::confidential_transfer::ciphertext_extraction::extract_commitment_from_grouped_ciphertext,
    solana_program::program_error::ProgramError,
    solana_zk_token_sdk::{
        instruction::{
            BatchedGroupedCiphertext2HandlesValidityProofContext, BatchedRangeProofContext,
        },
        zk_token_elgamal::pod::GroupedElGamalCiphertext2Handles,
    },
};

/// Wrapper for `GroupedElGamalCiphertext2Handles` when used during minting
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct MintAmountCiphertext(pub GroupedElGamalCiphertext2Handles);

/// The proof context information needed to process a [Transfer] instruction.
#[cfg(feature = "zk-ops")]
pub struct MintProofContextInfo {
    /// destination elgamal pubkey used in proof generation
    pub destination_pubkey: ElGamalPubkey,
    /// auditor elgamal pubkey used in proof generation
    pub auditor_pubkey: ElGamalPubkey,
    /// Ciphertext containing the low 16 bits of the transafer amount
    pub ciphertext_lo: MintAmountCiphertext,
    /// Ciphertext containing the high 32 bits of the transafer amount
    pub ciphertext_hi: MintAmountCiphertext,
}

#[cfg(feature = "zk-ops")]
impl MintProofContextInfo {
    /// Create a transfer proof context information needed to process a
    /// [Transfer] instruction from split proof contexts after verifying
    /// their consistency.
    pub fn verify_and_extract(
        ciphertext_validity_proof_context: &BatchedGroupedCiphertext2HandlesValidityProofContext,
        range_proof_context: &BatchedRangeProofContext,
    ) -> Result<Self, ProgramError> {
        // The ciphertext validity proof context consists of the destination ElGamal
        // public key, auditor ElGamal public key, and the transfer amount
        // ciphertexts. All of these fields should be returned as part of
        // `MintProofContextInfo`. In addition, the commitments pertaining
        // to the mint amount ciphertexts should be checked with range proof for
        // consistency.
        let BatchedGroupedCiphertext2HandlesValidityProofContext {
            destination_pubkey,
            auditor_pubkey,
            grouped_ciphertext_lo: mint_amount_ciphertext_lo,
            grouped_ciphertext_hi: mint_amount_ciphertext_hi,
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
        let mint_amount_commitment_lo =
            extract_commitment_from_grouped_ciphertext(mint_amount_ciphertext_lo);
        let mint_amount_commitment_hi =
            extract_commitment_from_grouped_ciphertext(mint_amount_ciphertext_hi);

        let expected_commitments = [mint_amount_commitment_lo, mint_amount_commitment_hi];

        if !range_proof_commitments
            .iter()
            .zip(expected_commitments.iter())
            .all(|(proof_commitment, expected_commitment)| proof_commitment == expected_commitment)
        {
            return Err(ProgramError::InvalidInstructionData);
        }

        // check that the range proof was created for the correct number of bits
        const MINT_AMOUNT_LO_BIT_LENGTH: u8 = 16;
        const MINT_AMOUNT_HI_BIT_LENGTH: u8 = 32;
        const PADDING_BIT_LENGTH: u8 = 16;
        let expected_bit_lengths = [
            MINT_AMOUNT_LO_BIT_LENGTH,
            MINT_AMOUNT_HI_BIT_LENGTH,
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

        Ok(Self {
            destination_pubkey: *destination_pubkey,
            auditor_pubkey: *auditor_pubkey,
            ciphertext_lo: MintAmountCiphertext(*mint_amount_ciphertext_lo),
            ciphertext_hi: MintAmountCiphertext(*mint_amount_ciphertext_hi),
        })
    }
}

/// Extract the mint amount ciphertext encrypted under the auditor ElGamal
/// public key.
///
/// A mint amount ciphertext consists of the following 32-byte components
/// that are serialized in order:
///   1. The `commitment` component that encodes the mint amount.
///      key.
///   2. The `decryption handle` component with respect to the destination
///      public key.
///   3. The `decryption handle` component with respect to the auditor public
///      key.
///
/// An ElGamal ciphertext for the auditor consists of the `commitment` component
/// and the `decryption handle` component with respect to the auditor.
pub fn mint_amount_auditor_ciphertext(
    transfer_amount_ciphertext: &MintAmountCiphertext,
) -> ElGamalCiphertext {
    let transfer_amount_ciphertext_bytes = bytemuck::bytes_of(transfer_amount_ciphertext);

    let mut auditor_ciphertext_bytes = [0u8; 64];
    auditor_ciphertext_bytes[..32].copy_from_slice(&transfer_amount_ciphertext_bytes[..32]);
    auditor_ciphertext_bytes[32..].copy_from_slice(&transfer_amount_ciphertext_bytes[64..96]);

    ElGamalCiphertext(auditor_ciphertext_bytes)
}

/// Extract the mint amount ciphertext encrypted under the destination ElGamal
/// public key.
///
/// Structure see `mint_amount_auditor_ciphertext`
pub fn mint_amount_destination_ciphertext(
    transfer_amount_ciphertext: &MintAmountCiphertext,
) -> ElGamalCiphertext {
    let transfer_amount_ciphertext_bytes = bytemuck::bytes_of(transfer_amount_ciphertext);

    let mut destination_ciphertext_bytes = [0u8; 64];
    destination_ciphertext_bytes[..32].copy_from_slice(&transfer_amount_ciphertext_bytes[..32]);
    destination_ciphertext_bytes[32..].copy_from_slice(&transfer_amount_ciphertext_bytes[32..64]);

    ElGamalCiphertext(destination_ciphertext_bytes)
}
