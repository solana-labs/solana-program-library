#![allow(missing_docs)]

use crate::errors::Error;
use bn::{Fr, Group, G1, arith::U256};
use elgamal_bn::{ciphertext::Ciphertext, private::SecretKey, public::{PublicKey, into_hex, from_hex}};
use rand::thread_rng;

pub(crate) type CiphertextSolidity = [U256; 4];
pub(crate) type Point = [U256; 2];
// two points and one scalar
pub(crate) type Proof = [U256; 7];

pub(crate) fn generate_keys() -> (SecretKey, PublicKey) {
    let mut csprng = thread_rng();
    let sk = SecretKey::new(&mut csprng);
    let pk = PublicKey::from(&sk);
    (sk, pk)
}

fn u256_from_str(s: &str) -> U256 {
    let s = if &s[0..2] == "0x" { &s[2..] } else {s};
    from_hex(s).unwrap()
}

fn u256_to_string(x: bn::arith::U256) -> String {
    format!("0x{}", into_hex(x).unwrap())
}

pub(crate) fn encode_proof_decryption(input: &[String; 7]) -> Result<Proof, ()> {
    let proof = [
        u256_from_str(&input[0]),
        u256_from_str(&input[1]),
        u256_from_str(&input[2]),
        u256_from_str(&input[3]),
        u256_from_str(&input[4]),
        u256_from_str(&input[5]),
        u256_from_str(&input[6]),
    ];
    Ok(proof)
}

pub(crate) fn encode_public_key(input: PublicKey) -> Result<Point, Error> {
    let (x, y) = input.get_point_hex_string().unwrap();
    let pk_point = [
        u256_from_str(&x),
        u256_from_str(&y),
    ];
    Ok(pk_point)
}

pub(crate) fn encode_input_ciphertext(input: Vec<Ciphertext>) -> Result<Vec<CiphertextSolidity>, Error> {
    let encoded_input = input
        .into_iter()
        .map(|x| {
            // todo: handle these unwraps
            let ((x0, x1), (y0, y1)) = x.get_points_hex_string().unwrap();
            let point = [
               u256_from_str(&x0),
               u256_from_str(&x1),
               u256_from_str(&y0),
               u256_from_str(&y1),
            ];
            point
        })
        .collect();
    Ok(encoded_input)
}

pub(crate) fn decode_ciphertext(
    raw_point: CiphertextSolidity,
    pk: PublicKey,
) -> Result<Ciphertext, Error> {
    let encrypted_encoded = Ciphertext::from_hex_string(
        (
            (u256_to_string(raw_point[0]), u256_to_string(raw_point[1])),
            (u256_to_string(raw_point[2]), u256_to_string(raw_point[3])),
        ),
        pk,
    );

    encrypted_encoded.map_err(|_| Error::ElGamalConversionError)
}

pub(crate) fn recover_scalar(point: G1, k: u32) -> Result<Fr, Error> {
    for i in 0..2u64.pow(k) {
        let scalar = match Fr::from_str(&i.to_string()) {
            Some(s) => s,
            None => Fr::one(),
        };
        if (G1::one() * scalar) == point {
            return Ok(scalar);
        }
    }
    println!("Encryped scalar too long");
    Err(Error::GeneralError)
}
