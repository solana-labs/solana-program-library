//! Pod types to be used with bytemuck for zero-copy serde

use {
    crate::error::AccountResolutionError,
    bytemuck::{Pod, Zeroable},
    solana_program::{
        account_info::AccountInfo, instruction::AccountMeta, program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_type_length_value::pod::{pod_from_bytes, pod_from_bytes_mut, PodU32},
};

/// Convert a slice into a mutable `Pod` slice (zero copy)
pub fn pod_slice_from_bytes<T: Pod>(bytes: &[u8]) -> Result<&[T], ProgramError> {
    bytemuck::try_cast_slice(bytes).map_err(|_| ProgramError::InvalidArgument)
}
/// Convert a slice into a mutable `Pod` slice (zero copy)
pub fn pod_slice_from_bytes_mut<T: Pod>(bytes: &mut [u8]) -> Result<&mut [T], ProgramError> {
    bytemuck::try_cast_slice_mut(bytes).map_err(|_| ProgramError::InvalidArgument)
}

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

impl From<&AccountMeta> for PodAccountMeta {
    fn from(meta: &AccountMeta) -> Self {
        Self {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer.into(),
            is_writable: meta.is_writable.into(),
        }
    }
}

impl From<&PodAccountMeta> for AccountMeta {
    fn from(meta: &PodAccountMeta) -> Self {
        Self {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer.into(),
            is_writable: meta.is_writable.into(),
        }
    }
}

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
            return Err(AccountResolutionError::BufferTooSmall.into());
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
            .ok_or_else(|| AccountResolutionError::CalculationFailure.into())
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
            return Err(AccountResolutionError::BufferTooSmall.into());
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
            Err(AccountResolutionError::BufferTooSmall.into())
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
        .ok_or(AccountResolutionError::CalculationFailure)?;
    // check that it isn't overallocated
    if max_len.saturating_mul(size) != data_len {
        Err(AccountResolutionError::BufferTooLarge.into())
    } else {
        Ok(max_len)
    }
}
