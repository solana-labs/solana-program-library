//! Pod types to be used with bytemuck for zero-copy serde

use {
    bytemuck::{Pod, Zeroable},
    solana_program::program_error::ProgramError,
};

/// Convert a slice into a `Pod` (zero copy)
pub fn pod_from_bytes<T: Pod>(bytes: &[u8]) -> Result<&T, ProgramError> {
    bytemuck::try_from_bytes(bytes).map_err(|_| ProgramError::InvalidArgument)
}
/// Convert a slice into a mutable `Pod` (zero copy)
pub fn pod_from_bytes_mut<T: Pod>(bytes: &mut [u8]) -> Result<&mut T, ProgramError> {
    bytemuck::try_from_bytes_mut(bytes).map_err(|_| ProgramError::InvalidArgument)
}

/// Simple macro for implementing conversion functions between Pod* ints and standard ints.
///
/// The standard int types can cause alignment issues when placed in a `Pod`,
/// so these replacements are usable in all `Pod`s.
macro_rules! impl_int_conversion {
    ($P:ty, $I:ty) => {
        impl From<$I> for $P {
            fn from(n: $I) -> Self {
                Self(n.to_le_bytes())
            }
        }
        impl From<$P> for $I {
            fn from(pod: $P) -> Self {
                Self::from_le_bytes(pod.0)
            }
        }
    };
}

/// `u32` type that can be used in `Pod`s
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodU32([u8; 4]);
impl_int_conversion!(PodU32, u32);
