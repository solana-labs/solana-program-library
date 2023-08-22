//! Special types for working with slices of `Pod`s

use {
    crate::{
        bytemuck::{
            pod_from_bytes, pod_from_bytes_mut, pod_slice_from_bytes, pod_slice_from_bytes_mut,
        },
        primitives::PodU32,
        error::PodSliceError
    },
    bytemuck::Pod,
    solana_program::program_error::ProgramError,
};

const LENGTH_SIZE: usize = std::mem::size_of::<PodU32>();
/// Special type for using a slice of `Pod`s in a zero-copy way
pub struct PodSlice<'data, T: Pod> {
    length: &'data PodU32,
    data: &'data [T],
}
impl<'data, T: Pod> PodSlice<'data, T> {
    /// Unpack the buffer into a slice
    pub fn unpack<'a>(data: &'a [u8]) -> Result<Self, ProgramError>
    where
        'a: 'data,
    {
        if data.len() < LENGTH_SIZE {
            return Err(PodSliceError::BufferTooSmall.into());
        }
        let (length, data) = data.split_at(LENGTH_SIZE);
        let length = pod_from_bytes::<PodU32>(length)?;
        let _max_length = max_len_for_type::<T>(data.len())?;
        let data = pod_slice_from_bytes(data)?;
        Ok(Self { length, data })
    }

    /// Get the slice data
    pub fn data(&self) -> &[T] {
        let length = u32::from(*self.length) as usize;
        &self.data[..length]
    }

    /// Get the amount of bytes used by `num_items`
    pub fn size_of(num_items: usize) -> Result<usize, ProgramError> {
        std::mem::size_of::<T>()
            .checked_mul(num_items)
            .and_then(|len| len.checked_add(LENGTH_SIZE))
            .ok_or_else(|| PodSliceError::CalculationFailure.into())
    }
}

/// Special type for using a slice of mutable `Pod`s in a zero-copy way
pub struct PodSliceMut<'data, T: Pod> {
    length: &'data mut PodU32,
    data: &'data mut [T],
    max_length: usize,
}
impl<'data, T: Pod> PodSliceMut<'data, T> {
    /// Unpack the mutable buffer into a mutable slice, with the option to
    /// initialize the data
    fn unpack_internal<'a>(data: &'a mut [u8], init: bool) -> Result<Self, ProgramError>
    where
        'a: 'data,
    {
        if data.len() < LENGTH_SIZE {
            return Err(PodSliceError::BufferTooSmall.into());
        }
        let (length, data) = data.split_at_mut(LENGTH_SIZE);
        let length = pod_from_bytes_mut::<PodU32>(length)?;
        if init {
            *length = 0.into();
        }
        let max_length = max_len_for_type::<T>(data.len())?;
        let data = pod_slice_from_bytes_mut(data)?;
        Ok(Self {
            length,
            data,
            max_length,
        })
    }

    /// Unpack the mutable buffer into a mutable slice
    pub fn unpack<'a>(data: &'a mut [u8]) -> Result<Self, ProgramError>
    where
        'a: 'data,
    {
        Self::unpack_internal(data, /* init */ false)
    }

    /// Unpack the mutable buffer into a mutable slice, and initialize the
    /// slice to 0-length
    pub fn init<'a>(data: &'a mut [u8]) -> Result<Self, ProgramError>
    where
        'a: 'data,
    {
        Self::unpack_internal(data, /* init */ true)
    }

    /// Add another item to the slice
    pub fn push(&mut self, t: T) -> Result<(), ProgramError> {
        let length = u32::from(*self.length);
        if length as usize == self.max_length {
            Err(PodSliceError::BufferTooSmall.into())
        } else {
            self.data[length as usize] = t;
            *self.length = length.saturating_add(1).into();
            Ok(())
        }
    }
}

fn max_len_for_type<T>(data_len: usize) -> Result<usize, ProgramError> {
    let size: usize = std::mem::size_of::<T>();
    let max_len = data_len
        .checked_div(size)
        .ok_or(PodSliceError::CalculationFailure)?;
    // check that it isn't overallocated
    if max_len.saturating_mul(size) != data_len {
        Err(PodSliceError::BufferTooLarge.into())
    } else {
        Ok(max_len)
    }
}
