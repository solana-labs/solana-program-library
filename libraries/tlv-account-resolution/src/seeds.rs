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

    fn derive_address<S: ProvidedSeedType>(program_id: &Pubkey, seeds: &Vec<S>) -> Pubkey {
        // To avoid temporary values, this has to be done over two lines. Hopefully can enhance this
        let seeds_vec: Vec<Vec<u8>> = seeds.iter().map(ProvidedSeedType::as_byte_vec).collect();
        let seeds_bytes: Vec<&[u8]> = seeds_vec.iter().map(AsRef::as_ref).collect();
        Pubkey::find_program_address(&seeds_bytes, program_id).0
    }

    fn match_seed_types<S: ProvidedSeedType>(required_seeds: &[Seed], seeds: &[S]) -> bool {
        seeds
            .iter()
            .map(ProvidedSeedType::cmp)
            .eq(required_seeds.iter())
    }

    /// Checks through a list of provided seeds and evaluates whether
    /// all provided seeds are the correct type and attempts to derive
    /// a program-derived address and match it with the provided pubkey
    pub fn evaluate<S: ProvidedSeedType>(
        program_id: &Pubkey,
        address: &Pubkey,
        required_seeds: Vec<Seed>,
        provided_seeds: Vec<Vec<S>>,
    ) -> Result<Pubkey, ProgramError> {
        for seeds_list in provided_seeds {
            // If they are the same length, continue
            if seeds_list.len() == required_seeds.len() {
                // If they are the same types in the same order, continue
                if Self::match_seed_types(required_seeds.as_slice(), seeds_list.as_slice()) {
                    // If the address matches the derived address, return the address
                    let derived_address = Self::derive_address(program_id, &seeds_list);
                    if address.eq(&derived_address) {
                        return Ok(derived_address);
                    }
                }
            }
        }
        Err(AccountResolutionError::NoProvidedSeedsMatched.into())
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

/// Trait to define the type of a seed for a given Rust type
pub trait ProvidedSeedType {
    /// Returns the type of seed as a `Seed` for comparing
    fn cmp(&self) -> &Seed;
    /// Turns the provided type into a `Vec<u8>` to use in
    /// `Pubkey::find_program_address`
    fn as_byte_vec(&self) -> Vec<u8>;
}
impl ProvidedSeedType for &str {
    fn cmp(&self) -> &Seed {
        &Seed::Lit
    }
    fn as_byte_vec(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}
impl ProvidedSeedType for u8 {
    fn cmp(&self) -> &Seed {
        &Seed::Arg(SeedArgType::U8)
    }
    fn as_byte_vec(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}
impl ProvidedSeedType for u16 {
    fn cmp(&self) -> &Seed {
        &Seed::Arg(SeedArgType::U16)
    }
    fn as_byte_vec(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}
impl ProvidedSeedType for u32 {
    fn cmp(&self) -> &Seed {
        &Seed::Arg(SeedArgType::U32)
    }
    fn as_byte_vec(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}
impl ProvidedSeedType for u64 {
    fn cmp(&self) -> &Seed {
        &Seed::Arg(SeedArgType::U64)
    }
    fn as_byte_vec(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}
impl ProvidedSeedType for u128 {
    fn cmp(&self) -> &Seed {
        &Seed::Arg(SeedArgType::U128)
    }
    fn as_byte_vec(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}
impl ProvidedSeedType for String {
    fn cmp(&self) -> &Seed {
        &Seed::Arg(SeedArgType::String)
    }
    fn as_byte_vec(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}
impl ProvidedSeedType for Pubkey {
    fn cmp(&self) -> &Seed {
        &Seed::Arg(SeedArgType::Pubkey)
    }
    fn as_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

/// This trait allows you to provide varying sized tuples with
/// varying Rust types to serve as seeds.
///
/// As you can see from the implementations below, we are
/// currently supporting up to 10 seeds - which is probably
/// more than necessary
pub trait ProvidedSeeds<S: ProvidedSeedType> {
    fn as_vec_of_bytes_vecs(&self) -> Vec<Vec<u8>>;
}
impl<S: ProvidedSeedType> ProvidedSeeds<S> for S {
    fn as_vec_of_bytes_vecs(&self) -> Vec<Vec<u8>> {
        vec![self.as_byte_vec()]
    }
}
impl<S: ProvidedSeedType> ProvidedSeeds<S> for (S, S) {
    fn as_vec_of_bytes_vecs(&self) -> Vec<Vec<u8>> {
        vec![self.0.as_byte_vec(), self.1.as_byte_vec()]
    }
}
impl<S: ProvidedSeedType> ProvidedSeeds<S> for (S, S, S) {
    fn as_vec_of_bytes_vecs(&self) -> Vec<Vec<u8>> {
        vec![
            self.0.as_byte_vec(),
            self.1.as_byte_vec(),
            self.2.as_byte_vec(),
        ]
    }
}
impl<S: ProvidedSeedType> ProvidedSeeds<S> for (S, S, S, S) {
    fn as_vec_of_bytes_vecs(&self) -> Vec<Vec<u8>> {
        vec![
            self.0.as_byte_vec(),
            self.1.as_byte_vec(),
            self.2.as_byte_vec(),
            self.3.as_byte_vec(),
        ]
    }
}
impl<S: ProvidedSeedType> ProvidedSeeds<S> for (S, S, S, S, S) {
    fn as_vec_of_bytes_vecs(&self) -> Vec<Vec<u8>> {
        vec![
            self.0.as_byte_vec(),
            self.1.as_byte_vec(),
            self.2.as_byte_vec(),
            self.3.as_byte_vec(),
            self.4.as_byte_vec(),
        ]
    }
}
impl<S: ProvidedSeedType> ProvidedSeeds<S> for (S, S, S, S, S, S) {
    fn as_vec_of_bytes_vecs(&self) -> Vec<Vec<u8>> {
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
impl<S: ProvidedSeedType> ProvidedSeeds<S> for (S, S, S, S, S, S, S) {
    fn as_vec_of_bytes_vecs(&self) -> Vec<Vec<u8>> {
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
impl<S: ProvidedSeedType> ProvidedSeeds<S> for (S, S, S, S, S, S, S, S) {
    fn as_vec_of_bytes_vecs(&self) -> Vec<Vec<u8>> {
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
impl<S: ProvidedSeedType> ProvidedSeeds<S> for (S, S, S, S, S, S, S, S, S) {
    fn as_vec_of_bytes_vecs(&self) -> Vec<Vec<u8>> {
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
impl<S: ProvidedSeedType> ProvidedSeeds<S> for (S, S, S, S, S, S, S, S, S, S) {
    fn as_vec_of_bytes_vecs(&self) -> Vec<Vec<u8>> {
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
