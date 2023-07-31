//! Types for managing seed configurations in TLV Account Resolution
//!
//! The largest possible seed configuration is 3 bytes (`InstructionArg`).
//! This means that an `AccountMetaPda` can store up to 10 seed configurations.

use {crate::error::AccountResolutionError, solana_program::program_error::ProgramError};

/// Enum to describe a required seed for a Program-Derived Address
#[derive(Clone, Debug, PartialEq)]
pub enum Seed {
    /// Uninitialized configuration byte space
    Uninitialized,
    /// A literal hard-coded argument
    Literal {
        /// The literal value repesented as a vector of bytes.
        /// For example, if a literal value is a string literal,
        /// such as "my-seed", this value would be
        /// `"my-seed".as_bytes().to_vec()`.
        bytes: Vec<u8>,
    },
    /// An instruction-provided argument, to be resolved from the instruction
    /// data
    InstructionArg {
        /// The index where the bytes of an instruction argument begin
        index: u8,
        /// The length of the instruction argument (number of bytes)
        ///
        /// Note: Max seed length is 32 bytes, so `u8` is appropriate here
        length: u8,
    },
    /// The public key of an account from the entire accounts list.
    /// Note: This includes an extra accounts required.
    AccountKey {
        /// The index of the account in the entire accounts list
        index: u8,
    },
}
impl Seed {
    /// Get the size of a seed configuration
    pub fn tlv_size(&self) -> u8 {
        match &self {
            // 1 byte for the discriminator
            Self::Uninitialized => 0,
            // 1 byte for the discriminator, 1 byte for the length of the bytes, then the raw bytes
            Self::Literal { bytes } => 1 + 1 + bytes.len() as u8,
            // 1 byte for the discriminator, 1 byte for the index, 1 byte for the length
            Self::InstructionArg { .. } => 1 + 1 + 1,
            // 1 byte for the discriminator, 1 byte for the index
            Self::AccountKey { .. } => 1 + 1,
        }
    }

    /// Packs a seed configuration into a slice
    pub fn pack(&self, dst: &mut [u8]) -> Result<(), ProgramError> {
        if dst.len() != self.tlv_size() as usize {
            return Err(AccountResolutionError::IncorrectAccount.into());
        }
        match &self {
            Self::Uninitialized => {
                dst[0] = 0;
            }
            Self::Literal { bytes } => {
                dst[0] = 1;
                dst[1] = bytes.len() as u8;
                dst[2..].copy_from_slice(bytes);
            }
            Self::InstructionArg { index, length } => {
                dst[0] = 2;
                dst[1] = *index;
                dst[2] = *length;
            }
            Self::AccountKey { index } => {
                dst[0] = 3;
                dst[1] = *index;
            }
        }
        Ok(())
    }

    /// Packs a vector of seed configurations into a 32-byte array,
    /// filling the rest with 0s. Errors if it overflows.
    pub fn pack_into_array(seeds: &[Self]) -> Result<[u8; 32], ProgramError> {
        let mut packed = [0u8; 32];
        let mut i: usize = 0;
        for seed in seeds {
            let seed_size = seed.tlv_size() as usize;
            let slice_end = i + seed_size;
            if slice_end > 32 {
                return Err(AccountResolutionError::SeedConfigsTooLarge.into());
            }
            seed.pack(&mut packed[i..slice_end])?;
            i = slice_end;
        }
        Ok(packed)
    }

    /// Unpacks a seed configuration from a slice
    pub fn unpack(bytes: &[u8]) -> Result<Self, ProgramError> {
        let (discrim, rest) = bytes
            .split_first()
            .ok_or::<ProgramError>(ProgramError::InvalidAccountData)?;
        match discrim {
            0 => Ok(Self::Uninitialized),
            1 => unpack_seed_literal(rest),
            2 => unpack_seed_instruction_arg(rest),
            3 => unpack_seed_account_key(rest),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    /// Unpacks all seed configurations from a 32-byte array.
    /// Stops when it hits uninitialized data (0s).
    pub fn unpack_array(bytes: &[u8; 32]) -> Result<Vec<Self>, ProgramError> {
        let mut seeds = vec![];
        let mut i = 0;
        while i < 32 {
            let seed = Self::unpack(&bytes[i..])?;
            let seed_size = seed.tlv_size() as usize;
            i += seed_size;
            if seed == Self::Uninitialized {
                break;
            }
            seeds.push(seed);
        }
        Ok(seeds)
    }

    /// Get all indices references by an `AccountKey` configuration
    pub fn get_account_key_indices(seed_configs: &[Self]) -> Vec<u8> {
        seed_configs
            .iter()
            .filter_map(|seed_config| match seed_config {
                Self::AccountKey { index } => Some(*index),
                _ => None,
            })
            .collect::<Vec<_>>()
    }
}

fn unpack_seed_literal(bytes: &[u8]) -> Result<Seed, ProgramError> {
    let (length, rest) = bytes
        .split_first()
        // Should be at least 1 byte
        .ok_or::<ProgramError>(AccountResolutionError::NotEnoughBytesForSeed.into())?;
    let length = *length as usize;
    if rest.len() < length {
        // Should be at least `length` bytes
        return Err(AccountResolutionError::NotEnoughBytesForSeed.into());
    }
    Ok(Seed::Literal {
        bytes: rest[..length].to_vec(),
    })
}

fn unpack_seed_instruction_arg(bytes: &[u8]) -> Result<Seed, ProgramError> {
    if bytes.len() < 2 {
        // Should be at least 2 bytes
        return Err(AccountResolutionError::NotEnoughBytesForSeed.into());
    }
    Ok(Seed::InstructionArg {
        index: bytes[0],
        length: bytes[1],
    })
}

fn unpack_seed_account_key(bytes: &[u8]) -> Result<Seed, ProgramError> {
    if bytes.is_empty() {
        // Should be at least 1 byte
        return Err(AccountResolutionError::NotEnoughBytesForSeed.into());
    }
    Ok(Seed::AccountKey { index: bytes[0] })
}
