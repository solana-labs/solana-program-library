//! Types for managing seed configurations in TLV Account Resolution
//!
//! The largest possible seed configuration is 3 bytes (`InstructionArg`).
//! This means that an `AccountMetaPda` can store up to 10 seed configurations.

use {crate::error::AccountResolutionError, solana_program::program_error::ProgramError};

/// Enum to describe a required seed for a Program-Derived Address
#[derive(Clone, Debug, PartialEq)]
pub enum Seed {
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
        /// The type of instruction argument to resolve
        ty: InstructionArgType,
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
            // 1 byte for the discriminator, 1 byte for the length of the bytes, then the raw bytes
            Seed::Literal { bytes } => 1 + 1 + bytes.len() as u8,
            // 1 byte for the discriminator, 1 byte for the index, 2 bytes for the type
            Seed::InstructionArg { .. } => 1 + 1 + InstructionArgType::TLV_SIZE,
            // 1 byte for the discriminator, 1 byte for the index
            Seed::AccountKey { .. } => 1 + 1,
        }
    }

    /// Packs a seed configuration into a byte vector
    pub fn pack(&self) -> Vec<u8> {
        // We have to start at 1 since configurations may not use the whole
        // 32-byte array of an `AccountMetaPda` (empty = 0)
        match &self {
            Seed::Literal { bytes } => {
                let mut packed = vec![1];
                packed.push(bytes.len() as u8);
                packed.extend_from_slice(bytes);
                packed
            }
            Seed::InstructionArg { index, ty } => {
                let mut packed = vec![2, *index];
                packed.extend_from_slice(&ty.pack());
                packed
            }
            Seed::AccountKey { index } => {
                vec![3, *index]
            }
        }
    }

    /// Packs a vector of seed configurations into a 32-byte array,
    /// filling the rest with 0s. Fails if overflows.
    pub fn pack_into_array(seeds: &[Self]) -> Result<[u8; 32], ProgramError> {
        let mut packed = vec![0u8; 32];
        let mut i: usize = 0;
        for seed in seeds {
            let seed_size = seed.tlv_size() as usize;
            if i + seed_size > 32 {
                return Err(AccountResolutionError::SeedConfigsTooLarge.into());
            }
            packed[i..i + seed_size].copy_from_slice(&seed.pack());
            i += seed_size;
        }
        Ok(packed.try_into().unwrap())
    }

    /// Unpacks a seed configuration from a buffer
    pub fn unpack(bytes: &[u8]) -> Result<Self, ProgramError> {
        // We have to start at 1 since configurations may not use the whole
        // 32-byte array of an `AccountMetaPda` (empty = 0)
        match bytes[0] {
            1 => {
                let bytes_length = bytes[1] as usize;
                Ok(Seed::Literal {
                    bytes: bytes[2..2 + bytes_length].to_vec(),
                })
            }
            2 => Ok(Seed::InstructionArg {
                index: bytes[1],
                ty: InstructionArgType::unpack(&bytes[2..4])?,
            }),
            3 => Ok(Seed::AccountKey { index: bytes[1] }),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    /// Unpacks all seed configurations from a 32-byte array
    pub fn unpack_array(bytes: &[u8; 32]) -> Result<Vec<Self>, ProgramError> {
        let mut seeds = vec![];
        let mut i = 0;
        while i < 32 && bytes[i] != 0 {
            let seed = match bytes[i] {
                1 => {
                    let bytes_length = bytes[i + 1] as usize;
                    Seed::Literal {
                        bytes: bytes[i + 2..i + 2 + bytes_length].to_vec(),
                    }
                }
                2 => Seed::InstructionArg {
                    index: bytes[i + 1],
                    ty: InstructionArgType::unpack(&bytes[i + 2..i + 4])?,
                },
                3 => Seed::AccountKey {
                    index: bytes[i + 1],
                },
                _ => return Err(ProgramError::InvalidAccountData),
            };
            let tlv_size = seed.tlv_size() as usize;
            seeds.push(seed);
            i += tlv_size;
        }
        Ok(seeds)
    }

    /// Get all indices references by an `AccountKey` configuration
    pub fn get_account_key_indices(seed_configs: &[Self]) -> Option<Vec<usize>> {
        let indices: Vec<usize> = seed_configs
            .iter()
            .filter_map(|seed_config| match seed_config {
                Seed::AccountKey { index } => Some(*index as usize),
                _ => None,
            })
            .collect();
        if indices.is_empty() {
            None
        } else {
            Some(indices)
        }
    }
}

/// Enum to describe the type of instruction argument to resolve
#[derive(Clone, Debug, PartialEq)]
pub enum InstructionArgType {
    /// A `u8` argument
    U8,
    /// A `u16` argument
    U16,
    /// A `u8` argument
    U32,
    /// A `u64` argument
    U64,
    /// A `u128` argument
    U128,
    /// A `Pubkey` argument
    Pubkey,
    /// A `[u8]` argument.
    /// Max seed length is 32, so `u8` for size is OK.
    /// Strings should be converted to bytes and use this.
    U8Array(u8),
}
impl InstructionArgType {
    /// The size of the TLV for an instruction argument type
    const TLV_SIZE: u8 = 2;

    /// Get the size of an instruction argument type
    pub fn arg_size(&self) -> u8 {
        match self {
            InstructionArgType::U8 => 1,
            InstructionArgType::U16 => 2,
            InstructionArgType::U32 => 4,
            InstructionArgType::U64 => 8,
            InstructionArgType::U128 => 16,
            InstructionArgType::Pubkey => 32,
            InstructionArgType::U8Array(size) => *size,
        }
    }

    /// Packs an instruction argument type into a byte vector
    /// Uses two bytes to describe the type and length of the arg
    pub fn pack(&self) -> Vec<u8> {
        match self {
            InstructionArgType::U8 => vec![0, 0],
            InstructionArgType::U16 => vec![0, 1],
            InstructionArgType::U32 => vec![0, 2],
            InstructionArgType::U64 => vec![0, 3],
            InstructionArgType::U128 => vec![0, 4],
            InstructionArgType::Pubkey => vec![0, 5],
            InstructionArgType::U8Array(size) => vec![1, *size],
        }
    }

    /// Unpacks an instruction argument type from a buffer
    pub fn unpack(bytes: &[u8]) -> Result<Self, ProgramError> {
        match bytes[0] {
            0 => match bytes[1] {
                0 => Ok(InstructionArgType::U8),
                1 => Ok(InstructionArgType::U16),
                2 => Ok(InstructionArgType::U32),
                3 => Ok(InstructionArgType::U64),
                4 => Ok(InstructionArgType::U128),
                5 => Ok(InstructionArgType::Pubkey),
                _ => Err(ProgramError::InvalidAccountData),
            },
            1 => Ok(InstructionArgType::U8Array(bytes[1])),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}
