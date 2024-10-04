use {
    crate::errors::TokenProofGenerationError,
    solana_zk_sdk::{
        encryption::{
            elgamal::{ElGamalCiphertext, ElGamalKeypair},
            pedersen::PedersenOpening,
        },
        zk_elgamal_proof_program::proof_data::CiphertextCiphertextEqualityProofData,
    },
};

pub fn supply_elgamal_pubkey_rotation_proof(
    current_supply: u64,
    supply_elgamal_keypair: &ElGamalKeypair,
    new_supply_elgamal_keypair: &ElGamalKeypair,
    current_supply_ciphertext: ElGamalCiphertext,
) -> Result<CiphertextCiphertextEqualityProofData, TokenProofGenerationError> {
    let new_supply_opening = PedersenOpening::new_rand();
    let new_supply_ciphertext = new_supply_elgamal_keypair
        .pubkey()
        .encrypt_with(current_supply, &new_supply_opening);

    Ok(CiphertextCiphertextEqualityProofData::new(
        supply_elgamal_keypair,
        new_supply_elgamal_keypair.pubkey(),
        &current_supply_ciphertext,
        &new_supply_ciphertext,
        &new_supply_opening,
        current_supply,
    )?)
}
