#![allow(missing_docs)]

use bincode::{rustc_serialize::encode, SizeLimit::Infinite};
use bn::{arith::U256, AffineG1, Fr, Group, G1};
use sha3::{Digest, Keccak256};
use solana_sdk::program_error::ProgramError;
use spl_token::pack::{IsInitialized, Pack, Sealed};

type Points = (G1, G1);

struct EncryptedAggregate {
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

fn keccak256(
    plaintext: G1,
    (ctxt_1, ctxt_2): Points,
    announcement_g: G1,
    announcement_ctx: G1,
    generator: G1,
    public_key: G1,
) -> Fr {
    let plaintext = AffineG1::from_jacobian(plaintext).unwrap();
    let ctxt_1 = AffineG1::from_jacobian(ctxt_1).unwrap();
    let ctxt_2 = AffineG1::from_jacobian(ctxt_2).unwrap();
    let announcement_g = AffineG1::from_jacobian(announcement_g).unwrap();
    let announcement_ctx = AffineG1::from_jacobian(announcement_ctx).unwrap();
    let generator = AffineG1::from_jacobian(generator).unwrap();
    let public_key = AffineG1::from_jacobian(public_key).unwrap();
    let hasher = Keccak256::new()
        .chain(encode(&plaintext, Infinite).unwrap())
        .chain(encode(&ctxt_1, Infinite).unwrap())
        .chain(encode(&ctxt_2, Infinite).unwrap())
        .chain(encode(&announcement_g, Infinite).unwrap())
        .chain(encode(&announcement_ctx, Infinite).unwrap())
        .chain(encode(&generator, Infinite).unwrap())
        .chain(encode(&public_key, Infinite).unwrap());

    let result = hasher.finalize();
    Fr::new_mul_factor(U256::from_slice(result.as_slice()).unwrap())
}

fn check_proof(
    (ctxt_1, ctxt_2): Points,
    plaintext: G1,
    public_key: G1,
    announcement_g: G1,
    announcement_ctx: G1,
    response: Fr,
) -> bool {
    let generator = G1::one();
    let challenge = keccak256(
        plaintext,
        (ctxt_1, ctxt_2),
        announcement_g,
        announcement_ctx,
        generator,
        public_key,
    );

    let check_1 = generator * response == announcement_g + public_key * challenge;
    let check_2 =
        ctxt_1 * response + plaintext * challenge == announcement_ctx + ctxt_2 * challenge;
    check_1 && check_2
}

#[derive(Default)]
pub struct User {
    encrypted_aggregate: EncryptedAggregate,
    is_initialized: bool,
    proof_verification: bool,
    payment_requests: Vec<PaymentRequests>,
}

impl Sealed for User {}
impl IsInitialized for User {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for User {
    const LEN: usize = 42;
    fn unpack_from_slice(_src: &[u8]) -> Result<Self, ProgramError> {
        todo!()
    }

    fn pack_into_slice(&self, _dst: &mut [u8]) {
        todo!();
    }
}

impl User {
    pub fn unpack(_input: &[u8]) -> Result<Self, ProgramError> {
        Ok(Self::default())
    }

    pub fn pack(_input: &mut [u8]) -> Result<(), ProgramError> {
        Ok(())
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
        let ciphertext = inner_product(&ciphertexts, &policies);
        self.encrypted_aggregate = EncryptedAggregate {
            ciphertext,
            public_key,
        };
        true
    }

    pub fn submit_proof_decryption(
        &mut self,
        plaintext: G1,
        announcement_g: G1,
        announcement_ctx: G1,
        response: Fr,
    ) -> bool {
        let ciphertext = self.fetch_encrypted_aggregate();
        let client_pk = self.fetch_public_key();
        self.proof_verification = check_proof(
            ciphertext,
            plaintext,
            client_pk,
            announcement_g,
            announcement_ctx,
            response,
        );
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

#[derive(Default)]
pub struct Policies {
    pub policies: Vec<Fr>
}

impl Sealed for Policies {}
impl IsInitialized for Policies {
    fn is_initialized(&self) -> bool {
        !self.policies.is_empty()
    }
}

impl Pack for Policies {
    const LEN: usize = 42;
    fn unpack_from_slice(_src: &[u8]) -> Result<Self, ProgramError> {
        todo!()
    }

    fn pack_into_slice(&self, _dst: &mut [u8]) {
        todo!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bn::{Fr, Group, G1};
    use elgamal_bn::{ciphertext::Ciphertext, private::SecretKey, public::PublicKey};
    use rand::thread_rng;

    pub(crate) fn generate_keys() -> (SecretKey, PublicKey) {
        let mut csprng = thread_rng();
        let sk = SecretKey::new(&mut csprng);
        let pk = PublicKey::from(&sk);
        (sk, pk)
    }

    pub(crate) fn recover_scalar(point: G1, k: u32) -> Fr {
        for i in 0..2u64.pow(k) {
            let scalar = Fr::new(i.into()).unwrap_or_else(Fr::one);
            if G1::one() * scalar == point {
                return scalar;
            }
        }
        panic!("Encryped scalar too long");
    }

    fn test_policy_contract(policies: &[U256], expected_scalar_aggregate: Fr) {
        let (sk, pk) = generate_keys();
        let interactions: Vec<_> = policies
            .iter()
            .map(|_| pk.encrypt(&G1::one()).points)
            .collect();
        let mut user = User::default();

        let policies: Vec<_> = policies.iter().map(|x| Fr::new_mul_factor(*x)).collect();
        let tx_receipt = user.calculate_aggregate(&interactions, pk.get_point(), &policies);
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
        let policies = vec![1.into(), 2.into()];
        test_policy_contract(&policies, Fr::new(3.into()).unwrap());
    }

    #[test]
    fn test_policy_contract_128ads() {
        let policies = vec![
            1.into(),
            1.into(),
            1.into(),
            1.into(),
            1.into(),
            1.into(),
            1.into(),
            1.into(),
            1.into(),
            1.into(), //10
            2.into(),
            2.into(),
            2.into(),
            2.into(),
            2.into(),
            2.into(),
            2.into(),
            2.into(),
            2.into(),
            2.into(), // 2 * 10
            1.into(),
            1.into(),
            1.into(),
            1.into(),
            1.into(),
            1.into(),
            1.into(),
            1.into(),
            1.into(),
            1.into(), //10
            2.into(),
            2.into(),
            2.into(),
            2.into(),
            2.into(),
            2.into(),
            2.into(),
            2.into(),
            2.into(),
            2.into(), // 2 * 10
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
        ];
        test_policy_contract(&policies, Fr::new(60.into()).unwrap());
    }
}
