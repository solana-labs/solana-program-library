#![allow(missing_docs)]

use bincode::{deserialize, serialize_into};
use curve25519_dalek::{
    constants::RISTRETTO_BASEPOINT_POINT,
    ristretto::{CompressedRistretto, RistrettoPoint},
    scalar::Scalar,
    traits::Identity,
};
use elgamal_ristretto::{ciphertext::Ciphertext, public::PublicKey, private::SecretKey};
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use solana_sdk::program_error::ProgramError;

type Points = (RistrettoPoint, RistrettoPoint);

#[derive(Serialize, Deserialize)]
struct EncryptedAggregate {
    ciphertext: Points,
    public_key: RistrettoPoint,
}

impl Default for EncryptedAggregate {
    fn default() -> Self {
        Self {
            ciphertext: (RistrettoPoint::identity(), RistrettoPoint::identity()),
            public_key: RistrettoPoint::identity(),
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct Policies {
    pub is_initialized: bool,
    pub scalars: Vec<Scalar>,
}

impl Policies {
    pub fn serialize(&self, data: &mut [u8]) -> Result<(), ProgramError> {
        serialize_into(data, &self).map_err(|_| ProgramError::AccountDataTooSmall)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, ProgramError> {
        deserialize(data).map_err(|_| ProgramError::InvalidAccountData)
    }
}

#[derive(Serialize, Deserialize)]
pub struct PaymentRequests {
    pub encrypted_aggregate: Points,
    pub decrypted_aggregate: RistrettoPoint,
    pub proof_correct_decryption: RistrettoPoint,
    pub valid: bool,
}

impl PaymentRequests {
    fn new(
        encrypted_aggregate: Points,
        decrypted_aggregate: RistrettoPoint,
        proof_correct_decryption: RistrettoPoint,
        valid: bool,
    ) -> Self {
        Self {
            encrypted_aggregate,
            decrypted_aggregate,
            proof_correct_decryption,
            valid,
        }
    }
}

fn inner_product(ciphertexts: &[Points], scalars: &[Scalar]) -> Points {
    let mut aggregate_x = RistrettoPoint::identity();
    let mut aggregate_y = RistrettoPoint::identity();

    for (&(x, y), &scalar) in ciphertexts.iter().zip(scalars) {
        aggregate_x = x * scalar + aggregate_x;
        aggregate_y = y * scalar + aggregate_y;
    }

    (aggregate_x, aggregate_y)
}

#[derive(Default, Serialize, Deserialize)]
pub struct User {
    encrypted_aggregate: EncryptedAggregate,
    pub is_initialized: bool,
    proof_verification: bool,
    payment_requests: Vec<PaymentRequests>,
}

impl User {
    pub fn serialize(&self, data: &mut [u8]) -> Result<(), ProgramError> {
        serialize_into(data, &self).map_err(|_| ProgramError::AccountDataTooSmall)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, ProgramError> {
        deserialize(data).map_err(|_| ProgramError::InvalidAccountData)
    }

    pub fn fetch_encrypted_aggregate(&self) -> Points {
        self.encrypted_aggregate.ciphertext
    }

    pub fn fetch_public_key(&self) -> RistrettoPoint {
        self.encrypted_aggregate.public_key
    }

    pub fn fetch_proof_verification(&self) -> bool {
        self.proof_verification
    }

    pub fn calculate_aggregate(
        &mut self,
        ciphertexts: &[Points],
        public_key: RistrettoPoint,
        policies: &[Scalar],
    ) -> bool {
        let ciphertext = inner_product(ciphertexts, &policies);
        self.encrypted_aggregate = EncryptedAggregate {
            ciphertext,
            public_key,
        };
        true
    }

    pub fn submit_proof_decryption(
        &mut self,
        plaintext: RistrettoPoint,
        announcement_g: CompressedRistretto,
        announcement_ctx: CompressedRistretto,
        response: Scalar,
    ) -> bool {
        let client_pk = PublicKey::from(self.fetch_public_key());
        let ciphertext = Ciphertext {
            points: self.fetch_encrypted_aggregate(),
            pk: client_pk,
        };
        self.proof_verification = client_pk.verify_correct_decryption_no_Merlin(
            &((announcement_g, announcement_ctx), response),
            &ciphertext,
            &plaintext,
        );
        true
    }

    pub fn request_payment(
        &mut self,
        encrypted_aggregate: Points,
        decrypted_aggregate: RistrettoPoint,
        proof_correct_decryption: RistrettoPoint,
    ) -> bool {
        // TODO: implement proof verification
        let proof_is_valid = true;
        let payment_request = PaymentRequests::new(
            encrypted_aggregate,
            decrypted_aggregate,
            proof_correct_decryption,
            proof_is_valid,
        );
        self.payment_requests.push(payment_request);
        proof_is_valid
    }
}

pub fn generate_keys() -> (SecretKey, PublicKey) {
    let mut csprng = thread_rng();
    let sk = SecretKey::new(&mut csprng);
    let pk = PublicKey::from(&sk);
    (sk, pk)
}

pub fn recover_scalar(point: RistrettoPoint, k: u32) -> Scalar {
    for i in 0..2u64.pow(k) {
        let scalar = i.into();
        if RISTRETTO_BASEPOINT_POINT * scalar == point {
            return scalar;
        }
    }
    panic!("Encryped scalar too long");
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn test_policy_contract(policies: &[Scalar], expected_scalar_aggregate: Scalar) {
        let (sk, pk) = generate_keys();
        let interactions: Vec<_> = policies
            .iter()
            .map(|_| pk.encrypt(&RISTRETTO_BASEPOINT_POINT).points)
            .collect();
        let mut user = User::default();

        let tx_receipt = user.calculate_aggregate(&interactions, pk.get_point(), policies);
        assert!(tx_receipt);

        let encrypted_point = user.fetch_encrypted_aggregate();
        let ciphertext = Ciphertext {
            points: encrypted_point,
            pk,
        };

        let decrypted_aggregate = sk.decrypt(&ciphertext);
        let scalar_aggregate = recover_scalar(decrypted_aggregate, 16);
        assert_eq!(scalar_aggregate, expected_scalar_aggregate);

        let ((announcement_g, announcement_ctx), response) =
            sk.prove_correct_decryption_no_Merlin(&ciphertext, &decrypted_aggregate);

        let tx_receipt_proof = user.submit_proof_decryption(
            decrypted_aggregate,
            announcement_g,
            announcement_ctx,
            response,
        );
        assert!(tx_receipt_proof);

        let proof_result = user.fetch_proof_verification();
        assert!(proof_result);
    }

    #[test]
    fn test_policy_contract_2ads() {
        let policies = vec![1u8.into(), 2u8.into()];
        test_policy_contract(&policies, 3u8.into());
    }

    #[test]
    fn test_policy_contract_128ads() {
        let policies = vec![
            1u8.into(),
            1u8.into(),
            1u8.into(),
            1u8.into(),
            1u8.into(),
            1u8.into(),
            1u8.into(),
            1u8.into(),
            1u8.into(),
            1u8.into(), //10
            2u8.into(),
            2u8.into(),
            2u8.into(),
            2u8.into(),
            2u8.into(),
            2u8.into(),
            2u8.into(),
            2u8.into(),
            2u8.into(),
            2u8.into(), // 2 * 10
            1u8.into(),
            1u8.into(),
            1u8.into(),
            1u8.into(),
            1u8.into(),
            1u8.into(),
            1u8.into(),
            1u8.into(),
            1u8.into(),
            1u8.into(), //10
            2u8.into(),
            2u8.into(),
            2u8.into(),
            2u8.into(),
            2u8.into(),
            2u8.into(),
            2u8.into(),
            2u8.into(),
            2u8.into(),
            2u8.into(), // 2 * 10
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
            0u8.into(),
        ];
        test_policy_contract(&policies, 60u8.into());
    }
}
