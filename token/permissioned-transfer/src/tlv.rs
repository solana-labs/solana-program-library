//! TLV structure manipulation

use {
    crate::{DISCRIMINATOR_LENGTH, error::PermissionedTransferError},
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

/// Length in TLV structure
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct Length(PodU32);
impl TryFrom<Length> for usize {
    type Error = ProgramError;
    fn try_from(n: Length) -> Result<Self, Self::Error> {
        Self::try_from(u32::from(n.0))
            .map_err(|_| ProgramError::AccountDataTooSmall)
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

/// Helper function to get the current TlvIndices from the current spot
fn get_tlv_indices(type_start: usize) -> TlvIndices {
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
fn get_value_indices<V: Value>(
    tlv_data: &[u8],
    init: bool,
) -> Result<TlvIndices, ProgramError> {
    let mut start_index = 0;
    while start_index < tlv_data.len() {
        let tlv_indices = get_tlv_indices(start_index);
        if tlv_data.len() < tlv_indices.value_start {
            return Err(ProgramError::InvalidAccountData);
        }
        let discriminator = Discriminator::try_from(&tlv_data[tlv_indices.type_start..tlv_indices.length_start])?;
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
            let value_end_index = tlv_indices.value_start.saturating_add(usize::try_from(*length)?);
            start_index = value_end_index;
        }
    }
    Err(ProgramError::InvalidAccountData)
}

fn get_discriminators(tlv_data: &[u8]) -> Result<Vec<Discriminator>, ProgramError> {
    let mut discriminators = vec![];
    let mut start_index = 0;
    while start_index < tlv_data.len() {
        let tlv_indices = get_tlv_indices(start_index);
        if tlv_data.len() < tlv_indices.length_start {
            // we got to the end, but there might be some uninitialized data after
            let remainder = &tlv_data[tlv_indices.type_start..];
            if remainder.iter().all(|&x| x == 0) {
                return Ok(discriminators)
            } else {
                return Err(ProgramError::InvalidAccountData)
            }
        }
        let discriminator = Discriminator::try_from(&tlv_data[tlv_indices.type_start..tlv_indices.length_start])?;
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

            let value_end_index = tlv_indices.value_start.saturating_add(usize::try_from(*length)?);
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
    } = get_value_indices::<V>(tlv_data, false)?;
    // get_value_indices has checked that tlv_data is long enough to include these indices
    let length = pod_from_bytes::<Length>(&tlv_data[length_start..value_start])?;
    let value_end = value_start.saturating_add(usize::try_from(*length)?);
    if tlv_data.len() < value_end {
        return Err(ProgramError::InvalidAccountData);
    }
    pod_from_bytes::<V>(&tlv_data[value_start..value_end])
}

fn check_initialized(tlv_data: &[u8]) -> Result<(), ProgramError> {
    let discriminators = get_discriminators(tlv_data)?;
    if discriminators.is_empty() {
        Err(ProgramError::InvalidAccountData)
    } else {
        Ok(())
    }
}

fn check_uninitialized(tlv_data: &[u8]) -> Result<(), ProgramError> {
    let discriminators = get_discriminators(tlv_data)?;
    if discriminators.is_empty() {
        Ok(())
    } else {
        Err(ProgramError::InvalidAccountData)
    }
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
        check_initialized(&data)?;
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
        check_initialized(data)?;
        Ok(Self { data, })
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
        check_initialized(&data)?;
        Ok(Self { data })
    }

    /// Unpacks uninitialized TLV state
    ///
    /// Fails if any state has been initialized
    pub fn unpack_uninitialized(data: &'data mut [u8]) -> Result<Self, ProgramError> {
        check_uninitialized(&data)?;
        Ok(Self { data })
    }

    /// Unpack a portion of the TLV data as the desired type that allows modifying the type
    pub fn get_value_mut<V: Value>(&mut self) -> Result<&mut V, ProgramError> {
        let TlvIndices {
            type_start,
            length_start,
            value_start,
        } = get_value_indices::<V>(self.data, false)?;

        if self.data[type_start..].len() < size_of::<V>() {
            return Err(ProgramError::InvalidAccountData);
        }
        let length = pod_from_bytes::<Length>(&self.data[length_start..value_start])?;
        let value_end = value_start.saturating_add(usize::try_from(*length)?);
        pod_from_bytes_mut::<V>(&mut self.data[value_start..value_end])
    }

    /// Packs the default extension data into an open slot if not already found in the
    /// data buffer. If extension is already found in the buffer, it overwrites the existing
    /// extension with the default state if `overwrite` is set. If extension found, but
    /// `overwrite` is not set, it returns error.
    pub fn init_value<V: Value>(
        &mut self,
        overwrite: bool,
    ) -> Result<&mut V, ProgramError> {
        let TlvIndices {
            type_start,
            length_start,
            value_start,
        } = get_value_indices::<V>(self.data, true)?;

        if self.data[type_start..].len() < size_of::<V>() {
            return Err(ProgramError::InvalidAccountData);
        }
        let discriminator = Discriminator::try_from(&self.data[type_start..length_start])?;
        if discriminator == Discriminator::UNINITIALIZED || overwrite {
            // write type
            let discriminator_ref = &mut self.data[type_start..length_start];
            discriminator_ref.copy_from_slice(&V::TYPE.as_ref());
            // write length
            let length_ref =
                pod_from_bytes_mut::<Length>(&mut self.data[length_start..value_start])?;
            // maybe this becomes smarter later for dynamically sized extensions
            let length = size_of::<V>();
            *length_ref = Length::try_from(length)?;

            let value_end = value_start.saturating_add(length);
            let extension_ref =
                pod_from_bytes_mut::<V>(&mut self.data[value_start..value_end])?;
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
        <[u8; DISCRIMINATOR_LENGTH]>::try_from(a).map(Self::from).map_err(|_| ProgramError::InvalidAccountData)
    }
}

/// Trait to be implemented by all value types in the TLV structure, specifying
/// the discriminator to check against
pub trait Value: Pod + Default {
    /// Associated value type enum, checked at the start of TLV entries
    const TYPE: Discriminator;
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::state::ValidationPubkeys,
    };

    const TEST_BUFFER: &[u8] = &[
        1, 1, 1, 1, 1, 1, 1, 1, // discriminator
        32, 0, 0, 0, // length
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, // value
        0, 0, // empty, not enough for a discriminator
    ];

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
    struct TestValue {
        data: [u8; 32]
    }
    impl Value for TestValue {
        const TYPE: Discriminator = Discriminator::new([1; DISCRIMINATOR_LENGTH]);
    }

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
    struct TestEmptyValue;
    impl Value for TestEmptyValue {
        const TYPE: Discriminator = Discriminator::new([2; DISCRIMINATOR_LENGTH]);
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
            TlvStateMut::unpack_uninitialized(&mut buffer),
            Err(ProgramError::InvalidAccountData)
        );

        /*
        // tweak the extension type
        let mut buffer = MINT_WITH_EXTENSION.to_vec();
        buffer[BASE_ACCOUNT_LENGTH + 1] = 2;
        let state = StateWithExtensions::<Mint>::unpack(&buffer).unwrap();
        assert_eq!(
            state.get_extension::<TransferFeeConfig>(),
            Err(ProgramError::Custom(
                TokenError::ExtensionTypeMismatch as u32
            ))
        );

        // tweak the length, too big
        let mut buffer = MINT_WITH_EXTENSION.to_vec();
        buffer[BASE_ACCOUNT_LENGTH + 3] = 100;
        let state = StateWithExtensions::<Mint>::unpack(&buffer).unwrap();
        assert_eq!(
            state.get_extension::<TransferFeeConfig>(),
            Err(ProgramError::InvalidAccountData)
        );

        // tweak the length, too small
        let mut buffer = MINT_WITH_EXTENSION.to_vec();
        buffer[BASE_ACCOUNT_LENGTH + 3] = 10;
        let state = StateWithExtensions::<Mint>::unpack(&buffer).unwrap();
        assert_eq!(
            state.get_extension::<TransferFeeConfig>(),
            Err(ProgramError::InvalidAccountData)
        );

        // data buffer is too small
        let buffer = &MINT_WITH_EXTENSION[..MINT_WITH_EXTENSION.len() - 1];
        let state = StateWithExtensions::<Mint>::unpack(buffer).unwrap();
        assert_eq!(
            state.get_extension::<MintCloseAuthority>(),
            Err(ProgramError::InvalidAccountData)
        );
        */
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
        assert_eq!(get_discriminators(&[0, 0, 0, 0, 0, 0, 0, 0]).unwrap(), vec![]);
    }

    #[test]
    fn value_pack_unpack() {
        /*
        let mint_size = ExtensionType::get_account_len::<Mint>(&[
            ExtensionType::MintCloseAuthority,
            ExtensionType::TransferFeeConfig,
        ]);
        let mut buffer = vec![0; mint_size];

        // fail unpack
        assert_eq!(
            StateWithExtensionsMut::<Mint>::unpack(&mut buffer),
            Err(ProgramError::UninitializedAccount),
        );

        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        // fail init account extension
        assert_eq!(
            state.init_extension::<TransferFeeAmount>(true),
            Err(ProgramError::InvalidAccountData),
        );

        // success write extension
        let close_authority = OptionalNonZeroPubkey::try_from(Some(Pubkey::new(&[1; 32]))).unwrap();
        let extension = state.init_extension::<MintCloseAuthority>(true).unwrap();
        extension.close_authority = close_authority;
        assert_eq!(
            &state.get_extension_types().unwrap(),
            &[ExtensionType::MintCloseAuthority]
        );

        // fail init extension when already initialized
        assert_eq!(
            state.init_extension::<MintCloseAuthority>(false),
            Err(ProgramError::Custom(
                TokenError::ExtensionAlreadyInitialized as u32
            ))
        );

        // fail unpack as account, a mint extension was written
        assert_eq!(
            StateWithExtensionsMut::<Account>::unpack_uninitialized(&mut buffer),
            Err(ProgramError::Custom(
                TokenError::ExtensionBaseMismatch as u32
            ))
        );

        // fail unpack again, still no base data
        assert_eq!(
            StateWithExtensionsMut::<Mint>::unpack(&mut buffer.clone()),
            Err(ProgramError::UninitializedAccount),
        );

        // write base mint
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        state.base = TEST_MINT;
        state.pack_base();
        state.init_account_type().unwrap();

        // check raw buffer
        let mut expect = TEST_MINT_SLICE.to_vec();
        expect.extend_from_slice(&[0; BASE_ACCOUNT_LENGTH - Mint::LEN]); // padding
        expect.push(AccountType::Mint.into());
        expect.extend_from_slice(&(ExtensionType::MintCloseAuthority as u16).to_le_bytes());
        expect
            .extend_from_slice(&(size_of::<MintCloseAuthority>() as u16).to_le_bytes());
        expect.extend_from_slice(&[1; 32]); // data
        expect.extend_from_slice(&[0; size_of::<ExtensionType>()]);
        expect.extend_from_slice(&[0; size_of::<Length>()]);
        expect.extend_from_slice(&[0; size_of::<TransferFeeConfig>()]);
        assert_eq!(expect, buffer);

        // unpack uninitialized will now fail because the Mint is now initialized
        assert_eq!(
            StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer.clone()),
            Err(TokenError::AlreadyInUse.into()),
        );

        // check unpacking
        let mut state = StateWithExtensionsMut::<Mint>::unpack(&mut buffer).unwrap();

        // update base
        state.base = TEST_MINT;
        state.base.supply += 100;
        state.pack_base();

        // check unpacking
        let mut unpacked_extension = state.get_extension_mut::<MintCloseAuthority>().unwrap();
        assert_eq!(*unpacked_extension, MintCloseAuthority { close_authority });

        // update extension
        let close_authority = OptionalNonZeroPubkey::try_from(None).unwrap();
        unpacked_extension.close_authority = close_authority;

        // check updates are propagated
        let base = state.base;
        let state = StateWithExtensions::<Mint>::unpack(&buffer).unwrap();
        assert_eq!(state.base, base);
        let unpacked_extension = state.get_extension::<MintCloseAuthority>().unwrap();
        assert_eq!(*unpacked_extension, MintCloseAuthority { close_authority });

        // check raw buffer
        let mut expect = vec![0; Mint::LEN];
        Mint::pack_into_slice(&base, &mut expect);
        expect.extend_from_slice(&[0; BASE_ACCOUNT_LENGTH - Mint::LEN]); // padding
        expect.push(AccountType::Mint.into());
        expect.extend_from_slice(&(ExtensionType::MintCloseAuthority as u16).to_le_bytes());
        expect
            .extend_from_slice(&(size_of::<MintCloseAuthority>() as u16).to_le_bytes());
        expect.extend_from_slice(&[0; 32]);
        expect.extend_from_slice(&[0; size_of::<ExtensionType>()]);
        expect.extend_from_slice(&[0; size_of::<Length>()]);
        expect.extend_from_slice(&[0; size_of::<TransferFeeConfig>()]);
        assert_eq!(expect, buffer);

        // fail unpack as an account
        assert_eq!(
            StateWithExtensions::<Account>::unpack(&buffer),
            Err(ProgramError::InvalidAccountData),
        );

        let mut state = StateWithExtensionsMut::<Mint>::unpack(&mut buffer).unwrap();
        // init one more extension
        let mint_transfer_fee = test_transfer_fee_config();
        let new_extension = state.init_extension::<TransferFeeConfig>(true).unwrap();
        new_extension.transfer_fee_config_authority =
            mint_transfer_fee.transfer_fee_config_authority;
        new_extension.withdraw_withheld_authority = mint_transfer_fee.withdraw_withheld_authority;
        new_extension.withheld_amount = mint_transfer_fee.withheld_amount;
        new_extension.older_transfer_fee = mint_transfer_fee.older_transfer_fee;
        new_extension.newer_transfer_fee = mint_transfer_fee.newer_transfer_fee;

        assert_eq!(
            &state.get_extension_types().unwrap(),
            &[
                ExtensionType::MintCloseAuthority,
                ExtensionType::TransferFeeConfig
            ]
        );

        // check raw buffer
        let mut expect = vec![0; Mint::LEN];
        Mint::pack_into_slice(&base, &mut expect);
        expect.extend_from_slice(&[0; BASE_ACCOUNT_LENGTH - Mint::LEN]); // padding
        expect.push(AccountType::Mint.into());
        expect.extend_from_slice(&(ExtensionType::MintCloseAuthority as u16).to_le_bytes());
        expect
            .extend_from_slice(&(size_of::<MintCloseAuthority>() as u16).to_le_bytes());
        expect.extend_from_slice(&[0; 32]); // data
        expect.extend_from_slice(&(ExtensionType::TransferFeeConfig as u16).to_le_bytes());
        expect.extend_from_slice(&(size_of::<TransferFeeConfig>() as u16).to_le_bytes());
        expect.extend_from_slice(pod_bytes_of(&mint_transfer_fee));
        assert_eq!(expect, buffer);

        // fail to init one more extension that does not fit
        let mut state = StateWithExtensionsMut::<Mint>::unpack(&mut buffer).unwrap();
        assert_eq!(
            state.init_extension::<MintPaddingTest>(true),
            Err(ProgramError::InvalidAccountData),
        );
        */
    }

    #[test]
    fn value_any_order() {
        /*
        let mint_size = ExtensionType::get_account_len::<Mint>(&[
            ExtensionType::MintCloseAuthority,
            ExtensionType::TransferFeeConfig,
        ]);
        let mut buffer = vec![0; mint_size];

        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        // write extensions
        let close_authority = OptionalNonZeroPubkey::try_from(Some(Pubkey::new(&[1; 32]))).unwrap();
        let extension = state.init_extension::<MintCloseAuthority>(true).unwrap();
        extension.close_authority = close_authority;

        let mint_transfer_fee = test_transfer_fee_config();
        let extension = state.init_extension::<TransferFeeConfig>(true).unwrap();
        extension.transfer_fee_config_authority = mint_transfer_fee.transfer_fee_config_authority;
        extension.withdraw_withheld_authority = mint_transfer_fee.withdraw_withheld_authority;
        extension.withheld_amount = mint_transfer_fee.withheld_amount;
        extension.older_transfer_fee = mint_transfer_fee.older_transfer_fee;
        extension.newer_transfer_fee = mint_transfer_fee.newer_transfer_fee;

        assert_eq!(
            &state.get_extension_types().unwrap(),
            &[
                ExtensionType::MintCloseAuthority,
                ExtensionType::TransferFeeConfig
            ]
        );

        // write base mint
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        state.base = TEST_MINT;
        state.pack_base();
        state.init_account_type().unwrap();

        let mut other_buffer = vec![0; mint_size];
        let mut state =
            StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut other_buffer).unwrap();

        // write base mint
        state.base = TEST_MINT;
        state.pack_base();
        state.init_account_type().unwrap();

        // write extensions in a different order
        let mint_transfer_fee = test_transfer_fee_config();
        let extension = state.init_extension::<TransferFeeConfig>(true).unwrap();
        extension.transfer_fee_config_authority = mint_transfer_fee.transfer_fee_config_authority;
        extension.withdraw_withheld_authority = mint_transfer_fee.withdraw_withheld_authority;
        extension.withheld_amount = mint_transfer_fee.withheld_amount;
        extension.older_transfer_fee = mint_transfer_fee.older_transfer_fee;
        extension.newer_transfer_fee = mint_transfer_fee.newer_transfer_fee;

        let close_authority = OptionalNonZeroPubkey::try_from(Some(Pubkey::new(&[1; 32]))).unwrap();
        let extension = state.init_extension::<MintCloseAuthority>(true).unwrap();
        extension.close_authority = close_authority;

        assert_eq!(
            &state.get_extension_types().unwrap(),
            &[
                ExtensionType::TransferFeeConfig,
                ExtensionType::MintCloseAuthority
            ]
        );

        // buffers are NOT the same because written in a different order
        assert_ne!(buffer, other_buffer);
        let state = StateWithExtensions::<Mint>::unpack(&buffer).unwrap();
        let other_state = StateWithExtensions::<Mint>::unpack(&other_buffer).unwrap();

        // BUT mint and extensions are the same
        assert_eq!(
            state.get_extension::<TransferFeeConfig>().unwrap(),
            other_state.get_extension::<TransferFeeConfig>().unwrap()
        );
        assert_eq!(
            state.get_extension::<MintCloseAuthority>().unwrap(),
            other_state.get_extension::<MintCloseAuthority>().unwrap()
        );
        assert_eq!(state.base, other_state.base);
        */
    }

    #[test]
    fn init_nonzero_default() {
        /*
        let mint_size = ExtensionType::get_account_len::<Mint>(&[ExtensionType::MintPaddingTest]);
        let mut buffer = vec![0; mint_size];
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        state.base = TEST_MINT;
        state.pack_base();
        state.init_account_type().unwrap();
        let extension = state.init_extension::<MintPaddingTest>(true).unwrap();
        assert_eq!(extension.padding1, [1; 128]);
        assert_eq!(extension.padding2, [2; 48]);
        assert_eq!(extension.padding3, [3; 9]);
        */
    }

    #[test]
    fn init_buffer_too_small() {
        /*
        let mint_size =
            ExtensionType::get_account_len::<Mint>(&[ExtensionType::MintCloseAuthority]);
        let mut buffer = vec![0; mint_size - 1];
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        let err = state
            .init_extension::<MintCloseAuthority>(true)
            .unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);

        state.tlv_data[0] = 3;
        state.tlv_data[2] = 32;
        let err = state.get_extension_mut::<MintCloseAuthority>().unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);

        let mut buffer = vec![0; Mint::LEN + 2];
        let err = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);

        // OK since there are two bytes for the type, which is `Uninitialized`
        let mut buffer = vec![0; BASE_ACCOUNT_LENGTH + 3];
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        let err = state.get_extension_mut::<MintCloseAuthority>().unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);

        assert_eq!(state.get_extension_types().unwrap(), vec![]);

        // malformed since there aren't two bytes for the type
        let mut buffer = vec![0; BASE_ACCOUNT_LENGTH + 2];
        let state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        assert_eq!(
            state.get_extension_types().unwrap_err(),
            ProgramError::InvalidAccountData
        );
        */
    }

    #[test]
    fn value_with_no_data() {
        /*
        let account_size =
            ExtensionType::get_account_len::<Account>(&[ExtensionType::ImmutableOwner]);
        let mut buffer = vec![0; account_size];
        let mut state =
            StateWithExtensionsMut::<Account>::unpack_uninitialized(&mut buffer).unwrap();
        state.base = TEST_ACCOUNT;
        state.pack_base();
        state.init_account_type().unwrap();

        let err = state.get_extension::<ImmutableOwner>().unwrap_err();
        assert_eq!(
            err,
            ProgramError::Custom(TokenError::ExtensionNotFound as u32)
        );

        state.init_extension::<ImmutableOwner>(true).unwrap();
        assert_eq!(
            get_first_extension_type(state.tlv_data).unwrap(),
            Some(ExtensionType::ImmutableOwner)
        );
        assert_eq!(
            get_extension_types(state.tlv_data).unwrap(),
            vec![ExtensionType::ImmutableOwner]
        );
        */
    }
}
