//! Pod types to be used with bytemuck for zero-copy serde

use {
    crate::error::PermissionedTransferError,
    bytemuck::{Pod, Zeroable},
    solana_program::{
        account_info::AccountInfo, instruction::AccountMeta, program_error::ProgramError,
        pubkey::Pubkey,
    },
};

/// Convert a slice into a `Pod` (zero copy)
pub fn pod_from_bytes<T: Pod>(bytes: &[u8]) -> Result<&T, ProgramError> {
    bytemuck::try_from_bytes(bytes).map_err(|_| ProgramError::InvalidArgument)
}
/// Convert a slice into a mutable `Pod` (zero copy)
pub fn pod_from_bytes_mut<T: Pod>(bytes: &mut [u8]) -> Result<&mut T, ProgramError> {
    bytemuck::try_from_bytes_mut(bytes).map_err(|_| ProgramError::InvalidArgument)
}
/// Convert a slice into a mutable `Pod` slice (zero copy)
pub fn pod_slice_from_bytes<T: Pod>(bytes: &[u8]) -> Result<&[T], ProgramError> {
    bytemuck::try_cast_slice(bytes).map_err(|_| ProgramError::InvalidArgument)
}
/// Convert a slice into a mutable `Pod` slice (zero copy)
pub fn pod_slice_from_bytes_mut<T: Pod>(bytes: &mut [u8]) -> Result<&mut [T], ProgramError> {
    bytemuck::try_cast_slice_mut(bytes).map_err(|_| ProgramError::InvalidArgument)
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

/// The standard `bool` is not a `Pod`, define a replacement that is
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodBool(u8);
impl From<bool> for PodBool {
    fn from(b: bool) -> Self {
        Self(if b { 1 } else { 0 })
    }
}
impl From<&PodBool> for bool {
    fn from(b: &PodBool) -> Self {
        b.0 != 0
    }
}
impl From<PodBool> for bool {
    fn from(b: PodBool) -> Self {
        b.0 != 0
    }
}

/// The standard `AccountMeta` is not a `Pod`, define a replacement that is
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PodAccountMeta {
    /// The pubkey of the account
    pub pubkey: Pubkey,
    /// Whether the account should sign
    pub is_signer: PodBool,
    /// Whether the account should be writable
    pub is_writable: PodBool,
}
impl PartialEq<AccountInfo<'_>> for PodAccountMeta {
    fn eq(&self, other: &AccountInfo) -> bool {
        self.pubkey == *other.key
            && self.is_signer == other.is_signer.into()
            && self.is_writable == other.is_writable.into()
    }
}

impl From<&AccountInfo<'_>> for PodAccountMeta {
    fn from(account_info: &AccountInfo) -> Self {
        Self {
            pubkey: *account_info.key,
            is_signer: account_info.is_signer.into(),
            is_writable: account_info.is_writable.into(),
        }
    }
}

impl From<PodAccountMeta> for AccountMeta {
    fn from(meta: PodAccountMeta) -> Self {
        Self {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer.into(),
            is_writable: meta.is_writable.into(),
        }
    }
}

const LENGTH_SIZE: usize = std::mem::size_of::<PodU16>();
/// Special type for using a slice of `Pod`s in a zero-copy way
pub struct PodSlice<'data, T: Pod> {
    length: &'data PodU16,
    data: &'data [T],
}
impl<'data, T: Pod> PodSlice<'data, T> {
    /// Unpack the buffer into a slice
    pub fn unpack<'a>(data: &'a [u8]) -> Result<Self, ProgramError>
    where
        'a: 'data,
    {
        if data.len() < LENGTH_SIZE {
            return Err(PermissionedTransferError::BufferTooSmall.into());
        }
        let (length, data) = data.split_at(LENGTH_SIZE);
        let length = pod_from_bytes::<PodU16>(length)?;
        let _max_length = max_len_for_type::<T>(data.len())?;
        let data = pod_slice_from_bytes(data)?;
        Ok(Self { length, data })
    }

    /// Get the slice data
    pub fn data(&self) -> &[T] {
        let length = u16::from(*self.length) as usize;
        &self.data[..length]
    }

    /// Get the amount of bytes used by `num_items`
    pub fn byte_size_of(num_items: usize) -> Result<usize, ProgramError> {
        std::mem::size_of::<T>()
            .checked_mul(num_items)
            .and_then(|len| len.checked_add(LENGTH_SIZE))
            .ok_or_else(|| PermissionedTransferError::CalculationFailure.into())
    }
}

/// Special type for using a slice of mutable `Pod`s in a zero-copy way
pub struct PodSliceMut<'data, T: Pod> {
    length: &'data mut PodU16,
    data: &'data mut [T],
    max_length: usize,
}
impl<'data, T: Pod> PodSliceMut<'data, T> {
    /// Unpack the mutable buffer into a mutable slice, with the option to
    /// initialize the data
    pub fn unpack<'a>(data: &'a mut [u8], init: bool) -> Result<Self, ProgramError>
    where
        'a: 'data,
    {
        if data.len() < LENGTH_SIZE {
            return Err(PermissionedTransferError::BufferTooSmall.into());
        }
        let (length, data) = data.split_at_mut(LENGTH_SIZE);
        let length = pod_from_bytes_mut::<PodU16>(length)?;
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

    /// Add another item to the slice
    pub fn push(&mut self, t: T) -> Result<(), ProgramError> {
        let length = u16::from(*self.length);
        if length as usize == self.max_length {
            Err(PermissionedTransferError::BufferTooSmall.into())
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
        .ok_or(PermissionedTransferError::CalculationFailure)?;
    // check that it isn't overallocated
    if max_len.saturating_mul(size) != data_len {
        Err(PermissionedTransferError::BufferTooLarge.into())
    } else {
        Ok(max_len)
    }
}
