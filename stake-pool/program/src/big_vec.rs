//! Big vector type, used with vectors that can't be serde'd

use {
    arrayref::array_ref,
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        program_error::ProgramError, program_memory::sol_memmove, program_pack::Pack,
    },
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

    /// Retain all elements that match the provided function, discard all others
    pub fn retain<T: Pack>(&mut self, predicate: fn(&[u8]) -> bool) -> Result<(), ProgramError> {
        let mut vec_len = self.len();
        let mut removals_found = 0;
        let mut dst_start_index = 0;

        let data_start_index = VEC_SIZE_BYTES;
        let data_end_index =
            data_start_index.saturating_add((vec_len as usize).saturating_mul(T::LEN));
        for start_index in (data_start_index..data_end_index).step_by(T::LEN) {
            let end_index = start_index + T::LEN;
            let slice = &self.data[start_index..end_index];
            if !predicate(slice) {
                let gap = removals_found * T::LEN;
                if removals_found > 0 {
                    // In case the compute budget is ever bumped up, allowing us
                    // to use this safe code instead:
                    // self.data.copy_within(dst_start_index + gap..start_index, dst_start_index);
                    unsafe {
                        sol_memmove(
                            self.data[dst_start_index..start_index - gap].as_mut_ptr(),
                            self.data[dst_start_index + gap..start_index].as_mut_ptr(),
                            start_index - gap - dst_start_index,
                        );
                    }
                }
                dst_start_index = start_index - gap;
                removals_found += 1;
                vec_len -= 1;
            }
        }

        // final memmove
        if removals_found > 0 {
            let gap = removals_found * T::LEN;
            // In case the compute budget is ever bumped up, allowing us
            // to use this safe code instead:
            //self.data.copy_within(dst_start_index + gap..data_end_index, dst_start_index);
            unsafe {
                sol_memmove(
                    self.data[dst_start_index..data_end_index - gap].as_mut_ptr(),
                    self.data[dst_start_index + gap..data_end_index].as_mut_ptr(),
                    data_end_index - gap - dst_start_index,
                );
            }
        }

        let mut vec_len_ref = &mut self.data[0..VEC_SIZE_BYTES];
        vec_len.serialize(&mut vec_len_ref)?;

        Ok(())
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

        let start_index = VEC_SIZE_BYTES.saturating_add(skip.saturating_mul(T::LEN));
        let end_index = start_index.saturating_add(len.saturating_mul(T::LEN));
        let mut deserialized = vec![];
        for slice in self.data[start_index..end_index].chunks_exact_mut(T::LEN) {
            deserialized.push(unsafe { &mut *(slice.as_ptr() as *mut T) });
        }
        Ok(deserialized)
    }

    /// Add new element to the end
    pub fn push<T: Pack>(&'a mut self, element: T) -> Result<(), ProgramError> {
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
        element.pack_into_slice(&mut element_ref);
        Ok(())
    }

    /// Rewrite the vec to remove an instance
    pub fn remove<T: Pack>(&'a mut self, _index: usize) {}

    /// Get an iterator for the type provided
    pub fn iter<T: Pack>(&'a self) -> Iter<'a, T> {
        Iter {
            len: self.len() as usize,
            current: 0,
            current_index: VEC_SIZE_BYTES,
            inner: self,
            phantom: PhantomData,
        }
    }

    /// Get a mutable iterator for the type provided
    pub fn iter_mut<T: Pack>(&'a mut self) -> IterMut<'a, T> {
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
