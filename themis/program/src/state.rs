#![allow(missing_docs)]

use bincode::{rustc_serialize::encode, SizeLimit::Infinite};
use bn::{AffineG1, Fq, Fr, Group, G1};
use primitive_types::U256;
use sha3::{Digest, Keccak256};
use std::collections::HashMap;

struct EncryptedAggregate {
    x0: U256,
    x1: U256,
    y0: U256,
    y1: U256,
    public_key: [U256; 2],
}

pub struct PaymentRequests {
    pub encrypted_aggregate: [U256; 4],
    pub decrypted_aggregate: [U256; 2],
    pub proof_correct_decryption: [U256; 2],
    pub valid: bool,
}

impl PaymentRequests {
    fn new(
        encrypted_aggregate: [U256; 4],
        decrypted_aggregate: [U256; 2],
        proof_correct_decryption: [U256; 2],
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

struct EcPoint {
    x_coord: U256,
    y_coord: U256,
}

impl EcPoint {
    fn new(x_coord: U256, y_coord: U256) -> Self {
        Self { x_coord, y_coord }
    }
}

struct Ciphertext {
    point1: EcPoint,
    point2: EcPoint,
}

impl Ciphertext {
    fn new(point1: EcPoint, point2: EcPoint) -> Self {
        Self { point1, point2 }
    }
}

fn primtypes_u256(n: bn::arith::U256) -> U256 {
    let mut buf = [0u8; 32];
    n.to_big_endian(&mut buf).unwrap();
    U256::from_big_endian(&buf)
}

fn bn_u256(n: U256) -> bn::arith::U256 {
    let mut buf = [0u8; 32];
    n.to_big_endian(&mut buf);
    bn::arith::U256::from_slice(&buf).unwrap()
}

fn read_point(input: &[U256]) -> ::bn::G1 {
    let px = Fq::from_u256(bn_u256(input[0])).unwrap();
    let py = Fq::from_u256(bn_u256(input[1])).unwrap();
    if px == Fq::zero() && py == Fq::zero() {
        G1::zero()
    } else {
        AffineG1::new(px, py).unwrap().into()
    }
}

// Can fail if any of the 2 points does not belong the bn128 curve
fn bn128_add(input: [U256; 4]) -> [U256; 2] {
    let p1 = read_point(&input[0..2]);
    let p2 = read_point(&input[2..4]);

    if let Some(sum) = AffineG1::from_jacobian(p1 + p2) {
        // point not at infinity
        [
            primtypes_u256(sum.x().into_u256()),
            primtypes_u256(sum.y().into_u256()),
        ]
    } else {
        eprintln!("bn128_add: infinity");
        [U256::from(0), U256::from(0)]
    }
}

// Can fail if first paramter (bn128 curve point) does not actually belong to the curve
fn bn128_multiply(input: [U256; 3]) -> [U256; 2] {
    let p = read_point(&input[0..2]);
    let fr = Fr::new_mul_factor(bn_u256(input[2]));

    if let Some(sum) = AffineG1::from_jacobian(p * fr) {
        // point not at infinity
        [
            primtypes_u256(sum.x().into_u256()),
            primtypes_u256(sum.y().into_u256()),
        ]
    } else {
        eprintln!("bn128_multiply: infinity");
        [U256::from(0), U256::from(0)]
    }
}

fn inner_product(ciphertext_vector: Vec<[U256; 4]>, scalar_vector: &[U256]) -> [U256; 4] {
    let mut aggregate_1 = [U256::from(0), U256::from(0)];
    let mut aggregate_2 = [U256::from(0), U256::from(0)];

    for i in 0..scalar_vector.len() {
        let ciphertext = ciphertext_vector[i];
        let scalar = scalar_vector[i];
        let result_mult_1 = bn128_multiply([ciphertext[0], ciphertext[1], scalar]);
        let result_mult_2 = bn128_multiply([ciphertext[2], ciphertext[3], scalar]);
        aggregate_1 = bn128_add([
            result_mult_1[0],
            result_mult_1[1],
            aggregate_1[0],
            aggregate_1[1],
        ]);
        aggregate_2 = bn128_add([
            result_mult_2[0],
            result_mult_2[1],
            aggregate_2[0],
            aggregate_2[1],
        ]);
    }

    [
        aggregate_1[0],
        aggregate_1[1],
        aggregate_2[0],
        aggregate_2[1],
    ]
}

// TODO: only for checking
pub fn add_points_and_check(input: &[&[U256]]) -> Vec<U256> {
    let addition = bn128_add([input[0][0], input[0][1], input[1][0], input[1][1]]);

    if !(addition[0] == input[2][0] && addition[1] == input[2][1]) {
        panic!("equality failed");
    }

    vec![input[0][0], input[0][1]]
}

fn proof_check_1(
    public_key: [U256; 2],
    announcement_g: [U256; 2],
    challenge: U256,
    response: U256,
) -> bool {
    let pk = EcPoint::new(public_key[0], public_key[1]);
    let generator = EcPoint::new(U256::from(1), U256::from(2));
    let lhs_check_1 = bn128_multiply([generator.x_coord, generator.y_coord, response]);
    let pk_times_challenge = bn128_multiply([pk.x_coord, pk.y_coord, challenge]);
    let rhs_check_1 = bn128_add([
        announcement_g[0],
        announcement_g[1],
        pk_times_challenge[0],
        pk_times_challenge[1],
    ]);
    lhs_check_1[0] == rhs_check_1[0] && lhs_check_1[1] == rhs_check_1[1]
}

fn proof_check_2(
    ciphertext: [U256; 4],
    plaintext: [U256; 2],
    announcement_ctx: [U256; 2],
    challenge: U256,
    response: U256,
) -> bool {
    let ctxt = Ciphertext::new(
        EcPoint::new(ciphertext[0], ciphertext[1]),
        EcPoint::new(ciphertext[2], ciphertext[3]),
    );
    let ptxt = EcPoint::new(plaintext[0], plaintext[1]);
    let lhs_mult_1_check_2 = bn128_multiply([ctxt.point1.x_coord, ctxt.point1.y_coord, response]);

    // the following, in the original check is computed in the rhs. We do it in the lhs for
    // simplicity.
    let lhs_mult_2_check_2 = bn128_multiply([ptxt.x_coord, ptxt.y_coord, challenge]);
    let lhs_check_2 = bn128_add([
        lhs_mult_1_check_2[0],
        lhs_mult_1_check_2[1],
        lhs_mult_2_check_2[0],
        lhs_mult_2_check_2[1],
    ]);

    let rhs_mult_check_2 = bn128_multiply([ctxt.point2.x_coord, ctxt.point2.y_coord, challenge]);
    let rhs_check_2 = bn128_add([
        announcement_ctx[0],
        announcement_ctx[1],
        rhs_mult_check_2[0],
        rhs_mult_check_2[1],
    ]);

    lhs_check_2[0] == rhs_check_2[0] && lhs_check_2[1] == rhs_check_2[1]
}

fn keccak256(
    plaintext: [U256; 2],
    ciphertext: [U256; 4],
    announcement_g: [U256; 2],
    announcement_ctx: [U256; 2],
    generator: [U256; 2],
    public_key: [U256; 2],
) -> U256 {
    let hasher = Keccak256::new()
        .chain(encode(&bn_u256(plaintext[0]), Infinite).unwrap())
        .chain(encode(&bn_u256(plaintext[1]), Infinite).unwrap())
        .chain(encode(&bn_u256(ciphertext[0]), Infinite).unwrap())
        .chain(encode(&bn_u256(ciphertext[1]), Infinite).unwrap())
        .chain(encode(&bn_u256(ciphertext[2]), Infinite).unwrap())
        .chain(encode(&bn_u256(ciphertext[3]), Infinite).unwrap())
        .chain(encode(&bn_u256(announcement_g[0]), Infinite).unwrap())
        .chain(encode(&bn_u256(announcement_g[1]), Infinite).unwrap())
        .chain(encode(&bn_u256(announcement_ctx[0]), Infinite).unwrap())
        .chain(encode(&bn_u256(announcement_ctx[1]), Infinite).unwrap())
        .chain(encode(&bn_u256(generator[0]), Infinite).unwrap())
        .chain(encode(&bn_u256(generator[1]), Infinite).unwrap())
        .chain(encode(&bn_u256(public_key[0]), Infinite).unwrap())
        .chain(encode(&bn_u256(public_key[1]), Infinite).unwrap());

    let result = hasher.finalize();
    U256::from(result.as_slice())
}

fn check_proof(
    ciphertext: [U256; 4],
    plaintext: [U256; 2],
    public_key: [U256; 2],
    announcement_g: [U256; 2],
    announcement_ctx: [U256; 2],
    response: U256,
) -> bool {
    let challenge = keccak256(
        plaintext,
        ciphertext,
        announcement_g,
        announcement_ctx,
        [U256::from(1), U256::from(2)],
        public_key,
    );
    let check_1 = proof_check_1(public_key, announcement_g, challenge, response);
    let check_2 = proof_check_2(ciphertext, plaintext, announcement_ctx, challenge, response);
    check_1 && check_2
}

#[derive(Default)]
pub struct PolicyContract {
    policies: Vec<U256>,
    payment_requests: HashMap<Vec<u8>, Vec<PaymentRequests>>,
    aggregate_storage: HashMap<Vec<u8>, EncryptedAggregate>,
    proof_verification_storage: HashMap<Vec<u8>, bool>,
}

impl PolicyContract {
    pub fn new(policies: Vec<U256>) -> Self {
        Self {
            policies,
            ..Self::default()
        }
    }
}

impl PolicyContract {
    pub fn fetch_encrypted_aggregate(&self, client_id: &[u8]) -> (U256, U256, U256, U256) {
        let encrypted_aggregate = &self.aggregate_storage[client_id];
        (
            encrypted_aggregate.x0,
            encrypted_aggregate.x1,
            encrypted_aggregate.y0,
            encrypted_aggregate.y1,
        )
    }

    pub fn fetch_encrypted_aggregate_array(&self, client_id: &[u8]) -> [U256; 4] {
        let encrypted_aggregate = &self.aggregate_storage[client_id];
        [
            encrypted_aggregate.x0,
            encrypted_aggregate.x1,
            encrypted_aggregate.y0,
            encrypted_aggregate.y1,
        ]
    }

    pub fn fetch_public_key(&self, client_id: &[u8]) -> [U256; 2] {
        self.aggregate_storage[client_id].public_key
    }

    pub fn fetch_proof_verification(&self, client_id: &[u8]) -> bool {
        self.proof_verification_storage[client_id]
    }

    pub fn calculate_aggregate(
        &mut self,
        input: Vec<[U256; 4]>,
        public_key: [U256; 2],
        client_id: Vec<u8>,
    ) -> bool {
        let aggregate = inner_product(input, &self.policies);
        let enc_aggr = EncryptedAggregate {
            x0: aggregate[0],
            x1: aggregate[1],
            y0: aggregate[2],
            y1: aggregate[3],
            public_key,
        };
        self.aggregate_storage.insert(client_id, enc_aggr);
        true
    }

    pub fn submit_proof_decryption(&mut self, input: [U256; 7], client_id: Vec<u8>) -> bool {
        let client_aggregate = self.fetch_encrypted_aggregate_array(&client_id);
        let client_pk = self.fetch_public_key(&client_id);
        dbg!(&client_pk);
        let plaintext = [input[0], input[1]];
        let announcement_g = [input[2], input[3]];
        let announcement_ctx = [input[4], input[5]];
        let response = input[6];
        let proof_verification = check_proof(
            client_aggregate,
            plaintext,
            client_pk,
            announcement_g,
            announcement_ctx,
            response,
        );
        self.proof_verification_storage
            .insert(client_id, proof_verification);
        true
    }

    pub fn request_payment(
        &mut self,
        encrypted_aggregate: [U256; 4],
        decrypted_aggregate: [U256; 2],
        proof_correct_decryption: [U256; 2],
        client_id: &[u8],
    ) -> bool {
        // TODO: implement proof verification
        let proof_is_valid = true;
        let payment_request = PaymentRequests::new(
            encrypted_aggregate,
            decrypted_aggregate,
            proof_correct_decryption,
            proof_is_valid,
        );
        self.payment_requests
            .get_mut(client_id)
            .unwrap()
            .push(payment_request);
        proof_is_valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils;

    #[test]
    fn test_u256_conversions() {
        let n = bn::arith::U256::one();
        let pn = primtypes_u256(n);
        assert_eq!(bn_u256(pn), n);

        let pn = U256::one();
        let n = bn_u256(pn);
        assert_eq!(pn, primtypes_u256(n));
    }

    fn test_policy_contract(policies: Vec<U256>, expected_scalar_aggregate: Fr) {
        let (sk, pk) = utils::generate_keys();
        let interactions: Vec<_> = policies.iter().map(|_| pk.encrypt(&G1::one())).collect();
        let client_id = vec![42];
        let mut policy_contract = PolicyContract::new(policies);

        let encoded_interactions = utils::encode_input_ciphertext(interactions).unwrap();
        let encoded_pk = utils::encode_public_key(pk).unwrap();
        let tx_receipt = policy_contract.calculate_aggregate(
            encoded_interactions,
            encoded_pk,
            client_id.clone(),
        );
        assert!(tx_receipt);

        let encrypted_point = policy_contract.fetch_encrypted_aggregate_array(&client_id);
        let encrypted_encoded = utils::decode_ciphertext(encrypted_point, pk).unwrap();

        let decrypted_aggregate = sk.decrypt(&encrypted_encoded);
        let scalar_aggregate = utils::recover_scalar(decrypted_aggregate, 16).unwrap();
        assert_eq!(scalar_aggregate, expected_scalar_aggregate);

        let proof_dec = sk
            .proof_decryption_as_string(&encrypted_encoded, &decrypted_aggregate)
            .unwrap();

        let encoded_proof_dec = utils::encode_proof_decryption(&proof_dec).unwrap();
        let tx_receipt_proof =
            policy_contract.submit_proof_decryption(encoded_proof_dec, client_id.clone());
        assert!(tx_receipt_proof);

        let proof_result = policy_contract.fetch_proof_verification(&client_id);
        assert!(proof_result);
    }

    #[test]
    fn test_policy_contract_2ads() {
        let policies = vec![U256::from(1), U256::from(2)];
        test_policy_contract(policies, Fr::from_str("3").unwrap());
    }

    #[test]
    fn test_policy_contract_128ads() {
        let policies = vec![
            U256::from(1),
            U256::from(1),
            U256::from(1),
            U256::from(1),
            U256::from(1),
            U256::from(1),
            U256::from(1),
            U256::from(1),
            U256::from(1),
            U256::from(1), //10
            U256::from(2),
            U256::from(2),
            U256::from(2),
            U256::from(2),
            U256::from(2),
            U256::from(2),
            U256::from(2),
            U256::from(2),
            U256::from(2),
            U256::from(2), // 2 * 10
            U256::from(1),
            U256::from(1),
            U256::from(1),
            U256::from(1),
            U256::from(1),
            U256::from(1),
            U256::from(1),
            U256::from(1),
            U256::from(1),
            U256::from(1), //10
            U256::from(2),
            U256::from(2),
            U256::from(2),
            U256::from(2),
            U256::from(2),
            U256::from(2),
            U256::from(2),
            U256::from(2),
            U256::from(2),
            U256::from(2), // 2 * 10
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
            U256::from(0),
        ];
        test_policy_contract(policies, Fr::from_str("60").unwrap());
    }
}
