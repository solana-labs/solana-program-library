use solana_zk_sdk::encryption::{
    elgamal::{DecryptHandle, ElGamalPubkey},
    grouped_elgamal::{GroupedElGamal, GroupedElGamalCiphertext},
    pedersen::{PedersenCommitment, PedersenOpening},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct TransferAmountCiphertext(pub(crate) GroupedElGamalCiphertext<3>);

impl TransferAmountCiphertext {
    pub fn new(
        amount: u64,
        source_pubkey: &ElGamalPubkey,
        destination_pubkey: &ElGamalPubkey,
        auditor_pubkey: &ElGamalPubkey,
    ) -> (Self, PedersenOpening) {
        let opening = PedersenOpening::new_rand();
        let grouped_ciphertext = GroupedElGamal::<3>::encrypt_with(
            [source_pubkey, destination_pubkey, auditor_pubkey],
            amount,
            &opening,
        );

        (Self(grouped_ciphertext), opening)
    }

    pub fn get_commitment(&self) -> &PedersenCommitment {
        &self.0.commitment
    }

    pub fn get_source_handle(&self) -> &DecryptHandle {
        // `TransferAmountCiphertext` is a wrapper for `GroupedElGamalCiphertext<3>`,
        // which holds exactly three decryption handles.
        self.0.handles.first().unwrap()
    }

    pub fn get_destination_handle(&self) -> &DecryptHandle {
        // `TransferAmountCiphertext` is a wrapper for `GroupedElGamalCiphertext<3>`,
        // which holds exactly three decryption handles.
        self.0.handles.get(1).unwrap()
    }

    pub fn get_auditor_handle(&self) -> &DecryptHandle {
        // `TransferAmountCiphertext` is a wrapper for `GroupedElGamalCiphertext<3>`,
        // which holds exactly three decryption handles.
        self.0.handles.get(2).unwrap()
    }
}
