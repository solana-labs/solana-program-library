use {
    solana_zk_sdk::{
        encryption::{auth_encryption::AeKey, elgamal::ElGamalKeypair},
        zk_elgamal_proof_program::proof_data::ZkProofData,
    },
    spl_token_confidential_transfer_proof_extraction::transfer::TransferProofContext,
    spl_token_confidential_transfer_proof_generation::transfer::transfer_split_proof_data,
};

#[test]
fn test_transfer_correctness() {
    test_proof_validity(0, 0);
    test_proof_validity(1, 0);
    test_proof_validity(1, 1);
    test_proof_validity(65535, 65535); // 2^16 - 1
    test_proof_validity(65536, 65536); // 2^16
    test_proof_validity(281474976710655, 281474976710655); // 2^48 - 1
}

fn test_proof_validity(spendable_balance: u64, transfer_amount: u64) {
    let source_keypair = ElGamalKeypair::new_rand();

    let aes_key = AeKey::new_rand();

    let destination_keypair = ElGamalKeypair::new_rand();
    let destination_pubkey = destination_keypair.pubkey();

    let auditor_keypair = ElGamalKeypair::new_rand();
    let auditor_pubkey = auditor_keypair.pubkey();

    let spendable_ciphertext = source_keypair.pubkey().encrypt(spendable_balance);
    let decryptable_balance = aes_key.encrypt(spendable_balance);

    let (equality_proof_data, validity_proof_data, range_proof_data) = transfer_split_proof_data(
        &spendable_ciphertext,
        &decryptable_balance,
        transfer_amount,
        &source_keypair,
        &aes_key,
        destination_pubkey,
        Some(auditor_pubkey),
    )
    .unwrap();

    equality_proof_data.verify_proof().unwrap();
    validity_proof_data.verify_proof().unwrap();
    range_proof_data.verify_proof().unwrap();

    TransferProofContext::verify_and_extract(
        equality_proof_data.context_data(),
        validity_proof_data.context_data(),
        range_proof_data.context_data(),
    )
    .unwrap();
}
