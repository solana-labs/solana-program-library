//! Types for managing seed configurations in TLV Account Resolution

use solana_program::{program_error::ProgramError, pubkey::Pubkey};

use crate::error::AccountResolutionError;

/// Enum to describe a required seed for a Program-Derived Address
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seed {
    /// A string literal seed
    Lit,
    /// A seed argument
    Arg(SeedArgType),
}

impl Seed {
    /// Packs a vector of `Seed` configs into an array of 32 bytes
    pub fn pack_slice(value: &[Seed]) -> Result<[u8; 32], ProgramError> {
        let len = value.len();
        if len > 32 {
            return Err(AccountResolutionError::SeedConfigsTooLarge.into());
        }
        let mut data = [0u8; 32];
        value
            .iter()
            .enumerate()
            .for_each(|(i, v)| data[i] = v.into());
        Ok(data)
    }

    /// Unpacks a vector of `Seed` configs from a slice
    pub fn unpack_to_vec(data: &[u8]) -> Result<Vec<Self>, ProgramError> {
        let len = data.len();
        // Length should be 32
        if len > 32 {
            return Err(AccountResolutionError::BufferTooLarge.into());
        }
        if len < 32 {
            return Err(AccountResolutionError::BufferTooSmall.into());
        }
        let mut res: Vec<Seed> = vec![];
        // We're expecting non-zero values for any seed configs,
        // then all zeroes for the rest of the buffer
        for byte in data.iter() {
            if *byte == 0 {
                break;
            }
            res.push(Self::from_u8(*byte)?);
        }
        Ok(res)
    }

    /// Converts a `u8` to a `Seed` and throws an error if
    /// the number is out of range
    pub fn from_u8(value: u8) -> Result<Self, ProgramError> {
        Ok(match value {
            1 => Seed::Lit,
            2 => Seed::Arg(SeedArgType::U8),
            3 => Seed::Arg(SeedArgType::U16),
            4 => Seed::Arg(SeedArgType::U32),
            5 => Seed::Arg(SeedArgType::U64),
            6 => Seed::Arg(SeedArgType::U128),
            7 => Seed::Arg(SeedArgType::String),
            8 => Seed::Arg(SeedArgType::Pubkey),
            _ => return Err(AccountResolutionError::InvalidByteValueForSeed.into()),
        })
    }
}

/// Enum to describe the type of required seed for a Program-Derived Address
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SeedArgType {
    /// A `u8` seed
    U8,
    /// A `u16` seed
    U16,
    /// A `u8` seed
    U32,
    /// A `u64` seed
    U64,
    /// A `u128` seed
    U128,
    /// A `String` seed
    String,
    /// A `Pubkey` seed
    Pubkey,
}

impl From<&SeedArgType> for u8 {
    fn from(value: &SeedArgType) -> Self {
        match value {
            SeedArgType::U8 => 2,
            SeedArgType::U16 => 3,
            SeedArgType::U32 => 4,
            SeedArgType::U64 => 5,
            SeedArgType::U128 => 6,
            SeedArgType::String => 7,
            SeedArgType::Pubkey => 8,
        }
    }
}

impl From<&Seed> for u8 {
    fn from(value: &Seed) -> Self {
        match value {
            Seed::Lit => 1,
            Seed::Arg(arg) => arg.into(),
        }
    }
}

/// Seed configurations to be provided as inputs to any functions that
/// attempt to add required accounts to an instruction.
///
/// Contains the seeds themselves and the types used to build them
pub struct SeedConfig {
    /// The `Seed` types used to create the seeds, so we can
    /// compare them against the validation account's stated seed
    /// configurations
    seed_types: Vec<Seed>,
    /// The seeds as vectors of `Vec<u8>` so we can use them
    /// in `Pubkey::find_program_address`
    byte_vectors: Vec<Vec<u8>>,
}
impl SeedConfig {
    /// Creates a new `SeedConfig` instance from any tuple of supported
    /// Rust types by the `ProvidedSeeds` trait (below)
    pub fn new(seeds: impl ProvidedSeeds) -> Self {
        Self {
            seed_types: seeds.seed_types(),
            byte_vectors: seeds.byte_vectors(),
        }
    }

