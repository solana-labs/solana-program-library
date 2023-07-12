//! Type-length-value structure definition and manipulation

use {
    crate::{
        error::TlvError,
        length::Length,
        pod::{pod_from_bytes, pod_from_bytes_mut},
        variable_len_pack::VariableLenPack,
    },
    bytemuck::Pod,
    solana_program::{account_info::AccountInfo, program_error::ProgramError},
    spl_discriminator::{ArrayDiscriminator, SplDiscriminate},
    std::{cmp::Ordering, mem::size_of},
};

/// Get the current TlvIndices from the current spot
const fn get_indices_unchecked(type_start: usize) -> TlvIndices {
    let length_start =
        type_start.saturating_add(size_of::<ArrayDiscriminator>());
    let value_start = length_start.saturating_add(size_of::<Length>());
    TlvIndices {
        type_start,
        length_start,
        value_start,
    }
}

/// Struct for returning the indices of the type, length, and
/// value in a TLV entry
#[derive(Debug)]
pub struct TlvIndices {
    /// Index where the type begins
    pub type_start: usize,
    /// Index where the length begins
    pub length_start: usize,
    /// Index where the value begins
    pub value_start: usize,
}

type TlvIndicesWithEntryNumber = (TlvIndices, usize);

fn get_indices(
    tlv_data: &[u8],
    value_discriminator: ArrayDiscriminator,
    init: bool,
    entry_number: Option<usize>,
) -> Result<TlvIndicesWithEntryNumber, ProgramError> {
    let mut current_entry_number = 0;
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
            if let Some(desired_entry_number) = entry_number {
                if current_entry_number == desired_entry_number {
                    return Ok((tlv_indices, current_entry_number));
                }
            }
            current_entry_number += 1;
        // got to an empty spot, init here, or error if we're searching, since
        // nothing is written after an Uninitialized spot
        } else if discriminator == ArrayDiscriminator::UNINITIALIZED {
            if init {
                return Ok((tlv_indices, current_entry_number));
            } else {
                return Err(TlvError::TypeNotFound.into());
            }
        }
        let length = pod_from_bytes::<Length>(
            &tlv_data[tlv_indices.length_start..tlv_indices.value_start],
        )?;
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
        let tlv_indices = get_indices_unchecked(start_index);
        if tlv_data.len() < tlv_indices.length_start {
            // we got to the end, but there might be some uninitialized data
            // after
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
) -> Result<&[u8], ProgramError> {
    let (
        TlvIndices {
            type_start: _,
            length_start,
            value_start,
        },
        _,
    ) = get_indices(tlv_data, V::SPL_DISCRIMINATOR, false, Some(0))?;
    // get_indices has checked that tlv_data is long enough to include these
    // indices
    let length =
        pod_from_bytes::<Length>(&tlv_data[length_start..value_start])?;
    let value_end = value_start.saturating_add(usize::try_from(*length)?);
    if tlv_data.len() < value_end {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(&tlv_data[value_start..value_end])
}

/// Same as the `get_bytes` function, but allows specifying which entry to
/// retrieve. This is useful for arrays of TLVs.
fn get_bytes_specific<V: SplDiscriminate>(
    tlv_data: &[u8],
    entry_number: usize,
) -> Result<&[u8], ProgramError> {
    let (
        TlvIndices {
            type_start: _,
            length_start,
            value_start,
        },
        _,
    ) = get_indices(tlv_data, V::SPL_DISCRIMINATOR, false, Some(entry_number))?;
    // get_indices has checked that tlv_data is long enough to include these
    // indices
    let length =
        pod_from_bytes::<Length>(&tlv_data[length_start..value_start])?;
    let value_end = value_start.saturating_add(usize::try_from(*length)?);
    if tlv_data.len() < value_end {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(&tlv_data[value_start..value_end])
}

/// Trait for "strict" TLV state - meaning discriminators are unique.
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
///     spl_type_length_value::state::{TlvStateStrict, TlvStateStrictBorrowed, TlvStateStrictMut},
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
/// let state = TlvStateStrictBorrowed::unpack(&buffer).unwrap();
/// let value = state.get_value::<MyPodValue>().unwrap();
/// assert_eq!(value.data, [0, 1, 0, 0, 0, 0, 0, 0]);
/// let value = state.get_value::<MyOtherPodValue>().unwrap();
/// assert_eq!(value.data, 4);
/// ```
///
/// See the README and tests for more examples on how to use these types.
pub trait TlvStateStrict {
    /// Get the full buffer containing all TLV data
    fn get_data(&self) -> &[u8];

    /// Unpack a portion of the TLV data as the desired Pod type
    fn get_value<V: SplDiscriminate + Pod>(&self) -> Result<&V, ProgramError> {
        let data = get_bytes::<V>(self.get_data())?;
        pod_from_bytes::<V>(data)
    }

    /// Unpacks a portion of the TLV data as the desired variable-length type
    fn get_variable_len_value<V: SplDiscriminate + VariableLenPack>(
        &self,
    ) -> Result<V, ProgramError> {
        let data = get_bytes::<V>(self.get_data())?;
        V::unpack_from_slice(data)
    }

    /// Unpack a portion of the TLV data as bytes
    fn get_bytes<V: SplDiscriminate>(&self) -> Result<&[u8], ProgramError> {
        get_bytes::<V>(self.get_data())
    }

    /// Iterates through the TLV entries, returning only the types
    fn get_discriminators(
        &self,
    ) -> Result<Vec<ArrayDiscriminator>, ProgramError> {
        get_discriminators_and_end_index(self.get_data()).map(|v| v.0)
    }

    /// Get the base size required for TLV data
    fn get_base_len() -> usize {
        get_base_len()
    }
}

/// Trait for "non-strict" TLV state - meaning discriminators are allowed to
/// repeat.
///
/// Stores data as any number of type-length-value structures underneath, where:
///
///   * the "type" is an `ArrayDiscriminator`, 8 bytes
///   * the "length" is a `Length`, 4 bytes
///   * the "value" is a slab of "length" bytes
///
/// With this structure, it's possible to hold onto any number of entries with
/// 8-byte discriminators, provided that the total underlying data has enough
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
///     spl_type_length_value::state::{TlvStateNonStrict, TlvStateNonStrictBorrowed, TlvStateNonStrictMut},
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
///   1, 1, 1, 1, 1, 1, 1, 1, // first type's discriminator (again)
///   8, 0, 0, 0,             // first type's length (again)
///   0, 1, 0, 0, 0, 0, 0, 0, // first type's value (again)
///   2, 2, 2, 2, 2, 2, 2, 2, // second type's discriminator
///   1, 0, 0, 0,             // second type's length
///   4,                      // second type's value
/// ];
/// let state = TlvStateNonStrictBorrowed::unpack(&buffer).unwrap();
/// let value = state.get_first::<MyPodValue>().unwrap();
/// assert_eq!(value.data, [0, 1, 0, 0, 0, 0, 0, 0]);
/// // Now retrieve another value of the same type at index 1
/// let value = state.get_value::<MyPodValue>(1).unwrap();
/// assert_eq!(value.data, [0, 1, 0, 0, 0, 0, 0, 0]);
/// let value = state.get_first::<MyOtherPodValue>().unwrap();
/// assert_eq!(value.data, 4);
/// ```
///
/// See the README and tests for more examples on how to use these types.
pub trait TlvStateNonStrict {
    /// Get the full buffer containing all TLV data
    fn get_data(&self) -> &[u8];

    /// Unpack a portion of the TLV data as the desired Pod type at the
    /// designated entry index
    fn get_value<V: SplDiscriminate + Pod>(
        &self,
        entry_number: usize,
    ) -> Result<&V, ProgramError> {
        let data = get_bytes_specific::<V>(self.get_data(), entry_number)?;
        pod_from_bytes::<V>(data)
    }

    /// Unpacks the first TLV entry as the desired Pod type
    fn get_first<V: SplDiscriminate + Pod>(&self) -> Result<&V, ProgramError> {
        self.get_value(0)
    }

    /// Unpacks a portion of the TLV data as the desired variable-length type at
    /// the designated entry index
    fn get_variable_len_value<V: SplDiscriminate + VariableLenPack>(
        &self,
        entry_number: usize,
    ) -> Result<V, ProgramError> {
        let data = get_bytes_specific::<V>(self.get_data(), entry_number)?;
        V::unpack_from_slice(data)
    }

    /// Unpacks the first TLV entry as the desired variable-length type
    fn get_first_variable_len_value<V: SplDiscriminate + VariableLenPack>(
        &self,
    ) -> Result<V, ProgramError> {
        self.get_variable_len_value(0)
    }

    /// Unpack a portion of the TLV data as bytes at the designated entry index
    fn get_bytes<V: SplDiscriminate>(
        &self,
        entry_number: usize,
    ) -> Result<&[u8], ProgramError> {
        get_bytes_specific::<V>(self.get_data(), entry_number)
    }

    /// Iterates through the TLV entries, returning only the types
    fn get_discriminators(
        &self,
    ) -> Result<Vec<ArrayDiscriminator>, ProgramError> {
        get_discriminators_and_end_index(self.get_data()).map(|v| v.0)
    }

    /// Get the base size required for TLV data
    fn get_base_len() -> usize {
        get_base_len()
    }
}

/// Encapsulates owned TLV data for "strict" TLV state
#[derive(Debug, PartialEq)]
pub struct TlvStateStrictOwned {
    /// Raw TLV data, deserialized on demand
    data: Vec<u8>,
}
impl TlvStateStrictOwned {
    /// Unpacks TLV state data
    ///
    /// Fails if no state is initialized or if data is too small
    pub fn unpack(data: Vec<u8>) -> Result<Self, ProgramError> {
        check_data(&data)?;
        Ok(Self { data })
    }
}
impl TlvStateStrict for TlvStateStrictOwned {
    fn get_data(&self) -> &[u8] {
        &self.data
    }
}

/// Encapsulates owned TLV data for "non-strict" TLV state
#[derive(Debug, PartialEq)]
pub struct TlvStateNonStrictOwned {
    /// Raw TLV data, deserialized on demand
    data: Vec<u8>,
}
impl TlvStateNonStrictOwned {
    /// Unpacks TLV state data
    ///
    /// Fails if no state is initialized or if data is too small
    pub fn unpack(data: Vec<u8>) -> Result<Self, ProgramError> {
        check_data(&data)?;
        Ok(Self { data })
    }
}
impl TlvStateNonStrict for TlvStateNonStrictOwned {
    fn get_data(&self) -> &[u8] {
        &self.data
    }
}

/// Encapsulates immutable base state data (mint or account) with possible
/// extensions for "strict" TLV state
#[derive(Debug, PartialEq)]
pub struct TlvStateStrictBorrowed<'data> {
    /// Slice of data containing all TLV data, deserialized on demand
    data: &'data [u8],
}
impl<'data> TlvStateStrictBorrowed<'data> {
    /// Unpacks TLV state data
    ///
    /// Fails if no state is initialized or if data is too small
    pub fn unpack(data: &'data [u8]) -> Result<Self, ProgramError> {
        check_data(data)?;
        Ok(Self { data })
    }
}
impl<'a> TlvStateStrict for TlvStateStrictBorrowed<'a> {
    fn get_data(&self) -> &[u8] {
        self.data
    }
}

/// Encapsulates immutable base state data for "non-strict" TLV state
#[derive(Debug, PartialEq)]
pub struct TlvStateNonStrictBorrowed<'data> {
    /// Slice of data containing all TLV data, deserialized on demand
    data: &'data [u8],
}
impl<'data> TlvStateNonStrictBorrowed<'data> {
    /// Unpacks TLV state data
    ///
    /// Fails if no state is initialized or if data is too small
    pub fn unpack(data: &'data [u8]) -> Result<Self, ProgramError> {
        check_data(data)?;
        Ok(Self { data })
    }
}
impl<'a> TlvStateNonStrict for TlvStateNonStrictBorrowed<'a> {
    fn get_data(&self) -> &[u8] {
        self.data
    }
}

