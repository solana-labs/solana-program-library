use crate::error::ZolError;
use borsh::{BorshDeserialize, BorshSerialize};
use curve25519_dalek::ristretto::RistrettoPoint;
use elgamal_ristretto::{ciphertext::Ciphertext, public::PublicKey};
use solana_program::program_error::ProgramError;

// TODO: Choose a range proof (probably Bulletproofs)
#[derive(BorshSerialize, BorshDeserialize)]
pub struct SolvencyProof {}

impl SolvencyProof {
    pub fn verify(&self) -> Result<(), ProgramError> {
        // TODO
        Ok(())
    }
}

// TODO: Choose a Sigma protocol
#[derive(BorshSerialize, BorshDeserialize)]
pub struct EquivalenceProof {}

impl EquivalenceProof {
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
