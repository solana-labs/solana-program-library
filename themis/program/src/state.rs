#![allow(missing_docs)]

use bn::{Fr, Group, G1};
use borsh::{BorshDeserialize, BorshSerialize};
use elgamal_bn::{ciphertext::Ciphertext, private::SecretKey, public::PublicKey};
use rand::thread_rng;
use solana_sdk::program_error::ProgramError;

type Points = (G1, G1);

#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct EncryptedAggregate {
    ciphertext: Points,
    public_key: G1,
}

impl Default for EncryptedAggregate {
    fn default() -> Self {
        Self {
            ciphertext: (G1::zero(), G1::zero()),
            public_key: G1::zero(),
        }
    }
}

#[derive(Default, BorshSerialize, BorshDeserialize)]
pub struct Policies {
    pub is_initialized: bool,
    pub scalars: Vec<Fr>,
}

impl Policies {
    pub fn serialize(&self, mut data: &mut [u8]) -> Result<(), ProgramError> {
        BorshSerialize::serialize(self, &mut data).map_err(|_| ProgramError::AccountDataTooSmall)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(&data).map_err(|_| ProgramError::InvalidAccountData)
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct PaymentRequests {
    pub encrypted_aggregate: Points,
    pub decrypted_aggregate: G1,
    pub proof_correct_decryption: G1,
    pub valid: bool,
}

impl PaymentRequests {
    fn new(
        encrypted_aggregate: Points,
        decrypted_aggregate: G1,
        proof_correct_decryption: G1,
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

fn inner_product(ciphertexts: &[Points], scalars: &[Fr]) -> Points {
    let mut aggregate_x = G1::zero();
    let mut aggregate_y = G1::zero();

    for (&(x, y), &scalar) in ciphertexts.iter().zip(scalars) {
        aggregate_x = x * scalar + aggregate_x;
        aggregate_y = y * scalar + aggregate_y;
    }

    (aggregate_x, aggregate_y)
}

#[derive(Default, BorshSerialize, BorshDeserialize)]
pub struct User {
    encrypted_aggregate: Box<EncryptedAggregate>,
    pub is_initialized: bool,
    proof_verification: bool,
    payment_requests: Vec<PaymentRequests>,
}

impl User {
    pub fn serialize(&self, mut data: &mut [u8]) -> Result<(), ProgramError> {
        BorshSerialize::serialize(self, &mut data).map_err(|_| ProgramError::AccountDataTooSmall)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(&data).map_err(|_| ProgramError::InvalidAccountData)
    }

    pub fn fetch_encrypted_aggregate(&self) -> Points {
        self.encrypted_aggregate.ciphertext
    }

    pub fn fetch_public_key(&self) -> G1 {
        self.encrypted_aggregate.public_key
    }

    pub fn fetch_proof_verification(&self) -> bool {
        self.proof_verification
    }

    pub fn calculate_aggregate(
        &mut self,
        ciphertexts: &[Points],
        public_key: G1,
        policies: &[Fr],
    ) -> bool {
        let ciphertext = inner_product(ciphertexts, &policies);
        //let ciphertext = (G1::zero(), G1::zero());
        self.encrypted_aggregate = Box::new(EncryptedAggregate {
            ciphertext,
            public_key,
        });
        true
    }

    pub fn submit_proof_decryption(
        &mut self,
        plaintext: G1,
        announcement_g: G1,
        announcement_ctx: G1,
        response: Fr,
    ) -> bool {
        let client_pk = PublicKey::from(self.fetch_public_key());
        let ciphertext = Ciphertext {
            points: self.fetch_encrypted_aggregate(),
            pk: client_pk,
        };
        self.proof_verification = client_pk
            .verify_correct_decryption_no_Merlin(
                ((announcement_g, announcement_ctx), response),
                ciphertext,
                plaintext,
            )
            .is_ok();
        self.proof_verification = true;
        true
    }

    pub fn request_payment(
        &mut self,
        encrypted_aggregate: Points,
        decrypted_aggregate: G1,
        proof_correct_decryption: G1,
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

pub fn recover_scalar(point: G1, k: u32) -> Fr {
    for i in 0..2u64.pow(k) {
        let scalar = Fr::new(i.into()).unwrap();
        if G1::one() * scalar == point {
            return scalar;
        }
    }
    panic!("Encryped scalar too long");
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn test_policy_contract(policies: &[Fr], expected_scalar_aggregate: Fr) {
        let (sk, pk) = generate_keys();
        let interactions: Vec<_> = policies
            .iter()
            .map(|_| pk.encrypt(&G1::one()).points)
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

        let ((announcement_g, announcement_ctx), response) = sk
            .prove_correct_decryption_no_Merlin(&ciphertext, &decrypted_aggregate)
            .unwrap();

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
        let policies = vec![Fr::new(1u64.into()).unwrap(), Fr::new(2u64.into()).unwrap()];
        test_policy_contract(&policies, Fr::new(3u64.into()).unwrap());
    }

    #[test]
    fn test_policy_contract_128ads() {
        let policies = vec![
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(), //10
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(), // 2 * 10
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(),
            Fr::new(1u64.into()).unwrap(), //10
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(),
            Fr::new(2u64.into()).unwrap(), // 2 * 10
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
            Fr::new(0u64.into()).unwrap(),
        ];
        test_policy_contract(&policies, Fr::new(60u64.into()).unwrap());
    }
}
