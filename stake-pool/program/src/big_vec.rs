//! Big vector type, used with vectors that can't be serde'd
#![allow(clippy::arithmetic_side_effects)] // checked math involves too many compute units

use {
    arrayref::array_ref,
    borsh::BorshDeserialize,
    bytemuck::Pod,
    solana_program::{program_error::ProgramError, program_memory::sol_memmove},
    std::mem,
};

/// Contains easy to use utilities for a big vector of Borsh-compatible types,
/// to avoid managing the entire struct on-chain and blow through stack limits.
pub struct BigVec<'data> {
    /// Underlying data buffer, pieces of which are serialized
    pub data: &'data mut [u8],
}

const VEC_SIZE_BYTES: usize = 4;

impl<'data> BigVec<'data> {
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
    pub fn retain<T: Pod, F: Fn(&[u8]) -> bool>(
        &mut self,
        predicate: F,
    ) -> Result<(), ProgramError> {
        let mut vec_len = self.len();
        let mut removals_found = 0;
        let mut dst_start_index = 0;

        let data_start_index = VEC_SIZE_BYTES;
        let data_end_index =
            data_start_index.saturating_add((vec_len as usize).saturating_mul(mem::size_of::<T>()));
        for start_index in (data_start_index..data_end_index).step_by(mem::size_of::<T>()) {
            let end_index = start_index + mem::size_of::<T>();
            let slice = &self.data[start_index..end_index];
            if !predicate(slice) {
                let gap = removals_found * mem::size_of::<T>();
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
            let gap = removals_found * mem::size_of::<T>();
            // In case the compute budget is ever bumped up, allowing us
            // to use this safe code instead:
            //    self.data.copy_within(
            //        dst_start_index + gap..data_end_index,
            //        dst_start_index,
            //    );
            unsafe {
                sol_memmove(
                    self.data[dst_start_index..data_end_index - gap].as_mut_ptr(),
                    self.data[dst_start_index + gap..data_end_index].as_mut_ptr(),
                    data_end_index - gap - dst_start_index,
                );
            }
        }

        let vec_len_ref = &mut self.data[0..VEC_SIZE_BYTES];
        borsh::to_writer(vec_len_ref, &vec_len)?;

        Ok(())
    }

    /// Extracts a slice of the data types
    pub fn deserialize_mut_slice<T: Pod>(
        &mut self,
        skip: usize,
        len: usize,
    ) -> Result<&mut [T], ProgramError> {
        let vec_len = self.len();
        let last_item_index = skip
            .checked_add(len)
            .ok_or(ProgramError::AccountDataTooSmall)?;
        if last_item_index > vec_len as usize {
            return Err(ProgramError::AccountDataTooSmall);
        }

        let start_index = VEC_SIZE_BYTES.saturating_add(skip.saturating_mul(mem::size_of::<T>()));
        let end_index = start_index.saturating_add(len.saturating_mul(mem::size_of::<T>()));
        bytemuck::try_cast_slice_mut(&mut self.data[start_index..end_index])
            .map_err(|_| ProgramError::InvalidAccountData)
    }

    /// Extracts a slice of the data types
    pub fn deserialize_slice<T: Pod>(&self, skip: usize, len: usize) -> Result<&[T], ProgramError> {
        let vec_len = self.len();
        let last_item_index = skip
            .checked_add(len)
            .ok_or(ProgramError::AccountDataTooSmall)?;
        if last_item_index > vec_len as usize {
            return Err(ProgramError::AccountDataTooSmall);
        }

        let start_index = VEC_SIZE_BYTES.saturating_add(skip.saturating_mul(mem::size_of::<T>()));
        let end_index = start_index.saturating_add(len.saturating_mul(mem::size_of::<T>()));
        bytemuck::try_cast_slice(&self.data[start_index..end_index])
            .map_err(|_| ProgramError::InvalidAccountData)
    }

    /// Add new element to the end
    pub fn push<T: Pod>(&mut self, element: T) -> Result<(), ProgramError> {
        let vec_len_ref = &mut self.data[0..VEC_SIZE_BYTES];
        let mut vec_len = u32::try_from_slice(vec_len_ref)?;

        let start_index = VEC_SIZE_BYTES + vec_len as usize * mem::size_of::<T>();
        let end_index = start_index + mem::size_of::<T>();

        vec_len += 1;
        borsh::to_writer(vec_len_ref, &vec_len)?;

        if self.data.len() < end_index {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let element_ref = bytemuck::try_from_bytes_mut(
            &mut self.data[start_index..start_index + mem::size_of::<T>()],
        )
        .map_err(|_| ProgramError::InvalidAccountData)?;
        *element_ref = element;
        Ok(())
    }

    /// Find matching data in the array
    pub fn find<T: Pod, F: Fn(&[u8]) -> bool>(&self, predicate: F) -> Option<&T> {
        let len = self.len() as usize;
        let mut current = 0;
        let mut current_index = VEC_SIZE_BYTES;
        while current != len {
            let end_index = current_index + mem::size_of::<T>();
            let current_slice = &self.data[current_index..end_index];
            if predicate(current_slice) {
                return Some(bytemuck::from_bytes(current_slice));
            }
            current_index = end_index;
            current += 1;
        }
        None
    }

    /// Find matching data in the array
    pub fn find_mut<T: Pod, F: Fn(&[u8]) -> bool>(&mut self, predicate: F) -> Option<&mut T> {
        let len = self.len() as usize;
        let mut current = 0;
        let mut current_index = VEC_SIZE_BYTES;
        while current != len {
            let end_index = current_index + mem::size_of::<T>();
            let current_slice = &self.data[current_index..end_index];
            if predicate(current_slice) {
                return Some(bytemuck::from_bytes_mut(
                    &mut self.data[current_index..end_index],
                ));
            }
            current_index = end_index;
            current += 1;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use {super::*, bytemuck::Zeroable};

    #[repr(C)]
    #[derive(Debug, Copy, Clone, PartialEq, Pod, Zeroable)]
    struct TestStruct {
        value: [u8; 8],
    }

    impl TestStruct {
        fn new(value: u8) -> Self {
            let value = [value, 0, 0, 0, 0, 0, 0, 0];
            Self { value }
        }
    }

    fn from_slice<'data>(data: &'data mut [u8], vec: &[u8]) -> BigVec<'data> {
        let mut big_vec = BigVec { data };
        for element in vec {
            big_vec.push(TestStruct::new(*element)).unwrap();
        }
        big_vec
    }

    fn check_big_vec_eq(big_vec: &BigVec, slice: &[u8]) {
        assert!(big_vec
            .deserialize_slice::<TestStruct>(0, big_vec.len() as usize)
            .unwrap()
            .iter()
            .map(|x| &x.value[0])
            .zip(slice.iter())
            .all(|(a, b)| a == b));
    }

    #[test]
    fn push() {
        let mut data = [0u8; 4 + 8 * 3];
        let mut v = BigVec { data: &mut data };
        v.push(TestStruct::new(1)).unwrap();
        check_big_vec_eq(&v, &[1]);
        v.push(TestStruct::new(2)).unwrap();
        check_big_vec_eq(&v, &[1, 2]);
        v.push(TestStruct::new(3)).unwrap();
        check_big_vec_eq(&v, &[1, 2, 3]);
        assert_eq!(
            v.push(TestStruct::new(4)).unwrap_err(),
            ProgramError::AccountDataTooSmall
        );
    }

    #[test]
    fn retain() {
        fn mod_2_predicate(data: &[u8]) -> bool {
            u64::try_from_slice(data).unwrap() % 2 == 0
        }

        let mut data = [0u8; 4 + 8 * 4];
        let mut v = from_slice(&mut data, &[1, 2, 3, 4]);
        v.retain::<TestStruct, _>(mod_2_predicate).unwrap();
        check_big_vec_eq(&v, &[2, 4]);
    }

    fn find_predicate(a: &[u8], b: u8) -> bool {
        if a.len() != 8 {
            false
        } else {
            a[0] == b
        }
    }

    #[test]
    fn find() {
        let mut data = [0u8; 4 + 8 * 4];
        let v = from_slice(&mut data, &[1, 2, 3, 4]);
        assert_eq!(
            v.find::<TestStruct, _>(|x| find_predicate(x, 1)),
            Some(&TestStruct::new(1))
        );
        assert_eq!(
            v.find::<TestStruct, _>(|x| find_predicate(x, 4)),
            Some(&TestStruct::new(4))
        );
        assert_eq!(v.find::<TestStruct, _>(|x| find_predicate(x, 5)), None);
    }

    #[test]
    fn find_mut() {
        let mut data = [0u8; 4 + 8 * 4];
        let mut v = from_slice(&mut data, &[1, 2, 3, 4]);
        let test_struct = v
            .find_mut::<TestStruct, _>(|x| find_predicate(x, 1))
            .unwrap();
        test_struct.value = [0; 8];
        check_big_vec_eq(&v, &[0, 2, 3, 4]);
        assert_eq!(v.find_mut::<TestStruct, _>(|x| find_predicate(x, 5)), None);
    }

    #[test]
    fn deserialize_mut_slice() {
        let mut data = [0u8; 4 + 8 * 4];
        let mut v = from_slice(&mut data, &[1, 2, 3, 4]);
        let slice = v.deserialize_mut_slice::<TestStruct>(1, 2).unwrap();
        slice[0].value[0] = 10;
        slice[1].value[0] = 11;
        check_big_vec_eq(&v, &[1, 10, 11, 4]);
        assert_eq!(
            v.deserialize_mut_slice::<TestStruct>(1, 4).unwrap_err(),
            ProgramError::AccountDataTooSmall
        );
        assert_eq!(
            v.deserialize_mut_slice::<TestStruct>(4, 1).unwrap_err(),
            ProgramError::AccountDataTooSmall
        );
    }
}
