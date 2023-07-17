//! The [`VariableLenPack`] serialization trait.

use solana_program::program_error::ProgramError;

/// Trait that mimics a lot of the functionality of
/// `solana_program::program_pack::Pack` but specifically works for
/// variable-size types.
pub trait VariableLenPack {
    /// Writes the serialized form of the instance into the given slice
    fn pack_into_slice(&self, dst: &mut [u8]) -> Result<(), ProgramError>;

    /// Deserializes the type from the given slice
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError>
    where
        Self: Sized;

    /// Gets the packed length for a given instance of the type
    fn get_packed_len(&self) -> Result<usize, ProgramError>;

    /// Safely write the contents to the type into the given slice
    fn pack(&self, dst: &mut [u8]) -> Result<(), ProgramError> {
        if dst.len() != self.get_packed_len()? {
            return Err(ProgramError::InvalidAccountData);
        }
        self.pack_into_slice(dst)
    }
}
