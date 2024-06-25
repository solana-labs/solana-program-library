//! Special types for working with slices of `Pod`s

use {
    crate::{
        bytemuck::{
            pod_from_bytes, pod_from_bytes_mut, pod_slice_from_bytes, pod_slice_from_bytes_mut,
        },
        error::PodSliceError,
        primitives::PodU32,
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
    // check that it isn't over or under allocated
    if max_len.saturating_mul(size) != data_len {
        if max_len == 0 {
            // Size of T is greater than buffer size
            Err(PodSliceError::BufferTooSmall.into())
        } else {
            Err(PodSliceError::BufferTooLarge.into())
        }
    } else {
        Ok(max_len)
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::bytemuck::pod_slice_to_bytes,
        bytemuck_derive::{Pod, Zeroable},
    };

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
    struct TestStruct {
        test_field: u8,
        test_pubkey: [u8; 32],
    }

    #[test]
    fn test_pod_slice() {
        let test_field_bytes = [0];
        let test_pubkey_bytes = [1; 32];
        let len_bytes = [2, 0, 0, 0];

        // Slice will contain 2 `TestStruct`
        let mut data_bytes = [0; 66];
        data_bytes[0..1].copy_from_slice(&test_field_bytes);
        data_bytes[1..33].copy_from_slice(&test_pubkey_bytes);
        data_bytes[33..34].copy_from_slice(&test_field_bytes);
        data_bytes[34..66].copy_from_slice(&test_pubkey_bytes);

        let mut pod_slice_bytes = [0; 70];
        pod_slice_bytes[0..4].copy_from_slice(&len_bytes);
        pod_slice_bytes[4..70].copy_from_slice(&data_bytes);

        let pod_slice = PodSlice::<TestStruct>::unpack(&pod_slice_bytes).unwrap();
        let pod_slice_data = pod_slice.data();

        assert_eq!(*pod_slice.length, PodU32::from(2));
        assert_eq!(pod_slice_to_bytes(pod_slice.data()), data_bytes);
        assert_eq!(pod_slice_data[0].test_field, test_field_bytes[0]);
        assert_eq!(pod_slice_data[0].test_pubkey, test_pubkey_bytes);
        assert_eq!(PodSlice::<TestStruct>::size_of(1).unwrap(), 37);
    }

    #[test]
    fn test_pod_slice_buffer_too_large() {
        // 1 `TestStruct` + length = 37 bytes
        // we pass 38 to trigger BufferTooLarge
        let pod_slice_bytes = [1; 38];
        let err = PodSlice::<TestStruct>::unpack(&pod_slice_bytes)
            .err()
            .unwrap();
        assert_eq!(
            err,
            PodSliceError::BufferTooLarge.into(),
            "Expected an `PodSliceError::BufferTooLarge` error"
        );
    }

    #[test]
    fn test_pod_slice_buffer_too_small() {
        // 1 `TestStruct` + length = 37 bytes
        // we pass 36 to trigger BufferTooSmall
        let pod_slice_bytes = [1; 36];
        let err = PodSlice::<TestStruct>::unpack(&pod_slice_bytes)
            .err()
            .unwrap();
        assert_eq!(
            err,
            PodSliceError::BufferTooSmall.into(),
            "Expected an `PodSliceError::BufferTooSmall` error"
        );
    }

    #[test]
    fn test_pod_slice_mut() {
        // slice can fit 2 `TestStruct`
        let mut pod_slice_bytes = [0; 70];
        // set length to 1, so we have room to push 1 more item
        let len_bytes = [1, 0, 0, 0];
        pod_slice_bytes[0..4].copy_from_slice(&len_bytes);

        let mut pod_slice = PodSliceMut::<TestStruct>::unpack(&mut pod_slice_bytes).unwrap();

        assert_eq!(*pod_slice.length, PodU32::from(1));
        pod_slice.push(TestStruct::default()).unwrap();
        assert_eq!(*pod_slice.length, PodU32::from(2));
        let err = pod_slice
            .push(TestStruct::default())
            .expect_err("Expected an `PodSliceError::BufferTooSmall` error");
        assert_eq!(err, PodSliceError::BufferTooSmall.into());
    }
}
