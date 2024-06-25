//! Type-length-value structure definition and manipulation

use {
    crate::{error::TlvError, length::Length, variable_len_pack::VariableLenPack},
    bytemuck::Pod,
    solana_program::{account_info::AccountInfo, program_error::ProgramError},
    spl_discriminator::{ArrayDiscriminator, SplDiscriminate},
    spl_pod::bytemuck::{pod_from_bytes, pod_from_bytes_mut},
    std::{cmp::Ordering, mem::size_of},
};

/// Get the current TlvIndices from the current spot
const fn get_indices_unchecked(type_start: usize, value_repetition_number: usize) -> TlvIndices {
    let length_start = type_start.saturating_add(size_of::<ArrayDiscriminator>());
    let value_start = length_start.saturating_add(size_of::<Length>());
    TlvIndices {
        type_start,
        length_start,
        value_start,
        value_repetition_number,
    }
}

/// Internal helper struct for returning the indices of the type, length, and
/// value in a TLV entry
#[derive(Debug)]
struct TlvIndices {
    pub type_start: usize,
    pub length_start: usize,
    pub value_start: usize,
    pub value_repetition_number: usize,
}

fn get_indices(
    tlv_data: &[u8],
    value_discriminator: ArrayDiscriminator,
    init: bool,
    repetition_number: Option<usize>,
) -> Result<TlvIndices, ProgramError> {
    let mut current_repetition_number = 0;
    let mut start_index = 0;
    while start_index < tlv_data.len() {
        let tlv_indices = get_indices_unchecked(start_index, current_repetition_number);
        if tlv_data.len() < tlv_indices.value_start {
            return Err(ProgramError::InvalidAccountData);
        }
        let discriminator = ArrayDiscriminator::try_from(
            &tlv_data[tlv_indices.type_start..tlv_indices.length_start],
        )?;
        if discriminator == value_discriminator {
            if let Some(desired_repetition_number) = repetition_number {
                if current_repetition_number == desired_repetition_number {
                    return Ok(tlv_indices);
                }
            }
            current_repetition_number += 1;
        // got to an empty spot, init here, or error if we're searching, since
        // nothing is written after an Uninitialized spot
        } else if discriminator == ArrayDiscriminator::UNINITIALIZED {
            if init {
                return Ok(tlv_indices);
            } else {
                return Err(TlvError::TypeNotFound.into());
            }
        }
        let length =
            pod_from_bytes::<Length>(&tlv_data[tlv_indices.length_start..tlv_indices.value_start])?;
        let value_end_index = tlv_indices
            .value_start
            .saturating_add(usize::try_from(*length)?);
        start_index = value_end_index;
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
        // This function is not concerned with repetitions, so we can just
        // arbitrarily pass `0` here
        let tlv_indices = get_indices_unchecked(start_index, 0);
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

fn get_bytes<V: SplDiscriminate>(
    tlv_data: &[u8],
    repetition_number: usize,
) -> Result<&[u8], ProgramError> {
    let TlvIndices {
        type_start: _,
        length_start,
        value_start,
        value_repetition_number: _,
    } = get_indices(
        tlv_data,
        V::SPL_DISCRIMINATOR,
        false,
        Some(repetition_number),
    )?;
    // get_indices has checked that tlv_data is long enough to include these
    // indices
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
/// `4` with the discriminator `[2, 2, 2, 2, 2, 2, 2, 2]`, we can deserialize
/// this buffer as follows:
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
/// let value = state.get_first_value::<MyPodValue>().unwrap();
/// assert_eq!(value.data, [0, 1, 0, 0, 0, 0, 0, 0]);
/// let value = state.get_first_value::<MyOtherPodValue>().unwrap();
/// assert_eq!(value.data, 4);
/// ```
///
/// See the README and tests for more examples on how to use these types.
pub trait TlvState {
    /// Get the full buffer containing all TLV data
    fn get_data(&self) -> &[u8];

    /// Unpack a portion of the TLV data as the desired Pod type for the entry
    /// number specified
    fn get_value_with_repetition<V: SplDiscriminate + Pod>(
        &self,
        repetition_number: usize,
    ) -> Result<&V, ProgramError> {
        let data = get_bytes::<V>(self.get_data(), repetition_number)?;
        pod_from_bytes::<V>(data)
    }

    /// Unpack a portion of the TLV data as the desired Pod type for the first
    /// entry found
    fn get_first_value<V: SplDiscriminate + Pod>(&self) -> Result<&V, ProgramError> {
        self.get_value_with_repetition::<V>(0)
    }

    /// Unpacks a portion of the TLV data as the desired variable-length type
    /// for the entry number specified
    fn get_variable_len_value_with_repetition<V: SplDiscriminate + VariableLenPack>(
        &self,
        repetition_number: usize,
    ) -> Result<V, ProgramError> {
        let data = get_bytes::<V>(self.get_data(), repetition_number)?;
        V::unpack_from_slice(data)
    }

    /// Unpacks a portion of the TLV data as the desired variable-length type
    /// for the first entry found
    fn get_first_variable_len_value<V: SplDiscriminate + VariableLenPack>(
        &self,
    ) -> Result<V, ProgramError> {
        self.get_variable_len_value_with_repetition::<V>(0)
    }

    /// Unpack a portion of the TLV data as bytes for the entry number specified
    fn get_bytes_with_repetition<V: SplDiscriminate>(
        &self,
        repetition_number: usize,
    ) -> Result<&[u8], ProgramError> {
        get_bytes::<V>(self.get_data(), repetition_number)
    }

    /// Unpack a portion of the TLV data as bytes for the first entry found
    fn get_first_bytes<V: SplDiscriminate>(&self) -> Result<&[u8], ProgramError> {
        self.get_bytes_with_repetition::<V>(0)
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

/// Encapsulates immutable base state data (mint or account) with possible
/// extensions
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

/// Encapsulates mutable base state data (mint or account) with possible
/// extensions
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

    /// Unpack a portion of the TLV data as the desired type that allows
    /// modifying the type for the entry number specified
    pub fn get_value_with_repetition_mut<V: SplDiscriminate + Pod>(
        &mut self,
        repetition_number: usize,
    ) -> Result<&mut V, ProgramError> {
        let data = self.get_bytes_with_repetition_mut::<V>(repetition_number)?;
        pod_from_bytes_mut::<V>(data)
    }

    /// Unpack a portion of the TLV data as the desired type that allows
    /// modifying the type for the first entry found
    pub fn get_first_value_mut<V: SplDiscriminate + Pod>(
        &mut self,
    ) -> Result<&mut V, ProgramError> {
        self.get_value_with_repetition_mut::<V>(0)
    }

    /// Unpack a portion of the TLV data as mutable bytes for the entry number
    /// specified
    pub fn get_bytes_with_repetition_mut<V: SplDiscriminate>(
        &mut self,
        repetition_number: usize,
    ) -> Result<&mut [u8], ProgramError> {
        let TlvIndices {
            type_start: _,
            length_start,
            value_start,
            value_repetition_number: _,
        } = get_indices(
            self.data,
            V::SPL_DISCRIMINATOR,
            false,
            Some(repetition_number),
        )?;

        let length = pod_from_bytes::<Length>(&self.data[length_start..value_start])?;
        let value_end = value_start.saturating_add(usize::try_from(*length)?);
        if self.data.len() < value_end {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(&mut self.data[value_start..value_end])
    }

    /// Unpack a portion of the TLV data as mutable bytes for the first entry
    /// found
    pub fn get_first_bytes_mut<V: SplDiscriminate>(&mut self) -> Result<&mut [u8], ProgramError> {
        self.get_bytes_with_repetition_mut::<V>(0)
    }

    /// Packs the default TLV data into the first open slot in the data buffer.
    /// Handles repetition based on the boolean arg provided:
    /// * `true`:   If extension is already found in the buffer,
    /// it returns an error.
    /// * `false`:  Will add a new entry to the next open slot.
    pub fn init_value<V: SplDiscriminate + Pod + Default>(
        &mut self,
        allow_repetition: bool,
    ) -> Result<(&mut V, usize), ProgramError> {
        let length = size_of::<V>();
        let (buffer, repetition_number) = self.alloc::<V>(length, allow_repetition)?;
        let extension_ref = pod_from_bytes_mut::<V>(buffer)?;
        *extension_ref = V::default();
        Ok((extension_ref, repetition_number))
    }

    /// Packs a variable-length value into its appropriate data segment, where
    /// repeating discriminators _are_ allowed
    pub fn pack_variable_len_value_with_repetition<V: SplDiscriminate + VariableLenPack>(
        &mut self,
        value: &V,
        repetition_number: usize,
    ) -> Result<(), ProgramError> {
        let data = self.get_bytes_with_repetition_mut::<V>(repetition_number)?;
        // NOTE: Do *not* use `pack`, since the length check will cause
        // reallocations to smaller sizes to fail
        value.pack_into_slice(data)
    }

    /// Packs a variable-length value into its appropriate data segment, where
    /// no repeating discriminators are allowed
    pub fn pack_first_variable_len_value<V: SplDiscriminate + VariableLenPack>(
        &mut self,
        value: &V,
    ) -> Result<(), ProgramError> {
        self.pack_variable_len_value_with_repetition::<V>(value, 0)
    }

    /// Allocate the given number of bytes for the given SplDiscriminate
    pub fn alloc<V: SplDiscriminate>(
        &mut self,
        length: usize,
        allow_repetition: bool,
    ) -> Result<(&mut [u8], usize), ProgramError> {
        let TlvIndices {
            type_start,
            length_start,
            value_start,
            value_repetition_number,
        } = get_indices(
            self.data,
            V::SPL_DISCRIMINATOR,
            true,
            if allow_repetition { None } else { Some(0) },
        )?;

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
            Ok((
                &mut self.data[value_start..value_end],
                value_repetition_number,
            ))
        } else {
            Err(TlvError::TypeAlreadyExists.into())
        }
    }

    /// Allocates and serializes a new TLV entry from a `VariableLenPack` type
    pub fn alloc_and_pack_variable_len_entry<V: SplDiscriminate + VariableLenPack>(
        &mut self,
        value: &V,
        allow_repetition: bool,
    ) -> Result<usize, ProgramError> {
        let length = value.get_packed_len()?;
        let (data, repetition_number) = self.alloc::<V>(length, allow_repetition)?;
        value.pack_into_slice(data)?;
        Ok(repetition_number)
    }

    /// Reallocate the given number of bytes for the given SplDiscriminate. If
    /// the new length is smaller, it will compact the rest of the buffer
    /// and zero out the difference at the end. If it's larger, it will move
    /// the rest of the buffer data and zero out the new data.
    pub fn realloc_with_repetition<V: SplDiscriminate>(
        &mut self,
        length: usize,
        repetition_number: usize,
    ) -> Result<&mut [u8], ProgramError> {
        let TlvIndices {
            type_start: _,
            length_start,
            value_start,
            value_repetition_number: _,
        } = get_indices(
            self.data,
            V::SPL_DISCRIMINATOR,
            false,
            Some(repetition_number),
        )?;
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

    /// Reallocate the given number of bytes for the given SplDiscriminate,
    /// where no repeating discriminators are allowed
    pub fn realloc_first<V: SplDiscriminate>(
        &mut self,
        length: usize,
    ) -> Result<&mut [u8], ProgramError> {
        self.realloc_with_repetition::<V>(length, 0)
    }
}

impl<'a> TlvState for TlvStateMut<'a> {
    fn get_data(&self) -> &[u8] {
        self.data
    }
}

/// Packs a variable-length value into an existing TLV space, reallocating
/// the account and TLV as needed to accommodate for any change in space
pub fn realloc_and_pack_variable_len_with_repetition<V: SplDiscriminate + VariableLenPack>(
    account_info: &AccountInfo,
    value: &V,
    repetition_number: usize,
) -> Result<(), ProgramError> {
    let previous_length = {
        let data = account_info.try_borrow_data()?;
        let TlvIndices {
            type_start: _,
            length_start,
            value_start,
            value_repetition_number: _,
        } = get_indices(&data, V::SPL_DISCRIMINATOR, false, Some(repetition_number))?;
        usize::try_from(*pod_from_bytes::<Length>(&data[length_start..value_start])?)?
    };
    let new_length = value.get_packed_len()?;
    let previous_account_size = account_info.try_data_len()?;
    if previous_length < new_length {
        // size increased, so realloc the account, then the TLV entry, then write data
        let additional_bytes = new_length
            .checked_sub(previous_length)
            .ok_or(ProgramError::AccountDataTooSmall)?;
        account_info.realloc(previous_account_size.saturating_add(additional_bytes), true)?;
        let mut buffer = account_info.try_borrow_mut_data()?;
        let mut state = TlvStateMut::unpack(&mut buffer)?;
        state.realloc_with_repetition::<V>(new_length, repetition_number)?;
        state.pack_variable_len_value_with_repetition(value, repetition_number)?;
    } else {
        // do it backwards otherwise, write the state, realloc TLV, then the account
        let mut buffer = account_info.try_borrow_mut_data()?;
        let mut state = TlvStateMut::unpack(&mut buffer)?;
        state.pack_variable_len_value_with_repetition(value, repetition_number)?;
        let removed_bytes = previous_length
            .checked_sub(new_length)
            .ok_or(ProgramError::AccountDataTooSmall)?;
        if removed_bytes > 0 {
            // we decreased the size, so need to realloc the TLV, then the account
            state.realloc_with_repetition::<V>(new_length, repetition_number)?;
            // this is probably fine, but be safe and avoid invalidating references
            drop(buffer);
            account_info.realloc(previous_account_size.saturating_sub(removed_bytes), false)?;
        }
    }
    Ok(())
}

/// Packs a variable-length value into an existing TLV space, where no repeating
/// discriminators are allowed
pub fn realloc_and_pack_first_variable_len<V: SplDiscriminate + VariableLenPack>(
    account_info: &AccountInfo,
    value: &V,
) -> Result<(), ProgramError> {
    realloc_and_pack_variable_len_with_repetition::<V>(account_info, value, 0)
}

/// Get the base size required for TLV data
const fn get_base_len() -> usize {
    get_indices_unchecked(0, 0).value_start
}

fn check_data(tlv_data: &[u8]) -> Result<(), ProgramError> {
    // should be able to iterate through all entries in the TLV structure
    let _ = get_discriminators_and_end_index(tlv_data)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use {
        super::*,
        bytemuck::{Pod, Zeroable},
    };

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
        let value = state.get_first_value::<TestValue>().unwrap();
        assert_eq!(value.data, [1; 32]);
        assert_eq!(
            state.get_first_value::<TestEmptyValue>(),
            Err(ProgramError::InvalidAccountData)
        );

        let mut test_buffer = TEST_BUFFER.to_vec();
        let state = TlvStateMut::unpack(&mut test_buffer).unwrap();
        let value = state.get_first_value::<TestValue>().unwrap();
        assert_eq!(value.data, [1; 32]);
        let state = TlvStateOwned::unpack(test_buffer).unwrap();
        let value = state.get_first_value::<TestValue>().unwrap();
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
            state.get_first_value::<TestValue>(),
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
            state.get_first_value::<TestValue>(),
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
            (vec![ArrayDiscriminator::from(1)], 12)
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
        let value = state.init_value::<TestValue>(false).unwrap().0;
        let data = [100; 32];
        value.data = data;
        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[TestValue::SPL_DISCRIMINATOR],
        );
        assert_eq!(&state.get_first_value::<TestValue>().unwrap().data, &data,);

        // fail init extension when already initialized
        assert_eq!(
            state.init_value::<TestValue>(false).unwrap_err(),
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
        let unpacked = state.get_first_value_mut::<TestValue>().unwrap();
        assert_eq!(*unpacked, TestValue { data });

        // update extension
        let new_data = [101; 32];
        unpacked.data = new_data;

        // check updates are propagated
        let state = TlvStateBorrowed::unpack(&buffer).unwrap();
        let unpacked = state.get_first_value::<TestValue>().unwrap();
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
        let new_value = state.init_value::<TestSmallValue>(false).unwrap().0;
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
            state.init_value::<TestEmptyValue>(false),
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
        let value = state.init_value::<TestValue>(false).unwrap().0;
        value.data = data;
        let value = state.init_value::<TestSmallValue>(false).unwrap().0;
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

        let value = state.init_value::<TestSmallValue>(false).unwrap().0;
        value.data = small_data;
        let value = state.init_value::<TestValue>(false).unwrap().0;
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
            state.get_first_value::<TestValue>().unwrap(),
            other_state.get_first_value::<TestValue>().unwrap()
        );
        assert_eq!(
            state.get_first_value::<TestSmallValue>().unwrap(),
            other_state.get_first_value::<TestSmallValue>().unwrap()
        );
    }

    #[test]
    fn init_nonzero_default() {
        let account_size = get_base_len() + size_of::<TestNonZeroDefault>();
        let mut buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
        let value = state.init_value::<TestNonZeroDefault>(false).unwrap().0;
        assert_eq!(value.data, TEST_NON_ZERO_DEFAULT_DATA);
    }

    #[test]
    fn init_buffer_too_small() {
        let account_size = get_base_len() + size_of::<TestValue>();
        let mut buffer = vec![0; account_size - 1];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
        let err = state.init_value::<TestValue>(false).unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);

        // hack the buffer to look like it was initialized, still fails
        let discriminator_ref = &mut state.data[0..ArrayDiscriminator::LENGTH];
        discriminator_ref.copy_from_slice(TestValue::SPL_DISCRIMINATOR.as_ref());
        state.data[ArrayDiscriminator::LENGTH] = 32;
        let err = state.get_first_value::<TestValue>().unwrap_err();
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
            state.get_first_value::<TestEmptyValue>().unwrap_err(),
            TlvError::TypeNotFound.into(),
        );

        state.init_value::<TestEmptyValue>(false).unwrap();
        state.get_first_value::<TestEmptyValue>().unwrap();

        // re-init fails
        assert_eq!(
            state.init_value::<TestEmptyValue>(false).unwrap_err(),
            TlvError::TypeAlreadyExists.into(),
        );
    }

    #[test]
    fn alloc_first() {
        let tlv_size = 1;
        let account_size = get_base_len() + tlv_size;
        let mut buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        // not enough room
        let data = state.alloc::<TestValue>(tlv_size, false).unwrap().0;
        assert_eq!(
            pod_from_bytes_mut::<TestValue>(data).unwrap_err(),
            ProgramError::InvalidArgument,
        );

        // can't double alloc
        assert_eq!(
            state.alloc::<TestValue>(tlv_size, false).unwrap_err(),
            TlvError::TypeAlreadyExists.into(),
        );
    }

    #[test]
    fn alloc_with_repetition() {
        let tlv_size = 1;
        let account_size = (get_base_len() + tlv_size) * 2;
        let mut buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        let (data, repetition_number) = state.alloc::<TestValue>(tlv_size, true).unwrap();
        assert_eq!(repetition_number, 0);

        // not enough room
        assert_eq!(
            pod_from_bytes_mut::<TestValue>(data).unwrap_err(),
            ProgramError::InvalidArgument,
        );

        // Can alloc again!
        let (_data, repetition_number) = state.alloc::<TestValue>(tlv_size, true).unwrap();
        assert_eq!(repetition_number, 1);
    }

    #[test]
    fn realloc_first() {
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
        let _ = state.alloc::<TestValue>(TLV_SIZE, false).unwrap();
        let _ = state.init_value::<TestNonZeroDefault>(false).unwrap();

        // realloc first entry to larger, all 0
        let data = state
            .realloc_first::<TestValue>(TLV_SIZE + EXTRA_SPACE)
            .unwrap();
        assert_eq!(data, [0; TLV_SIZE + EXTRA_SPACE]);
        let value = state.get_first_value::<TestNonZeroDefault>().unwrap();
        assert_eq!(*value, TestNonZeroDefault::default());

        // realloc to smaller, still all 0
        let data = state.realloc_first::<TestValue>(SMALL_SIZE).unwrap();
        assert_eq!(data, [0; SMALL_SIZE]);
        let value = state.get_first_value::<TestNonZeroDefault>().unwrap();
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
                .realloc_first::<TestValue>(TLV_SIZE + EXTRA_SPACE + 1)
                .unwrap_err(),
            ProgramError::InvalidAccountData,
        );
    }

    #[test]
    fn realloc_with_repeating_entries() {
        const TLV_SIZE: usize = 10;
        const EXTRA_SPACE: usize = 5;
        const SMALL_SIZE: usize = 2;
        const ACCOUNT_SIZE: usize = get_base_len()
            + TLV_SIZE
            + EXTRA_SPACE
            + get_base_len()
            + TLV_SIZE
            + get_base_len()
            + size_of::<TestNonZeroDefault>();
        let mut buffer = vec![0; ACCOUNT_SIZE];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        // alloc both types, two for the first type and one for the second
        let _ = state.alloc::<TestValue>(TLV_SIZE, true).unwrap();
        let _ = state.alloc::<TestValue>(TLV_SIZE, true).unwrap();
        let _ = state.init_value::<TestNonZeroDefault>(true).unwrap();

        // realloc first entry to larger, all 0
        let data = state
            .realloc_with_repetition::<TestValue>(TLV_SIZE + EXTRA_SPACE, 0)
            .unwrap();
        assert_eq!(data, [0; TLV_SIZE + EXTRA_SPACE]);
        let value = state.get_bytes_with_repetition::<TestValue>(0).unwrap();
        assert_eq!(*value, [0; TLV_SIZE + EXTRA_SPACE]);
        let value = state.get_bytes_with_repetition::<TestValue>(1).unwrap();
        assert_eq!(*value, [0; TLV_SIZE]);
        let value = state.get_first_value::<TestNonZeroDefault>().unwrap();
        assert_eq!(*value, TestNonZeroDefault::default());

        // realloc to smaller, still all 0
        let data = state
            .realloc_with_repetition::<TestValue>(SMALL_SIZE, 0)
            .unwrap();
        assert_eq!(data, [0; SMALL_SIZE]);
        let value = state.get_bytes_with_repetition::<TestValue>(0).unwrap();
        assert_eq!(*value, [0; SMALL_SIZE]);
        let value = state.get_bytes_with_repetition::<TestValue>(1).unwrap();
        assert_eq!(*value, [0; TLV_SIZE]);
        let value = state.get_first_value::<TestNonZeroDefault>().unwrap();
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
                .realloc_with_repetition::<TestValue>(TLV_SIZE + EXTRA_SPACE + 1, 0)
                .unwrap_err(),
            ProgramError::InvalidAccountData,
        );
    }

    #[derive(Clone, Debug, PartialEq)]
    struct TestVariableLen {
        data: String, // test with a variable length type
    }
    impl SplDiscriminate for TestVariableLen {
        const SPL_DISCRIMINATOR: ArrayDiscriminator =
            ArrayDiscriminator::new([5; ArrayDiscriminator::LENGTH]);
    }
    impl VariableLenPack for TestVariableLen {
        fn pack_into_slice(&self, dst: &mut [u8]) -> Result<(), ProgramError> {
            let bytes = self.data.as_bytes();
            let end = 8 + bytes.len();
            if dst.len() < end {
                Err(ProgramError::InvalidAccountData)
            } else {
                dst[..8].copy_from_slice(&self.data.len().to_le_bytes());
                dst[8..end].copy_from_slice(bytes);
                Ok(())
            }
        }
        fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
            let length = u64::from_le_bytes(src[..8].try_into().unwrap()) as usize;
            if src[8..8 + length].len() != length {
                return Err(ProgramError::InvalidAccountData);
            }
            let data = std::str::from_utf8(&src[8..8 + length])
                .unwrap()
                .to_string();
            Ok(Self { data })
        }
        fn get_packed_len(&self) -> Result<usize, ProgramError> {
            Ok(size_of::<u64>().saturating_add(self.data.len()))
        }
    }

    #[test]
    fn first_variable_len_value() {
        let initial_data = "This is a pretty cool test!";
        // exactly the right size
        let tlv_size = 8 + initial_data.len();
        let account_size = get_base_len() + tlv_size;
        let mut buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        // don't actually need to hold onto the data!
        let _ = state.alloc::<TestVariableLen>(tlv_size, false).unwrap();
        let test_variable_len = TestVariableLen {
            data: initial_data.to_string(),
        };
        state
            .pack_first_variable_len_value(&test_variable_len)
            .unwrap();
        let deser = state
            .get_first_variable_len_value::<TestVariableLen>()
            .unwrap();
        assert_eq!(deser, test_variable_len);

        // writing too much data fails
        let too_much_data = "This is a pretty cool test!?";
        assert_eq!(
            state
                .pack_first_variable_len_value(&TestVariableLen {
                    data: too_much_data.to_string(),
                })
                .unwrap_err(),
            ProgramError::InvalidAccountData
        );
    }

    #[test]
    fn variable_len_value_with_repetition() {
        let variable_len_1 = TestVariableLen {
            data: "Let's see if we can pack multiple variable length values".to_string(),
        };
        let tlv_size_1 = 8 + variable_len_1.data.len();

        let variable_len_2 = TestVariableLen {
            data: "I think we can".to_string(),
        };
        let tlv_size_2 = 8 + variable_len_2.data.len();

        let variable_len_3 = TestVariableLen {
            data: "In fact, I know we can!".to_string(),
        };
        let tlv_size_3 = 8 + variable_len_3.data.len();

        let variable_len_4 = TestVariableLen {
            data: "How cool is this?".to_string(),
        };
        let tlv_size_4 = 8 + variable_len_4.data.len();

        let account_size = get_base_len()
            + tlv_size_1
            + get_base_len()
            + tlv_size_2
            + get_base_len()
            + tlv_size_3
            + get_base_len()
            + tlv_size_4;
        let mut buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        let (_, repetition_number) = state.alloc::<TestVariableLen>(tlv_size_1, true).unwrap();
        state
            .pack_variable_len_value_with_repetition(&variable_len_1, repetition_number)
            .unwrap();
        assert_eq!(repetition_number, 0);
        assert_eq!(
            state
                .get_first_variable_len_value::<TestVariableLen>()
                .unwrap(),
            variable_len_1,
        );

        let (_, repetition_number) = state.alloc::<TestVariableLen>(tlv_size_2, true).unwrap();
        state
            .pack_variable_len_value_with_repetition(&variable_len_2, repetition_number)
            .unwrap();
        assert_eq!(repetition_number, 1);
        assert_eq!(
            state
                .get_variable_len_value_with_repetition::<TestVariableLen>(repetition_number)
                .unwrap(),
            variable_len_2,
        );

        let (_, repetition_number) = state.alloc::<TestVariableLen>(tlv_size_3, true).unwrap();
        state
            .pack_variable_len_value_with_repetition(&variable_len_3, repetition_number)
            .unwrap();
        assert_eq!(repetition_number, 2);
        assert_eq!(
            state
                .get_variable_len_value_with_repetition::<TestVariableLen>(repetition_number)
                .unwrap(),
            variable_len_3,
        );

        let (_, repetition_number) = state.alloc::<TestVariableLen>(tlv_size_4, true).unwrap();
        state
            .pack_variable_len_value_with_repetition(&variable_len_4, repetition_number)
            .unwrap();
        assert_eq!(repetition_number, 3);
        assert_eq!(
            state
                .get_variable_len_value_with_repetition::<TestVariableLen>(repetition_number)
                .unwrap(),
            variable_len_4,
        );
    }

    #[test]
    fn add_entry_mix_and_match() {
        let mut buffer = vec![];

        // Add an entry for a fixed length value
        let fixed_data = TestValue { data: [1; 32] };
        let tlv_size = get_base_len() + size_of::<TestValue>();
        buffer.extend(vec![0; tlv_size]);
        {
            let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
            let (value, repetition_number) = state.init_value::<TestValue>(true).unwrap();
            value.data = fixed_data.data;
            assert_eq!(repetition_number, 0);
            assert_eq!(*value, fixed_data);
        }

        // Add an entry for a variable length value
        let variable_data = TestVariableLen {
            data: "This is my first variable length entry!".to_string(),
        };
        let tlv_size = get_base_len() + 8 + variable_data.data.len();
        buffer.extend(vec![0; tlv_size]);
        {
            let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
            let repetition_number = state
                .alloc_and_pack_variable_len_entry(&variable_data, true)
                .unwrap();
            let value = state
                .get_variable_len_value_with_repetition::<TestVariableLen>(repetition_number)
                .unwrap();
            assert_eq!(repetition_number, 0);
            assert_eq!(value, variable_data);
        }

        // Add another entry for a variable length value
        let variable_data = TestVariableLen {
            data: "This is actually my second variable length entry!".to_string(),
        };
        let tlv_size = get_base_len() + 8 + variable_data.data.len();
        buffer.extend(vec![0; tlv_size]);
        {
            let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
            let repetition_number = state
                .alloc_and_pack_variable_len_entry(&variable_data, true)
                .unwrap();
            let value = state
                .get_variable_len_value_with_repetition::<TestVariableLen>(repetition_number)
                .unwrap();
            assert_eq!(repetition_number, 1);
            assert_eq!(value, variable_data);
        }

        // Add another entry for a fixed length value
        let fixed_data = TestValue { data: [2; 32] };
        let tlv_size = get_base_len() + size_of::<TestValue>();
        buffer.extend(vec![0; tlv_size]);
        {
            let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
            let (value, repetition_number) = state.init_value::<TestValue>(true).unwrap();
            value.data = fixed_data.data;
            assert_eq!(repetition_number, 1);
            assert_eq!(*value, fixed_data);
        }

        // Add another entry for a fixed length value
        let fixed_data = TestValue { data: [3; 32] };
        let tlv_size = get_base_len() + size_of::<TestValue>();
        buffer.extend(vec![0; tlv_size]);
        {
            let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
            let (value, repetition_number) = state.init_value::<TestValue>(true).unwrap();
            value.data = fixed_data.data;
            assert_eq!(repetition_number, 2);
            assert_eq!(*value, fixed_data);
        }

        // Add another entry for a variable length value
        let variable_data = TestVariableLen {
            data: "Wow! My third variable length entry!".to_string(),
        };
        let tlv_size = get_base_len() + 8 + variable_data.data.len();
        buffer.extend(vec![0; tlv_size]);
        {
            let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
            let repetition_number = state
                .alloc_and_pack_variable_len_entry(&variable_data, true)
                .unwrap();
            let value = state
                .get_variable_len_value_with_repetition::<TestVariableLen>(repetition_number)
                .unwrap();
            assert_eq!(repetition_number, 2);
            assert_eq!(value, variable_data);
        }
    }
}
