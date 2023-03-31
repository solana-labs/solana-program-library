//! TLV structure manipulation

use {
    crate::{error::PermissionedTransferError, DISCRIMINATOR_LENGTH},
    bytemuck::{Pod, Zeroable},
    solana_program::program_error::ProgramError,
    std::mem::size_of,
};

/// Convert a slice into a `Pod` (zero copy)
pub fn pod_from_bytes<T: Pod>(bytes: &[u8]) -> Result<&T, ProgramError> {
    bytemuck::try_from_bytes(bytes).map_err(|_| ProgramError::InvalidArgument)
}
/// Convert a slice into a mutable `Pod` (zero copy)
pub fn pod_from_bytes_mut<T: Pod>(bytes: &mut [u8]) -> Result<&mut T, ProgramError> {
    bytemuck::try_from_bytes_mut(bytes).map_err(|_| ProgramError::InvalidArgument)
}

/// Simple macro for implementing conversion functions between Pod* ints and standard ints.
///
/// The standard int types can cause alignment issues when placed in a `Pod`,
/// so these replacements are usable in all `Pod`s.
macro_rules! impl_int_conversion {
    ($P:ty, $I:ty) => {
        impl From<$I> for $P {
            fn from(n: $I) -> Self {
                Self(n.to_le_bytes())
            }
        }
        impl From<$P> for $I {
            fn from(pod: $P) -> Self {
                Self::from_le_bytes(pod.0)
            }
        }
    };
}

/// `u32` type that can be used in `Pod`s
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodU32([u8; 4]);
impl_int_conversion!(PodU32, u32);

/// `u16` type that can be used in `Pod`s
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodU16([u8; 2]);
impl_int_conversion!(PodU16, u16);

/// Length in TLV structure
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct Length(PodU32);
impl TryFrom<Length> for usize {
    type Error = ProgramError;
    fn try_from(n: Length) -> Result<Self, Self::Error> {
        Self::try_from(u32::from(n.0)).map_err(|_| ProgramError::AccountDataTooSmall)
    }
}
impl TryFrom<usize> for Length {
    type Error = ProgramError;
    fn try_from(n: usize) -> Result<Self, Self::Error> {
        u32::try_from(n)
            .map(|v| Self(PodU32::from(v)))
            .map_err(|_| ProgramError::AccountDataTooSmall)
    }
}

/// Get the current TlvIndices from the current spot
fn get_indices_unchecked(type_start: usize) -> TlvIndices {
    let length_start = type_start.saturating_add(size_of::<Discriminator>());
    let value_start = length_start.saturating_add(size_of::<Length>());
    TlvIndices {
        type_start,
        length_start,
        value_start,
    }
}

