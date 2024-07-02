//! Crate containing `Pod` types and `bytemuck` utils used in SPL

pub mod bytemuck;
pub mod error;
pub mod option;
pub mod optional_keys;
pub mod primitives;
pub mod slice;

// Export current sdk types for downstream users building with a different sdk
// version
pub use solana_program;
