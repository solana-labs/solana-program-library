use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

use super::UNINITIALIZED_VERSION;
/// Timelock version
pub const TIMELOCK_VERSION: u8 = 1;
/// Global app state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelockProgram {
    /// Version of app
    pub version: u8,
    /// program id
    pub program_id: Pubkey,
    /// token_program_key
    pub token_program_id: Pubkey,
}
impl Sealed for TimelockProgram {}
impl IsInitialized for TimelockProgram {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

const TIMELOCK_LEN: usize = 65 + 300;
impl Pack for TimelockProgram {
    const LEN: usize = 65 + 300;
    /// Unpacks a byte buffer into a [TimelockProgram](struct.TimelockProgram.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, TIMELOCK_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (version, program_id, token_program_id, _padding) = array_refs![input, 1, 32, 32, 300];
        let version = u8::from_le_bytes(*version);
        match version {
            TIMELOCK_VERSION | UNINITIALIZED_VERSION => Ok(Self {
                version,
                program_id: Pubkey::new_from_array(*program_id),
                token_program_id: Pubkey::new_from_array(*token_program_id),
            }),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, TIMELOCK_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (version, program_id, token_program_id, _padding) =
            mut_array_refs![output, 1, 32, 32, 300];
        *version = self.version.to_le_bytes();
        program_id.copy_from_slice(self.program_id.as_ref());
        token_program_id.copy_from_slice(self.token_program_id.as_ref())
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
