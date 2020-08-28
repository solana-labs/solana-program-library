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

    /// Borrow `Self` from `input` for the duration of the call to `f`, but first check that `Self`
    /// is initialized
    #[inline(never)]
    fn unpack_mut<F, U>(input: &mut [u8], f: &mut F) -> Result<U, ProgramError>
    where
        F: FnMut(&mut Self) -> Result<U, ProgramError>,
        Self: IsInitialized,
    {
        let mut t = unpack(input)?;
        let u = f(&mut t)?;
        pack(t, input)?;
        Ok(u)
    }

    /// Borrow `Self` from `input` for the duration of the call to `f`, without checking that
    /// `Self` has been initialized
    #[inline(never)]
    fn unpack_unchecked_mut<F, U>(input: &mut [u8], f: &mut F) -> Result<U, ProgramError>
    where
        F: FnMut(&mut Self) -> Result<U, ProgramError>,
    {
        let mut t = unpack_unchecked(input)?;
        let u = f(&mut t)?;
        pack(t, input)?;
        Ok(u)
    }
}

fn pack<T: Pack>(src: T, dst: &mut [u8]) -> Result<(), ProgramError> {
    if dst.len() < T::LEN {
        println!("dlen {:?} tlen {:?}", dst.len(), T::LEN);
        return Err(ProgramError::InvalidAccountData);
    }
    src.pack_into_slice(dst);
    Ok(())
}

fn unpack<T: Pack + IsInitialized>(input: &[u8]) -> Result<T, ProgramError> {
    let value: T = unpack_unchecked(input)?;
    if value.is_initialized() {
        Ok(value)
    } else {
        Err(TokenError::UninitializedState.into())
    }
}

fn unpack_unchecked<T: Pack>(input: &[u8]) -> Result<T, ProgramError> {
    if input.len() < T::LEN {
        println!("ilen {:?} tlen {:?}", input.len(), T::LEN);
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(T::unpack_from_slice(input)?)
}
