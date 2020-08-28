//! State transition types

use crate::error::TokenError;
use solana_sdk::program_error::ProgramError;

/// Check is a token state is initialized
pub trait IsInitialized {
    /// Is initialized
    fn is_initialized(&self) -> bool;
}

/// Depends on Sized
pub trait Sealed: Sized {}

/// Safely and efficiently (de)serialize account state
pub trait Pack: Sealed {
    /// The length, in bytes, of the packed representation
    const LEN: usize;
    #[doc(hidden)]
    fn pack_into_slice(&self, dst: &mut [u8]);
    #[doc(hidden)]
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError>;

    /// Unpack from slice and check if initialized
    fn unpack(input: &[u8]) -> Result<Self, ProgramError>
    where
        Self: IsInitialized,
    {
        let value = Self::unpack_unchecked(input)?;
        if value.is_initialized() {
            Ok(value)
        } else {
            Err(TokenError::UninitializedState.into())
        }
    }

    /// Unpack from slice without checking if initialized
    fn unpack_unchecked(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < Self::LEN {
            println!("ilen {:?} tlen {:?}", input.len(), Self::LEN);
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(Self::unpack_from_slice(input)?)
    }

    /// Borrow `Self` from `input` for the duration of the call to `f`, but first check that `Self`
    /// is initialized
    #[inline(never)]
    fn unpack_mut<F, U>(input: &mut [u8], f: &mut F) -> Result<U, ProgramError>
    where
        F: FnMut(&mut Self) -> Result<U, ProgramError>,
        Self: IsInitialized,
    {
        let mut t = Self::unpack(input)?;
        let u = f(&mut t)?;
        Self::pack(t, input)?;
        Ok(u)
    }

    /// Borrow `Self` from `input` for the duration of the call to `f`, without checking that
    /// `Self` has been initialized
    #[inline(never)]
    fn unpack_unchecked_mut<F, U>(input: &mut [u8], f: &mut F) -> Result<U, ProgramError>
    where
        F: FnMut(&mut Self) -> Result<U, ProgramError>,
    {
        let mut t = Self::unpack_unchecked(input)?;
        let u = f(&mut t)?;
        Self::pack(t, input)?;
        Ok(u)
    }

    /// Pack into slice
    fn pack(src: Self, dst: &mut [u8]) -> Result<(), ProgramError> {
        if dst.len() < Self::LEN {
            println!("dlen {:?} tlen {:?}", dst.len(), Self::LEN);
            return Err(ProgramError::InvalidAccountData);
        }
        src.pack_into_slice(dst);
        Ok(())
    }
}
