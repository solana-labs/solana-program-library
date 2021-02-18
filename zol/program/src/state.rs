use crate::error::ZolError;
use borsh::{BorshDeserialize, BorshSerialize};
use bulletproofs::RangeProof;
use curve25519_dalek::ristretto::RistrettoPoint;
use elgamal_ristretto::{ciphertext::Ciphertext, public::PublicKey};
use solana_program::program_error::ProgramError;
use std::io::{Error, ErrorKind, Write};

pub struct SolvencyProof(RangeProof);

impl BorshSerialize for SolvencyProof {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        BorshSerialize::serialize(&self.0.to_bytes(), writer)
    }
}

impl BorshDeserialize for SolvencyProof {
    fn deserialize(buf: &mut &[u8]) -> Result<Self, Error> {
        let bytes: Vec<u8> = BorshDeserialize::deserialize(buf)?;
        let proof = RangeProof::from_bytes(&bytes).map_err(|_| {
            Error::new(
                ErrorKind::InvalidInput,
                "range proof deserialization failed",
            )
        })?;
        Ok(Self(proof))
    }
}

impl SolvencyProof {
    pub fn new(proof: RangeProof) -> Self {
        Self(proof)
    }

    pub fn verify(&self, ciphertext: &Ciphertext) -> Result<(), ProgramError> {
        use bulletproofs::{BulletproofGens, PedersenGens};
        use merlin::Transcript;

        let pc_gens = PedersenGens::default();
        let bp_gens = BulletproofGens::new(64, 1);

        // TODO: This doesn't make any sense
        let committed_value = ciphertext.get_points().0.compress();

        // Verification requires a transcript with identical initial state:
        let mut verifier_transcript = Transcript::new(b"example");
        let _result: Result<(), ProgramError> = self
            .0
            .verify_single(
                &bp_gens,
                &pc_gens,
                &mut verifier_transcript,
                &committed_value,
                32,
            )
            .map_err(|_| ZolError::SolvencyProofVerificationFailed.into());

        // TODO: return real result
        Ok(())
    }
}

// TODO: Choose a Sigma protocol
#[derive(BorshSerialize, BorshDeserialize)]
pub struct EquivalenceProof {}

impl EquivalenceProof {
    pub fn new() -> Self {
        Self {}
    }

    pub fn verify(&self) -> Result<(), ProgramError> {
        // TODO
        Ok(())
    }
}

/// User account state
#[derive(BorshSerialize, BorshDeserialize)]
pub struct User {
    /// The amount of SOL in this account
    pub encrypted_amount: Ciphertext,
}

impl User {
    pub fn new(encryption_pubkey: PublicKey) -> Self {
        Self {
            encrypted_amount: encryption_pubkey.encrypt(&RistrettoPoint::default()),
        }
    }
}

/// ZOL account state
#[derive(BorshSerialize, BorshDeserialize)]
pub enum State {
    /// An uninitialized account
    Uninitialized,

    /// An account that stores SOL
    Vault,

    /// An account that stores a user's ZOL
    User(User),
}

impl State {
    pub fn serialize(&self, mut data: &mut [u8]) -> Result<(), ProgramError> {
        BorshSerialize::serialize(self, &mut data).map_err(|_| ProgramError::AccountDataTooSmall)
    }

    pub fn deserialize(mut data: &[u8]) -> Result<Self, ProgramError> {
        BorshDeserialize::deserialize(&mut data).map_err(|_| ProgramError::InvalidAccountData)
    }

    pub fn packed_len(&self) -> usize {
        BorshSerialize::try_to_vec(self).unwrap().len()
    }

    pub fn user_mut(&mut self) -> Result<&mut User, ProgramError> {
        if let State::User(user) = self {
            return Ok(user);
        }
        Err(ZolError::UnexpectedStateType.into())
    }
}
