//! Type-length-value structure definition and manipulation

use {
    crate::{
        error::TlvError,
        length::Length,
        pod::{pod_from_bytes, pod_from_bytes_mut},
    },
    bytemuck::Pod,
    solana_program::program_error::ProgramError,
    spl_discriminator::{ArrayDiscriminator, SplDiscriminate},
    std::{cmp::Ordering, mem::size_of},
};

/// Get the current TlvIndices from the current spot
const fn get_indices_unchecked(type_start: usize) -> TlvIndices {
    let length_start = type_start.saturating_add(size_of::<ArrayDiscriminator>());
    let value_start = length_start.saturating_add(size_of::<Length>());
    TlvIndices {
        type_start,
        length_start,
        value_start,
    }
}

/// Internal helper struct for returning the indices of the type, length, and
/// value in a TLV entry
#[derive(Debug)]
struct TlvIndices {
    pub type_start: usize,
    pub length_start: usize,
    pub value_start: usize,
}
fn get_indices(
    tlv_data: &[u8],
    value_discriminator: ArrayDiscriminator,
    init: bool,
) -> Result<TlvIndices, ProgramError> {
    let mut start_index = 0;
    while start_index < tlv_data.len() {
        let tlv_indices = get_indices_unchecked(start_index);
        if tlv_data.len() < tlv_indices.value_start {
            return Err(ProgramError::InvalidAccountData);
        }
        let discriminator = ArrayDiscriminator::try_from(
            &tlv_data[tlv_indices.type_start..tlv_indices.length_start],
        )?;
        if discriminator == value_discriminator {
            // found an instance of the extension that we're initializing, return!
            return Ok(tlv_indices);
        // got to an empty spot, init here, or error if we're searching, since
        // nothing is written after an Uninitialized spot
        } else if discriminator == ArrayDiscriminator::UNINITIALIZED {
            if init {
                return Ok(tlv_indices);
            } else {
                return Err(TlvError::TypeNotFound.into());
            }
        } else {
            let length = pod_from_bytes::<Length>(
                &tlv_data[tlv_indices.length_start..tlv_indices.value_start],
            )?;
            let value_end_index = tlv_indices
                .value_start
                .saturating_add(usize::try_from(*length)?);
            start_index = value_end_index;
        }
    }
    Err(ProgramError::InvalidAccountData)
}

// This function is doing two separate things at once, and would probably be
// better served by some custom iterator, but let's leave that for another day.
fn get_discriminators_and_end_index(
    tlv_data: &[u8],
) -> Result<(Vec<ArrayDiscriminator>, usize), ProgramError> {
    let mut discriminators = vec![];
    let mut start_index = 0;
    while start_index < tlv_data.len() {
        let tlv_indices = get_indices_unchecked(start_index);
        if tlv_data.len() < tlv_indices.length_start {
            // we got to the end, but there might be some uninitialized data after
            let remainder = &tlv_data[tlv_indices.type_start..];
            if remainder.iter().all(|&x| x == 0) {
                return Ok((discriminators, tlv_indices.type_start));
            } else {
                return Err(ProgramError::InvalidAccountData);
            }
        }
        let discriminator = ArrayDiscriminator::try_from(
            &tlv_data[tlv_indices.type_start..tlv_indices.length_start],
        )?;
        if discriminator == ArrayDiscriminator::UNINITIALIZED {
            return Ok((discriminators, tlv_indices.type_start));
        } else {
            if tlv_data.len() < tlv_indices.value_start {
                // not enough bytes to store the length, malformed
                return Err(ProgramError::InvalidAccountData);
            }
            discriminators.push(discriminator);
            let length = pod_from_bytes::<Length>(
                &tlv_data[tlv_indices.length_start..tlv_indices.value_start],
            )?;

            let value_end_index = tlv_indices
                .value_start
                .saturating_add(usize::try_from(*length)?);
            if value_end_index > tlv_data.len() {
                // value blows past the size of the slice, malformed
                return Err(ProgramError::InvalidAccountData);
            }
            start_index = value_end_index;
        }
    }
    Ok((discriminators, start_index))
}

