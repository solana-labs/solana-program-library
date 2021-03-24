//! Extra borsh utils
//! TODO delete once try_from_slice_unchecked has been published

use borsh::{maybestd::io::Error, BorshDeserialize};

/// Deserializes something and allows for incomplete reading
pub fn try_from_slice_unchecked<T: BorshDeserialize>(data: &[u8]) -> Result<T, Error> {
    let mut data_mut = data;
    let result = T::deserialize(&mut data_mut)?;
    Ok(result)
}