/// Encapsulates mutable base state data (mint or account) with possible
/// extensions for "strict" TLV state
#[derive(Debug, PartialEq)]
pub struct TlvStateStrictMut<'data> {
    /// Slice of data containing all TLV data, deserialized on demand
    data: &'data mut [u8],
}
impl<'data> TlvStateStrictMut<'data> {
    /// Unpacks TLV state data
    ///
    /// Fails if no state is initialized or if data is too small
    pub fn unpack(data: &'data mut [u8]) -> Result<Self, ProgramError> {
        check_data(data)?;
        Ok(Self { data })
    }

    /// Unpack a portion of the TLV data as the desired type that allows
    /// modifying the type
    pub fn get_value_mut<V: SplDiscriminate + Pod>(
        &mut self,
    ) -> Result<&mut V, ProgramError> {
        let data = self.get_bytes_mut::<V>()?;
        pod_from_bytes_mut::<V>(data)
    }

    /// Unpack a portion of the TLV data as mutable bytes
    pub fn get_bytes_mut<V: SplDiscriminate>(
        &mut self,
    ) -> Result<&mut [u8], ProgramError> {
        let (
            TlvIndices {
                type_start: _,
                length_start,
                value_start,
            },
            _,
        ) = get_indices(self.data, V::SPL_DISCRIMINATOR, false, Some(0))?;

        let length =
            pod_from_bytes::<Length>(&self.data[length_start..value_start])?;
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

    /// Packs a variable-length value into its appropriate data segment. Assumes
    /// that space has already been allocated for the given type
    pub fn pack_variable_len_value<V: SplDiscriminate + VariableLenPack>(
        &mut self,
        value: &V,
    ) -> Result<(), ProgramError> {
        let data = self.get_bytes_mut::<V>()?;
        // NOTE: Do *not* use `pack`, since the length check will cause
        // reallocations to smaller sizes to fail
        value.pack_into_slice(data)
    }

    /// Allocate the given number of bytes for the given SplDiscriminate
    /// where no repeating discriminators are allowed
    pub fn alloc<V: SplDiscriminate>(
        &mut self,
        length: usize,
    ) -> Result<&mut [u8], ProgramError> {
        let (
            TlvIndices {
                type_start,
                length_start,
                value_start,
            },
            _,
        ) = get_indices(self.data, V::SPL_DISCRIMINATOR, true, Some(0))?;

        let discriminator =
            ArrayDiscriminator::try_from(&self.data[type_start..length_start])?;
        if discriminator == ArrayDiscriminator::UNINITIALIZED {
            // write type
            let discriminator_ref = &mut self.data[type_start..length_start];
            discriminator_ref.copy_from_slice(V::SPL_DISCRIMINATOR.as_ref());
            // write length
            let length_ref = pod_from_bytes_mut::<Length>(
                &mut self.data[length_start..value_start],
            )?;
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

    /// Allocates and serializes a new TLV entry from a Pod type, where no
    /// repeating discriminators are allowed
    pub fn add_entry<V: SplDiscriminate + Pod>(
        &mut self,
        value: &V,
    ) -> Result<(), ProgramError> {
        let data = self.alloc::<V>(size_of::<V>())?;
        data.copy_from_slice(bytemuck::bytes_of(value));
        Ok(())
    }

    /// Allocates and serializes a new TLV entry from a `VariableLenPack` type,
    /// where no repeating discriminators are allowed
    pub fn add_variable_len_entry<V: SplDiscriminate + VariableLenPack>(
        &mut self,
        value: &V,
    ) -> Result<(), ProgramError> {
        let length = value.get_packed_len()?;
        let data = self.alloc::<V>(length)?;
        value.pack_into_slice(data)?;
        Ok(())
    }

    /// Reallocate the given number of bytes for the given SplDiscriminate. If
    /// the new length is smaller, it will compact the rest of the buffer
    /// and zero out the difference at the end. If it's larger, it will move
    /// the rest of the buffer data and zero out the new data.
    pub fn realloc<V: SplDiscriminate>(
        &mut self,
        length: usize,
    ) -> Result<&mut [u8], ProgramError> {
        let (
            TlvIndices {
                type_start: _,
                length_start,
                value_start,
            },
            _,
        ) = get_indices(self.data, V::SPL_DISCRIMINATOR, false, Some(0))?;
        let (_, end_index) = get_discriminators_and_end_index(self.data)?;
        let data_len = self.data.len();

        let length_ref = pod_from_bytes_mut::<Length>(
            &mut self.data[length_start..value_start],
        )?;
        let old_length = usize::try_from(*length_ref)?;

        // check that we're not going to panic during `copy_within`
        if old_length < length {
            let new_end_index =
                end_index.saturating_add(length.saturating_sub(old_length));
            if new_end_index > data_len {
                return Err(ProgramError::InvalidAccountData);
            }
        }

        // write new length after the check, to avoid getting into a bad
        // situation if trying to recover from an error
        *length_ref = Length::try_from(length)?;

        let old_value_end = value_start.saturating_add(old_length);
        let new_value_end = value_start.saturating_add(length);
        self.data
            .copy_within(old_value_end..end_index, new_value_end);
        match old_length.cmp(&length) {
            Ordering::Greater => {
                // realloc to smaller, fill the end
                let new_end_index =
                    end_index.saturating_sub(old_length.saturating_sub(length));
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
impl<'a> TlvStateStrict for TlvStateStrictMut<'a> {
    fn get_data(&self) -> &[u8] {
        self.data
    }
}

/// Encapsulates mutable base state data for "non-strict" TLV state
#[derive(Debug, PartialEq)]
pub struct TlvStateNonStrictMut<'data> {
    /// Slice of data containing all TLV data, deserialized on demand
    data: &'data mut [u8],
}
impl<'data> TlvStateNonStrictMut<'data> {
    /// Unpacks TLV state data
    ///
    /// Fails if no state is initialized or if data is too small
    pub fn unpack(data: &'data mut [u8]) -> Result<Self, ProgramError> {
        check_data(data)?;
        Ok(Self { data })
    }

    /// Unpack a portion of the TLV data as the desired type that allows
    /// modifying the type, where the particular entry can be found by
    /// index.
    pub fn get_value_mut<V: SplDiscriminate + Pod>(
        &mut self,
        entry_number: usize,
    ) -> Result<&mut V, ProgramError> {
        let data = self.get_bytes_mut::<V>(entry_number)?;
        pod_from_bytes_mut::<V>(data)
    }

    /// Unpack a portion of the TLV data as the desired `Pod` type that allows
    /// modifying the type, where the particular entry can be found by
    /// searching the TLV data for the entry.
    pub fn find_value_mut<V: SplDiscriminate + Pod>(
        &mut self,
        entry: &V,
    ) -> Result<(&mut V, usize), ProgramError> {
        let entry_bytes = bytemuck::bytes_of(entry);
        let mut entry_number = 0;
        loop {
            let found_value = self.get_value::<V>(entry_number)?;
            let found_value_bytes = bytemuck::bytes_of(found_value);
            if found_value_bytes == entry_bytes {
                return Ok((
                    self.get_value_mut::<V>(entry_number)?,
                    entry_number,
                ));
            }
            entry_number += 1;
        }
    }

    /// Unpack a portion of the TLV data as mutable bytes, where the particular
    /// entry can be found by index.
    pub fn get_bytes_mut<V: SplDiscriminate>(
        &mut self,
        entry_number: usize,
    ) -> Result<&mut [u8], ProgramError> {
        let (
            TlvIndices {
                type_start: _,
                length_start,
                value_start,
            },
            _,
        ) = get_indices(
            self.data,
            V::SPL_DISCRIMINATOR,
            false,
            Some(entry_number),
        )?;

        let length =
            pod_from_bytes::<Length>(&self.data[length_start..value_start])?;
        let value_end = value_start.saturating_add(usize::try_from(*length)?);
        if self.data.len() < value_end {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(&mut self.data[value_start..value_end])
    }

    /// Packs the default TLV data into the first open slot in the data buffer.
    /// Does not check for duplicates. Will add a new entry to the next open
    /// slot provided there is enough space.
    pub fn init_value<V: SplDiscriminate + Pod + Default>(
        &mut self,
    ) -> Result<(&mut V, usize), ProgramError> {
        let length = size_of::<V>();
        let (buffer, entry_number) = self.alloc::<V>(length)?;
        let extension_ref = pod_from_bytes_mut::<V>(buffer)?;
        *extension_ref = V::default();
        Ok((extension_ref, entry_number))
    }

    /// Packs a variable-length value into its appropriate data segment. Assumes
    /// that space has already been allocated for the given type
    pub fn pack_variable_len_value<V: SplDiscriminate + VariableLenPack>(
        &mut self,
        value: &V,
        entry_number: usize,
    ) -> Result<(), ProgramError> {
        let data = self.get_bytes_mut::<V>(entry_number)?;
        // NOTE: Do *not* use `pack`, since the length check will cause
        // reallocations to smaller sizes to fail
        value.pack_into_slice(data)
    }

    /// Allocate the given number of bytes for the given SplDiscriminate
    /// where repeating discriminators _are_ allowed. Will add a new entry to
    /// the next open slot provided there is enough space.
    pub fn alloc<V: SplDiscriminate>(
        &mut self,
        length: usize,
    ) -> Result<(&mut [u8], usize), ProgramError> {
        let (
            TlvIndices {
                type_start,
                length_start,
                value_start,
            },
            entry_number,
        ) = get_indices(self.data, V::SPL_DISCRIMINATOR, true, None)?;

        // write type
        let discriminator_ref = &mut self.data[type_start..length_start];
        discriminator_ref.copy_from_slice(V::SPL_DISCRIMINATOR.as_ref());
        // write length
        let length_ref = pod_from_bytes_mut::<Length>(
            &mut self.data[length_start..value_start],
        )?;
        *length_ref = Length::try_from(length)?;

        let value_end = value_start.saturating_add(length);
        if self.data.len() < value_end {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok((&mut self.data[value_start..value_end], entry_number))
    }

    /// Allocates and serializes a new TLV entry from a Pod type, where
    /// repeating discriminators _are_ allowed. Will add a new entry to the
    /// next open slot provided there is enough space.
    pub fn add_entry<V: SplDiscriminate + Pod>(
        &mut self,
        value: &V,
    ) -> Result<(), ProgramError> {
        let (data, _) = self.alloc::<V>(size_of::<V>())?;
        data.copy_from_slice(bytemuck::bytes_of(value));
        Ok(())
    }

    /// Allocates and serializes a new TLV entry from a `VariableLenPack` type,
    /// where repeating discriminators _are_ allowed. Will add a new entry
    /// to the next open slot provided there is enough space.
    pub fn add_variable_len_entry<V: SplDiscriminate + VariableLenPack>(
        &mut self,
        value: &V,
    ) -> Result<(), ProgramError> {
        let length = value.get_packed_len()?;
        let (data, _) = self.alloc::<V>(length)?;
        value.pack_into_slice(data)?;
        Ok(())
    }

    /// Reallocate the given number of bytes for the given SplDiscriminate. If
    /// the new length is smaller, it will compact the rest of the buffer
    /// and zero out the difference at the end. If it's larger, it will move
    /// the rest of the buffer data and zero out the new data.
    pub fn realloc<V: SplDiscriminate>(
        &mut self,
        length: usize,
        entry_number: usize,
    ) -> Result<&mut [u8], ProgramError> {
        let (
            TlvIndices {
                type_start: _,
                length_start,
                value_start,
            },
            _,
        ) = get_indices(
            self.data,
            V::SPL_DISCRIMINATOR,
            false,
            Some(entry_number),
        )?;
        let (_, end_index) = get_discriminators_and_end_index(self.data)?;
        let data_len = self.data.len();

        let length_ref = pod_from_bytes_mut::<Length>(
            &mut self.data[length_start..value_start],
        )?;
        let old_length = usize::try_from(*length_ref)?;

        // check that we're not going to panic during `copy_within`
        if old_length < length {
            let new_end_index =
                end_index.saturating_add(length.saturating_sub(old_length));
            if new_end_index > data_len {
                return Err(ProgramError::InvalidAccountData);
            }
        }

        // write new length after the check, to avoid getting into a bad
        // situation if trying to recover from an error
        *length_ref = Length::try_from(length)?;

        let old_value_end = value_start.saturating_add(old_length);
        let new_value_end = value_start.saturating_add(length);
        self.data
            .copy_within(old_value_end..end_index, new_value_end);
        match old_length.cmp(&length) {
            Ordering::Greater => {
                // realloc to smaller, fill the end
                let new_end_index =
                    end_index.saturating_sub(old_length.saturating_sub(length));
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
impl<'a> TlvStateNonStrict for TlvStateNonStrictMut<'a> {
    fn get_data(&self) -> &[u8] {
        self.data
    }
}
impl<'data> IntoIterator for TlvStateNonStrictMut<'data> {
    type Item = Result<TlvIndices, ProgramError>;
    type IntoIter = TlvIterator<'data>;

    fn into_iter(self) -> Self::IntoIter {
        TlvIterator::new(self.data)
    }
}

/// Packs a variable-length value into an existing TLV space, reallocating
/// the account and TLV as needed to accommodate for any change in space
pub fn realloc_and_pack_variable_len_strict<
    V: SplDiscriminate + VariableLenPack,
>(
    account_info: &AccountInfo,
    value: &V,
) -> Result<(), ProgramError> {
    let previous_length = {
        let data = account_info.try_borrow_data()?;
        let (
            TlvIndices {
                type_start: _,
                length_start,
                value_start,
            },
            _,
        ) = get_indices(&data, V::SPL_DISCRIMINATOR, false, Some(0))?;
        usize::try_from(*pod_from_bytes::<Length>(
            &data[length_start..value_start],
        )?)?
    };
    let new_length = value.get_packed_len()?;
    let previous_account_size = account_info.try_data_len()?;
    if previous_length < new_length {
        // size increased, so realloc the account, then the TLV entry, then
        // write data
        let additional_bytes = new_length
            .checked_sub(previous_length)
            .ok_or(ProgramError::AccountDataTooSmall)?;
        account_info.realloc(
            previous_account_size.saturating_add(additional_bytes),
            true,
        )?;
        let mut buffer = account_info.try_borrow_mut_data()?;
        let mut state = TlvStateStrictMut::unpack(&mut buffer)?;
        state.realloc::<V>(new_length)?;
        state.pack_variable_len_value(value)?;
    } else {
        // do it backwards otherwise, write the state, realloc TLV, then the
        // account
        let mut buffer = account_info.try_borrow_mut_data()?;
        let mut state = TlvStateStrictMut::unpack(&mut buffer)?;
        state.pack_variable_len_value(value)?;
        let removed_bytes = previous_length
            .checked_sub(new_length)
            .ok_or(ProgramError::AccountDataTooSmall)?;
        if removed_bytes > 0 {
            // we decreased the size, so need to realloc the TLV, then the
            // account
            state.realloc::<V>(new_length)?;
            // this is probably fine, but be safe and avoid invalidating
            // references
            drop(buffer);
            account_info.realloc(
                previous_account_size.saturating_sub(removed_bytes),
                false,
            )?;
        }
    }
    Ok(())
}

/// An iterator over TLV state data
pub struct TlvIterator<'data> {
    data: &'data [u8],
    current: (usize, usize, usize),
    next: (usize, usize, usize),
}
impl<'data> TlvIterator<'data> {
    /// Create a new instance of the `TlvIterator`
    pub fn new(data: &'data [u8]) -> Self {
        let current = (0, 0, 0);
        let next_indices = get_indices_unchecked(0);
        let next = (
            next_indices.type_start,
            next_indices.length_start,
            next_indices.value_start,
        );
        Self {
            data,
            current,
            next,
        }
    }
    /// Get the next TLV indices
    pub fn next_tlv(&mut self) -> Result<TlvIndices, ProgramError> {
        if let Some(next) = self.next() {
            next
        } else {
            Err(TlvError::TlvIteratorEnd.into())
        }
    }

    /// Get the next TLV entry as a `Pod` type
    pub fn next_tlv_entry<V: SplDiscriminate + Pod>(
        &mut self,
    ) -> Result<&V, ProgramError> {
        loop {
            let indices = self.next_tlv()?;
            let length = usize::try_from(*pod_from_bytes::<Length>(
                &self.data[indices.length_start..indices.value_start],
            )?)?;
            let value_end = indices.value_start.saturating_add(length);
            match pod_from_bytes::<V>(
                &self.data[indices.value_start..value_end],
            ) {
                Ok(value) => return Ok(value),
                Err(_) => continue,
            }
        }
    }

    /// Get the next TLV entry as a `VariableLenPack` type
    pub fn next_variable_len_tlv_entry<V: SplDiscriminate + VariableLenPack>(
        &mut self,
    ) -> Result<V, ProgramError> {
        loop {
            let indices = self.next_tlv()?;
            let length = usize::try_from(*pod_from_bytes::<Length>(
                &self.data[indices.length_start..indices.value_start],
            )?)?;
            let value_end = indices.value_start.saturating_add(length);
            match V::unpack_from_slice(
                &self.data[indices.value_start..value_end],
            ) {
                Ok(value) => return Ok(value),
                Err(_) => continue,
            }
        }
    }
}

impl Iterator for TlvIterator<'_> {
    type Item = Result<TlvIndices, ProgramError>;

    fn next(&mut self) -> Option<Self::Item> {
        let pod_length = match pod_from_bytes::<Length>(
            &self.data[self.next.1..self.next.2],
        ) {
            Ok(length) => length,
            Err(e) => return Some(Err(e)),
        };
        let length = match usize::try_from(*pod_length) {
            Ok(length) => length,
            Err(e) => return Some(Err(e)),
        };

        let value_end_index = self.next.2.saturating_add(length);
        let new_next = get_indices_unchecked(value_end_index);

        if self.data[self.next.0..].len() < 8 {
            return Some(Err(ProgramError::InvalidAccountData));
        } else {
            let discriminator = match ArrayDiscriminator::try_from(
                &self.data[self.next.0..self.next.1],
            ) {
                Ok(discriminator) => discriminator,
                Err(e) => return Some(Err(e)),
            };
            if discriminator == ArrayDiscriminator::UNINITIALIZED {
                return None; // For now
            }
        }
        self.current = std::mem::replace(
            &mut self.next,
            (
                new_next.type_start,
                new_next.length_start,
                new_next.value_start,
            ),
        );

        Some(Ok(TlvIndices {
            type_start: self.current.0,
            length_start: self.current.1,
            value_start: self.current.2,
        }))
    }
}

/// Get the base size required for TLV data
pub const fn get_base_len() -> usize {
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
    use {
        super::*,
        bytemuck::{Pod, Zeroable},
    };

    const TEST_BUFFER: &[u8] = &[
        1, 1, 1, 1, 1, 1, 1, 1, // discriminator
        32, 0, 0, 0, // length
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, // value
        0, 0, // empty, not enough for a discriminator
    ];

    const TEST_BIG_BUFFER: &[u8] = &[
        1, 1, 1, 1, 1, 1, 1, 1, // discriminator
        32, 0, 0, 0, // length
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, // value
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
        let state = TlvStateStrictBorrowed::unpack(TEST_BUFFER).unwrap();
        let value = state.get_value::<TestValue>().unwrap();
        assert_eq!(value.data, [1; 32]);
        assert_eq!(
            state.get_value::<TestEmptyValue>(),
            Err(ProgramError::InvalidAccountData)
        );

        let mut test_buffer = TEST_BUFFER.to_vec();
        let state = TlvStateStrictMut::unpack(&mut test_buffer).unwrap();
        let value = state.get_value::<TestValue>().unwrap();
        assert_eq!(value.data, [1; 32]);
        let state = TlvStateStrictOwned::unpack(test_buffer).unwrap();
        let value = state.get_value::<TestValue>().unwrap();
        assert_eq!(value.data, [1; 32]);
    }

    #[test]
    fn fail_unpack_opaque_buffer() {
        // input buffer too small
        let mut buffer = vec![0, 3];
        assert_eq!(
            TlvStateStrictBorrowed::unpack(&buffer),
            Err(ProgramError::InvalidAccountData)
        );
        assert_eq!(
            TlvStateStrictMut::unpack(&mut buffer),
            Err(ProgramError::InvalidAccountData)
        );
        assert_eq!(
            TlvStateStrictMut::unpack(&mut buffer),
            Err(ProgramError::InvalidAccountData)
        );

        // tweak the discriminator
        let mut buffer = TEST_BUFFER.to_vec();
        buffer[0] += 1;
        let state = TlvStateStrictMut::unpack(&mut buffer).unwrap();
        assert_eq!(
            state.get_value::<TestValue>(),
            Err(ProgramError::InvalidAccountData)
        );

        // tweak the length, too big
        let mut buffer = TEST_BUFFER.to_vec();
        buffer[ArrayDiscriminator::LENGTH] += 10;
        assert_eq!(
            TlvStateStrictMut::unpack(&mut buffer),
            Err(ProgramError::InvalidAccountData)
        );

        // tweak the length, too small
        let mut buffer = TEST_BIG_BUFFER.to_vec();
        buffer[ArrayDiscriminator::LENGTH] -= 1;
        let state = TlvStateStrictMut::unpack(&mut buffer).unwrap();
        assert_eq!(
            state.get_value::<TestValue>(),
            Err(ProgramError::InvalidArgument)
        );

        // data buffer is too small for type
        let buffer = &TEST_BUFFER[..TEST_BUFFER.len() - 5];
        assert_eq!(
            TlvStateStrictBorrowed::unpack(buffer),
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
            get_discriminators_and_end_index(&[
                1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ])
            .unwrap(),
            (vec![ArrayDiscriminator::try_from(1).unwrap()], 12)
        );
        // correct since it's just uninitialized data
        assert_eq!(
            get_discriminators_and_end_index(&[0, 0, 0, 0, 0, 0, 0, 0])
                .unwrap(),
            (vec![], 0)
        );
    }

    #[test]
    fn value_pack_unpack() {
        let account_size = get_base_len()
            + size_of::<TestValue>()
            + get_base_len()
            + size_of::<TestSmallValue>();
        let mut buffer = vec![0; account_size];

        let mut state = TlvStateStrictMut::unpack(&mut buffer).unwrap();

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
        expect.extend_from_slice(
            &u32::try_from(size_of::<TestValue>()).unwrap().to_le_bytes(),
        );
        expect.extend_from_slice(&data);
        expect.extend_from_slice(&[0; size_of::<ArrayDiscriminator>()]);
        expect.extend_from_slice(&[0; size_of::<Length>()]);
        expect.extend_from_slice(&[0; size_of::<TestSmallValue>()]);
        assert_eq!(expect, buffer);

        // check unpacking
        let mut state = TlvStateStrictMut::unpack(&mut buffer).unwrap();
        let mut unpacked = state.get_value_mut::<TestValue>().unwrap();
        assert_eq!(*unpacked, TestValue { data });

        // update extension
        let new_data = [101; 32];
        unpacked.data = new_data;

        // check updates are propagated
        let state = TlvStateStrictBorrowed::unpack(&buffer).unwrap();
        let unpacked = state.get_value::<TestValue>().unwrap();
        assert_eq!(*unpacked, TestValue { data: new_data });

        // check raw buffer
        let mut expect = vec![];
        expect.extend_from_slice(TestValue::SPL_DISCRIMINATOR.as_ref());
        expect.extend_from_slice(
            &u32::try_from(size_of::<TestValue>()).unwrap().to_le_bytes(),
        );
        expect.extend_from_slice(&new_data);
        expect.extend_from_slice(&[0; size_of::<ArrayDiscriminator>()]);
        expect.extend_from_slice(&[0; size_of::<Length>()]);
        expect.extend_from_slice(&[0; size_of::<TestSmallValue>()]);
        assert_eq!(expect, buffer);

        let mut state = TlvStateStrictMut::unpack(&mut buffer).unwrap();
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
        expect.extend_from_slice(
            &u32::try_from(size_of::<TestValue>()).unwrap().to_le_bytes(),
        );
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
        let mut state = TlvStateStrictMut::unpack(&mut buffer).unwrap();
        assert_eq!(
            state.init_value::<TestEmptyValue>(),
            Err(ProgramError::InvalidAccountData),
        );
    }

    #[test]
    fn value_any_order() {
        let account_size = get_base_len()
            + size_of::<TestValue>()
            + get_base_len()
            + size_of::<TestSmallValue>();
        let mut buffer = vec![0; account_size];

        let mut state = TlvStateStrictMut::unpack(&mut buffer).unwrap();

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
        let mut state = TlvStateStrictMut::unpack(&mut other_buffer).unwrap();

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
        let state = TlvStateStrictBorrowed::unpack(&buffer).unwrap();
        let other_state =
            TlvStateStrictBorrowed::unpack(&other_buffer).unwrap();

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
        let mut state = TlvStateStrictMut::unpack(&mut buffer).unwrap();
        let value = state.init_value::<TestNonZeroDefault>().unwrap();
        assert_eq!(value.data, TEST_NON_ZERO_DEFAULT_DATA);
    }

    #[test]
    fn init_buffer_too_small() {
        let account_size = get_base_len() + size_of::<TestValue>();
        let mut buffer = vec![0; account_size - 1];
        let mut state = TlvStateStrictMut::unpack(&mut buffer).unwrap();
        let err = state.init_value::<TestValue>().unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);

        // hack the buffer to look like it was initialized, still fails
        let discriminator_ref = &mut state.data[0..ArrayDiscriminator::LENGTH];
        discriminator_ref
            .copy_from_slice(TestValue::SPL_DISCRIMINATOR.as_ref());
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
        let mut state = TlvStateStrictMut::unpack(&mut buffer).unwrap();

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
        let mut state = TlvStateStrictMut::unpack(&mut buffer).unwrap();

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
        let mut state = TlvStateStrictMut::unpack(&mut buffer).unwrap();

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
        let mut state = TlvStateStrictMut::unpack(&mut buffer).unwrap();
        // realloc too much, fails
        assert_eq!(
            state
                .realloc::<TestValue>(TLV_SIZE + EXTRA_SPACE + 1)
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
            let length =
                u64::from_le_bytes(src[..8].try_into().unwrap()) as usize;
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
    fn variable_len_value() {
        let initial_data = "This is a pretty cool test!";
        // exactly the right size
        let tlv_size = 8 + initial_data.len();
        let account_size = get_base_len() + tlv_size;
        let mut buffer = vec![0; account_size];
        let mut state = TlvStateStrictMut::unpack(&mut buffer).unwrap();

        // don't actually need to hold onto the data!
        let _ = state.alloc::<TestVariableLen>(tlv_size).unwrap();
        let test_variable_len = TestVariableLen {
            data: initial_data.to_string(),
        };
        state.pack_variable_len_value(&test_variable_len).unwrap();
        let deser = state.get_variable_len_value::<TestVariableLen>().unwrap();
        assert_eq!(deser, test_variable_len);

        // writing too much data fails
        let too_much_data = "This is a pretty cool test!?";
        assert_eq!(
            state
                .pack_variable_len_value(&TestVariableLen {
                    data: too_much_data.to_string(),
                })
                .unwrap_err(),
            ProgramError::InvalidAccountData
        );
    }
}

#[cfg(all(test, feature = "derive"))]
mod strict_nonstrict_tests {
    use {
        super::*,
        crate::SplBorshVariableLenPack,
        borsh::{BorshDeserialize, BorshSerialize},
        bytemuck::{Pod, Zeroable},
        spl_discriminator::SplDiscriminate,
        std::mem::size_of,
    };

    #[repr(C)]
    #[derive(
        Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, SplDiscriminate,
    )]
    #[discriminator_hash_input("vehicle::chevrolet_fixed")]
    pub struct ChevroletFixed {
        vin: [u8; 8],
        plate: [u8; 7],
    }

    #[repr(C)]
    #[derive(
        Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, SplDiscriminate,
    )]
    #[discriminator_hash_input("vehicle::ford_fixed")]
    pub struct FordFixed {
        vin: [u8; 8],
        plate: [u8; 7],
    }

    #[cfg_attr(feature = "derive", derive(SplBorshVariableLenPack))]
    #[derive(
        Clone,
        Debug,
        Default,
        PartialEq,
        BorshDeserialize,
        BorshSerialize,
        SplDiscriminate,
    )]
    #[discriminator_hash_input("vehicle::chevrolet_variable")]
    pub struct ChevroletVariable {
        vin: Vec<u8>,
        plate: Vec<u8>,
    }

    #[cfg_attr(feature = "derive", derive(SplBorshVariableLenPack))]
    #[derive(
        Clone,
        Debug,
        Default,
        PartialEq,
        BorshDeserialize,
        BorshSerialize,
        SplDiscriminate,
    )]
    #[discriminator_hash_input("vehicle::ford_variable")]
    pub struct FordVariable {
        vin: Vec<u8>,
        plate: Vec<u8>,
    }

    #[test]
    fn test_strict() {
        let chevrolet_fixed = ChevroletFixed {
            vin: *b"12345678",
            plate: *b"ABC1234",
        };
        let ford_fixed = FordFixed {
            vin: *b"12345678",
            plate: *b"ABC1234",
        };

        let account_size = get_base_len()
            + size_of::<ChevroletFixed>()
            + get_base_len()
            + size_of::<FordFixed>();
        let mut buffer = vec![0; account_size];

        let mut state = TlvStateStrictMut::unpack(&mut buffer).unwrap();

        // Write a `ChevroletFixed`
        state.add_entry::<ChevroletFixed>(&chevrolet_fixed).unwrap();
        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[ChevroletFixed::SPL_DISCRIMINATOR]
        );
        assert_eq!(
            state.get_value::<ChevroletFixed>().unwrap(),
            &chevrolet_fixed
        );

        // Should fail if we try to write another `ChevroletFixed`
        assert_eq!(
            state.init_value::<ChevroletFixed>().unwrap_err(),
            TlvError::TypeAlreadyExists.into(),
        );

        // Write a `FordFixed`
        state.add_entry::<FordFixed>(&ford_fixed).unwrap();
        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[
                ChevroletFixed::SPL_DISCRIMINATOR,
                FordFixed::SPL_DISCRIMINATOR
            ]
        );
        assert_eq!(state.get_value::<FordFixed>().unwrap(), &ford_fixed);

        // Should fail if we try to write another `FordFixed`
        assert_eq!(
            state.init_value::<FordFixed>().unwrap_err(),
            TlvError::TypeAlreadyExists.into(),
        );
    }

    #[test]
    fn test_nonstrict() {
        let chevrolet_fixed1 = ChevroletFixed {
            vin: *b"12345678",
            plate: *b"ABC1234",
        };
        let chevrolet_fixed2 = ChevroletFixed {
            vin: *b"87654321",
            plate: *b"XYZ4321",
        };
        let ford_fixed1 = FordFixed {
            vin: *b"12345678",
            plate: *b"ABC1234",
        };
        let ford_fixed2 = FordFixed {
            vin: *b"87654321",
            plate: *b"XYZ4321",
        };

        let account_size = get_base_len()
            + size_of::<ChevroletFixed>()
            + get_base_len()
            + size_of::<ChevroletFixed>()
            + get_base_len()
            + size_of::<FordFixed>()
            + get_base_len()
            + size_of::<FordFixed>();
        let mut buffer = vec![0; account_size];

        let mut state = TlvStateNonStrictMut::unpack(&mut buffer).unwrap();

        // Write a `ChevroletFixed`
        state
            .add_entry::<ChevroletFixed>(&chevrolet_fixed1)
            .unwrap();
        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[ChevroletFixed::SPL_DISCRIMINATOR]
        );
        assert_eq!(
            state.get_first::<ChevroletFixed>().unwrap(),
            &chevrolet_fixed1
        );

        // Write another `ChevroletFixed`
        state
            .add_entry::<ChevroletFixed>(&chevrolet_fixed2)
            .unwrap();
        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[
                ChevroletFixed::SPL_DISCRIMINATOR,
                ChevroletFixed::SPL_DISCRIMINATOR,
            ]
        );
        assert_eq!(
            state.get_value::<ChevroletFixed>(1).unwrap(),
            &chevrolet_fixed2
        );