    /// For any `SeedConfig` instance, evaluates a list of required seeds
    /// (from the validation account) against itself.
    ///
    /// If the evaluation passes, returns the PDA address
    pub fn evaluate(
        &self,
        program_id: &Pubkey,
        required_seeds: Vec<Seed>,
    ) -> Result<Pubkey, ProgramError> {
        if self.seed_types != required_seeds {
            return Err(AccountResolutionError::SeedsMismatch.into());
        }
        let seeds_bytes: Vec<&[u8]> = self.byte_vectors.iter().map(AsRef::as_ref).collect();
        Ok(Pubkey::find_program_address(&seeds_bytes, program_id).0)
    }
}

/// This trait allows you to provide varying sized tuples with
/// varying Rust types to serve as seeds.
///
/// As you can see from the implementations below, we are
/// currently supporting up to 10 seeds - which is probably
/// more than necessary
pub trait ProvidedSeeds {
    /// A vector of the type of seeds used to build the config
    fn seed_types(&self) -> Vec<Seed>;
    /// A vector of bytes for use in `Pubkey::find_program_address`
    fn byte_vectors(&self) -> Vec<Vec<u8>>;
}
impl<S0> ProvidedSeeds for (S0,)
where
    S0: ProvidedSeedType,
{
    fn seed_types(&self) -> Vec<Seed> {
        vec![self.0.seed_type()]
    }
    fn byte_vectors(&self) -> Vec<Vec<u8>> {
        vec![self.0.as_byte_vec()]
    }
}
impl<S0, S1> ProvidedSeeds for (S0, S1)
where
    S0: ProvidedSeedType,
    S1: ProvidedSeedType,
{
    fn seed_types(&self) -> Vec<Seed> {
        vec![self.0.seed_type(), self.1.seed_type()]
    }
    fn byte_vectors(&self) -> Vec<Vec<u8>> {
        vec![self.0.as_byte_vec(), self.1.as_byte_vec()]
    }
}
impl<S0, S1, S2> ProvidedSeeds for (S0, S1, S2)
where
    S0: ProvidedSeedType,
    S1: ProvidedSeedType,
    S2: ProvidedSeedType,
{
    fn seed_types(&self) -> Vec<Seed> {
        vec![self.0.seed_type(), self.1.seed_type(), self.2.seed_type()]
    }
    fn byte_vectors(&self) -> Vec<Vec<u8>> {
        vec![
            self.0.as_byte_vec(),
            self.1.as_byte_vec(),
            self.2.as_byte_vec(),
        ]
    }
}
impl<S0, S1, S2, S3> ProvidedSeeds for (S0, S1, S2, S3)
where
    S0: ProvidedSeedType,
    S1: ProvidedSeedType,
    S2: ProvidedSeedType,
    S3: ProvidedSeedType,
{
    fn seed_types(&self) -> Vec<Seed> {
        vec![
            self.0.seed_type(),
            self.1.seed_type(),
            self.2.seed_type(),
            self.3.seed_type(),
        ]
    }
    fn byte_vectors(&self) -> Vec<Vec<u8>> {
        vec![
            self.0.as_byte_vec(),
            self.1.as_byte_vec(),
            self.2.as_byte_vec(),
            self.3.as_byte_vec(),
        ]
    }
}
impl<S0, S1, S2, S3, S4> ProvidedSeeds for (S0, S1, S2, S3, S4)
where
    S0: ProvidedSeedType,
    S1: ProvidedSeedType,
    S2: ProvidedSeedType,
    S3: ProvidedSeedType,
    S4: ProvidedSeedType,
{
    fn seed_types(&self) -> Vec<Seed> {
        vec![
            self.0.seed_type(),
            self.1.seed_type(),
            self.2.seed_type(),
            self.3.seed_type(),
            self.4.seed_type(),
        ]
    }
    fn byte_vectors(&self) -> Vec<Vec<u8>> {
        vec![
            self.0.as_byte_vec(),
            self.1.as_byte_vec(),
            self.2.as_byte_vec(),
            self.3.as_byte_vec(),
            self.4.as_byte_vec(),
        ]
    }
}
impl<S0, S1, S2, S3, S4, S5> ProvidedSeeds for (S0, S1, S2, S3, S4, S5)
where
    S0: ProvidedSeedType,
    S1: ProvidedSeedType,
    S2: ProvidedSeedType,
    S3: ProvidedSeedType,
    S4: ProvidedSeedType,
    S5: ProvidedSeedType,
{
    fn seed_types(&self) -> Vec<Seed> {
        vec![
            self.0.seed_type(),
            self.1.seed_type(),
            self.2.seed_type(),
            self.3.seed_type(),
            self.4.seed_type(),
            self.5.seed_type(),
        ]
    }
    fn byte_vectors(&self) -> Vec<Vec<u8>> {
        vec![
            self.0.as_byte_vec(),
            self.1.as_byte_vec(),
            self.2.as_byte_vec(),
            self.3.as_byte_vec(),
            self.4.as_byte_vec(),
            self.5.as_byte_vec(),
        ]
    }
}
impl<S0, S1, S2, S3, S4, S5, S6> ProvidedSeeds for (S0, S1, S2, S3, S4, S5, S6)
where
    S0: ProvidedSeedType,
    S1: ProvidedSeedType,
    S2: ProvidedSeedType,
    S3: ProvidedSeedType,
    S4: ProvidedSeedType,
    S5: ProvidedSeedType,
    S6: ProvidedSeedType,
{
    fn seed_types(&self) -> Vec<Seed> {
        vec![
            self.0.seed_type(),
            self.1.seed_type(),
            self.2.seed_type(),
            self.3.seed_type(),
            self.4.seed_type(),
            self.5.seed_type(),
            self.6.seed_type(),
        ]
    }
    fn byte_vectors(&self) -> Vec<Vec<u8>> {
        vec![
            self.0.as_byte_vec(),
            self.1.as_byte_vec(),
            self.2.as_byte_vec(),
            self.3.as_byte_vec(),
            self.4.as_byte_vec(),
            self.5.as_byte_vec(),
            self.6.as_byte_vec(),
        ]
    }
}
impl<S0, S1, S2, S3, S4, S5, S6, S7> ProvidedSeeds for (S0, S1, S2, S3, S4, S5, S6, S7)
where
    S0: ProvidedSeedType,
    S1: ProvidedSeedType,
    S2: ProvidedSeedType,
    S3: ProvidedSeedType,
    S4: ProvidedSeedType,
    S5: ProvidedSeedType,
    S6: ProvidedSeedType,
    S7: ProvidedSeedType,
{
    fn seed_types(&self) -> Vec<Seed> {
        vec![
            self.0.seed_type(),
            self.1.seed_type(),
            self.2.seed_type(),
            self.3.seed_type(),
            self.4.seed_type(),
            self.5.seed_type(),
            self.6.seed_type(),
            self.7.seed_type(),
        ]
    }
    fn byte_vectors(&self) -> Vec<Vec<u8>> {
        vec![
            self.0.as_byte_vec(),
            self.1.as_byte_vec(),
            self.2.as_byte_vec(),
            self.3.as_byte_vec(),
            self.4.as_byte_vec(),
            self.5.as_byte_vec(),
            self.6.as_byte_vec(),
            self.7.as_byte_vec(),
        ]
    }
}
impl<S0, S1, S2, S3, S4, S5, S6, S7, S8> ProvidedSeeds for (S0, S1, S2, S3, S4, S5, S6, S7, S8)
where
    S0: ProvidedSeedType,
    S1: ProvidedSeedType,
    S2: ProvidedSeedType,
    S3: ProvidedSeedType,
    S4: ProvidedSeedType,
    S5: ProvidedSeedType,
    S6: ProvidedSeedType,
    S7: ProvidedSeedType,
    S8: ProvidedSeedType,
{
    fn seed_types(&self) -> Vec<Seed> {
        vec![
            self.0.seed_type(),
            self.1.seed_type(),
            self.2.seed_type(),
            self.3.seed_type(),
            self.4.seed_type(),
            self.5.seed_type(),
            self.6.seed_type(),
            self.7.seed_type(),
            self.8.seed_type(),
        ]
    }
    fn byte_vectors(&self) -> Vec<Vec<u8>> {
        vec![
            self.0.as_byte_vec(),
            self.1.as_byte_vec(),
            self.2.as_byte_vec(),
            self.3.as_byte_vec(),
            self.4.as_byte_vec(),
            self.5.as_byte_vec(),
            self.6.as_byte_vec(),
            self.7.as_byte_vec(),
            self.8.as_byte_vec(),
        ]
    }
}
impl<S0, S1, S2, S3, S4, S5, S6, S7, S8, S9> ProvidedSeeds
    for (S0, S1, S2, S3, S4, S5, S6, S7, S8, S9)
