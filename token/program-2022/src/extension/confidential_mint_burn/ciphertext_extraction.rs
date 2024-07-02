#[cfg(feature = "zk-ops")]
use crate::{
    error::TokenError,
    extension::confidential_transfer::ciphertext_extraction::extract_commitment_from_grouped_ciphertext,
    solana_program::program_error::ProgramError,
    solana_zk_token_sdk::{
        instruction::{
            BatchedGroupedCiphertext2HandlesValidityProofContext, BatchedRangeProofContext,
        },
        zk_token_elgamal::pod::GroupedElGamalCiphertext2Handles,
    },
};
#[cfg(feature = "zk-ops")]
use bytemuck::{Pod, Zeroable};
#[cfg(feature = "zk-ops")]
#[cfg(not(target_os = "solana"))]
use solana_zk_token_sdk::encryption::grouped_elgamal::GroupedElGamalCiphertext;
#[cfg(feature = "zk-ops")]
use solana_zk_token_sdk::{
    instruction::CiphertextCommitmentEqualityProofContext,
    zk_token_elgamal::pod::{ElGamalCiphertext, ElGamalPubkey},
};

/// Wrapper for `GroupedElGamalCiphertext2Handles` when used during minting
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct MintBurnAmountCiphertext(pub(crate) GroupedElGamalCiphertext2Handles);

#[cfg(not(target_os = "solana"))]
impl From<GroupedElGamalCiphertext<2>> for MintBurnAmountCiphertext {
    fn from(value: GroupedElGamalCiphertext<2>) -> Self {
        Self(value.into())
    }
}

/// Trait to retrieve auditor amounts from proof context information
#[cfg(feature = "zk-ops")]
pub trait AuditableProofContextInfo {
    /// Return the low 16 bits of the amount to be audited
    fn auditor_amount_lo(&self) -> ElGamalCiphertext;
    /// Return the high 32 bits of the amount to be audited
    fn auditor_amount_hi(&self) -> ElGamalCiphertext;
    /// Return the auditors ElGamal public key
    fn auditor_pubkey(&self) -> &ElGamalPubkey;
}

/// The proof context information needed to process a [Transfer] instruction.
#[cfg(feature = "zk-ops")]
pub struct MintProofContextInfo {
    /// destination elgamal pubkey used in proof generation
    pub destination_pubkey: ElGamalPubkey,
    /// auditor elgamal pubkey used in proof generation
    pub auditor_pubkey: ElGamalPubkey,
    /// Ciphertext containing the low 16 bits of the mint amount
    pub ciphertext_lo: MintBurnAmountCiphertext,
    /// Ciphertext containing the high 32 bits of the mint amount
    pub ciphertext_hi: MintBurnAmountCiphertext,
}

#[cfg(feature = "zk-ops")]
impl AuditableProofContextInfo for MintProofContextInfo {
    fn auditor_amount_lo(&self) -> ElGamalCiphertext {
        mint_burn_amount_auditor_ciphertext(&self.ciphertext_lo)
    }
    fn auditor_amount_hi(&self) -> ElGamalCiphertext {
        mint_burn_amount_auditor_ciphertext(&self.ciphertext_hi)
    }
    fn auditor_pubkey(&self) -> &ElGamalPubkey {
        &self.auditor_pubkey
    }
}

/// The proof context information needed to process a [Transfer] instruction.
#[cfg(feature = "zk-ops")]
pub struct BurnProofContextInfo {
    /// destination elgamal pubkey used in proof generation
    pub burner_pubkey: ElGamalPubkey,
    /// auditor elgamal pubkey used in proof generation
    pub auditor_pubkey: ElGamalPubkey,
    /// Ciphertext containing the low 16 bits of the burn amount
    pub ciphertext_lo: MintBurnAmountCiphertext,
    /// Ciphertext containing the high 32 bits of the burn amount
    pub ciphertext_hi: MintBurnAmountCiphertext,
    /// The new available balance ciphertext for the burning account
    pub new_burner_ciphertext: ElGamalCiphertext,
}

