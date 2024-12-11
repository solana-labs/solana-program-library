use {
    super::ConfidentialMintBurn,
    crate::{
        error::TokenError,
        extension::confidential_transfer::{
            ConfidentialTransferAccount, DecryptableBalance, EncryptedBalance,
        },
    },
    bytemuck::{Pod, Zeroable},
    solana_zk_sdk::{
        encryption::{
            auth_encryption::{AeCiphertext, AeKey},
            elgamal::{ElGamalCiphertext, ElGamalKeypair, ElGamalPubkey},
            pedersen::PedersenOpening,
            pod::{
                auth_encryption::PodAeCiphertext,
                elgamal::{PodElGamalCiphertext, PodElGamalPubkey},
            },
        },
        zk_elgamal_proof_program::proof_data::CiphertextCiphertextEqualityProofData,
    },
    spl_token_confidential_transfer_proof_generation::{
        burn::{burn_split_proof_data, BurnProofData},
        mint::{mint_split_proof_data, MintProofData},
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
    /// The supply's ElGamal pubkey
    pub supply_elgamal_pubkey: PodElGamalPubkey,
}

impl SupplyAccountInfo {
    /// Creates a `SupplyAccountInfo` from `ConfidentialMintBurn` extension
    /// account data
    pub fn new(extension: &ConfidentialMintBurn) -> Self {
        Self {
            current_supply: extension.confidential_supply,
            decryptable_supply: extension.decryptable_supply,
            supply_elgamal_pubkey: extension.supply_elgamal_pubkey,
        }
    }

    /// Computes the current supply from the decryptable supply and the
    /// difference between the decryptable supply and the ElGamal encrypted
    /// supply ciphertext
    pub fn decrypted_current_supply(
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
        current_supply_elgamal_keypair: &ElGamalKeypair,
        new_supply_elgamal_keypair: &ElGamalKeypair,
        aes_key: &AeKey,
    ) -> Result<CiphertextCiphertextEqualityProofData, TokenError> {
        let current_supply =
            self.decrypted_current_supply(aes_key, current_supply_elgamal_keypair)?;

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

    /// Create a mint proof data that is split into equality, ciphertext
    /// validity, and range proof.
    pub fn generate_split_mint_proof_data(
        &self,
        mint_amount: u64,
        current_supply: u64,
        supply_elgamal_keypair: &ElGamalKeypair,
        destination_elgamal_pubkey: &ElGamalPubkey,
        auditor_elgamal_pubkey: Option<&ElGamalPubkey>,
    ) -> Result<MintProofData, TokenError> {
        let current_supply_ciphertext = self
            .current_supply
            .try_into()
            .map_err(|_| TokenError::MalformedCiphertext)?;

        mint_split_proof_data(
            &current_supply_ciphertext,
            mint_amount,
            current_supply,
            supply_elgamal_keypair,
            destination_elgamal_pubkey,
            auditor_elgamal_pubkey,
        )
        .map_err(|e| -> TokenError { e.into() })
    }

    /// Compute the new decryptable supply.
    pub fn new_decryptable_supply(
        &self,
        mint_amount: u64,
        elgamal_keypair: &ElGamalKeypair,
        aes_key: &AeKey,
    ) -> Result<AeCiphertext, TokenError> {
        let current_decrypted_supply = self.decrypted_current_supply(aes_key, elgamal_keypair)?;
        let new_decrypted_available_balance = current_decrypted_supply
            .checked_add(mint_amount)
            .ok_or(TokenError::Overflow)?;

        Ok(aes_key.encrypt(new_decrypted_available_balance))
    }
}

/// Confidential Mint Burn extension information needed to construct a
/// `Burn` instruction.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct BurnAccountInfo {
    /// The available balance (encrypted by `encryption_pubkey`)
    pub available_balance: EncryptedBalance,
    /// The decryptable available balance
    pub decryptable_available_balance: DecryptableBalance,
}

impl BurnAccountInfo {
    /// Create the `ApplyPendingBalance` instruction account information from
    /// `ConfidentialTransferAccount`.
    pub fn new(account: &ConfidentialTransferAccount) -> Self {
        Self {
            available_balance: account.available_balance,
            decryptable_available_balance: account.decryptable_available_balance,
        }
    }

    /// Create a burn proof data that is split into equality, ciphertext
    /// validity, and range proof.
    pub fn generate_split_burn_proof_data(
        &self,
        burn_amount: u64,
        source_elgamal_keypair: &ElGamalKeypair,
        aes_key: &AeKey,
        supply_elgamal_pubkey: &ElGamalPubkey,
        auditor_elgamal_pubkey: Option<&ElGamalPubkey>,
    ) -> Result<BurnProofData, TokenError> {
        let current_available_balance_ciphertext = self
            .available_balance
            .try_into()
            .map_err(|_| TokenError::MalformedCiphertext)?;
        let current_decryptable_available_balance = self
            .decryptable_available_balance
            .try_into()
            .map_err(|_| TokenError::MalformedCiphertext)?;

        burn_split_proof_data(
            &current_available_balance_ciphertext,
            &current_decryptable_available_balance,
            burn_amount,
            source_elgamal_keypair,
            aes_key,
            auditor_elgamal_pubkey,
            supply_elgamal_pubkey,
        )
        .map_err(|e| -> TokenError { e.into() })
    }
}