        // Write a `FordFixed`
        state.add_entry::<FordFixed>(&ford_fixed1).unwrap();
        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[
                ChevroletFixed::SPL_DISCRIMINATOR,
                ChevroletFixed::SPL_DISCRIMINATOR,
                FordFixed::SPL_DISCRIMINATOR,
            ]
        );
        assert_eq!(state.get_first::<FordFixed>().unwrap(), &ford_fixed1);

        // Write another `FordFixed`
        state.add_entry::<FordFixed>(&ford_fixed2).unwrap();
        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[
                ChevroletFixed::SPL_DISCRIMINATOR,
                ChevroletFixed::SPL_DISCRIMINATOR,
                FordFixed::SPL_DISCRIMINATOR,
                FordFixed::SPL_DISCRIMINATOR,
            ]
        );
        assert_eq!(state.get_value::<FordFixed>(1).unwrap(), &ford_fixed2);
    }

    #[test]
    fn test_nonstrict_mix_and_match() {
        let chevrolet_fixed1 = ChevroletFixed {
            vin: *b"12345678",
            plate: *b"ABC1234",
        };
        let chevrolet_fixed2 = ChevroletFixed {
            vin: *b"87654321",
            plate: *b"XYZ4321",
        };
        let chevrolet_variable = ChevroletVariable {
            vin: b"12345678".to_vec(),
            plate: b"ABC1234".to_vec(),
        };
        let ford_fixed1 = FordFixed {
            vin: *b"12345678",
            plate: *b"ABC1234",
        };
        let ford_fixed2 = FordFixed {
            vin: *b"87654321",
            plate: *b"XYZ4321",
        };
        let ford_variable = FordVariable {
            vin: b"12345678".to_vec(),
            plate: b"ABC1234".to_vec(),
        };

        let account_size = get_base_len()
            + size_of::<ChevroletFixed>()
            + get_base_len()
            + size_of::<ChevroletFixed>()
            + get_base_len()
            + size_of::<ChevroletVariable>()
            + get_base_len()
            + size_of::<FordFixed>()
            + get_base_len()
            + size_of::<FordFixed>()
            + get_base_len()
            + size_of::<FordVariable>();
        let mut buffer = vec![0; account_size];

        let mut state = TlvStateNonStrictMut::unpack(&mut buffer).unwrap();

        // Write a `ChevroletFixed`
        state
            .add_entry::<ChevroletFixed>(&chevrolet_fixed1)
            .unwrap();
        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[ChevroletFixed::SPL_DISCRIMINATOR]
        );
        assert_eq!(
            state.get_first::<ChevroletFixed>().unwrap(),
            &chevrolet_fixed1
        );

        // Write another `ChevroletFixed`
        state
            .add_entry::<ChevroletFixed>(&chevrolet_fixed2)
            .unwrap();
        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[
                ChevroletFixed::SPL_DISCRIMINATOR,
                ChevroletFixed::SPL_DISCRIMINATOR,
            ]
        );
        assert_eq!(
            state.get_value::<ChevroletFixed>(1).unwrap(),
            &chevrolet_fixed2
        );

        // Write a `ChevroletVariable`
        state
            .add_variable_len_entry::<ChevroletVariable>(&chevrolet_variable)
            .unwrap();
        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[
                ChevroletFixed::SPL_DISCRIMINATOR,
                ChevroletFixed::SPL_DISCRIMINATOR,
                ChevroletVariable::SPL_DISCRIMINATOR,
            ]
        );
        assert_eq!(
            state
                .get_first_variable_len_value::<ChevroletVariable>()
                .unwrap(),
            chevrolet_variable
        );

        // Write a `FordFixed`
        state.add_entry::<FordFixed>(&ford_fixed1).unwrap();
        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[
                ChevroletFixed::SPL_DISCRIMINATOR,
                ChevroletFixed::SPL_DISCRIMINATOR,
                ChevroletVariable::SPL_DISCRIMINATOR,
                FordFixed::SPL_DISCRIMINATOR,
            ]
        );
        assert_eq!(state.get_first::<FordFixed>().unwrap(), &ford_fixed1);

        // Write another `FordFixed`
        state.add_entry::<FordFixed>(&ford_fixed2).unwrap();
        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[
                ChevroletFixed::SPL_DISCRIMINATOR,
                ChevroletFixed::SPL_DISCRIMINATOR,
                ChevroletVariable::SPL_DISCRIMINATOR,
                FordFixed::SPL_DISCRIMINATOR,
                FordFixed::SPL_DISCRIMINATOR,
            ]
        );
        assert_eq!(state.get_value::<FordFixed>(1).unwrap(), &ford_fixed2);

        // Write a `FordVariable`
        state
            .add_variable_len_entry::<FordVariable>(&ford_variable)
            .unwrap();
        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[
                ChevroletFixed::SPL_DISCRIMINATOR,
                ChevroletFixed::SPL_DISCRIMINATOR,
                ChevroletVariable::SPL_DISCRIMINATOR,
                FordFixed::SPL_DISCRIMINATOR,
                FordFixed::SPL_DISCRIMINATOR,
                FordVariable::SPL_DISCRIMINATOR,
            ]
        );
        assert_eq!(
            state
                .get_first_variable_len_value::<FordVariable>()
                .unwrap(),
            ford_variable
        );
    }

    #[test]
    fn test_iterator() {
        let chevrolet_fixed1 = ChevroletFixed {
            vin: *b"12345678",
            plate: *b"ABC1234",
        };
        let chevrolet_fixed2 = ChevroletFixed {
            vin: *b"87654321",
            plate: *b"XYZ4321",
        };
        let chevrolet_variable = ChevroletVariable {
            vin: b"12345678".to_vec(),
            plate: b"ABC1234".to_vec(),
        };
        let ford_fixed1 = FordFixed {
            vin: *b"12345678",
            plate: *b"ABC1234",
        };
        let ford_fixed2 = FordFixed {
            vin: *b"87654321",
            plate: *b"XYZ4321",
        };
        let ford_variable = FordVariable {
            vin: b"12345678".to_vec(),
            plate: b"ABC1234".to_vec(),
        };

        let account_size = get_base_len()
            + size_of::<ChevroletFixed>()
            + get_base_len()
            + size_of::<ChevroletFixed>()
            + get_base_len()
            + size_of::<ChevroletVariable>()
            + get_base_len()
            + size_of::<FordFixed>()
            + get_base_len()
            + size_of::<FordFixed>()
            + get_base_len()
            + size_of::<FordVariable>();
        let mut buffer = vec![0; account_size];

        let mut state = TlvStateNonStrictMut::unpack(&mut buffer).unwrap();
        state
            .add_entry::<ChevroletFixed>(&chevrolet_fixed1)
            .unwrap();
        state
            .add_entry::<ChevroletFixed>(&chevrolet_fixed2)
            .unwrap();
        state
            .add_variable_len_entry::<ChevroletVariable>(&chevrolet_variable)
            .unwrap();
        state.add_entry::<FordFixed>(&ford_fixed1).unwrap();
        state.add_entry::<FordFixed>(&ford_fixed2).unwrap();
        state
            .add_variable_len_entry::<FordVariable>(&ford_variable)
            .unwrap();

        let mut iter = state.into_iter();
        assert_eq!(
            iter.next_tlv_entry::<ChevroletFixed>().unwrap(),
            &chevrolet_fixed1
        );
        assert_eq!(
            iter.next_tlv_entry::<ChevroletFixed>().unwrap(),
            &chevrolet_fixed2
        );
        assert_eq!(
            iter.next_variable_len_tlv_entry::<ChevroletVariable>()
                .unwrap(),
            chevrolet_variable
        );
        assert_eq!(iter.next_tlv_entry::<FordFixed>().unwrap(), &ford_fixed1);
        assert_eq!(iter.next_tlv_entry::<FordFixed>().unwrap(), &ford_fixed2);
        assert_eq!(
            iter.next_variable_len_tlv_entry::<FordVariable>().unwrap(),
            ford_variable
        );
    }
}
