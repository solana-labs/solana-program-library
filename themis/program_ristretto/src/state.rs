#![allow(missing_docs)]

use borsh::{BorshDeserialize, BorshSerialize};
use curve25519_dalek::{
    constants::RISTRETTO_BASEPOINT_POINT, ristretto::RistrettoPoint, scalar::Scalar,
    traits::Identity,
};
use elgamal_ristretto::{
    ciphertext::Ciphertext, multiply::ristretto_mul, private::SecretKey, public::PublicKey,
};
use rand::thread_rng;
use solana_program::program_error::ProgramError;

type Points = (RistrettoPoint, RistrettoPoint);

#[derive(Default, BorshSerialize, BorshDeserialize)]
pub struct Policies {
    pub is_initialized: bool,
    pub num_scalars: u8,
    pub scalars: Vec<Scalar>,
}

impl Policies {
    pub fn serialize(&self, mut data: &mut [u8]) -> Result<(), ProgramError> {
        BorshSerialize::serialize(self, &mut data).map_err(|_| ProgramError::AccountDataTooSmall)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(&data).map_err(|_| ProgramError::InvalidAccountData)
    }

    pub fn new(num_scalars: u8) -> Self {
        Self {
            is_initialized: true,
            num_scalars,
            scalars: vec![Scalar::zero(); num_scalars as usize],
        }
    }

    /// Useful for testing
    pub fn new_with_scalars(scalars: Vec<Scalar>) -> Self {
        let mut policies = Self::new(scalars.len() as u8);
        policies.scalars = scalars;
        policies
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct PaymentRequest {
    pub encrypted_aggregate: Points,
    pub decrypted_aggregate: RistrettoPoint,
    pub proof_correct_decryption: RistrettoPoint,
    pub valid: bool,
}

impl PaymentRequest {
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

fn inner_product(
    (mut aggregate_x, mut aggregate_y): Points,
    ciphertexts: &[(u8, Points)],
    scalars: &[Scalar],
) -> Points {
    for &(i, (x, y)) in ciphertexts {
        aggregate_x = ristretto_mul(&x, &scalars[i as usize]).unwrap() + aggregate_x;
        aggregate_y = ristretto_mul(&y, &scalars[i as usize]).unwrap() + aggregate_y;
    }

    (aggregate_x, aggregate_y)
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct User {
    encrypted_aggregate: Points,
    public_key: PublicKey,
    pub is_initialized: bool,
    proof_verification: bool,
    payment_requests: Vec<PaymentRequest>,
}

impl Default for User {
    fn default() -> Self {
        Self {
            encrypted_aggregate: (RistrettoPoint::identity(), RistrettoPoint::identity()),
            public_key: PublicKey::from(RistrettoPoint::identity()),
            is_initialized: false,
            proof_verification: false,
            payment_requests: vec![],
        }
    }
}

impl User {
    pub fn serialize(&self, mut data: &mut [u8]) -> Result<(), ProgramError> {
        BorshSerialize::serialize(self, &mut data).map_err(|_| ProgramError::AccountDataTooSmall)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(&data).map_err(|_| ProgramError::InvalidAccountData)
    }

    pub fn new(public_key: PublicKey) -> Self {
        Self {
            public_key,
            ..Self::default()
        }
    }

    pub fn fetch_encrypted_aggregate(&self) -> Points {
        self.encrypted_aggregate
    }

    pub fn fetch_public_key(&self) -> PublicKey {
        self.public_key
    }

    pub fn fetch_proof_verification(&self) -> bool {
        self.proof_verification
    }

    pub fn submit_interactions(
        &mut self,
        interactions: &[(u8, Points)],
        policies: &[Scalar],
    ) -> bool {
        self.encrypted_aggregate = inner_product(self.encrypted_aggregate, interactions, &policies);
        true
    }

    pub fn submit_proof_decryption(
        &mut self,
        plaintext: RistrettoPoint,
        announcement_g: RistrettoPoint,
        announcement_ctx: RistrettoPoint,
        response: Scalar,
    ) -> bool {
        let client_pk = self.fetch_public_key();
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
        let payment_request = PaymentRequest::new(
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
    panic!("Encrypted scalar too long");
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn test_policy_contract(policies: &[Scalar], expected_scalar_aggregate: Scalar) {
        let (sk, pk) = generate_keys();
        let interactions: Vec<_> = (0..policies.len())
            .map(|i| (i as u8, pk.encrypt(&RISTRETTO_BASEPOINT_POINT).points))
            .collect();
        let mut user = User::new(pk);

        let tx_receipt = user.submit_interactions(&interactions, policies);
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
        let policies = vec![1u64.into(), 2u64.into()];
        test_policy_contract(&policies, 3u64.into());
    }

    #[test]
    fn test_policy_contract_128ads() {
        let policies = vec![
            1u64.into(),
            1u64.into(),
            1u64.into(),
            1u64.into(),
            1u64.into(),
            1u64.into(),
            1u64.into(),
            1u64.into(),
            1u64.into(),
            1u64.into(), //10
            2u64.into(),
            2u64.into(),
            2u64.into(),
            2u64.into(),
            2u64.into(),
            2u64.into(),
            2u64.into(),
            2u64.into(),
            2u64.into(),
            2u64.into(), // 2 * 10
            1u64.into(),
            1u64.into(),
            1u64.into(),
            1u64.into(),
            1u64.into(),
            1u64.into(),
            1u64.into(),
            1u64.into(),
            1u64.into(),
            1u64.into(), //10
            2u64.into(),
            2u64.into(),
            2u64.into(),
            2u64.into(),
            2u64.into(),
            2u64.into(),
            2u64.into(),
            2u64.into(),
            2u64.into(),
            2u64.into(), // 2 * 10
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
            0u64.into(),
        ];
        test_policy_contract(&policies, 60u64.into());
    }
}
