use super::UNINITIALIZED_VERSION;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// STRUCT VERSION
pub const METADATA_VERSION: u8 = 1;

/// max name length
pub const NAME_LENGTH: usize = 32;

/// max symbol length
pub const SYMBOL_LENGTH: usize = 10;

/// max uri length
pub const URI_LENGTH: usize = 200;

/// Metadata
#[derive(Clone)]
pub struct Metadata {
    ///version
    pub version: u8,
    /// mint
    pub mint: Pubkey,
    /// name
    pub name: [u8; NAME_LENGTH],
    /// symbol
    pub symbol: [u8; SYMBOL_LENGTH],
    /// uri
    pub uri: [u8; URI_LENGTH],
}

impl Sealed for Metadata {}
impl IsInitialized for Metadata {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

/// Len of  metadata config
pub const METADATA_LEN: usize = 1 + 32 + NAME_LENGTH + SYMBOL_LENGTH + URI_LENGTH + 200;
impl Pack for Metadata {
    const LEN: usize = 1 + 32 + NAME_LENGTH + SYMBOL_LENGTH + URI_LENGTH + 200;
    /// Unpacks a byte buffer into a [Metadata](struct.Metadata.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, METADATA_LEN];
        // TODO think up better way than txn_* usage here - new to rust
        #[allow(clippy::ptr_offset_with_cast)]
        let (version, mint, name, symbol, uri, _padding) =
            array_refs![input, 1, 32, NAME_LENGTH, SYMBOL_LENGTH, URI_LENGTH, 200];
        let version = u8::from_le_bytes(*version);

        match version {
            METADATA_VERSION | UNINITIALIZED_VERSION => Ok(Self {
                version,
                mint: Pubkey::new_from_array(*mint),
                name: *name,
                symbol: *symbol,
                uri: *uri,
            }),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, METADATA_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (version, mint, name, symbol, uri, _padding) =
            mut_array_refs![output, 1, 32, NAME_LENGTH, SYMBOL_LENGTH, URI_LENGTH, 200];
        *version = self.version.to_le_bytes();
        mint.copy_from_slice(self.mint.as_ref());
        name.copy_from_slice(self.name.as_ref());
        symbol.copy_from_slice(self.symbol.as_ref());
        uri.copy_from_slice(self.uri.as_ref());
    }

    fn get_packed_len() -> usize {
        Self::LEN
    }

    fn unpack(input: &[u8]) -> Result<Self, ProgramError>
    where
        Self: IsInitialized,
    {
        let value = Self::unpack_unchecked(input)?;
        if value.is_initialized() {
            Ok(value)
        } else {
            Err(ProgramError::UninitializedAccount)
        }
    }

    fn unpack_unchecked(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(Self::unpack_from_slice(input)?)
    }

    fn pack(src: Self, dst: &mut [u8]) -> Result<(), ProgramError> {
        if dst.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        src.pack_into_slice(dst);
        Ok(())
    }
}
