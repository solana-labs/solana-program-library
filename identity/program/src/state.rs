//! State transition types

use crate::instruction::MAX_SIGNERS;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use num_enum::TryFromPrimitive;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
    info
};
use std::{io, io::Write, io::Error};

// #[cfg(feature = "serde")]
// impl Serialize for Pubkey {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//         where S: Serializer
//     {
//         use serde::ser::SerializeTuple;
//         let mut tup = serializer.serialize_tuple(32)?;
//         for byte in self.iter() {
//             tup.serialize_element(byte)?;
//         }
//         tup.end()
//
//     }
// }

/// A version of Solana's Pubkey type that is serializable using Borsh
#[derive(Clone, PartialEq, Debug, Default)]
pub struct SerializablePubkey(pub Pubkey);
impl BorshSerialize for SerializablePubkey {
    // fn serialize(&self, mut data: &mut [u8]) -> Result<(), ProgramError> {
    //     BorshSerialize::serialize(&self.0.to_bytes(), &mut data).map_err(|_| ProgramError::AccountDataTooSmall)
    // }

    fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.0.to_bytes().serialize(writer)
    }
}
impl BorshDeserialize for SerializablePubkey {
    fn deserialize(data: &mut &[u8]) -> io::Result<Self> {
        Self::try_from_slice(&data)
    }

    // fn try_from_slice(data: &[u8]) -> Result<Self, Error> {
    //     BorshDeserialize::try_from_slice(&data)
    // }
}
// impl BorshDeserialize for SerializablePubkey {
//     fn deserialize(data: &mut &[u8]) -> io::Result<Self> {
//         Self::try_from_slice(&data).map_err(|_| ProgramError::InvalidAccountData)
//     }
// }

impl From<Pubkey> for SerializablePubkey {
    fn from(pubkey: Pubkey) -> Self {
        SerializablePubkey(pubkey)
    }
}

/// An attestation by an identity validator (IDV) for some claims on an Identity
#[derive(Clone, Debug, Default, BorshSerialize, BorshDeserialize)]
pub struct Attestation {
    /// The IDV that made the attestation
    pub idv: SerializablePubkey,
    /// The attestation data
    pub attestation_data: Vec<u8>
}
impl Attestation {
    fn matches(&self, attestation: &Attestation) -> bool {
        self.idv == attestation.idv && self.attestation_data == attestation.attestation_data
    }
}

/// Identity Account data.
#[repr(C)]
#[derive(Clone, Debug, Default, BorshSerialize, BorshDeserialize)]
pub struct IdentityAccount {
    /// The owner of this account.
    pub owner: SerializablePubkey,
    /// The account's state
    pub state: AccountState,
    /// The size of the attestations vector
    pub num_attestations: u8,
    /// Attestations added to the account
    pub attestations: Vec<Attestation>,
}
impl IdentityAccount {
    /// Serialize the account into a byte array
    pub fn serialize(&self, mut data: &mut [u8]) -> Result<(), ProgramError> {
        BorshSerialize::serialize(self, &mut data).map_err(|_| ProgramError::AccountDataTooSmall)
    }
    /// Deserialize the account from a byte array
    pub fn deserialize(data: &[u8]) -> Result<Self, ProgramError> {
        info!("IdentityAccount: deserialize");
        Self::try_from_slice(&data).map_err(|_| ProgramError::InvalidAccountData)
    }

    // fn try_from_slice(data: &[u8]) -> Result<Self, Error> {
    //     info!("IdentityAccount: try_from_slice");
    //     BorshDeserialize::try_from_slice(&data)
    // }

    /// Create a new identity with no attestations
    pub fn new(owner: Pubkey) -> Self {
        Self {
            owner: SerializablePubkey(owner),
            state: AccountState::Initialized,
            num_attestations: 0,
            attestations: vec![],
        }
    }

    // /// Useful for testing
    // pub fn new_with_attestations(owner: Pubkey, attestations: &Vec<Attestation>) -> Self {
    //     let mut account = Self::new(owner);
    //     account.attestations = *attestations;
    //     account.num_attestations = attestations.len() as u8;
    //     account
    // }

    fn verify(&self, attestation: &Attestation) -> bool {
        self.attestations.iter().any(|att| att.matches(attestation))
    }
}
impl Sealed for IdentityAccount {}
impl IsInitialized for IdentityAccount {
    fn is_initialized(&self) -> bool {
        self.state != AccountState::Uninitialized
    }
}

/// Account state.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, TryFromPrimitive, BorshSerialize, BorshDeserialize)]
pub enum AccountState {
    /// Account is not yet initialized
    Uninitialized,
    /// Account is initialized; the account owner and/or delegate may perform permitted operations
    /// on this account
    Initialized,
}

impl Default for AccountState {
    fn default() -> Self {
        AccountState::Uninitialized
    }
}

/// Multisignature data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Multisig {
    /// Number of signers required
    pub m: u8,
    /// Number of valid signers
    pub n: u8,
    /// Is `true` if this structure has been initialized
    pub is_initialized: bool,
    /// Signer public keys
    pub signers: [Pubkey; MAX_SIGNERS],
}
impl Sealed for Multisig {}
impl IsInitialized for Multisig {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl Pack for Multisig {
    const LEN: usize = 355;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, 355];
        #[allow(clippy::ptr_offset_with_cast)]
        let (m, n, is_initialized, signers_flat) = array_refs![src, 1, 1, 1, 32 * MAX_SIGNERS];
        let mut result = Multisig {
            m: m[0],
            n: n[0],
            is_initialized: match is_initialized {
                [0] => false,
                [1] => true,
                _ => return Err(ProgramError::InvalidAccountData),
            },
            signers: [Pubkey::new_from_array([0u8; 32]); MAX_SIGNERS],
        };
        for (src, dst) in signers_flat.chunks(32).zip(result.signers.iter_mut()) {
            *dst = Pubkey::new(src);
        }
        Ok(result)
    }
    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, 355];
        #[allow(clippy::ptr_offset_with_cast)]
        let (m, n, is_initialized, signers_flat) = mut_array_refs![dst, 1, 1, 1, 32 * MAX_SIGNERS];
        *m = [self.m];
        *n = [self.n];
        *is_initialized = [self.is_initialized as u8];
        for (i, src) in self.signers.iter().enumerate() {
            let dst_array = array_mut_ref![signers_flat, 32 * i, 32];
            dst_array.copy_from_slice(src.as_ref());
        }
    }
}