fn get_bytes<V: SplDiscriminate>(tlv_data: &[u8]) -> Result<&[u8], ProgramError> {
    let TlvIndices {
        type_start: _,
        length_start,
        value_start,
    } = get_indices(tlv_data, V::SPL_DISCRIMINATOR, false)?;
    // get_indices has checked that tlv_data is long enough to include these indices
    let length = pod_from_bytes::<Length>(&tlv_data[length_start..value_start])?;
    let value_end = value_start.saturating_add(usize::try_from(*length)?);
    if tlv_data.len() < value_end {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(&tlv_data[value_start..value_end])
}

/// Trait for all TLV state
///
/// Stores data as any number of type-length-value structures underneath, where:
///
///   * the "type" is an `ArrayDiscriminator`, 8 bytes
///   * the "length" is a `Length`, 4 bytes
///   * the "value" is a slab of "length" bytes
///
/// With this structure, it's possible to hold onto any number of entries with
/// unique discriminators, provided that the total underlying data has enough
/// bytes for every entry.
///
/// For example, if we have two distinct types, one which is an 8-byte array
/// of value `[0, 1, 0, 0, 0, 0, 0, 0]` and discriminator
/// `[1, 1, 1, 1, 1, 1, 1, 1]`, and another which is just a single `u8` of value
/// `4` with the discriminator `[2, 2, 2, 2, 2, 2, 2, 2]`, we can deserialize this
/// buffer as follows:
///
/// ```
/// use {
///     bytemuck::{Pod, Zeroable},
///     spl_discriminator::{ArrayDiscriminator, SplDiscriminate},
///     spl_type_length_value::state::{TlvState, TlvStateBorrowed, TlvStateMut},
/// };
/// #[repr(C)]
/// #[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
/// struct MyPodValue {
///     data: [u8; 8],
/// }
/// impl SplDiscriminate for MyPodValue {
///     const SPL_DISCRIMINATOR: ArrayDiscriminator = ArrayDiscriminator::new([1; ArrayDiscriminator::LENGTH]);
/// }
/// #[repr(C)]
/// #[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
/// struct MyOtherPodValue {
///     data: u8,
/// }
/// impl SplDiscriminate for MyOtherPodValue {
///     const SPL_DISCRIMINATOR: ArrayDiscriminator = ArrayDiscriminator::new([2; ArrayDiscriminator::LENGTH]);
/// }
/// let buffer = [
///   1, 1, 1, 1, 1, 1, 1, 1, // first type's discriminator
///   8, 0, 0, 0,             // first type's length
///   0, 1, 0, 0, 0, 0, 0, 0, // first type's value
///   2, 2, 2, 2, 2, 2, 2, 2, // second type's discriminator
///   1, 0, 0, 0,             // second type's length
///   4,                      // second type's value
/// ];
/// let state = TlvStateBorrowed::unpack(&buffer).unwrap();
/// let value = state.get_value::<MyPodValue>().unwrap();
/// assert_eq!(value.data, [0, 1, 0, 0, 0, 0, 0, 0]);
/// let value = state.get_value::<MyOtherPodValue>().unwrap();
/// assert_eq!(value.data, 4);
/// ```
///
/// See the README and tests for more examples on how to use these types.
pub trait TlvState {
    /// Get the full buffer containing all TLV data
    fn get_data(&self) -> &[u8];

    /// Unpack a portion of the TLV data as the desired Pod type
    fn get_value<V: SplDiscriminate + Pod>(&self) -> Result<&V, ProgramError> {
        let data = get_bytes::<V>(self.get_data())?;
        pod_from_bytes::<V>(data)
    }

    /// Unpacks a portion of the TLV data as the desired Borsh type
    #[cfg(feature = "borsh")]
    fn borsh_deserialize<V: SplDiscriminate + borsh::BorshDeserialize>(
        &self,
    ) -> Result<V, ProgramError> {
        let data = get_bytes::<V>(self.get_data())?;
        solana_program::borsh::try_from_slice_unchecked::<V>(data).map_err(Into::into)
    }

    /// Unpack a portion of the TLV data as bytes
    fn get_bytes<V: SplDiscriminate>(&self) -> Result<&[u8], ProgramError> {
        get_bytes::<V>(self.get_data())
    }

    /// Iterates through the TLV entries, returning only the types
    fn get_discriminators(&self) -> Result<Vec<ArrayDiscriminator>, ProgramError> {
        get_discriminators_and_end_index(self.get_data()).map(|v| v.0)
    }

    /// Get the base size required for TLV data
    fn get_base_len() -> usize {
        get_base_len()
    }
}

/// Encapsulates owned TLV data
#[derive(Debug, PartialEq)]
pub struct TlvStateOwned {
    /// Raw TLV data, deserialized on demand
    data: Vec<u8>,
}
impl TlvStateOwned {
    /// Unpacks TLV state data
    ///
    /// Fails if no state is initialized or if data is too small
    pub fn unpack(data: Vec<u8>) -> Result<Self, ProgramError> {
        check_data(&data)?;
        Ok(Self { data })
    }
}
impl TlvState for TlvStateOwned {
    fn get_data(&self) -> &[u8] {
        &self.data
    }
}

/// Encapsulates immutable base state data (mint or account) with possible extensions
#[derive(Debug, PartialEq)]
pub struct TlvStateBorrowed<'data> {
    /// Slice of data containing all TLV data, deserialized on demand
    data: &'data [u8],
}
impl<'data> TlvStateBorrowed<'data> {
    /// Unpacks TLV state data
    ///
    /// Fails if no state is initialized or if data is too small
    pub fn unpack(data: &'data [u8]) -> Result<Self, ProgramError> {
        check_data(data)?;
        Ok(Self { data })
    }
}
impl<'a> TlvState for TlvStateBorrowed<'a> {
    fn get_data(&self) -> &[u8] {
        self.data
    }
}

