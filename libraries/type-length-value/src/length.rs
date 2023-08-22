//! Module for the length portion of a Type-Length-Value structure
use {
    bytemuck::{Pod, Zeroable},
    solana_program::program_error::ProgramError,
    spl_pod::primitives::PodU32,
};

/// Length in TLV structure
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct Length(PodU32);
impl TryFrom<Length> for usize {
    type Error = ProgramError;
    fn try_from(n: Length) -> Result<Self, Self::Error> {
        Self::try_from(u32::from(n.0)).map_err(|_| ProgramError::AccountDataTooSmall)
    }
}
impl TryFrom<usize> for Length {
    type Error = ProgramError;
    fn try_from(n: usize) -> Result<Self, Self::Error> {
        u32::try_from(n)
            .map(|v| Self(PodU32::from(v)))
            .map_err(|_| ProgramError::AccountDataTooSmall)
    }
}
