//! Big vector type, used with Borsh vectors that can't be serde'd

use {
    arrayref::array_ref,
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{program_error::ProgramError, program_pack::Pack},
    std::marker::PhantomData,
};

/// Contains easy to use utilities for a big vector of Borsh-compatible types,
/// to avoid managing the entire struct on-chain and blow through stack limits.
pub struct BigVec<'a> {
    /// Underlying data buffer, pieces of which are serialized
    pub data: &'a mut [u8],
}

const VEC_SIZE_BYTES: usize = 4;

impl<'a> BigVec<'a> {
    /// Get the length of the vector
    pub fn len(&self) -> u32 {
        let vec_len = array_ref![self.data, 0, VEC_SIZE_BYTES];
        u32::from_le_bytes(*vec_len)
    }

    /// Find out if the vector has no contents (as demanded by clippy)
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Extracts a slice of the data types
    pub fn deserialize_mut_slice<T: Pack>(
        self,
        skip: usize,
        len: usize,
    ) -> Result<Vec<&'a mut T>, ProgramError> {
        let vec_len = self.len();
        if skip + len > vec_len as usize {
            return Err(ProgramError::AccountDataTooSmall);
        }

        let start_index = 4usize.saturating_add(skip.saturating_mul(T::LEN));
        let end_index = start_index.saturating_add(len.saturating_mul(T::LEN));
        let mut deserialized = vec![];
        for slice in self.data[start_index..end_index].chunks_exact_mut(T::LEN) {
            deserialized.push(unsafe { &mut *(slice.as_ptr() as *mut T) });
        }
        Ok(deserialized)
    }

    /// Writes slice data to some part of the buffer
    pub fn serialize_slice<T: BorshSerialize + Pack>(
        &mut self,
        skip: usize,
        slice: &[T],
    ) -> Result<(), ProgramError> {
        let vec_len = self.len();
        if skip + slice.len() > vec_len as usize {
            return Err(ProgramError::AccountDataTooSmall);
        }

        let instance_index = 4usize.saturating_add(skip.saturating_mul(T::LEN));
        let mut data_mut = &mut self.data[instance_index..];
        for instance in slice {
            instance.serialize(&mut data_mut)?;
        }
        Ok(())
    }

    /// Add new element to the end
    pub fn push<T: BorshSerialize + Pack>(&'a mut self, element: T) -> Result<(), ProgramError> {
        let mut vec_len_ref = &mut self.data[0..VEC_SIZE_BYTES];
        let mut vec_len = u32::try_from_slice(vec_len_ref)?;

        let start_index = VEC_SIZE_BYTES + vec_len as usize * T::LEN;
        let end_index = start_index + T::LEN;

        vec_len += 1;
        vec_len.serialize(&mut vec_len_ref)?;

        if self.data.len() < end_index {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let mut element_ref = &mut self.data[start_index..start_index + T::LEN];
        element.serialize(&mut element_ref)?;
        Ok(())
    }

    /// Rewrite the vec to remove an instance
    pub fn remove<T: Pack>(&'a mut self, _index: usize) {}

    /// Get an iterator for the type provided
    pub fn iter<T: BorshDeserialize + Pack>(&'a self) -> Iter<'a, T> {
        Iter {
            len: self.len() as usize,
            current: 0,
            current_index: VEC_SIZE_BYTES,
            inner: self,
            phantom: PhantomData,
        }
    }

    /// Get a mutable iterator for the type provided
    pub fn iter_mut<T: BorshDeserialize + Pack>(&'a mut self) -> IterMut<'a, T> {
        IterMut {
            len: self.len() as usize,
            current: 0,
            current_index: VEC_SIZE_BYTES,
            inner: self,
            phantom: PhantomData,
        }
    }

    /// Find matching data in the array
    pub fn find<'b, T: Pack>(
        &'a self,
        data: &'b [u8],
        predicate: fn(&[u8], &[u8]) -> bool,
    ) -> Option<&T> {
        let len = self.len() as usize;
        let mut current = 0;
        let mut current_index = VEC_SIZE_BYTES;
        while current != len {
            let end_index = current_index + T::LEN;
            let current_slice = &self.data[current_index..end_index];
            if predicate(current_slice, data) {
                return Some(unsafe { &*(current_slice.as_ptr() as *const T) });
            }
            current_index = end_index;
            current += 1;
        }
        None
    }

    /// Find matching data in the array
    pub fn find_mut<'b, T: Pack>(
        &'a mut self,
        data: &'b [u8],
        predicate: fn(&[u8], &[u8]) -> bool,
    ) -> Option<&mut T> {
        let len = self.len() as usize;
        let mut current = 0;
        let mut current_index = VEC_SIZE_BYTES;
        while current != len {
            let end_index = current_index + T::LEN;
            let current_slice = &self.data[current_index..end_index];
            if predicate(current_slice, data) {
                return Some(unsafe { &mut *(current_slice.as_ptr() as *mut T) });
            }
            current_index = end_index;
            current += 1;
        }
        None
    }
}

/// Iterator wrapper over a BigVec
pub struct Iter<'a, T> {
    len: usize,
    current: usize,
    current_index: usize,
    inner: &'a BigVec<'a>,
    phantom: PhantomData<T>,
}

impl<'a, T: Pack + 'a> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.len {
            None
        } else {
            let end_index = self.current_index + T::LEN;
            let value = Some(unsafe {
                &*(self.inner.data[self.current_index..end_index].as_ptr() as *const T)
            });
            self.current += 1;
            self.current_index = end_index;
            value
        }
    }
}

/// Iterator wrapper over a BigVec
pub struct IterMut<'a, T> {
    len: usize,
    current: usize,
    current_index: usize,
    inner: &'a mut BigVec<'a>,
    phantom: PhantomData<T>,
}

impl<'a, T: Pack + 'a> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.len {
            None
        } else {
            let end_index = self.current_index + T::LEN;
            let value = Some(unsafe {
                &mut *(self.inner.data[self.current_index..end_index].as_ptr() as *mut T)
            });
            self.current += 1;
            self.current_index = end_index;
            value
        }
    }
}