/// Encapsulates mutable base state data (mint or account) with possible extensions
#[derive(Debug, PartialEq)]
pub struct TlvStateMut<'data> {
    /// Slice of data containing all TLV data, deserialized on demand
    data: &'data mut [u8],
}
impl<'data> TlvStateMut<'data> {
    /// Unpacks TLV state data
    ///
    /// Fails if no state is initialized or if data is too small
    pub fn unpack(data: &'data mut [u8]) -> Result<Self, ProgramError> {
        check_data(data)?;
        Ok(Self { data })
    }

    /// Unpack a portion of the TLV data as the desired type that allows modifying the type
    pub fn get_value_mut<V: SplDiscriminate + Pod>(&mut self) -> Result<&mut V, ProgramError> {
        let data = self.get_bytes_mut::<V>()?;
        pod_from_bytes_mut::<V>(data)
    }

    /// Unpack a portion of the TLV data as mutable bytes
    pub fn get_bytes_mut<V: SplDiscriminate>(&mut self) -> Result<&mut [u8], ProgramError> {
        let TlvIndices {
            type_start: _,
            length_start,
            value_start,
        } = get_indices(self.data, V::SPL_DISCRIMINATOR, false)?;

        let length = pod_from_bytes::<Length>(&self.data[length_start..value_start])?;
        let value_end = value_start.saturating_add(usize::try_from(*length)?);
        if self.data.len() < value_end {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(&mut self.data[value_start..value_end])
    }

    /// Packs the default TLV data into the first open slot in the data buffer.
    /// If extension is already found in the buffer, it returns an error.
    pub fn init_value<V: SplDiscriminate + Pod + Default>(
        &mut self,
    ) -> Result<&mut V, ProgramError> {
        let length = size_of::<V>();
        let buffer = self.alloc::<V>(length)?;
        let extension_ref = pod_from_bytes_mut::<V>(buffer)?;
        *extension_ref = V::default();
        Ok(extension_ref)
    }

    /// Packs a borsh-serializable value into its appropriate data segment. Assumes
    /// that space has already been allocated for the given type
    #[cfg(feature = "borsh")]
    pub fn borsh_serialize<V: SplDiscriminate + borsh::BorshSerialize>(
        &mut self,
        value: &V,
    ) -> Result<(), ProgramError> {
        let data = self.get_bytes_mut::<V>()?;
        borsh::to_writer(&mut data[..], value).map_err(Into::into)
    }

    /// Allocate the given number of bytes for the given SplDiscriminate
    pub fn alloc<V: SplDiscriminate>(&mut self, length: usize) -> Result<&mut [u8], ProgramError> {
        let TlvIndices {
            type_start,
            length_start,
            value_start,
        } = get_indices(self.data, V::SPL_DISCRIMINATOR, true)?;

        let discriminator = ArrayDiscriminator::try_from(&self.data[type_start..length_start])?;
        if discriminator == ArrayDiscriminator::UNINITIALIZED {
            // write type
            let discriminator_ref = &mut self.data[type_start..length_start];
            discriminator_ref.copy_from_slice(V::SPL_DISCRIMINATOR.as_ref());
            // write length
            let length_ref =
                pod_from_bytes_mut::<Length>(&mut self.data[length_start..value_start])?;
            *length_ref = Length::try_from(length)?;

            let value_end = value_start.saturating_add(length);
            if self.data.len() < value_end {
                return Err(ProgramError::InvalidAccountData);
            }
            Ok(&mut self.data[value_start..value_end])
        } else {
            Err(TlvError::TypeAlreadyExists.into())
        }
    }

    /// Reallocate the given number of bytes for the given SplDiscriminate. If the new
    /// length is smaller, it will compact the rest of the buffer and zero out
    /// the difference at the end. If it's larger, it will move the rest of
    /// the buffer data and zero out the new data.
    pub fn realloc<V: SplDiscriminate>(
        &mut self,
        length: usize,
    ) -> Result<&mut [u8], ProgramError> {
        let TlvIndices {
            type_start: _,
            length_start,
            value_start,
        } = get_indices(self.data, V::SPL_DISCRIMINATOR, false)?;
        let (_, end_index) = get_discriminators_and_end_index(self.data)?;
        let data_len = self.data.len();

        let length_ref = pod_from_bytes_mut::<Length>(&mut self.data[length_start..value_start])?;
        let old_length = usize::try_from(*length_ref)?;

        // check that we're not going to panic during `copy_within`
        if old_length < length {
            let new_end_index = end_index.saturating_add(length.saturating_sub(old_length));
            if new_end_index > data_len {
                return Err(ProgramError::InvalidAccountData);
            }
        }

        // write new length after the check, to avoid getting into a bad situation
        // if trying to recover from an error
        *length_ref = Length::try_from(length)?;

        let old_value_end = value_start.saturating_add(old_length);
        let new_value_end = value_start.saturating_add(length);
        self.data
            .copy_within(old_value_end..end_index, new_value_end);
        match old_length.cmp(&length) {
            Ordering::Greater => {
                // realloc to smaller, fill the end
                let new_end_index = end_index.saturating_sub(old_length.saturating_sub(length));
                self.data[new_end_index..end_index].fill(0);
            }
            Ordering::Less => {
                // realloc to bigger, fill the moved part
                self.data[old_value_end..new_value_end].fill(0);
            }
            Ordering::Equal => {} // nothing needed!
        }

        Ok(&mut self.data[value_start..new_value_end])
    }
}
impl<'a> TlvState for TlvStateMut<'a> {
    fn get_data(&self) -> &[u8] {
        self.data
    }
}

/// Packs a borsh-serializable value into an existing TLV space, reallocating
/// the account and TLV as needed to accommodate for any change in space
#[cfg(feature = "borsh")]
pub fn realloc_and_borsh_serialize<V: SplDiscriminate + borsh::BorshSerialize>(
    account_info: &solana_program::account_info::AccountInfo,
    value: &V,
) -> Result<(), ProgramError> {
    let previous_length = {
        let data = account_info.try_borrow_data()?;
        let TlvIndices {
            type_start: _,
            length_start,
            value_start,
        } = get_indices(&data, V::SPL_DISCRIMINATOR, false)?;
        usize::try_from(*pod_from_bytes::<Length>(&data[length_start..value_start])?)?
    };
    let new_length = solana_program::borsh::get_instance_packed_len(&value)?;
    let previous_account_size = account_info.try_data_len()?;
    if previous_length < new_length {
        // size increased, so realloc the account, then the TLV entry, then write data
        let additional_bytes = new_length
            .checked_sub(previous_length)
            .ok_or(ProgramError::AccountDataTooSmall)?;
        account_info.realloc(previous_account_size.saturating_add(additional_bytes), true)?;
        let mut buffer = account_info.try_borrow_mut_data()?;
        let mut state = TlvStateMut::unpack(&mut buffer)?;
        state.realloc::<V>(new_length)?;
        state.borsh_serialize(value)?;
    } else {
        // do it backwards otherwise, write the state, realloc TLV, then the account
        let mut buffer = account_info.try_borrow_mut_data()?;
        let mut state = TlvStateMut::unpack(&mut buffer)?;
        state.borsh_serialize(value)?;
        let removed_bytes = previous_length
            .checked_sub(new_length)
            .ok_or(ProgramError::AccountDataTooSmall)?;
        if removed_bytes > 0 {
            // we decreased the size, so need to realloc the TLV, then the account
            state.realloc::<V>(new_length)?;
            // this is probably fine, but be safe and avoid invalidating references
            drop(buffer);
            account_info.realloc(previous_account_size.saturating_sub(removed_bytes), false)?;
        }
    }
    Ok(())
}

/// Get the base size required for TLV data
const fn get_base_len() -> usize {
    let indices = get_indices_unchecked(0);
    indices.value_start
}

fn check_data(tlv_data: &[u8]) -> Result<(), ProgramError> {
    // should be able to iterate through all entries in the TLV structure
    let _ = get_discriminators_and_end_index(tlv_data)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use bytemuck::{Pod, Zeroable};

    const TEST_BUFFER: &[u8] = &[
        1, 1, 1, 1, 1, 1, 1, 1, // discriminator
        32, 0, 0, 0, // length
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, // value
        0, 0, // empty, not enough for a discriminator
    ];

    const TEST_BIG_BUFFER: &[u8] = &[
        1, 1, 1, 1, 1, 1, 1, 1, // discriminator
        32, 0, 0, 0, // length
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, // value
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, // empty, but enough for a discriminator and empty value
    ];

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
    struct TestValue {
        data: [u8; 32],
    }
    impl SplDiscriminate for TestValue {
        const SPL_DISCRIMINATOR: ArrayDiscriminator =
            ArrayDiscriminator::new([1; ArrayDiscriminator::LENGTH]);
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
    struct TestSmallValue {
        data: [u8; 3],
    }
    impl SplDiscriminate for TestSmallValue {
        const SPL_DISCRIMINATOR: ArrayDiscriminator =
            ArrayDiscriminator::new([2; ArrayDiscriminator::LENGTH]);
    }

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
    struct TestEmptyValue;
    impl SplDiscriminate for TestEmptyValue {
        const SPL_DISCRIMINATOR: ArrayDiscriminator =
            ArrayDiscriminator::new([3; ArrayDiscriminator::LENGTH]);
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
    struct TestNonZeroDefault {
        data: [u8; 5],
    }
    const TEST_NON_ZERO_DEFAULT_DATA: [u8; 5] = [4; 5];
    impl SplDiscriminate for TestNonZeroDefault {
        const SPL_DISCRIMINATOR: ArrayDiscriminator =
            ArrayDiscriminator::new([4; ArrayDiscriminator::LENGTH]);
    }
    impl Default for TestNonZeroDefault {
        fn default() -> Self {
            Self {
                data: TEST_NON_ZERO_DEFAULT_DATA,
            }
        }
    }

    #[test]
    fn unpack_opaque_buffer() {
        let state = TlvStateBorrowed::unpack(TEST_BUFFER).unwrap();
        let value = state.get_value::<TestValue>().unwrap();
        assert_eq!(value.data, [1; 32]);
        assert_eq!(
            state.get_value::<TestEmptyValue>(),
            Err(ProgramError::InvalidAccountData)
        );

        let mut test_buffer = TEST_BUFFER.to_vec();
        let state = TlvStateMut::unpack(&mut test_buffer).unwrap();
        let value = state.get_value::<TestValue>().unwrap();
        assert_eq!(value.data, [1; 32]);
        let state = TlvStateOwned::unpack(test_buffer).unwrap();
        let value = state.get_value::<TestValue>().unwrap();
        assert_eq!(value.data, [1; 32]);
    }

    #[test]
    fn fail_unpack_opaque_buffer() {
        // input buffer too small
        let mut buffer = vec![0, 3];
        assert_eq!(
            TlvStateBorrowed::unpack(&buffer),
            Err(ProgramError::InvalidAccountData)
        );
        assert_eq!(
            TlvStateMut::unpack(&mut buffer),
            Err(ProgramError::InvalidAccountData)
        );
        assert_eq!(
            TlvStateMut::unpack(&mut buffer),
            Err(ProgramError::InvalidAccountData)
        );

        // tweak the discriminator
        let mut buffer = TEST_BUFFER.to_vec();
        buffer[0] += 1;
        let state = TlvStateMut::unpack(&mut buffer).unwrap();
        assert_eq!(
            state.get_value::<TestValue>(),
            Err(ProgramError::InvalidAccountData)
        );

        // tweak the length, too big
        let mut buffer = TEST_BUFFER.to_vec();
        buffer[ArrayDiscriminator::LENGTH] += 10;
        assert_eq!(
            TlvStateMut::unpack(&mut buffer),
            Err(ProgramError::InvalidAccountData)
        );

        // tweak the length, too small
        let mut buffer = TEST_BIG_BUFFER.to_vec();
        buffer[ArrayDiscriminator::LENGTH] -= 1;
        let state = TlvStateMut::unpack(&mut buffer).unwrap();
        assert_eq!(
            state.get_value::<TestValue>(),
            Err(ProgramError::InvalidArgument)
        );

        // data buffer is too small for type
        let buffer = &TEST_BUFFER[..TEST_BUFFER.len() - 5];
        assert_eq!(
            TlvStateBorrowed::unpack(buffer),
            Err(ProgramError::InvalidAccountData)
        );
    }

    #[test]
    fn get_discriminators_with_opaque_buffer() {
        // incorrect due to the length
        assert_eq!(
            get_discriminators_and_end_index(&[1, 0, 1, 1]).unwrap_err(),
            ProgramError::InvalidAccountData,
        );
        // correct due to the good discriminator length and zero length
        assert_eq!(
            get_discriminators_and_end_index(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
            (vec![ArrayDiscriminator::try_from(1).unwrap()], 12)
        );
        // correct since it's just uninitialized data
        assert_eq!(
            get_discriminators_and_end_index(&[0, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
            (vec![], 0)
        );
    }

    #[test]
    fn value_pack_unpack() {
        let account_size =
            get_base_len() + size_of::<TestValue>() + get_base_len() + size_of::<TestSmallValue>();
        let mut buffer = vec![0; account_size];

        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        // success init and write value
        let value = state.init_value::<TestValue>().unwrap();
        let data = [100; 32];
        value.data = data;
        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[TestValue::SPL_DISCRIMINATOR],
        );
        assert_eq!(&state.get_value::<TestValue>().unwrap().data, &data,);

        // fail init extension when already initialized
        assert_eq!(
            state.init_value::<TestValue>().unwrap_err(),
            TlvError::TypeAlreadyExists.into(),
        );

        // check raw buffer
        let mut expect = vec![];
        expect.extend_from_slice(TestValue::SPL_DISCRIMINATOR.as_ref());
        expect.extend_from_slice(&u32::try_from(size_of::<TestValue>()).unwrap().to_le_bytes());
        expect.extend_from_slice(&data);
        expect.extend_from_slice(&[0; size_of::<ArrayDiscriminator>()]);
        expect.extend_from_slice(&[0; size_of::<Length>()]);
        expect.extend_from_slice(&[0; size_of::<TestSmallValue>()]);
        assert_eq!(expect, buffer);

        // check unpacking
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
        let mut unpacked = state.get_value_mut::<TestValue>().unwrap();
        assert_eq!(*unpacked, TestValue { data });

        // update extension
        let new_data = [101; 32];
        unpacked.data = new_data;

        // check updates are propagated
        let state = TlvStateBorrowed::unpack(&buffer).unwrap();
        let unpacked = state.get_value::<TestValue>().unwrap();
        assert_eq!(*unpacked, TestValue { data: new_data });

        // check raw buffer
        let mut expect = vec![];
        expect.extend_from_slice(TestValue::SPL_DISCRIMINATOR.as_ref());
        expect.extend_from_slice(&u32::try_from(size_of::<TestValue>()).unwrap().to_le_bytes());
        expect.extend_from_slice(&new_data);
        expect.extend_from_slice(&[0; size_of::<ArrayDiscriminator>()]);
        expect.extend_from_slice(&[0; size_of::<Length>()]);
        expect.extend_from_slice(&[0; size_of::<TestSmallValue>()]);
        assert_eq!(expect, buffer);

        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
        // init one more value
        let new_value = state.init_value::<TestSmallValue>().unwrap();
        let small_data = [102; 3];
        new_value.data = small_data;

        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[
                TestValue::SPL_DISCRIMINATOR,
                TestSmallValue::SPL_DISCRIMINATOR
            ]
        );

        // check raw buffer
        let mut expect = vec![];
        expect.extend_from_slice(TestValue::SPL_DISCRIMINATOR.as_ref());
        expect.extend_from_slice(&u32::try_from(size_of::<TestValue>()).unwrap().to_le_bytes());
        expect.extend_from_slice(&new_data);
        expect.extend_from_slice(TestSmallValue::SPL_DISCRIMINATOR.as_ref());
        expect.extend_from_slice(
            &u32::try_from(size_of::<TestSmallValue>())
                .unwrap()
                .to_le_bytes(),
        );
        expect.extend_from_slice(&small_data);
        assert_eq!(expect, buffer);

        // fail to init one more extension that does not fit
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
        assert_eq!(
            state.init_value::<TestEmptyValue>(),
            Err(ProgramError::InvalidAccountData),
        );
    }

    #[test]
    fn value_any_order() {
        let account_size =
            get_base_len() + size_of::<TestValue>() + get_base_len() + size_of::<TestSmallValue>();
        let mut buffer = vec![0; account_size];

        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        let data = [99; 32];
        let small_data = [98; 3];

        // write values
        let value = state.init_value::<TestValue>().unwrap();
        value.data = data;
        let value = state.init_value::<TestSmallValue>().unwrap();
        value.data = small_data;

        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[
                TestValue::SPL_DISCRIMINATOR,
                TestSmallValue::SPL_DISCRIMINATOR,
            ]
        );

        // write values in a different order
        let mut other_buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut other_buffer).unwrap();

        let value = state.init_value::<TestSmallValue>().unwrap();
        value.data = small_data;
        let value = state.init_value::<TestValue>().unwrap();
        value.data = data;

        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[
                TestSmallValue::SPL_DISCRIMINATOR,
                TestValue::SPL_DISCRIMINATOR,
            ]
        );

        // buffers are NOT the same because written in a different order
        assert_ne!(buffer, other_buffer);
        let state = TlvStateBorrowed::unpack(&buffer).unwrap();
        let other_state = TlvStateBorrowed::unpack(&other_buffer).unwrap();

        // BUT values are the same
        assert_eq!(
            state.get_value::<TestValue>().unwrap(),
            other_state.get_value::<TestValue>().unwrap()
        );
        assert_eq!(
            state.get_value::<TestSmallValue>().unwrap(),
            other_state.get_value::<TestSmallValue>().unwrap()
        );
    }

    #[test]
    fn init_nonzero_default() {
        let account_size = get_base_len() + size_of::<TestNonZeroDefault>();
        let mut buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
        let value = state.init_value::<TestNonZeroDefault>().unwrap();
        assert_eq!(value.data, TEST_NON_ZERO_DEFAULT_DATA);
    }

    #[test]
    fn init_buffer_too_small() {
        let account_size = get_base_len() + size_of::<TestValue>();
        let mut buffer = vec![0; account_size - 1];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
        let err = state.init_value::<TestValue>().unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);

        // hack the buffer to look like it was initialized, still fails
        let discriminator_ref = &mut state.data[0..ArrayDiscriminator::LENGTH];
        discriminator_ref.copy_from_slice(TestValue::SPL_DISCRIMINATOR.as_ref());
        state.data[ArrayDiscriminator::LENGTH] = 32;
        let err = state.get_value::<TestValue>().unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);
        assert_eq!(
            state.get_discriminators().unwrap_err(),
            ProgramError::InvalidAccountData
        );
    }

    #[test]
    fn value_with_no_data() {
        let account_size = get_base_len() + size_of::<TestEmptyValue>();
        let mut buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        assert_eq!(
            state.get_value::<TestEmptyValue>().unwrap_err(),
            TlvError::TypeNotFound.into(),
        );

        state.init_value::<TestEmptyValue>().unwrap();
        state.get_value::<TestEmptyValue>().unwrap();

        // re-init fails
        assert_eq!(
            state.init_value::<TestEmptyValue>().unwrap_err(),
            TlvError::TypeAlreadyExists.into(),
        );
    }

    #[test]
    fn alloc() {
        let tlv_size = 1;
        let account_size = get_base_len() + tlv_size;
        let mut buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        // not enough room
        let data = state.alloc::<TestValue>(tlv_size).unwrap();
        assert_eq!(
            pod_from_bytes_mut::<TestValue>(data).unwrap_err(),
            ProgramError::InvalidArgument,
        );

        // can't double alloc
        assert_eq!(
            state.alloc::<TestValue>(tlv_size).unwrap_err(),
            TlvError::TypeAlreadyExists.into(),
        );
    }

    #[test]
    fn realloc() {
        const TLV_SIZE: usize = 10;
        const EXTRA_SPACE: usize = 5;
        const SMALL_SIZE: usize = 2;
        const ACCOUNT_SIZE: usize = get_base_len()
            + TLV_SIZE
            + EXTRA_SPACE
            + get_base_len()
            + size_of::<TestNonZeroDefault>();
        let mut buffer = vec![0; ACCOUNT_SIZE];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        // alloc both types
        let _ = state.alloc::<TestValue>(TLV_SIZE).unwrap();
        let _ = state.init_value::<TestNonZeroDefault>().unwrap();

        // realloc first entry to larger, all 0
        let data = state.realloc::<TestValue>(TLV_SIZE + EXTRA_SPACE).unwrap();
        assert_eq!(data, [0; TLV_SIZE + EXTRA_SPACE]);
        let value = state.get_value::<TestNonZeroDefault>().unwrap();
        assert_eq!(*value, TestNonZeroDefault::default());

        // realloc to smaller, still all 0
        let data = state.realloc::<TestValue>(SMALL_SIZE).unwrap();
        assert_eq!(data, [0; SMALL_SIZE]);
        let value = state.get_value::<TestNonZeroDefault>().unwrap();
        assert_eq!(*value, TestNonZeroDefault::default());
        let (_, end_index) = get_discriminators_and_end_index(&buffer).unwrap();
        assert_eq!(
            &buffer[end_index..ACCOUNT_SIZE],
            [0; TLV_SIZE + EXTRA_SPACE - SMALL_SIZE]
        );

        // unpack again since we dropped the last `state`
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
        // realloc too much, fails
        assert_eq!(
            state
                .realloc::<TestValue>(TLV_SIZE + EXTRA_SPACE + 1)
                .unwrap_err(),
            ProgramError::InvalidAccountData,
        );
    }
}
#[cfg(all(test, feature = "borsh"))]
mod borsh_test {
    use super::*;
    #[derive(Clone, Debug, PartialEq, borsh::BorshDeserialize, borsh::BorshSerialize)]
    struct TestBorsh {
        data: String, // test with a variable length type
        inner: TestInnerBorsh,
    }
    #[derive(Clone, Debug, PartialEq, borsh::BorshDeserialize, borsh::BorshSerialize)]
    struct TestInnerBorsh {
        data: String,
    }
    impl SplDiscriminate for TestBorsh {
        const SPL_DISCRIMINATOR: ArrayDiscriminator =
            ArrayDiscriminator::new([5; ArrayDiscriminator::LENGTH]);
    }
    #[test]
    fn borsh_value() {
        let initial_data = "This is a pretty cool test!";
        let initial_inner_data = "And it gets even cooler!";
        // exactly the right size
        let tlv_size = 4 + initial_data.len() + 4 + initial_inner_data.len();
        let account_size = get_base_len() + tlv_size;
        let mut buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        // don't actually need to hold onto the data!
        let _ = state.alloc::<TestBorsh>(tlv_size).unwrap();
        let test_borsh = TestBorsh {
            data: initial_data.to_string(),
            inner: TestInnerBorsh {
                data: initial_inner_data.to_string(),
            },
        };
        state.borsh_serialize(&test_borsh).unwrap();
        let deser = state.borsh_deserialize::<TestBorsh>().unwrap();
        assert_eq!(deser, test_borsh);

        // writing too much data fails
        let too_much_data = "This is a pretty cool test!?";
        assert_eq!(
            state
                .borsh_serialize(&TestBorsh {
                    data: too_much_data.to_string(),
                    inner: TestInnerBorsh {
                        data: initial_inner_data.to_string(),
                    }
                })
                .unwrap_err(),
            ProgramError::BorshIoError("failed to write whole buffer".to_string()),
        );
    }
}
