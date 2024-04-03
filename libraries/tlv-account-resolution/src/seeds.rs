//! Types for managing seed configurations in TLV Account Resolution
//!
//! As determined by the `address_config` field of `ExtraAccountMeta`,
//! seed configurations are limited to a maximum of 32 bytes.
//! This means that the maximum number of seed configurations that can be
//! packed into a single `ExtraAccountMeta` will depend directly on the size
//! of the seed configurations themselves.
//!
//! Sizes are as follows:
//!     * `Seed::Literal`: 1 + 1 + N
//!         * 1 - Discriminator
//!         * 1 - Length of literal
//!         * N - Literal bytes themselves
//!     * `Seed::InstructionData`: 1 + 1 + 1 = 3
//!         * 1 - Discriminator
//!         * 1 - Start index of instruction data
//!         * 1 - Length of instruction data starting at index
//!     * `Seed::AccountKey` - 1 + 1 = 2
//!         * 1 - Discriminator
//!         * 1 - Index of account in accounts list
//!     * `Seed::AccountData`: 1 + 1 + 1 + 1 = 4
//!         * 1 - Discriminator
//!         * 1 - Index of account in accounts list
//!         * 1 - Start index of account data
//!         * 1 - Length of account data starting at index
//!
//! No matter which types of seeds you choose, the total size of all seed
//! configurations must be less than or equal to 32 bytes.

#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
use {crate::error::AccountResolutionError, solana_program::program_error::ProgramError};

