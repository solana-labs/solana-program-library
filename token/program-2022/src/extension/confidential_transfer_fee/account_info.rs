use {
    crate::{error::TokenError, extension::confidential_transfer_fee::EncryptedWithheldAmount},
    bytemuck::{Pod, Zeroable},
    solana_zk_sdk::{
        encryption::{
            elgamal::{ElGamalCiphertext, ElGamalKeypair, ElGamalPubkey},
            pedersen::PedersenOpening,
        },
        zk_elgamal_proof_program::proof_data::ciphertext_ciphertext_equality::CiphertextCiphertextEqualityProofData,
    },
};

/// Confidential transfer fee extension information needed to construct a
/// `WithdrawWithheldTokensFromMint` or `WithdrawWithheldTokensFromAccounts`
/// instruction.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct WithheldTokensInfo {
    /// The available balance
    pub(crate) withheld_amount: EncryptedWithheldAmount,
}
impl WithheldTokensInfo {
    /// Create a `WithheldTokensInfo` from an ElGamal ciphertext.
    pub fn new(withheld_amount: &EncryptedWithheldAmount) -> Self {
        Self {
            withheld_amount: *withheld_amount,
        }
    }

    /// Create withdraw withheld proof data.
    pub fn generate_proof_data(
        &self,
        withdraw_withheld_authority_elgamal_keypair: &ElGamalKeypair,
        destination_elgamal_pubkey: &ElGamalPubkey,
    ) -> Result<CiphertextCiphertextEqualityProofData, TokenError> {
        let withheld_amount_in_mint: ElGamalCiphertext = self
            .withheld_amount
            .try_into()
            .map_err(|_| TokenError::AccountDecryption)?;

        let decrypted_withheld_amount_in_mint = withheld_amount_in_mint
            .decrypt_u32(withdraw_withheld_authority_elgamal_keypair.secret())
            .ok_or(TokenError::AccountDecryption)?;

        let destination_opening = PedersenOpening::new_rand();

        let destination_ciphertext = destination_elgamal_pubkey
            .encrypt_with(decrypted_withheld_amount_in_mint, &destination_opening);

        CiphertextCiphertextEqualityProofData::new(
            withdraw_withheld_authority_elgamal_keypair,
            destination_elgamal_pubkey,
            &withheld_amount_in_mint,
            &destination_ciphertext,
            &destination_opening,
            decrypted_withheld_amount_in_mint,
        )
        .map_err(|_| TokenError::ProofGeneration)
    }
}
