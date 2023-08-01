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
            return Err(AccountResolutionError::NotEnoughBytesForSeed.into());
        }
        if dst.len() > 32 {
            return Err(AccountResolutionError::SeedConfigsTooLarge.into());
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
        while i <= 32 {
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
        .ok_or::<ProgramError>(AccountResolutionError::InvalidBytesForSeed.into())?;
    let length = *length as usize;
    if rest.len() < length {
        // Should be at least `length` bytes
        return Err(AccountResolutionError::InvalidBytesForSeed.into());
    }
    Ok(Seed::Literal {
        bytes: rest[..length].to_vec(),
    })
}

fn unpack_seed_instruction_arg(bytes: &[u8]) -> Result<Seed, ProgramError> {
    if bytes.len() < 2 {
        // Should be at least 2 bytes
        return Err(AccountResolutionError::InvalidBytesForSeed.into());
    }
    Ok(Seed::InstructionArg {
        index: bytes[0],
        length: bytes[1],
    })
}

fn unpack_seed_account_key(bytes: &[u8]) -> Result<Seed, ProgramError> {
    if bytes.is_empty() {
        // Should be at least 1 byte
        return Err(AccountResolutionError::InvalidBytesForSeed.into());
    }
    Ok(Seed::AccountKey { index: bytes[0] })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_pack_unpack_seed(seed: Seed, mixed: &mut Vec<Seed>) {
        let tlv_size = seed.tlv_size() as usize;
        let mut packed = vec![0u8; tlv_size];
        seed.pack(&mut packed).unwrap();
        let unpacked = Seed::unpack(&packed).unwrap();
        assert_eq!(seed, unpacked);
        mixed.push(seed);
    }

    #[test]
    fn test_pack() {
        // Seed too large
        let seed = Seed::Literal { bytes: vec![1; 33] };
        let mut packed = vec![0u8; seed.tlv_size() as usize];
        assert_eq!(
            seed.pack(&mut packed).unwrap_err(),
            AccountResolutionError::SeedConfigsTooLarge.into()
        );
        assert_eq!(
            Seed::pack_into_array(&[seed]).unwrap_err(),
            AccountResolutionError::SeedConfigsTooLarge.into()
        );

        // Should fail if the length is wrong
        let seed = Seed::Literal { bytes: vec![1; 12] };
        let mut packed = vec![0u8; seed.tlv_size() as usize - 1];
        assert_eq!(
            seed.pack(&mut packed).unwrap_err(),
            AccountResolutionError::NotEnoughBytesForSeed.into()
        );
    }

    #[test]
    fn test_unpack() {
        // Can unpack zeroes
        let zeroes = [0u8; 32];
        let seeds = Seed::unpack_array(&zeroes).unwrap();
        assert_eq!(seeds, vec![]);

        // Should fail for empty bytes
        let bytes = [];
        assert_eq!(
            Seed::unpack(&bytes).unwrap_err(),
            ProgramError::InvalidAccountData
        );

        // Should fail if bytes are malformed for literal seed
        let bytes = [
            1, // Discrim (Literal)
            4, // Length
            1, 1, 1, // Incorrect length
        ];
        assert_eq!(
            Seed::unpack(&bytes).unwrap_err(),
            AccountResolutionError::InvalidBytesForSeed.into()
        );

        // Should fail if bytes are malformed for literal seed
        let bytes = [
            2, // Discrim (InstructionArg)
            2, // Index (Length missing)
        ];
        assert_eq!(
            Seed::unpack(&bytes).unwrap_err(),
            AccountResolutionError::InvalidBytesForSeed.into()
        );

        // Should fail if bytes are malformed for literal seed
        let bytes = [
            3, // Discrim (AccountKey, Index missing)
        ];
        assert_eq!(
            Seed::unpack(&bytes).unwrap_err(),
            AccountResolutionError::InvalidBytesForSeed.into()
        );
    }

    #[test]
    fn test_pack_unpack() {
        let mut mixed = vec![];

        // Literals

        let bytes = b"hello";
        let seed = Seed::Literal {
            bytes: bytes.to_vec(),
        };
        test_pack_unpack_seed(seed, &mut mixed);

        let bytes = 8u8.to_le_bytes();
        let seed = Seed::Literal {
            bytes: bytes.to_vec(),
        };
        test_pack_unpack_seed(seed, &mut mixed);

        let bytes = 32u32.to_le_bytes();
        let seed = Seed::Literal {
            bytes: bytes.to_vec(),
        };
        test_pack_unpack_seed(seed, &mut mixed);

        // Instruction args

        let seed = Seed::InstructionArg {
            index: 0,
            length: 0,
        };
        test_pack_unpack_seed(seed, &mut mixed);

        let seed = Seed::InstructionArg {
            index: 6,
            length: 9,
        };
        test_pack_unpack_seed(seed, &mut mixed);

        // Account keys

        let seed = Seed::AccountKey { index: 0 };
        test_pack_unpack_seed(seed, &mut mixed);

        let seed = Seed::AccountKey { index: 9 };
        test_pack_unpack_seed(seed, &mut mixed);

        // Arrays

        let packed_array = Seed::pack_into_array(&mixed).unwrap();
        let unpacked_array = Seed::unpack_array(&packed_array).unwrap();
        assert_eq!(mixed, unpacked_array);

        let mut shuffled_mixed = mixed.clone();
        shuffled_mixed.swap(0, 5);
        shuffled_mixed.swap(1, 4);
        shuffled_mixed.swap(3, 6);
        shuffled_mixed.swap(3, 0);

        let packed_array = Seed::pack_into_array(&shuffled_mixed).unwrap();
        let unpacked_array = Seed::unpack_array(&packed_array).unwrap();
        assert_eq!(shuffled_mixed, unpacked_array);
    }
}