/// Enum to describe a required seed for a Program-Derived Address
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
pub enum Seed {
    /// Uninitialized configuration byte space
    Uninitialized,
    /// A literal hard-coded argument
    /// Packed as:
    ///     * 1 - Discriminator
    ///     * 1 - Length of literal
    ///     * N - Literal bytes themselves
    Literal {
        /// The literal value represented as a vector of bytes.
        ///
        /// For example, if a literal value is a string literal,
        /// such as "my-seed", this value would be
        /// `"my-seed".as_bytes().to_vec()`.
        bytes: Vec<u8>,
    },
    /// An instruction-provided argument, to be resolved from the instruction
    /// data
    /// Packed as:
    ///     * 1 - Discriminator
    ///     * 1 - Start index of instruction data
    ///     * 1 - Length of instruction data starting at index
    InstructionData {
        /// The index where the bytes of an instruction argument begin
        index: u8,
        /// The length of the instruction argument (number of bytes)
        ///
        /// Note: Max seed length is 32 bytes, so `u8` is appropriate here
        length: u8,
    },
    /// The public key of an account from the entire accounts list.
    /// Note: This includes an extra accounts required.
    ///
    /// Packed as:
    ///     * 1 - Discriminator
    ///     * 1 - Index of account in accounts list
    AccountKey {
        /// The index of the account in the entire accounts list
        index: u8,
    },
    /// An argument to be resolved from the inner data of some account
    /// Packed as:
    ///     * 1 - Discriminator
    ///     * 1 - Index of account in accounts list
    ///     * 1 - Start index of account data
    ///     * 1 - Length of account data starting at index
    #[cfg_attr(
        feature = "serde-traits",
        serde(rename_all = "camelCase", alias = "account_data")
    )]
    AccountData {
        /// The index of the account in the entire accounts list
        account_index: u8,
        /// The index where the bytes of an account data argument begin
        data_index: u8,
        /// The length of the argument (number of bytes)
        ///
        /// Note: Max seed length is 32 bytes, so `u8` is appropriate here
        length: u8,
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
            Self::InstructionData { .. } => 1 + 1 + 1,
            // 1 byte for the discriminator, 1 byte for the index
            Self::AccountKey { .. } => 1 + 1,
            // 1 byte for the discriminator, 1 byte for the account index,
            // 1 byte for the data index 1 byte for the length
            Self::AccountData { .. } => 1 + 1 + 1 + 1,
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
            Self::Uninitialized => return Err(AccountResolutionError::InvalidSeedConfig.into()),
            Self::Literal { bytes } => {
                dst[0] = 1;
                dst[1] = bytes.len() as u8;
                dst[2..].copy_from_slice(bytes);
            }
            Self::InstructionData { index, length } => {
                dst[0] = 2;
                dst[1] = *index;
                dst[2] = *length;
            }
            Self::AccountKey { index } => {
                dst[0] = 3;
                dst[1] = *index;
            }
            Self::AccountData {
                account_index,
                data_index,
                length,
            } => {
                dst[0] = 4;
                dst[1] = *account_index;
                dst[2] = *data_index;
                dst[3] = *length;
            }
        }
        Ok(())
    }

    /// Packs a vector of seed configurations into a 32-byte array,
    /// filling the rest with 0s. Errors if it overflows.
    pub fn pack_into_address_config(seeds: &[Self]) -> Result<[u8; 32], ProgramError> {
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
            4 => unpack_seed_account_data(rest),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    /// Unpacks all seed configurations from a 32-byte array.
    /// Stops when it hits uninitialized data (0s).
    pub fn unpack_address_config(address_config: &[u8; 32]) -> Result<Vec<Self>, ProgramError> {
        let mut seeds = vec![];
        let mut i = 0;
        while i < 32 {
            let seed = Self::unpack(&address_config[i..])?;
            let seed_size = seed.tlv_size() as usize;
            i += seed_size;
            if seed == Self::Uninitialized {
                break;
            }
            seeds.push(seed);
        }
        Ok(seeds)
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
    Ok(Seed::InstructionData {
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

fn unpack_seed_account_data(bytes: &[u8]) -> Result<Seed, ProgramError> {
    if bytes.len() < 3 {
        // Should be at least 3 bytes
        return Err(AccountResolutionError::InvalidBytesForSeed.into());
    }
    Ok(Seed::AccountData {
        account_index: bytes[0],
        data_index: bytes[1],
        length: bytes[2],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
            Seed::pack_into_address_config(&[seed]).unwrap_err(),
            AccountResolutionError::SeedConfigsTooLarge.into()
        );

        // Should fail if the length is wrong
        let seed = Seed::Literal { bytes: vec![1; 12] };
        let mut packed = vec![0u8; seed.tlv_size() as usize - 1];
        assert_eq!(
            seed.pack(&mut packed).unwrap_err(),
            AccountResolutionError::NotEnoughBytesForSeed.into()
        );

        // Can't pack a `Seed::Uninitialized`
        let seed = Seed::Uninitialized;
        let mut packed = vec![0u8; seed.tlv_size() as usize];
        assert_eq!(
            seed.pack(&mut packed).unwrap_err(),
            AccountResolutionError::InvalidSeedConfig.into()
        );
    }

    #[test]
    fn test_pack_address_config() {
        // Should fail if one seed is too large
        let seed = Seed::Literal { bytes: vec![1; 36] };
        assert_eq!(
            Seed::pack_into_address_config(&[seed]).unwrap_err(),
            AccountResolutionError::SeedConfigsTooLarge.into()
        );

        // Should fail if the combination of all seeds is too large
        let seed1 = Seed::Literal { bytes: vec![1; 30] }; // 30 bytes
        let seed2 = Seed::InstructionData {
            index: 0,
            length: 4,
        }; // 3 bytes
        assert_eq!(
            Seed::pack_into_address_config(&[seed1, seed2]).unwrap_err(),
            AccountResolutionError::SeedConfigsTooLarge.into()
        );
    }

    #[test]
    fn test_unpack() {
        // Can unpack zeroes
        let zeroes = [0u8; 32];
        let seeds = Seed::unpack_address_config(&zeroes).unwrap();
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
            2, // Discrim (InstructionData)
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
    fn test_unpack_address_config() {
        // Should fail if bytes are malformed
        let bytes = [
            1, // Discrim (Literal)
            4, // Length
            1, 1, 1, 1, // 4
            6, // Discrim (Invalid)
            2, // Index
            1, // Length
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(
            Seed::unpack_address_config(&bytes).unwrap_err(),
            ProgramError::InvalidAccountData
        );

        // Should fail if 32nd byte is not zero, but it would be the
        // start of a config
        //
        // Namely, if a seed config is unpacked and leaves 1 byte remaining,
        // it has to be 0, since no valid seed config can be 1 byte long
        let bytes = [
            1,  // Discrim (Literal)
            16, // Length
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,  // 16
            1,  // Discrim (Literal)
            11, // Length
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 11
            2, // Non-zero byte
        ];
        assert_eq!(
            Seed::unpack_address_config(&bytes).unwrap_err(),
            AccountResolutionError::InvalidBytesForSeed.into(),
        );

        // Should pass if 31st byte is not zero, but it would be
        // the start of a config
        //
        // Similar to above, however we now have 2 bytes to work with,
        // which could be a valid seed config
        let bytes = [
            1,  // Discrim (Literal)
            16, // Length
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,  // 16
            1,  // Discrim (Literal)
            10, // Length
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 10
            3, // Non-zero byte - Discrim (AccountKey)
            0, // Index
        ];
        assert_eq!(
            Seed::unpack_address_config(&bytes).unwrap(),
            vec![
                Seed::Literal {
                    bytes: vec![1u8; 16]
                },
                Seed::Literal {
                    bytes: vec![1u8; 10]
                },
                Seed::AccountKey { index: 0 }
            ],
        );

        // Should fail if 31st byte is not zero and a valid seed config
        // discriminator, but the seed config requires more than 2 bytes
        let bytes = [
            1,  // Discrim (Literal)
            16, // Length
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,  // 16
            1,  // Discrim (Literal)
            10, // Length
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 10
            2, // Non-zero byte - Discrim (InstructionData)
            0, // Index (Length missing)
        ];
        assert_eq!(
            Seed::unpack_address_config(&bytes).unwrap_err(),
            AccountResolutionError::InvalidBytesForSeed.into(),
        );
    }

    fn test_pack_unpack_seed(seed: Seed) {
        let tlv_size = seed.tlv_size() as usize;
        let mut packed = vec![0u8; tlv_size];
        seed.pack(&mut packed).unwrap();
        let unpacked = Seed::unpack(&packed).unwrap();
        assert_eq!(seed, unpacked);
    }

    #[test]
    fn test_pack_unpack() {
        let mut mixed = vec![];

        // Literals

        let bytes = b"hello";
        let seed = Seed::Literal {
            bytes: bytes.to_vec(),
        };
        test_pack_unpack_seed(seed);

        let bytes = 8u8.to_le_bytes();
        let seed = Seed::Literal {
            bytes: bytes.to_vec(),
        };
        test_pack_unpack_seed(seed.clone());
        mixed.push(seed);

        let bytes = 32u32.to_le_bytes();
        let seed = Seed::Literal {
            bytes: bytes.to_vec(),
        };
        test_pack_unpack_seed(seed.clone());
        mixed.push(seed);

        // Instruction args

        let seed = Seed::InstructionData {
            index: 0,
            length: 0,
        };
        test_pack_unpack_seed(seed);

        let seed = Seed::InstructionData {
            index: 6,
            length: 9,
        };
        test_pack_unpack_seed(seed.clone());
        mixed.push(seed);

        // Account keys

        let seed = Seed::AccountKey { index: 0 };
        test_pack_unpack_seed(seed);

        let seed = Seed::AccountKey { index: 9 };
        test_pack_unpack_seed(seed.clone());
        mixed.push(seed);

        // Account data

        let seed = Seed::AccountData {
            account_index: 0,
            data_index: 0,
            length: 0,
        };
        test_pack_unpack_seed(seed);

        let seed = Seed::AccountData {
            account_index: 0,
            data_index: 0,
            length: 9,
        };
        test_pack_unpack_seed(seed.clone());
        mixed.push(seed);

        // Arrays

        let packed_array = Seed::pack_into_address_config(&mixed).unwrap();
        let unpacked_array = Seed::unpack_address_config(&packed_array).unwrap();
        assert_eq!(mixed, unpacked_array);

        let mut shuffled_mixed = mixed.clone();
        shuffled_mixed.swap(0, 1);
        shuffled_mixed.swap(1, 4);
        shuffled_mixed.swap(3, 0);

        let packed_array = Seed::pack_into_address_config(&shuffled_mixed).unwrap();
        let unpacked_array = Seed::unpack_address_config(&packed_array).unwrap();
        assert_eq!(shuffled_mixed, unpacked_array);
    }
}