#[cfg(feature = "zk-ops")]
impl AuditableProofContextInfo for BurnProofContextInfo {
    fn auditor_amount_lo(&self) -> ElGamalCiphertext {
        mint_burn_amount_auditor_ciphertext(&self.ciphertext_lo)
    }
    fn auditor_amount_hi(&self) -> ElGamalCiphertext {
        mint_burn_amount_auditor_ciphertext(&self.ciphertext_hi)
    }
    fn auditor_pubkey(&self) -> &ElGamalPubkey {
        &self.auditor_pubkey
    }
}

#[cfg(feature = "zk-ops")]
impl MintProofContextInfo {
    /// Create the mint proof context information needed to process a
    /// [ConfidentialMint] instruction from context state accounts
    /// after verifying their consistency.
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
            ciphertext_lo: MintBurnAmountCiphertext(*mint_amount_ciphertext_lo),
            ciphertext_hi: MintBurnAmountCiphertext(*mint_amount_ciphertext_hi),
        })
    }
}

#[cfg(feature = "zk-ops")]
impl BurnProofContextInfo {
    /// Create a transfer proof context information needed to process a
    /// [Transfer] instruction from split proof contexts after verifying
    /// their consistency.
    pub fn verify_and_extract(
        equality_proof_context: &CiphertextCommitmentEqualityProofContext,
        ciphertext_validity_proof_context: &BatchedGroupedCiphertext2HandlesValidityProofContext,
        range_proof_context: &BatchedRangeProofContext,
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
            destination_pubkey: burner_pubkey,
            auditor_pubkey,
            grouped_ciphertext_lo: transfer_amount_ciphertext_lo,
            grouped_ciphertext_hi: transfer_amount_ciphertext_hi,
        } = ciphertext_validity_proof_context;

        if burner_pubkey != source_pubkey {
            return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
        }

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

        Ok(Self {
            burner_pubkey: *burner_pubkey,
            auditor_pubkey: *auditor_pubkey,
            ciphertext_lo: MintBurnAmountCiphertext(*transfer_amount_ciphertext_lo),
            ciphertext_hi: MintBurnAmountCiphertext(*transfer_amount_ciphertext_hi),
            new_burner_ciphertext: *new_source_ciphertext,
        })
    }
}

/// Extract the mint amount ciphertext encrypted under the auditor ElGamal
/// public key.
///
/// A mint amount ciphertext consists of the following 32-byte components
/// that are serialized in order:
///   1. The `commitment` component that encodes the mint amount. key.
///   2. The `decryption handle` component with respect to the destination or
///      source public key.
///   3. The `decryption handle` component with respect to the auditor public
///      key.
///
/// An ElGamal ciphertext for the auditor consists of the `commitment` component
/// and the `decryption handle` component with respect to the auditor.
pub fn mint_burn_amount_auditor_ciphertext(
    transfer_amount_ciphertext: &MintBurnAmountCiphertext,
) -> ElGamalCiphertext {
    let transfer_amount_ciphertext_bytes = bytemuck::bytes_of(transfer_amount_ciphertext);

    let mut auditor_ciphertext_bytes = [0u8; 64];
    auditor_ciphertext_bytes[..32].copy_from_slice(&transfer_amount_ciphertext_bytes[..32]);
    auditor_ciphertext_bytes[32..].copy_from_slice(&transfer_amount_ciphertext_bytes[64..96]);

    ElGamalCiphertext(auditor_ciphertext_bytes)
}

/// Extract the mint amount ciphertext encrypted under the destination or source
/// ElGamal public key, for mint and burn respectively.
///
/// Structure see `mint_amount_auditor_ciphertext`
pub fn mint_burn_amount_target_ciphertext(
    transfer_amount_ciphertext: &MintBurnAmountCiphertext,
) -> ElGamalCiphertext {
    let transfer_amount_ciphertext_bytes = bytemuck::bytes_of(transfer_amount_ciphertext);

    let mut destination_ciphertext_bytes = [0u8; 64];
    destination_ciphertext_bytes[..32].copy_from_slice(&transfer_amount_ciphertext_bytes[..32]);
    destination_ciphertext_bytes[32..].copy_from_slice(&transfer_amount_ciphertext_bytes[32..64]);

    ElGamalCiphertext(destination_ciphertext_bytes)
}
