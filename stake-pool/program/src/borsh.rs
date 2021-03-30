//! Extra borsh utils
//! TODO delete once try_from_slice_unchecked has been published

use {
    borsh::{maybestd::io::Error, BorshDeserialize, BorshSerialize},
    std::io::{Result as IoResult, Write},
};

/// Deserializes something and allows for incomplete reading
pub fn try_from_slice_unchecked<T: BorshDeserialize>(data: &[u8]) -> Result<T, Error> {
    let mut data_mut = data;
    let result = T::deserialize(&mut data_mut)?;
    Ok(result)
}

/// Helper struct which to count how much data would be written during serialization
#[derive(Default)]
struct WriteCounter {
    count: usize,
}

impl Write for WriteCounter {
    fn write(&mut self, data: &[u8]) -> IoResult<usize> {
        let amount = data.len();
        self.count += amount;
        Ok(amount)
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}

/// Get the worst-case packed length for the given BorshSchema
pub fn get_instance_packed_len<T: BorshSerialize>(instance: &T) -> Result<usize, Error> {
    let mut counter = WriteCounter::default();
    instance.serialize(&mut counter)?;
    Ok(counter.count)
}