/// Helper struct for returning the indices of the type, length, and value in
/// a TLV entry
#[derive(Debug)]
struct TlvIndices {
    pub type_start: usize,
    pub length_start: usize,
    pub value_start: usize,
}
fn get_indices<V: Value>(tlv_data: &[u8], init: bool) -> Result<TlvIndices, ProgramError> {
    let mut start_index = 0;
    while start_index < tlv_data.len() {
        let tlv_indices = get_indices_unchecked(start_index);
        if tlv_data.len() < tlv_indices.value_start {
            return Err(ProgramError::InvalidAccountData);
        }
        let discriminator =
            Discriminator::try_from(&tlv_data[tlv_indices.type_start..tlv_indices.length_start])?;
        if discriminator == V::TYPE {
            // found an instance of the extension that we're initializing, return!
            return Ok(tlv_indices);
        // got to an empty spot, init here, or error if we're searching, since
        // nothing is written after an Uninitialized spot
        } else if discriminator == Discriminator::UNINITIALIZED {
            if init {
                return Ok(tlv_indices);
            } else {
                return Err(PermissionedTransferError::TypeNotFound.into());
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

fn get_discriminators(tlv_data: &[u8]) -> Result<Vec<Discriminator>, ProgramError> {
    let mut discriminators = vec![];
    let mut start_index = 0;
    while start_index < tlv_data.len() {
        let tlv_indices = get_indices_unchecked(start_index);
        if tlv_data.len() < tlv_indices.length_start {
            // we got to the end, but there might be some uninitialized data after
            let remainder = &tlv_data[tlv_indices.type_start..];
            if remainder.iter().all(|&x| x == 0) {
                return Ok(discriminators);
            } else {
                return Err(ProgramError::InvalidAccountData);
            }
        }
        let discriminator =
            Discriminator::try_from(&tlv_data[tlv_indices.type_start..tlv_indices.length_start])?;
        if discriminator == Discriminator::UNINITIALIZED {
            return Ok(discriminators);
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
    Ok(discriminators)
}

fn get_value<V: Value>(tlv_data: &[u8]) -> Result<&V, ProgramError> {
    let TlvIndices {
        type_start: _,
        length_start,
        value_start,
    } = get_indices::<V>(tlv_data, false)?;
    // get_indices has checked that tlv_data is long enough to include these indices
    let length = pod_from_bytes::<Length>(&tlv_data[length_start..value_start])?;
    let value_end = value_start.saturating_add(usize::try_from(*length)?);
    if tlv_data.len() < value_end {
        return Err(ProgramError::InvalidAccountData);
    }
    V::try_from_bytes(&tlv_data[value_start..value_end])
}

/// Trait for all TLV state
pub trait TlvState {
    /// Get the full buffer containing all TLV data
    fn get_data(&self) -> &[u8];

    /// Unpack a portion of the TLV data as the desired type
    fn get_value<V: Value>(&self) -> Result<&V, ProgramError> {
        get_value::<V>(self.get_data())
    }

    /// Iterates through the TLV entries, returning only the types
    fn get_discriminators(&self) -> Result<Vec<Discriminator>, ProgramError> {
        get_discriminators(self.get_data())
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
    pub fn get_value_mut<V: Value>(&mut self) -> Result<&mut V, ProgramError> {
        let TlvIndices {
            type_start,
            length_start,
            value_start,
        } = get_indices::<V>(self.data, false)?;

        if self.data[type_start..].len() < get_len::<V>() {
            return Err(ProgramError::InvalidAccountData);
        }
        let length = pod_from_bytes::<Length>(&self.data[length_start..value_start])?;
        let value_end = value_start.saturating_add(usize::try_from(*length)?);
        V::try_from_bytes_mut(&mut self.data[value_start..value_end])
    }

    /// Packs the default extension data into an open slot if not already found in the
    /// data buffer. If extension is already found in the buffer, it overwrites the existing
    /// extension with the default state if `overwrite` is set. If extension found, but
    /// `overwrite` is not set, it returns error.
    pub fn init_value<V: Value>(&mut self, overwrite: bool) -> Result<&mut V, ProgramError> {
        let TlvIndices {
            type_start,
            length_start,
            value_start,
        } = get_indices::<V>(self.data, true)?;

        if self.data[type_start..].len() < get_len::<V>() {
            return Err(ProgramError::InvalidAccountData);
        }
        let discriminator = Discriminator::try_from(&self.data[type_start..length_start])?;
        if discriminator == Discriminator::UNINITIALIZED || overwrite {
            // write type
            let discriminator_ref = &mut self.data[type_start..length_start];
            discriminator_ref.copy_from_slice(V::TYPE.as_ref());
            // write length
            let length_ref =
                pod_from_bytes_mut::<Length>(&mut self.data[length_start..value_start])?;
            // maybe this becomes smarter later for dynamically sized extensions
            let length = size_of::<V>();
            *length_ref = Length::try_from(length)?;

            let value_end = value_start.saturating_add(length);
            let extension_ref = V::try_from_bytes_mut(&mut self.data[value_start..value_end])?;
            *extension_ref = V::default();
            Ok(extension_ref)
        } else {
            // extension is already initialized, but no overwrite permission
            Err(PermissionedTransferError::TypeAlreadyExists.into())
        }
    }
}
impl<'a> TlvState for TlvStateMut<'a> {
    fn get_data(&self) -> &[u8] {
        self.data
    }
}

/// Discriminator used as the type in the TLV structure
/// Type in TLV structure
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct Discriminator([u8; DISCRIMINATOR_LENGTH]);
impl Discriminator {
    const UNINITIALIZED: Self = Self::new([0; DISCRIMINATOR_LENGTH]);
    /// Creates a discriminator from an array
    pub const fn new(value: [u8; DISCRIMINATOR_LENGTH]) -> Self {
        Self(value)
    }
}
impl AsRef<[u8]> for Discriminator {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}
impl From<u64> for Discriminator {
    fn from(from: u64) -> Self {
        Self(from.to_le_bytes())
    }
}
impl From<[u8; DISCRIMINATOR_LENGTH]> for Discriminator {
    fn from(from: [u8; DISCRIMINATOR_LENGTH]) -> Self {
        Self(from)
    }
}
impl TryFrom<&[u8]> for Discriminator {
    type Error = ProgramError;
    fn try_from(a: &[u8]) -> Result<Self, Self::Error> {
        <[u8; DISCRIMINATOR_LENGTH]>::try_from(a)
            .map(Self::from)
            .map_err(|_| ProgramError::InvalidAccountData)
    }
}

/// Trait to be implemented by all value types in the TLV structure, specifying
/// the discriminator to check against
pub trait Value: Default {
    /// Associated value type enum, checked at the start of TLV entries
    const TYPE: Discriminator;

    /// Turn raw bytes into a reference of the underlying type. If the type
    /// implements `Pod`, then you can simply do `pod_from_bytes`.
    fn try_from_bytes(bytes: &[u8]) -> Result<&Self, ProgramError>;

    /// Turn raw bytes into a mutable reference of the underlying type. If the
    /// type implements `Pod`, then you can simply do `pod_from_bytes`.
    fn try_from_bytes_mut(bytes: &mut [u8]) -> Result<&mut Self, ProgramError>;
}

/// Get the size required for this value as TLV
pub fn get_len<V: Value>() -> usize {
    let indices = get_indices_unchecked(0);
    indices.value_start.saturating_add(size_of::<V>())
}

fn check_data(tlv_data: &[u8]) -> Result<(), ProgramError> {
    // should be able to fetch discriminators
    let _discriminators = get_discriminators(tlv_data)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

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
    impl Value for TestValue {
        const TYPE: Discriminator = Discriminator::new([1; DISCRIMINATOR_LENGTH]);

        fn try_from_bytes(bytes: &[u8]) -> Result<&Self, ProgramError> {
            pod_from_bytes(bytes)
        }

        fn try_from_bytes_mut(bytes: &mut [u8]) -> Result<&mut Self, ProgramError> {
            pod_from_bytes_mut(bytes)
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
    struct TestSmallValue {
        data: [u8; 3],
    }
    impl Value for TestSmallValue {
        const TYPE: Discriminator = Discriminator::new([2; DISCRIMINATOR_LENGTH]);

        fn try_from_bytes(bytes: &[u8]) -> Result<&Self, ProgramError> {
            pod_from_bytes(bytes)
        }

        fn try_from_bytes_mut(bytes: &mut [u8]) -> Result<&mut Self, ProgramError> {
            pod_from_bytes_mut(bytes)
        }
    }

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
    struct TestEmptyValue;
    impl Value for TestEmptyValue {
        const TYPE: Discriminator = Discriminator::new([3; DISCRIMINATOR_LENGTH]);

        fn try_from_bytes(bytes: &[u8]) -> Result<&Self, ProgramError> {
            pod_from_bytes(bytes)
        }

        fn try_from_bytes_mut(bytes: &mut [u8]) -> Result<&mut Self, ProgramError> {
            pod_from_bytes_mut(bytes)
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
    struct TestNonZeroDefault {
        data: [u8; 5],
    }
    const TEST_NON_ZERO_DEFAULT_DATA: [u8; 5] = [4; 5];
    impl Value for TestNonZeroDefault {
        const TYPE: Discriminator = Discriminator::new([4; DISCRIMINATOR_LENGTH]);

        fn try_from_bytes(bytes: &[u8]) -> Result<&Self, ProgramError> {
            pod_from_bytes(bytes)
        }

        fn try_from_bytes_mut(bytes: &mut [u8]) -> Result<&mut Self, ProgramError> {
            pod_from_bytes_mut(bytes)
        }
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
        buffer[DISCRIMINATOR_LENGTH] += 10;
        assert_eq!(
            TlvStateMut::unpack(&mut buffer),
            Err(ProgramError::InvalidAccountData)
        );

        // tweak the length, too small
        let mut buffer = TEST_BIG_BUFFER.to_vec();
        buffer[DISCRIMINATOR_LENGTH] -= 1;
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
            get_discriminators(&[1, 0, 1, 1]).unwrap_err(),
            ProgramError::InvalidAccountData,
        );
        // correct due to the good discriminator length and zero length
        assert_eq!(
            get_discriminators(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
            vec![Discriminator::try_from(1).unwrap()]
        );
        // correct since it's just uninitialized data
        assert_eq!(
            get_discriminators(&[0, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
            vec![]
        );
    }

    #[test]
    fn value_pack_unpack() {
        let account_size = get_len::<TestValue>() + get_len::<TestSmallValue>();
        let mut buffer = vec![0; account_size];

        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        // success init and write value
        let value = state.init_value::<TestValue>(false).unwrap();
        let data = [100; 32];
        value.data = data;
        assert_eq!(&state.get_discriminators().unwrap(), &[TestValue::TYPE],);
        assert_eq!(&state.get_value::<TestValue>().unwrap().data, &data,);

        // fail init extension when already initialized
        assert_eq!(
            state.init_value::<TestValue>(false).unwrap_err(),
            PermissionedTransferError::TypeAlreadyExists.into(),
        );

        // check raw buffer
        let mut expect = vec![];
        expect.extend_from_slice(TestValue::TYPE.as_ref());
        expect.extend_from_slice(&u32::try_from(size_of::<TestValue>()).unwrap().to_le_bytes());
        expect.extend_from_slice(&data);
        expect.extend_from_slice(&[0; size_of::<Discriminator>()]);
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
        expect.extend_from_slice(TestValue::TYPE.as_ref());
        expect.extend_from_slice(&u32::try_from(size_of::<TestValue>()).unwrap().to_le_bytes());
        expect.extend_from_slice(&new_data);
        expect.extend_from_slice(&[0; size_of::<Discriminator>()]);
        expect.extend_from_slice(&[0; size_of::<Length>()]);
        expect.extend_from_slice(&[0; size_of::<TestSmallValue>()]);
        assert_eq!(expect, buffer);

        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
        // init one more value
        let new_value = state.init_value::<TestSmallValue>(false).unwrap();
        let small_data = [102; 3];
        new_value.data = small_data;

        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[TestValue::TYPE, TestSmallValue::TYPE]
        );

        // check raw buffer
        let mut expect = vec![];
        expect.extend_from_slice(TestValue::TYPE.as_ref());
        expect.extend_from_slice(&u32::try_from(size_of::<TestValue>()).unwrap().to_le_bytes());
        expect.extend_from_slice(&new_data);
        expect.extend_from_slice(TestSmallValue::TYPE.as_ref());
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
            state.init_value::<TestEmptyValue>(true),
            Err(ProgramError::InvalidAccountData),
        );
    }

    #[test]
    fn value_any_order() {
        let account_size = get_len::<TestValue>() + get_len::<TestSmallValue>();
        let mut buffer = vec![0; account_size];

        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        let data = [99; 32];
        let small_data = [98; 3];

        // write values
        let value = state.init_value::<TestValue>(false).unwrap();
        value.data = data;
        let value = state.init_value::<TestSmallValue>(false).unwrap();
        value.data = small_data;

        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[TestValue::TYPE, TestSmallValue::TYPE,]
        );

        // write values in a different order
        let mut other_buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut other_buffer).unwrap();

        let value = state.init_value::<TestSmallValue>(false).unwrap();
        value.data = small_data;
        let value = state.init_value::<TestValue>(false).unwrap();
        value.data = data;

        assert_eq!(
            &state.get_discriminators().unwrap(),
            &[TestSmallValue::TYPE, TestValue::TYPE,]
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
        let account_size = get_len::<TestNonZeroDefault>();
        let mut buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
        let value = state.init_value::<TestNonZeroDefault>(false).unwrap();
        assert_eq!(value.data, TEST_NON_ZERO_DEFAULT_DATA);
    }

    #[test]
    fn init_buffer_too_small() {
        let account_size = get_len::<TestValue>();
        let mut buffer = vec![0; account_size - 1];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();
        let err = state.init_value::<TestValue>(true).unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);

        // hack the buffer to look like it was initialized, still fails
        let discriminator_ref = &mut state.data[0..DISCRIMINATOR_LENGTH];
        discriminator_ref.copy_from_slice(TestValue::TYPE.as_ref());
        state.data[DISCRIMINATOR_LENGTH] = 32;
        let err = state.get_value::<TestValue>().unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);
        assert_eq!(
            state.get_discriminators().unwrap_err(),
            ProgramError::InvalidAccountData
        );
    }

    #[test]
    fn value_with_no_data() {
        let account_size = get_len::<TestEmptyValue>();
        let mut buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        assert_eq!(
            state.get_value::<TestEmptyValue>().unwrap_err(),
            PermissionedTransferError::TypeNotFound.into(),
        );

        // init without overwrite works
        state.init_value::<TestEmptyValue>(false).unwrap();
        state.get_value::<TestEmptyValue>().unwrap();

        // re-init with overwrite works
        state.init_value::<TestEmptyValue>(true).unwrap();

        // re-init without overwrite fails
        assert_eq!(
            state.init_value::<TestEmptyValue>(false).unwrap_err(),
            PermissionedTransferError::TypeAlreadyExists.into(),
        );
    }
}
