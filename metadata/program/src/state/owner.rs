use super::UNINITIALIZED_VERSION;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// STRUCT VERSION
pub const OWNER_VERSION: u8 = 1;

/// Metadata
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Owner {
    ///version
    pub version: u8,
    /// owner
    pub owner: Pubkey,
    /// metadata
    pub metadata: Pubkey,
}

impl Sealed for Owner {}
impl IsInitialized for Owner {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

/// Len of  owner
pub const OWNER_LEN: usize = 1 + 32 + 32 + 100;
impl Pack for Owner {
    const LEN: usize = 1 + 32 + 32 + 100;
    /// Unpacks a byte buffer into a [Owner](struct.Owner.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, OWNER_LEN];
        // TODO think up better way than txn_* usage here - new to rust
        #[allow(clippy::ptr_offset_with_cast)]
        let (version, owner, metadata, _padding) = array_refs![input, 1, 32, 32, 100];
        let version = u8::from_le_bytes(*version);

        match version {
            OWNER_VERSION | UNINITIALIZED_VERSION => Ok(Self {
                version,
                owner: Pubkey::new_from_array(*owner),
                metadata: Pubkey::new_from_array(*metadata),
            }),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, OWNER_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (version, owner, metadata, _padding) = mut_array_refs![output, 1, 32, 32, 100];
        *version = self.version.to_le_bytes();
        owner.copy_from_slice(self.owner.as_ref());
        metadata.copy_from_slice(self.metadata.as_ref());
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