where
    S0: ProvidedSeedType,
    S1: ProvidedSeedType,
    S2: ProvidedSeedType,
    S3: ProvidedSeedType,
    S4: ProvidedSeedType,
    S5: ProvidedSeedType,
    S6: ProvidedSeedType,
    S7: ProvidedSeedType,
    S8: ProvidedSeedType,
    S9: ProvidedSeedType,
{
    fn seed_types(&self) -> Vec<Seed> {
        vec![
            self.0.seed_type(),
            self.1.seed_type(),
            self.2.seed_type(),
            self.3.seed_type(),
            self.4.seed_type(),
            self.5.seed_type(),
            self.6.seed_type(),
            self.7.seed_type(),
            self.8.seed_type(),
            self.9.seed_type(),
        ]
    }
    fn byte_vectors(&self) -> Vec<Vec<u8>> {
        vec![
            self.0.as_byte_vec(),
            self.1.as_byte_vec(),
            self.2.as_byte_vec(),
            self.3.as_byte_vec(),
            self.4.as_byte_vec(),
            self.5.as_byte_vec(),
            self.6.as_byte_vec(),
            self.7.as_byte_vec(),
            self.8.as_byte_vec(),
            self.9.as_byte_vec(),
        ]
    }
}

/// Trait to define the type of `Seed` for a given Rust type
pub trait ProvidedSeedType {
    /// Returns the type of seed as a `Seed` for comparing
    fn seed_type(&self) -> Seed;
    /// Turns the provided type into a `Vec<u8>` to use in
    /// `Pubkey::find_program_address`
    fn as_byte_vec(&self) -> Vec<u8>;
}
impl ProvidedSeedType for &str {
    fn seed_type(&self) -> Seed {
        Seed::Lit
    }
    fn as_byte_vec(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}
impl ProvidedSeedType for u8 {
    fn seed_type(&self) -> Seed {
        Seed::Arg(SeedArgType::U8)
    }
    fn as_byte_vec(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}
impl ProvidedSeedType for u16 {
    fn seed_type(&self) -> Seed {
        Seed::Arg(SeedArgType::U16)
    }
    fn as_byte_vec(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}
impl ProvidedSeedType for u32 {
    fn seed_type(&self) -> Seed {
        Seed::Arg(SeedArgType::U32)
    }
    fn as_byte_vec(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}
impl ProvidedSeedType for u64 {
    fn seed_type(&self) -> Seed {
        Seed::Arg(SeedArgType::U64)
    }
    fn as_byte_vec(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}
impl ProvidedSeedType for u128 {
    fn seed_type(&self) -> Seed {
        Seed::Arg(SeedArgType::U128)
    }
    fn as_byte_vec(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}
impl ProvidedSeedType for String {
    fn seed_type(&self) -> Seed {
        Seed::Arg(SeedArgType::String)
    }
    fn as_byte_vec(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}
impl ProvidedSeedType for Pubkey {
    fn seed_type(&self) -> Seed {
        Seed::Arg(SeedArgType::Pubkey)
    }
    fn as_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}
