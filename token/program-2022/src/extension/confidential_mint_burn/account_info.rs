use {
    super::ConfidentialMintBurn,
    crate::error::TokenError,
    bytemuck::{Pod, Zeroable},
    solana_zk_sdk::{
        encryption::{
            auth_encryption::{AeCiphertext, AeKey},
            elgamal::{ElGamalCiphertext, ElGamalKeypair},
            pedersen::PedersenOpening,
            pod::{
                auth_encryption::PodAeCiphertext,
                elgamal::{PodElGamalCiphertext, PodElGamalPubkey},
            },
        },
        zk_elgamal_proof_program::proof_data::CiphertextCiphertextEqualityProofData,
    },
};

/// Confidential Mint Burn extension information needed to construct a
/// `RotateSupplyElgamalPubkey` instruction.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct SupplyAccountInfo {
    /// The available balance (encrypted by `supply_elgamal_pubkey`)
    pub current_supply: PodElGamalCiphertext,
    /// The decryptable supply
    pub decryptable_supply: PodAeCiphertext,
    /// The supply's elgamal pubkey
    pub supply_elgamal_pubkey: PodElGamalPubkey,
}

impl SupplyAccountInfo {
    /// Creates a SupplyAccountInfo from ConfidentialMintBurn extension account
    /// data
    pub fn new(extension: &ConfidentialMintBurn) -> Self {
        Self {
            current_supply: extension.confidential_supply,
            decryptable_supply: extension.decryptable_supply,
            supply_elgamal_pubkey: extension.supply_elgamal_pubkey,
        }
    }

    /// Computes the current supply from the decryptable supply and the
    /// difference between the decryptable supply and the elgamal encrypted
    /// supply ciphertext
    pub fn decrypt_current_supply(
        &self,
        aes_key: &AeKey,
        elgamal_keypair: &ElGamalKeypair,
    ) -> Result<u64, TokenError> {
        // decrypt the decryptable supply
        let current_decyptable_supply = AeCiphertext::try_from(self.decryptable_supply)
            .map_err(|_| TokenError::MalformedCiphertext)?
            .decrypt(aes_key)
            .ok_or(TokenError::MalformedCiphertext)?;

        // get the difference between the supply ciphertext and the decryptable supply
        // explanation see https://github.com/solana-labs/solana-program-library/pull/6881#issuecomment-2385579058
        let decryptable_supply_ciphertext =
            elgamal_keypair.pubkey().encrypt(current_decyptable_supply);
        #[allow(clippy::arithmetic_side_effects)]
        let supply_delta_ciphertext = decryptable_supply_ciphertext
            - ElGamalCiphertext::try_from(self.current_supply)
                .map_err(|_| TokenError::MalformedCiphertext)?;
        let decryptable_to_current_diff = elgamal_keypair
            .secret()
            .decrypt_u32(&supply_delta_ciphertext)
            .ok_or(TokenError::MalformedCiphertext)?;

        // compute the current supply
        current_decyptable_supply
            .checked_sub(decryptable_to_current_diff)
            .ok_or(TokenError::Overflow)
    }

    /// Generates the `CiphertextCiphertextEqualityProofData` needed for a
    /// `RotateSupplyElgamalPubkey` instruction
    pub fn generate_rotate_supply_elgamal_pubkey_proof(
        &self,
        aes_key: &AeKey,
        current_supply_elgamal_keypair: &ElGamalKeypair,
        new_supply_elgamal_keypair: &ElGamalKeypair,
    ) -> Result<CiphertextCiphertextEqualityProofData, TokenError> {
        let current_supply =
            self.decrypt_current_supply(aes_key, current_supply_elgamal_keypair)?;

        let new_supply_opening = PedersenOpening::new_rand();
        let new_supply_ciphertext = new_supply_elgamal_keypair
            .pubkey()
            .encrypt_with(current_supply, &new_supply_opening);

        CiphertextCiphertextEqualityProofData::new(
            current_supply_elgamal_keypair,
            new_supply_elgamal_keypair.pubkey(),
            &self
                .current_supply
                .try_into()
                .map_err(|_| TokenError::MalformedCiphertext)?,
            &new_supply_ciphertext,
            &new_supply_opening,
            current_supply,
        )
        .map_err(|_| TokenError::ProofGeneration)
    }
}
