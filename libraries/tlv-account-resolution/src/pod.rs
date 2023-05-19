//! Pod types to be used with bytemuck for zero-copy serde

use {
    crate::{
        account::{AccountMetaPda, RequiredAccount},
        error::AccountResolutionError,
        seeds::Seed,
    },
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
impl From<&bool> for PodBool {
    fn from(b: &bool) -> Self {
        Self(if *b { 1 } else { 0 })
    }
}
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
    /// Discriminator to tell whether this represents a standard
    /// `AccountMeta` or `AccountMetaPda`
    pub discriminator: u8,
    /// This `address_config` field can either be the pubkey of the account
    /// or the seeds used to derive the pubkey from provided inputs
    pub address_config: [u8; 32],
    /// Whether the account should sign
    pub is_signer: PodBool,
    /// Whether the account should be writable
    pub is_writable: PodBool,
}

impl From<&AccountMeta> for PodAccountMeta {
    fn from(meta: &AccountMeta) -> Self {
        Self {
            discriminator: 0,
            address_config: meta.pubkey.to_bytes(),
            is_signer: meta.is_signer.into(),
            is_writable: meta.is_writable.into(),
        }
    }
}

impl From<&AccountInfo<'_>> for PodAccountMeta {
    fn from(account_info: &AccountInfo) -> Self {
        Self {
            discriminator: 0,
            address_config: account_info.key.to_bytes(),
            is_signer: account_info.is_signer.into(),
            is_writable: account_info.is_writable.into(),
        }
    }
}

impl From<&AccountMetaPda> for PodAccountMeta {
    fn from(pda: &AccountMetaPda) -> Self {
        Self {
            discriminator: 1,
            address_config: pda.seeds,
            is_signer: pda.is_signer.into(),
            is_writable: pda.is_writable.into(),
        }
    }
}

impl TryFrom<&PodAccountMeta> for AccountMeta {
    type Error = ProgramError;

    fn try_from(pod: &PodAccountMeta) -> Result<Self, Self::Error> {
        if pod.discriminator == 0 {
            Ok(AccountMeta {
                pubkey: Pubkey::new(&pod.address_config),
                is_signer: pod.is_signer.into(),
                is_writable: pod.is_writable.into(),
            })
        } else {
            Err(AccountResolutionError::RequiredAccountNotAccountMeta.into())
        }
    }
}

impl TryFrom<&PodAccountMeta> for RequiredAccount {
    type Error = ProgramError;

    fn try_from(pod: &PodAccountMeta) -> Result<Self, Self::Error> {
        if pod.discriminator == 0 {
            Ok(RequiredAccount::Account {
                pubkey: Pubkey::new(&pod.address_config),
                is_signer: pod.is_signer.into(),
                is_writable: pod.is_writable.into(),
            })
        } else {
            Ok(RequiredAccount::Pda {
                seeds: Seed::unpack_to_vec(&pod.address_config)?,
                is_signer: pod.is_signer.into(),
                is_writable: pod.is_writable.into(),
            })
        }
    }
}

/// Unfortunately this has to be its own trait in order for the
/// trait constraint in `ExtraAccountMetas::init` to work properly.
///
/// The `?` can't resolve to a `ProgramError` using just `TryFrom<T>`
pub trait TryFromAccountType<T>: Sized {
    /// Mimics the functionality of `try_from(T)` for `PodAccountMeta`
    fn try_from_account(value: T) -> Result<Self, ProgramError>;
}

impl TryFromAccountType<&AccountInfo<'_>> for PodAccountMeta {
    fn try_from_account(account_info: &AccountInfo<'_>) -> Result<Self, ProgramError> {
        Ok(PodAccountMeta {
            discriminator: 0,
            address_config: account_info.key.to_bytes(),
            is_signer: account_info.is_signer.into(),
            is_writable: account_info.is_writable.into(),
        })
    }
}

impl TryFromAccountType<&AccountMeta> for PodAccountMeta {
    fn try_from_account(meta: &AccountMeta) -> Result<Self, ProgramError> {
        Ok(PodAccountMeta {
            discriminator: 0,
            address_config: meta.pubkey.to_bytes(),
            is_signer: meta.is_signer.into(),
            is_writable: meta.is_writable.into(),
        })
    }
}

impl TryFromAccountType<&RequiredAccount> for PodAccountMeta {
    fn try_from_account(value: &RequiredAccount) -> Result<Self, ProgramError> {
        match value {
            RequiredAccount::Account {
                pubkey,
                is_signer,
                is_writable,
            } => Ok(PodAccountMeta {
                discriminator: 0,
                address_config: pubkey.to_bytes(),
                is_signer: is_signer.into(),
                is_writable: is_writable.into(),
            }),
            RequiredAccount::Pda {
                seeds,
                is_signer,
                is_writable,
            } => Ok(PodAccountMeta {
                discriminator: 1,
                address_config: Seed::pack_slice(seeds)?,
                is_signer: is_signer.into(),
                is_writable: is_writable.into(),
            }),
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
