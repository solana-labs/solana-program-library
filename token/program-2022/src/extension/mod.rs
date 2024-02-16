//! Extensions available to token mints and accounts

#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
use {
    crate::{
        error::TokenError,
        extension::{
            confidential_transfer::{ConfidentialTransferAccount, ConfidentialTransferMint},
            confidential_transfer_fee::{
                ConfidentialTransferFeeAmount, ConfidentialTransferFeeConfig,
            },
            cpi_guard::CpiGuard,
            default_account_state::DefaultAccountState,
            group_member_pointer::GroupMemberPointer,
            group_pointer::GroupPointer,
            immutable_owner::ImmutableOwner,
            interest_bearing_mint::InterestBearingConfig,
            memo_transfer::MemoTransfer,
            metadata_pointer::MetadataPointer,
            mint_close_authority::MintCloseAuthority,
            non_transferable::{NonTransferable, NonTransferableAccount},
            permanent_delegate::PermanentDelegate,
            transfer_fee::{TransferFeeAmount, TransferFeeConfig},
            transfer_hook::{TransferHook, TransferHookAccount},
        },
        state::{Account, Mint, Multisig},
    },
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        account_info::AccountInfo,
        program_error::ProgramError,
        program_pack::{IsInitialized, Pack},
    },
    spl_pod::{
        bytemuck::{pod_from_bytes, pod_from_bytes_mut, pod_get_packed_len},
        primitives::PodU16,
    },
    spl_token_group_interface::state::{TokenGroup, TokenGroupMember},
    spl_type_length_value::variable_len_pack::VariableLenPack,
    std::{
        cmp::Ordering,
        convert::{TryFrom, TryInto},
        mem::size_of,
    },
};

/// Confidential Transfer extension
pub mod confidential_transfer;
/// Confidential Transfer Fee extension
pub mod confidential_transfer_fee;
/// CPI Guard extension
pub mod cpi_guard;
/// Default Account State extension
pub mod default_account_state;
/// Group Member Pointer extension
pub mod group_member_pointer;
/// Group Pointer extension
pub mod group_pointer;
/// Immutable Owner extension
pub mod immutable_owner;
/// Interest-Bearing Mint extension
pub mod interest_bearing_mint;
/// Memo Transfer extension
pub mod memo_transfer;
/// Metadata Pointer extension
pub mod metadata_pointer;
/// Mint Close Authority extension
pub mod mint_close_authority;
/// Non Transferable extension
pub mod non_transferable;
/// Permanent Delegate extension
pub mod permanent_delegate;
/// Utility to reallocate token accounts
pub mod reallocate;
/// Token-group extension
pub mod token_group;
/// Token-metadata extension
pub mod token_metadata;
/// Transfer Fee extension
pub mod transfer_fee;
/// Transfer Hook extension
pub mod transfer_hook;

/// Length in TLV structure
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct Length(PodU16);
impl From<Length> for usize {
    fn from(n: Length) -> Self {
        Self::from(u16::from(n.0))
    }
}
impl TryFrom<usize> for Length {
    type Error = ProgramError;
    fn try_from(n: usize) -> Result<Self, Self::Error> {
        u16::try_from(n)
            .map(|v| Self(PodU16::from(v)))
            .map_err(|_| ProgramError::AccountDataTooSmall)
    }
}

/// Helper function to get the current TlvIndices from the current spot
fn get_tlv_indices(type_start: usize) -> TlvIndices {
    let length_start = type_start.saturating_add(size_of::<ExtensionType>());
    let value_start = length_start.saturating_add(pod_get_packed_len::<Length>());
    TlvIndices {
        type_start,
        length_start,
        value_start,
    }
}

/// Helper function to tack on the size of an extension bytes if an account with
/// extensions is exactly the size of a multisig
const fn adjust_len_for_multisig(account_len: usize) -> usize {
    if account_len == Multisig::LEN {
        account_len.saturating_add(size_of::<ExtensionType>())
    } else {
        account_len
    }
}

/// Helper function to calculate exactly how many bytes a value will take up,
/// given the value's length
const fn add_type_and_length_to_len(value_len: usize) -> usize {
    value_len
        .saturating_add(size_of::<ExtensionType>())
        .saturating_add(pod_get_packed_len::<Length>())
}

/// Helper struct for returning the indices of the type, length, and value in
/// a TLV entry
#[derive(Debug)]
struct TlvIndices {
    pub type_start: usize,
    pub length_start: usize,
    pub value_start: usize,
}
fn get_extension_indices<V: Extension>(
    tlv_data: &[u8],
    init: bool,
) -> Result<TlvIndices, ProgramError> {
    let mut start_index = 0;
    let v_account_type = V::TYPE.get_account_type();
    while start_index < tlv_data.len() {
        let tlv_indices = get_tlv_indices(start_index);
        if tlv_data.len() < tlv_indices.value_start {
            return Err(ProgramError::InvalidAccountData);
        }
        let extension_type =
            ExtensionType::try_from(&tlv_data[tlv_indices.type_start..tlv_indices.length_start])?;
        let account_type = extension_type.get_account_type();
        if extension_type == V::TYPE {
            // found an instance of the extension that we're initializing, return!
            return Ok(tlv_indices);
        // got to an empty spot, init here, or error if we're searching, since
        // nothing is written after an Uninitialized spot
        } else if extension_type == ExtensionType::Uninitialized {
            if init {
                return Ok(tlv_indices);
            } else {
                return Err(TokenError::ExtensionNotFound.into());
            }
        } else if v_account_type != account_type {
            return Err(TokenError::ExtensionTypeMismatch.into());
        } else {
            let length = pod_from_bytes::<Length>(
                &tlv_data[tlv_indices.length_start..tlv_indices.value_start],
            )?;
            let value_end_index = tlv_indices.value_start.saturating_add(usize::from(*length));
            start_index = value_end_index;
        }
    }
    Err(ProgramError::InvalidAccountData)
}

/// Basic information about the TLV buffer, collected from iterating through all
/// entries
#[derive(Debug, PartialEq)]
struct TlvDataInfo {
    /// The extension types written in the TLV buffer
    extension_types: Vec<ExtensionType>,
    /// The total number bytes allocated for all TLV entries.
    ///
    /// Each TLV entry's allocated bytes comprises two bytes for the `type`, two
    /// bytes for the `length`, and `length` number of bytes for the `value`.
    used_len: usize,
}

/// Fetches basic information about the TLV buffer by iterating through all
/// TLV entries.
fn get_tlv_data_info(tlv_data: &[u8]) -> Result<TlvDataInfo, ProgramError> {
    let mut extension_types = vec![];
    let mut start_index = 0;
    while start_index < tlv_data.len() {
        let tlv_indices = get_tlv_indices(start_index);
        if tlv_data.len() < tlv_indices.length_start {
            // There aren't enough bytes to store the next type, which means we
            // got to the end. The last byte could be used during a realloc!
            return Ok(TlvDataInfo {
                extension_types,
                used_len: tlv_indices.type_start,
            });
        }
        let extension_type =
            ExtensionType::try_from(&tlv_data[tlv_indices.type_start..tlv_indices.length_start])?;
        if extension_type == ExtensionType::Uninitialized {
            return Ok(TlvDataInfo {
                extension_types,
                used_len: tlv_indices.type_start,
            });
        } else {
            if tlv_data.len() < tlv_indices.value_start {
                // not enough bytes to store the length, malformed
                return Err(ProgramError::InvalidAccountData);
            }
            extension_types.push(extension_type);
            let length = pod_from_bytes::<Length>(
                &tlv_data[tlv_indices.length_start..tlv_indices.value_start],
            )?;

            let value_end_index = tlv_indices.value_start.saturating_add(usize::from(*length));
            if value_end_index > tlv_data.len() {
                // value blows past the size of the slice, malformed
                return Err(ProgramError::InvalidAccountData);
            }
            start_index = value_end_index;
        }
    }
    Ok(TlvDataInfo {
        extension_types,
        used_len: start_index,
    })
}

fn get_first_extension_type(tlv_data: &[u8]) -> Result<Option<ExtensionType>, ProgramError> {
    if tlv_data.is_empty() {
        Ok(None)
    } else {
        let tlv_indices = get_tlv_indices(0);
        if tlv_data.len() <= tlv_indices.length_start {
            return Ok(None);
        }
        let extension_type =
            ExtensionType::try_from(&tlv_data[tlv_indices.type_start..tlv_indices.length_start])?;
        if extension_type == ExtensionType::Uninitialized {
            Ok(None)
        } else {
            Ok(Some(extension_type))
        }
    }
}

fn check_min_len_and_not_multisig(input: &[u8], minimum_len: usize) -> Result<(), ProgramError> {
    if input.len() == Multisig::LEN || input.len() < minimum_len {
        Err(ProgramError::InvalidAccountData)
    } else {
        Ok(())
    }
}

fn check_account_type<S: BaseState>(account_type: AccountType) -> Result<(), ProgramError> {
    if account_type != S::ACCOUNT_TYPE {
        Err(ProgramError::InvalidAccountData)
    } else {
        Ok(())
    }
}

/// Any account with extensions must be at least `Account::LEN`.  Both mints and
/// accounts can have extensions
/// A mint with extensions that takes it past 165 could be indiscernible from an
/// Account with an extension, even if we add the account type. For example,
/// let's say we have:
///
/// Account: 165 bytes... + [2, 0, 3, 0, 100, ....]
///                          ^     ^       ^     ^
///                     acct type  extension length data...
///
/// Mint: 82 bytes... + 83 bytes of other extension data
///     + [2, 0, 3, 0, 100, ....]
///      (data in extension just happens to look like this)
///
/// With this approach, we only start writing the TLV data after Account::LEN,
/// which means we always know that the account type is going to be right after
/// that. We do a special case checking for a Multisig length, because those
/// aren't extensible under any circumstances.
const BASE_ACCOUNT_LENGTH: usize = Account::LEN;
/// Helper that tacks on the AccountType length, which gives the minimum for any
/// account with extensions
const BASE_ACCOUNT_AND_TYPE_LENGTH: usize = BASE_ACCOUNT_LENGTH + size_of::<AccountType>();

fn type_and_tlv_indices<S: BaseState>(
    rest_input: &[u8],
) -> Result<Option<(usize, usize)>, ProgramError> {
    if rest_input.is_empty() {
        Ok(None)
    } else {
        let account_type_index = BASE_ACCOUNT_LENGTH.saturating_sub(S::LEN);
        // check padding is all zeroes
        let tlv_start_index = account_type_index.saturating_add(size_of::<AccountType>());
        if rest_input.len() <= tlv_start_index {
            return Err(ProgramError::InvalidAccountData);
        }
        if rest_input[..account_type_index] != vec![0; account_type_index] {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(Some((account_type_index, tlv_start_index)))
        }
    }
}

/// Checks a base buffer to verify if it is an Account without having to
/// completely deserialize it
fn is_initialized_account(input: &[u8]) -> Result<bool, ProgramError> {
    const ACCOUNT_INITIALIZED_INDEX: usize = 108; // See state.rs#L99

    if input.len() != BASE_ACCOUNT_LENGTH {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(input[ACCOUNT_INITIALIZED_INDEX] != 0)
}

fn get_extension_bytes<S: BaseState, V: Extension>(tlv_data: &[u8]) -> Result<&[u8], ProgramError> {
    if V::TYPE.get_account_type() != S::ACCOUNT_TYPE {
        return Err(ProgramError::InvalidAccountData);
    }
    let TlvIndices {
        type_start: _,
        length_start,
        value_start,
    } = get_extension_indices::<V>(tlv_data, false)?;
    // get_extension_indices has checked that tlv_data is long enough to include
    // these indices
    let length = pod_from_bytes::<Length>(&tlv_data[length_start..value_start])?;
    let value_end = value_start.saturating_add(usize::from(*length));
    if tlv_data.len() < value_end {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(&tlv_data[value_start..value_end])
}

fn get_extension_bytes_mut<S: BaseState, V: Extension>(
    tlv_data: &mut [u8],
) -> Result<&mut [u8], ProgramError> {
    if V::TYPE.get_account_type() != S::ACCOUNT_TYPE {
        return Err(ProgramError::InvalidAccountData);
    }
    let TlvIndices {
        type_start: _,
        length_start,
        value_start,
    } = get_extension_indices::<V>(tlv_data, false)?;
    // get_extension_indices has checked that tlv_data is long enough to include
    // these indices
    let length = pod_from_bytes::<Length>(&tlv_data[length_start..value_start])?;
    let value_end = value_start.saturating_add(usize::from(*length));
    if tlv_data.len() < value_end {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(&mut tlv_data[value_start..value_end])
}

/// Calculate the new expected size if the state allocates the given number
/// of bytes for the given extension type.
///
/// Provides the correct answer regardless if the extension is already present
/// in the TLV data.
fn try_get_new_account_len_for_extension_len<S: BaseState, V: Extension>(
    tlv_data: &[u8],
    new_extension_len: usize,
) -> Result<usize, ProgramError> {
    // get the new length used by the extension
    let new_extension_tlv_len = add_type_and_length_to_len(new_extension_len);
    let tlv_info = get_tlv_data_info(tlv_data)?;
    // If we're adding an extension, then we must have at least BASE_ACCOUNT_LENGTH
    // and account type
    let current_len = tlv_info
        .used_len
        .saturating_add(BASE_ACCOUNT_AND_TYPE_LENGTH);
    // get the current length used by the extension
    let current_extension_len = get_extension_bytes::<S, V>(tlv_data)
        .map(|x| add_type_and_length_to_len(x.len()))
        .unwrap_or(0);
    let new_len = current_len
        .saturating_sub(current_extension_len)
        .saturating_add(new_extension_tlv_len);
    Ok(adjust_len_for_multisig(new_len))
}

/// Trait for base state with extension
pub trait BaseStateWithExtensions<S: BaseState> {
    /// Get the buffer containing all extension data
    fn get_tlv_data(&self) -> &[u8];

    /// Fetch the bytes for a TLV entry
    fn get_extension_bytes<V: Extension>(&self) -> Result<&[u8], ProgramError> {
        get_extension_bytes::<S, V>(self.get_tlv_data())
    }

    /// Unpack a portion of the TLV data as the desired type
    fn get_extension<V: Extension + Pod>(&self) -> Result<&V, ProgramError> {
        pod_from_bytes::<V>(self.get_extension_bytes::<V>()?)
    }

    /// Unpacks a portion of the TLV data as the desired variable-length type
    fn get_variable_len_extension<V: Extension + VariableLenPack>(
        &self,
    ) -> Result<V, ProgramError> {
        let data = get_extension_bytes::<S, V>(self.get_tlv_data())?;
        V::unpack_from_slice(data)
    }

    /// Iterates through the TLV entries, returning only the types
    fn get_extension_types(&self) -> Result<Vec<ExtensionType>, ProgramError> {
        get_tlv_data_info(self.get_tlv_data()).map(|x| x.extension_types)
    }

    /// Get just the first extension type, useful to track mixed initializations
    fn get_first_extension_type(&self) -> Result<Option<ExtensionType>, ProgramError> {
        get_first_extension_type(self.get_tlv_data())
    }

    /// Get the total number of bytes used by TLV entries and the base type
    fn try_get_account_len(&self) -> Result<usize, ProgramError> {
        let tlv_info = get_tlv_data_info(self.get_tlv_data())?;
        if tlv_info.extension_types.is_empty() {
            Ok(S::LEN)
        } else {
            let total_len = tlv_info
                .used_len
                .saturating_add(BASE_ACCOUNT_AND_TYPE_LENGTH);
            Ok(adjust_len_for_multisig(total_len))
        }
    }
    /// Calculate the new expected size if the state allocates the given
    /// fixed-length extension instance.
    /// If the state already has the extension, the resulting account length
    /// will be unchanged.
    fn try_get_new_account_len<V: Extension + Pod>(&self) -> Result<usize, ProgramError> {
        try_get_new_account_len_for_extension_len::<S, V>(
            self.get_tlv_data(),
            pod_get_packed_len::<V>(),
        )
    }

    /// Calculate the new expected size if the state allocates the given
    /// variable-length extension instance.
    fn try_get_new_account_len_for_variable_len_extension<V: Extension + VariableLenPack>(
        &self,
        new_extension: &V,
    ) -> Result<usize, ProgramError> {
        try_get_new_account_len_for_extension_len::<S, V>(
            self.get_tlv_data(),
            new_extension.get_packed_len()?,
        )
    }
}

/// Encapsulates owned immutable base state data (mint or account) with possible
/// extensions
#[derive(Clone, Debug, PartialEq)]
pub struct StateWithExtensionsOwned<S: BaseState> {
    /// Unpacked base data
    pub base: S,
    /// Raw TLV data, deserialized on demand
    tlv_data: Vec<u8>,
}
impl<S: BaseState> StateWithExtensionsOwned<S> {
    /// Unpack base state, leaving the extension data as a slice
    ///
    /// Fails if the base state is not initialized.
    pub fn unpack(mut input: Vec<u8>) -> Result<Self, ProgramError> {
        check_min_len_and_not_multisig(&input, S::LEN)?;
        let mut rest = input.split_off(S::LEN);
        let base = S::unpack(&input)?;
        if let Some((account_type_index, tlv_start_index)) = type_and_tlv_indices::<S>(&rest)? {
            // type_and_tlv_indices() checks that returned indexes are within range
            let account_type = AccountType::try_from(rest[account_type_index])
                .map_err(|_| ProgramError::InvalidAccountData)?;
            check_account_type::<S>(account_type)?;
            let tlv_data = rest.split_off(tlv_start_index);
            Ok(Self { base, tlv_data })
        } else {
            Ok(Self {
                base,
                tlv_data: vec![],
            })
        }
    }
}

impl<S: BaseState> BaseStateWithExtensions<S> for StateWithExtensionsOwned<S> {
    fn get_tlv_data(&self) -> &[u8] {
        &self.tlv_data
    }
}

/// Encapsulates immutable base state data (mint or account) with possible
/// extensions
#[derive(Debug, PartialEq)]
pub struct StateWithExtensions<'data, S: BaseState> {
    /// Unpacked base data
    pub base: S,
    /// Slice of data containing all TLV data, deserialized on demand
    tlv_data: &'data [u8],
}
impl<'data, S: BaseState> StateWithExtensions<'data, S> {
    /// Unpack base state, leaving the extension data as a slice
    ///
    /// Fails if the base state is not initialized.
    pub fn unpack(input: &'data [u8]) -> Result<Self, ProgramError> {
        check_min_len_and_not_multisig(input, S::LEN)?;
        let (base_data, rest) = input.split_at(S::LEN);
        let base = S::unpack(base_data)?;
        if let Some((account_type_index, tlv_start_index)) = type_and_tlv_indices::<S>(rest)? {
            // type_and_tlv_indices() checks that returned indexes are within range
            let account_type = AccountType::try_from(rest[account_type_index])
                .map_err(|_| ProgramError::InvalidAccountData)?;
            check_account_type::<S>(account_type)?;
            Ok(Self {
                base,
                tlv_data: &rest[tlv_start_index..],
            })
        } else {
            Ok(Self {
                base,
                tlv_data: &[],
            })
        }
    }
}
impl<'a, S: BaseState> BaseStateWithExtensions<S> for StateWithExtensions<'a, S> {
    fn get_tlv_data(&self) -> &[u8] {
        self.tlv_data
    }
}

/// Encapsulates mutable base state data (mint or account) with possible
/// extensions
#[derive(Debug, PartialEq)]
pub struct StateWithExtensionsMut<'data, S: BaseState> {
    /// Unpacked base data
    pub base: S,
    /// Raw base data
    base_data: &'data mut [u8],
    /// Writable account type
    account_type: &'data mut [u8],
    /// Slice of data containing all TLV data, deserialized on demand
    tlv_data: &'data mut [u8],
}
impl<'data, S: BaseState> StateWithExtensionsMut<'data, S> {
    /// Unpack base state, leaving the extension data as a mutable slice
    ///
    /// Fails if the base state is not initialized.
    pub fn unpack(input: &'data mut [u8]) -> Result<Self, ProgramError> {
        check_min_len_and_not_multisig(input, S::LEN)?;
        let (base_data, rest) = input.split_at_mut(S::LEN);
        let base = S::unpack(base_data)?;
        if let Some((account_type_index, tlv_start_index)) = type_and_tlv_indices::<S>(rest)? {
            // type_and_tlv_indices() checks that returned indexes are within range
            let account_type = AccountType::try_from(rest[account_type_index])
                .map_err(|_| ProgramError::InvalidAccountData)?;
            check_account_type::<S>(account_type)?;
            let (account_type, tlv_data) = rest.split_at_mut(tlv_start_index);
            Ok(Self {
                base,
                base_data,
                account_type: &mut account_type[account_type_index..tlv_start_index],
                tlv_data,
            })
        } else {
            Ok(Self {
                base,
                base_data,
                account_type: &mut [],
                tlv_data: &mut [],
            })
        }
    }

    /// Unpack an uninitialized base state, leaving the extension data as a
    /// mutable slice
    ///
    /// Fails if the base state has already been initialized.
    pub fn unpack_uninitialized(input: &'data mut [u8]) -> Result<Self, ProgramError> {
        check_min_len_and_not_multisig(input, S::LEN)?;
        let (base_data, rest) = input.split_at_mut(S::LEN);
        let base = S::unpack_unchecked(base_data)?;
        if base.is_initialized() {
            return Err(TokenError::AlreadyInUse.into());
        }
        if let Some((account_type_index, tlv_start_index)) = type_and_tlv_indices::<S>(rest)? {
            // type_and_tlv_indices() checks that returned indexes are within range
            let account_type = AccountType::try_from(rest[account_type_index])
                .map_err(|_| ProgramError::InvalidAccountData)?;
            if account_type != AccountType::Uninitialized {
                return Err(ProgramError::InvalidAccountData);
            }
            let (account_type, tlv_data) = rest.split_at_mut(tlv_start_index);
            let state = Self {
                base,
                base_data,
                account_type: &mut account_type[account_type_index..tlv_start_index],
                tlv_data,
            };
            if let Some(extension_type) = state.get_first_extension_type()? {
                let account_type = extension_type.get_account_type();
                if account_type != S::ACCOUNT_TYPE {
                    return Err(TokenError::ExtensionBaseMismatch.into());
                }
            }
            Ok(state)
        } else {
            Ok(Self {
                base,
                base_data,
                account_type: &mut [],
                tlv_data: &mut [],
            })
        }
    }

    /// Unpack a portion of the TLV data as the base mutable bytes
    pub fn get_extension_bytes_mut<V: Extension>(&mut self) -> Result<&mut [u8], ProgramError> {
        get_extension_bytes_mut::<S, V>(self.tlv_data)
    }

    /// Unpack a portion of the TLV data as the desired type that allows
    /// modifying the type
    pub fn get_extension_mut<V: Extension + Pod>(&mut self) -> Result<&mut V, ProgramError> {
        pod_from_bytes_mut::<V>(self.get_extension_bytes_mut::<V>()?)
    }

    /// Packs a variable-length extension into its appropriate data segment.
    /// Fails if space hasn't already been allocated for the given extension
    pub fn pack_variable_len_extension<V: Extension + VariableLenPack>(
        &mut self,
        extension: &V,
    ) -> Result<(), ProgramError> {
        let data = self.get_extension_bytes_mut::<V>()?;
        // NOTE: Do *not* use `pack`, since the length check will cause
        // reallocations to smaller sizes to fail
        extension.pack_into_slice(data)
    }

    /// Packs base state data into the base data portion
    pub fn pack_base(&mut self) {
        S::pack_into_slice(&self.base, self.base_data);
    }

    /// Packs the default extension data into an open slot if not already found
    /// in the data buffer. If extension is already found in the buffer, it
    /// overwrites the existing extension with the default state if
    /// `overwrite` is set. If extension found, but `overwrite` is not set,
    /// it returns error.
    pub fn init_extension<V: Extension + Pod + Default>(
        &mut self,
        overwrite: bool,
    ) -> Result<&mut V, ProgramError> {
        let length = pod_get_packed_len::<V>();
        let buffer = self.alloc::<V>(length, overwrite)?;
        let extension_ref = pod_from_bytes_mut::<V>(buffer)?;
        *extension_ref = V::default();
        Ok(extension_ref)
    }

    /// Reallocate and overwite the TLV entry for the given variable-length
    /// extension.
    ///
    /// Returns an error if the extension is not present, or if there is not
    /// enough space in the buffer.
    pub fn realloc_variable_len_extension<V: Extension + VariableLenPack>(
        &mut self,
        new_extension: &V,
    ) -> Result<(), ProgramError> {
        let data = self.realloc::<V>(new_extension.get_packed_len()?)?;
        new_extension.pack_into_slice(data)
    }

    /// Reallocate the TLV entry for the given extension to the given number of
    /// bytes.
    ///
    /// If the new length is smaller, it will compact the rest of the buffer and
    /// zero out the difference at the end. If it's larger, it will move the
    /// rest of the buffer data and zero out the new data.
    ///
    /// Returns an error if the extension is not present, or if this is not
    /// enough space in the buffer.
    fn realloc<V: Extension + VariableLenPack>(
        &mut self,
        length: usize,
    ) -> Result<&mut [u8], ProgramError> {
        let TlvIndices {
            type_start: _,
            length_start,
            value_start,
        } = get_extension_indices::<V>(self.tlv_data, false)?;
        let tlv_len = get_tlv_data_info(self.tlv_data).map(|x| x.used_len)?;
        let data_len = self.tlv_data.len();

        let length_ref =
            pod_from_bytes_mut::<Length>(&mut self.tlv_data[length_start..value_start])?;
        let old_length = usize::from(*length_ref);

        // Length check to avoid a panic later in `copy_within`
        if old_length < length {
            let new_tlv_len = tlv_len.saturating_add(length.saturating_sub(old_length));
            if new_tlv_len > data_len {
                return Err(ProgramError::InvalidAccountData);
            }
        }

        // write new length after the check, to avoid getting into a bad situation
        // if trying to recover from an error
        *length_ref = Length::try_from(length)?;

        let old_value_end = value_start.saturating_add(old_length);
        let new_value_end = value_start.saturating_add(length);
        self.tlv_data
            .copy_within(old_value_end..tlv_len, new_value_end);
        match old_length.cmp(&length) {
            Ordering::Greater => {
                // realloc to smaller, zero out the end
                let new_tlv_len = tlv_len.saturating_sub(old_length.saturating_sub(length));
                self.tlv_data[new_tlv_len..tlv_len].fill(0);
            }
            Ordering::Less => {
                // realloc to bigger, zero out the new bytes
                self.tlv_data[old_value_end..new_value_end].fill(0);
            }
            Ordering::Equal => {} // nothing needed!
        }

        Ok(&mut self.tlv_data[value_start..new_value_end])
    }

    /// Allocate the given number of bytes for the given variable-length
    /// extension and write its contents into the TLV buffer.
    ///
    /// This can only be used for variable-sized types, such as `String` or
    /// `Vec`. `Pod` types must use `init_extension`
    pub fn init_variable_len_extension<V: Extension + VariableLenPack>(
        &mut self,
        extension: &V,
        overwrite: bool,
    ) -> Result<(), ProgramError> {
        let data = self.alloc::<V>(extension.get_packed_len()?, overwrite)?;
        extension.pack_into_slice(data)
    }

    fn alloc<V: Extension>(
        &mut self,
        length: usize,
        overwrite: bool,
    ) -> Result<&mut [u8], ProgramError> {
        if V::TYPE.get_account_type() != S::ACCOUNT_TYPE {
            return Err(ProgramError::InvalidAccountData);
        }
        let TlvIndices {
            type_start,
            length_start,
            value_start,
        } = get_extension_indices::<V>(self.tlv_data, true)?;

        if self.tlv_data[type_start..].len() < add_type_and_length_to_len(length) {
            return Err(ProgramError::InvalidAccountData);
        }
        let extension_type = ExtensionType::try_from(&self.tlv_data[type_start..length_start])?;

        if extension_type == ExtensionType::Uninitialized || overwrite {
            // write extension type
            let extension_type_array: [u8; 2] = V::TYPE.into();
            let extension_type_ref = &mut self.tlv_data[type_start..length_start];
            extension_type_ref.copy_from_slice(&extension_type_array);
            // write length
            let length_ref =
                pod_from_bytes_mut::<Length>(&mut self.tlv_data[length_start..value_start])?;

            // check that the length is the same if we're doing an alloc
            // with overwrite, otherwise a realloc should be done
            if overwrite && extension_type == V::TYPE && usize::from(*length_ref) != length {
                return Err(TokenError::InvalidLengthForAlloc.into());
            }

            *length_ref = Length::try_from(length)?;

            let value_end = value_start.saturating_add(length);
            Ok(&mut self.tlv_data[value_start..value_end])
        } else {
            // extension is already initialized, but no overwrite permission
            Err(TokenError::ExtensionAlreadyInitialized.into())
        }
    }

    /// If `extension_type` is an Account-associated ExtensionType that requires
    /// initialization on InitializeAccount, this method packs the default
    /// relevant Extension of an ExtensionType into an open slot if not
    /// already found in the data buffer, otherwise overwrites the
    /// existing extension with the default state. For all other ExtensionTypes,
    /// this is a no-op.
    pub fn init_account_extension_from_type(
        &mut self,
        extension_type: ExtensionType,
    ) -> Result<(), ProgramError> {
        if extension_type.get_account_type() != AccountType::Account {
            return Ok(());
        }
        match extension_type {
            ExtensionType::TransferFeeAmount => {
                self.init_extension::<TransferFeeAmount>(true).map(|_| ())
            }
            ExtensionType::ImmutableOwner => {
                self.init_extension::<ImmutableOwner>(true).map(|_| ())
            }
            ExtensionType::NonTransferableAccount => self
                .init_extension::<NonTransferableAccount>(true)
                .map(|_| ()),
            ExtensionType::TransferHookAccount => {
                self.init_extension::<TransferHookAccount>(true).map(|_| ())
            }
            // ConfidentialTransfers are currently opt-in only, so this is a no-op for extra safety
            // on InitializeAccount
            ExtensionType::ConfidentialTransferAccount => Ok(()),
            #[cfg(test)]
            ExtensionType::AccountPaddingTest => {
                self.init_extension::<AccountPaddingTest>(true).map(|_| ())
            }
            _ => unreachable!(),
        }
    }

    /// Write the account type into the buffer, done during the base
    /// state initialization
    /// Noops if there is no room for an extension in the account, needed for
    /// pure base mints / accounts.
    pub fn init_account_type(&mut self) -> Result<(), ProgramError> {
        if !self.account_type.is_empty() {
            if let Some(extension_type) = self.get_first_extension_type()? {
                let account_type = extension_type.get_account_type();
                if account_type != S::ACCOUNT_TYPE {
                    return Err(TokenError::ExtensionBaseMismatch.into());
                }
            }
            self.account_type[0] = S::ACCOUNT_TYPE.into();
        }
        Ok(())
    }
}
impl<'a, S: BaseState> BaseStateWithExtensions<S> for StateWithExtensionsMut<'a, S> {
    fn get_tlv_data(&self) -> &[u8] {
        self.tlv_data
    }
}

/// If AccountType is uninitialized, set it to the BaseState's ACCOUNT_TYPE;
/// if AccountType is already set, check is set correctly for BaseState
/// This method assumes that the `base_data` has already been packed with data
/// of the desired type.
pub fn set_account_type<S: BaseState>(input: &mut [u8]) -> Result<(), ProgramError> {
    check_min_len_and_not_multisig(input, S::LEN)?;
    let (base_data, rest) = input.split_at_mut(S::LEN);
    if S::ACCOUNT_TYPE == AccountType::Account && !is_initialized_account(base_data)? {
        return Err(ProgramError::InvalidAccountData);
    }
    if let Some((account_type_index, _tlv_start_index)) = type_and_tlv_indices::<S>(rest)? {
        let mut account_type = AccountType::try_from(rest[account_type_index])
            .map_err(|_| ProgramError::InvalidAccountData)?;
        if account_type == AccountType::Uninitialized {
            rest[account_type_index] = S::ACCOUNT_TYPE.into();
            account_type = S::ACCOUNT_TYPE;
        }
        check_account_type::<S>(account_type)?;
        Ok(())
    } else {
        Err(ProgramError::InvalidAccountData)
    }
}

/// Different kinds of accounts. Note that `Mint`, `Account`, and `Multisig`
/// types are determined exclusively by the size of the account, and are not
/// included in the account data. `AccountType` is only included if extensions
/// have been initialized.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, TryFromPrimitive, IntoPrimitive)]
pub enum AccountType {
    /// Marker for 0 data
    Uninitialized,
    /// Mint account with additional extensions
    Mint,
    /// Token holding account with additional extensions
    Account,
}
impl Default for AccountType {
    fn default() -> Self {
        Self::Uninitialized
    }
}

/// Extensions that can be applied to mints or accounts.  Mint extensions must
/// only be applied to mint accounts, and account extensions must only be
/// applied to token holding accounts.
#[repr(u16)]
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, TryFromPrimitive, IntoPrimitive)]
pub enum ExtensionType {
    /// Used as padding if the account size would otherwise be 355, same as a
    /// multisig
    Uninitialized,
    /// Includes transfer fee rate info and accompanying authorities to withdraw
    /// and set the fee
    TransferFeeConfig,
    /// Includes withheld transfer fees
    TransferFeeAmount,
    /// Includes an optional mint close authority
    MintCloseAuthority,
    /// Auditor configuration for confidential transfers
    ConfidentialTransferMint,
    /// State for confidential transfers
    ConfidentialTransferAccount,
    /// Specifies the default Account::state for new Accounts
    DefaultAccountState,
    /// Indicates that the Account owner authority cannot be changed
    ImmutableOwner,
    /// Require inbound transfers to have memo
    MemoTransfer,
    /// Indicates that the tokens from this mint can't be transfered
    NonTransferable,
    /// Tokens accrue interest over time,
    InterestBearingConfig,
    /// Locks privileged token operations from happening via CPI
    CpiGuard,
    /// Includes an optional permanent delegate
    PermanentDelegate,
    /// Indicates that the tokens in this account belong to a non-transferable
    /// mint
    NonTransferableAccount,
    /// Mint requires a CPI to a program implementing the "transfer hook"
    /// interface
    TransferHook,
    /// Indicates that the tokens in this account belong to a mint with a
    /// transfer hook
    TransferHookAccount,
    /// Includes encrypted withheld fees and the encryption public that they are
    /// encrypted under
    ConfidentialTransferFeeConfig,
    /// Includes confidential withheld transfer fees
    ConfidentialTransferFeeAmount,
    /// Mint contains a pointer to another account (or the same account) that
    /// holds metadata
    MetadataPointer,
    /// Mint contains token-metadata
    TokenMetadata,
    /// Mint contains a pointer to another account (or the same account) that
    /// holds group configurations
    GroupPointer,
    /// Mint contains token group configurations
    TokenGroup,
    /// Mint contains a pointer to another account (or the same account) that
    /// holds group member configurations
    GroupMemberPointer,
    /// Mint contains token group member configurations
    TokenGroupMember,
    /// Test variable-length mint extension
    #[cfg(test)]
    VariableLenMintTest = u16::MAX - 2,
    /// Padding extension used to make an account exactly Multisig::LEN, used
    /// for testing
    #[cfg(test)]
    AccountPaddingTest,
    /// Padding extension used to make a mint exactly Multisig::LEN, used for
    /// testing
    #[cfg(test)]
    MintPaddingTest,
}
impl TryFrom<&[u8]> for ExtensionType {
    type Error = ProgramError;
    fn try_from(a: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from(u16::from_le_bytes(
            a.try_into().map_err(|_| ProgramError::InvalidAccountData)?,
        ))
        .map_err(|_| ProgramError::InvalidAccountData)
    }
}
impl From<ExtensionType> for [u8; 2] {
    fn from(a: ExtensionType) -> Self {
        u16::from(a).to_le_bytes()
    }
}
impl ExtensionType {
    /// Returns true if the given extension type is sized
    ///
    /// Most extension types should be sized, so any variable-length extension
    /// types should be added here by hand
    const fn sized(&self) -> bool {
        match self {
            ExtensionType::TokenMetadata => false,
            #[cfg(test)]
            ExtensionType::VariableLenMintTest => false,
            _ => true,
        }
    }

    /// Get the data length of the type associated with the enum
    ///
    /// Fails if the extension type has a variable length
    fn try_get_type_len(&self) -> Result<usize, ProgramError> {
        if !self.sized() {
            return Err(ProgramError::InvalidArgument);
        }
        Ok(match self {
            ExtensionType::Uninitialized => 0,
            ExtensionType::TransferFeeConfig => pod_get_packed_len::<TransferFeeConfig>(),
            ExtensionType::TransferFeeAmount => pod_get_packed_len::<TransferFeeAmount>(),
            ExtensionType::MintCloseAuthority => pod_get_packed_len::<MintCloseAuthority>(),
            ExtensionType::ImmutableOwner => pod_get_packed_len::<ImmutableOwner>(),
            ExtensionType::ConfidentialTransferMint => {
                pod_get_packed_len::<ConfidentialTransferMint>()
            }
            ExtensionType::ConfidentialTransferAccount => {
                pod_get_packed_len::<ConfidentialTransferAccount>()
            }
            ExtensionType::DefaultAccountState => pod_get_packed_len::<DefaultAccountState>(),
            ExtensionType::MemoTransfer => pod_get_packed_len::<MemoTransfer>(),
            ExtensionType::NonTransferable => pod_get_packed_len::<NonTransferable>(),
            ExtensionType::InterestBearingConfig => pod_get_packed_len::<InterestBearingConfig>(),
            ExtensionType::CpiGuard => pod_get_packed_len::<CpiGuard>(),
            ExtensionType::PermanentDelegate => pod_get_packed_len::<PermanentDelegate>(),
            ExtensionType::NonTransferableAccount => pod_get_packed_len::<NonTransferableAccount>(),
            ExtensionType::TransferHook => pod_get_packed_len::<TransferHook>(),
            ExtensionType::TransferHookAccount => pod_get_packed_len::<TransferHookAccount>(),
            ExtensionType::ConfidentialTransferFeeConfig => {
                pod_get_packed_len::<ConfidentialTransferFeeConfig>()
            }
            ExtensionType::ConfidentialTransferFeeAmount => {
                pod_get_packed_len::<ConfidentialTransferFeeAmount>()
            }
            ExtensionType::MetadataPointer => pod_get_packed_len::<MetadataPointer>(),
            ExtensionType::TokenMetadata => unreachable!(),
            ExtensionType::GroupPointer => pod_get_packed_len::<GroupPointer>(),
            ExtensionType::TokenGroup => pod_get_packed_len::<TokenGroup>(),
            ExtensionType::GroupMemberPointer => pod_get_packed_len::<GroupMemberPointer>(),
            ExtensionType::TokenGroupMember => pod_get_packed_len::<TokenGroupMember>(),
            #[cfg(test)]
            ExtensionType::AccountPaddingTest => pod_get_packed_len::<AccountPaddingTest>(),
            #[cfg(test)]
            ExtensionType::MintPaddingTest => pod_get_packed_len::<MintPaddingTest>(),
            #[cfg(test)]
            ExtensionType::VariableLenMintTest => unreachable!(),
        })
    }

    /// Get the TLV length for an ExtensionType
    ///
    /// Fails if the extension type has a variable length
    fn try_get_tlv_len(&self) -> Result<usize, ProgramError> {
        Ok(add_type_and_length_to_len(self.try_get_type_len()?))
    }

    /// Get the TLV length for a set of ExtensionTypes
    ///
    /// Fails if any of the extension types has a variable length
    fn try_get_total_tlv_len(extension_types: &[Self]) -> Result<usize, ProgramError> {
        // dedupe extensions
        let mut extensions = vec![];
        for extension_type in extension_types {
            if !extensions.contains(&extension_type) {
                extensions.push(extension_type);
            }
        }
        extensions.iter().map(|e| e.try_get_tlv_len()).sum()
    }

    /// Get the required account data length for the given ExtensionTypes
    ///
    /// Fails if any of the extension types has a variable length
    pub fn try_calculate_account_len<S: BaseState>(
        extension_types: &[Self],
    ) -> Result<usize, ProgramError> {
        if extension_types.is_empty() {
            Ok(S::LEN)
        } else {
            let extension_size = Self::try_get_total_tlv_len(extension_types)?;
            let total_len = extension_size.saturating_add(BASE_ACCOUNT_AND_TYPE_LENGTH);
            Ok(adjust_len_for_multisig(total_len))
        }
    }

    /// Get the associated account type
    pub fn get_account_type(&self) -> AccountType {
        match self {
            ExtensionType::Uninitialized => AccountType::Uninitialized,
            ExtensionType::TransferFeeConfig
            | ExtensionType::MintCloseAuthority
            | ExtensionType::ConfidentialTransferMint
            | ExtensionType::DefaultAccountState
            | ExtensionType::NonTransferable
            | ExtensionType::InterestBearingConfig
            | ExtensionType::PermanentDelegate
            | ExtensionType::TransferHook
            | ExtensionType::ConfidentialTransferFeeConfig
            | ExtensionType::MetadataPointer
            | ExtensionType::TokenMetadata
            | ExtensionType::GroupPointer
            | ExtensionType::TokenGroup
            | ExtensionType::GroupMemberPointer
            | ExtensionType::TokenGroupMember => AccountType::Mint,
            ExtensionType::ImmutableOwner
            | ExtensionType::TransferFeeAmount
            | ExtensionType::ConfidentialTransferAccount
            | ExtensionType::MemoTransfer
            | ExtensionType::NonTransferableAccount
            | ExtensionType::TransferHookAccount
            | ExtensionType::CpiGuard
            | ExtensionType::ConfidentialTransferFeeAmount => AccountType::Account,
            #[cfg(test)]
            ExtensionType::VariableLenMintTest => AccountType::Mint,
            #[cfg(test)]
            ExtensionType::AccountPaddingTest => AccountType::Account,
            #[cfg(test)]
            ExtensionType::MintPaddingTest => AccountType::Mint,
        }
    }

    /// Based on a set of AccountType::Mint ExtensionTypes, get the list of
    /// AccountType::Account ExtensionTypes required on InitializeAccount
    pub fn get_required_init_account_extensions(mint_extension_types: &[Self]) -> Vec<Self> {
        let mut account_extension_types = vec![];
        for extension_type in mint_extension_types {
            match extension_type {
                ExtensionType::TransferFeeConfig => {
                    account_extension_types.push(ExtensionType::TransferFeeAmount);
                }
                ExtensionType::NonTransferable => {
                    account_extension_types.push(ExtensionType::NonTransferableAccount);
                    account_extension_types.push(ExtensionType::ImmutableOwner);
                }
                ExtensionType::TransferHook => {
                    account_extension_types.push(ExtensionType::TransferHookAccount);
                }
                #[cfg(test)]
                ExtensionType::MintPaddingTest => {
                    account_extension_types.push(ExtensionType::AccountPaddingTest);
                }
                _ => {}
            }
        }
        account_extension_types
    }

    /// Check for invalid combination of mint extensions
    pub fn check_for_invalid_mint_extension_combinations(
        mint_extension_types: &[Self],
    ) -> Result<(), TokenError> {
        let mut transfer_fee_config = false;
        let mut confidential_transfer_mint = false;
        let mut confidential_transfer_fee_config = false;

        for extension_type in mint_extension_types {
            match extension_type {
                ExtensionType::TransferFeeConfig => transfer_fee_config = true,
                ExtensionType::ConfidentialTransferMint => confidential_transfer_mint = true,
                ExtensionType::ConfidentialTransferFeeConfig => {
                    confidential_transfer_fee_config = true
                }
                _ => (),
            }
        }

        if confidential_transfer_fee_config && !(transfer_fee_config && confidential_transfer_mint)
        {
            return Err(TokenError::InvalidExtensionCombination);
        }

        if transfer_fee_config && confidential_transfer_mint && !confidential_transfer_fee_config {
            return Err(TokenError::InvalidExtensionCombination);
        }

        Ok(())
    }
}

/// Trait for base states, specifying the associated enum
pub trait BaseState: Pack + IsInitialized {
    /// Associated extension type enum, checked at the start of TLV entries
    const ACCOUNT_TYPE: AccountType;
}
impl BaseState for Account {
    const ACCOUNT_TYPE: AccountType = AccountType::Account;
}
impl BaseState for Mint {
    const ACCOUNT_TYPE: AccountType = AccountType::Mint;
}

/// Trait to be implemented by all extension states, specifying which extension
/// and account type they are associated with
pub trait Extension {
    /// Associated extension type enum, checked at the start of TLV entries
    const TYPE: ExtensionType;
}

/// Padding a mint account to be exactly Multisig::LEN.
/// We need to pad 185 bytes, since Multisig::LEN = 355, Account::LEN = 165,
/// size_of AccountType = 1, size_of ExtensionType = 2, size_of Length = 2.
/// 355 - 165 - 1 - 2 - 2 = 185
#[cfg(test)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct MintPaddingTest {
    /// Largest value under 185 that implements Pod
    pub padding1: [u8; 128],
    /// Largest value under 57 that implements Pod
    pub padding2: [u8; 48],
    /// Exact value needed to finish the padding
    pub padding3: [u8; 9],
}
#[cfg(test)]
impl Extension for MintPaddingTest {
    const TYPE: ExtensionType = ExtensionType::MintPaddingTest;
}
#[cfg(test)]
impl Default for MintPaddingTest {
    fn default() -> Self {
        Self {
            padding1: [1; 128],
            padding2: [2; 48],
            padding3: [3; 9],
        }
    }
}
/// Account version of the MintPadding
#[cfg(test)]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct AccountPaddingTest(MintPaddingTest);
#[cfg(test)]
impl Extension for AccountPaddingTest {
    const TYPE: ExtensionType = ExtensionType::AccountPaddingTest;
}

/// Packs a fixed-length extension into a TLV space
///
/// This function reallocates the account as needed to accommodate for the
/// change in space.
///
/// If the extension already exists, it will overwrite the existing extension
/// if `overwrite` is `true`, otherwise it will return an error.
///
/// If the extension does not exist, it will reallocate the account and write
/// the extension into the TLV buffer.
///
/// NOTE: Since this function deals with fixed-size extensions, it does not
/// handle _decreasing_ the size of an account's data buffer, like the function
/// `alloc_and_serialize_variable_len_extension` does.
pub fn alloc_and_serialize<S: BaseState, V: Default + Extension + Pod>(
    account_info: &AccountInfo,
    new_extension: &V,
    overwrite: bool,
) -> Result<(), ProgramError> {
    let previous_account_len = account_info.try_data_len()?;
    let new_account_len = {
        let data = account_info.try_borrow_data()?;
        let state = StateWithExtensions::<S>::unpack(&data)?;
        state.try_get_new_account_len::<V>()?
    };

    // Realloc the account first, if needed
    if new_account_len > previous_account_len {
        account_info.realloc(new_account_len, false)?;
    }
    let mut buffer = account_info.try_borrow_mut_data()?;
    if previous_account_len <= BASE_ACCOUNT_LENGTH {
        set_account_type::<S>(*buffer)?;
    }
    let mut state = StateWithExtensionsMut::<S>::unpack(&mut buffer)?;

    // Write the extension
    let extension = state.init_extension::<V>(overwrite)?;
    *extension = *new_extension;

    Ok(())
}

/// Packs a variable-length extension into a TLV space
///
/// This function reallocates the account as needed to accommodate for the
/// change in space, then reallocates in the TLV buffer, and finally writes the
/// bytes.
///
/// NOTE: Unlike the `reallocate` instruction, this function will reduce the
/// size of an account if it has too many bytes allocated for the given value.
pub fn alloc_and_serialize_variable_len_extension<S: BaseState, V: Extension + VariableLenPack>(
    account_info: &AccountInfo,
    new_extension: &V,
    overwrite: bool,
) -> Result<(), ProgramError> {
    let previous_account_len = account_info.try_data_len()?;
    let (new_account_len, extension_already_exists) = {
        let data = account_info.try_borrow_data()?;
        let state = StateWithExtensions::<S>::unpack(&data)?;
        let new_account_len =
            state.try_get_new_account_len_for_variable_len_extension(new_extension)?;
        let extension_already_exists = state.get_extension_bytes::<V>().is_ok();
        (new_account_len, extension_already_exists)
    };

    if extension_already_exists && !overwrite {
        return Err(TokenError::ExtensionAlreadyInitialized.into());
    }

    if previous_account_len < new_account_len {
        // account size increased, so realloc the account, then the TLV entry, then
        // write data
        account_info.realloc(new_account_len, false)?;
        let mut buffer = account_info.try_borrow_mut_data()?;
        if extension_already_exists {
            let mut state = StateWithExtensionsMut::<S>::unpack(&mut buffer)?;
            state.realloc_variable_len_extension(new_extension)?;
        } else {
            if previous_account_len <= BASE_ACCOUNT_LENGTH {
                set_account_type::<S>(*buffer)?;
            }
            // now alloc in the TLV buffer and write the data
            let mut state = StateWithExtensionsMut::<S>::unpack(&mut buffer)?;
            state.init_variable_len_extension(new_extension, false)?;
        }
    } else {
        // do it backwards otherwise, write the state, realloc TLV, then the account
        let mut buffer = account_info.try_borrow_mut_data()?;
        let mut state = StateWithExtensionsMut::<S>::unpack(&mut buffer)?;
        if extension_already_exists {
            state.realloc_variable_len_extension(new_extension)?;
        } else {
            // this situation can happen if we have an overallocated buffer
            state.init_variable_len_extension(new_extension, false)?;
        }

        let removed_bytes = previous_account_len
            .checked_sub(new_account_len)
            .ok_or(ProgramError::AccountDataTooSmall)?;
        if removed_bytes > 0 {
            // this is probably fine, but be safe and avoid invalidating references
            drop(buffer);
            account_info.realloc(new_account_len, false)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::state::test::{TEST_ACCOUNT, TEST_ACCOUNT_SLICE, TEST_MINT, TEST_MINT_SLICE},
        bytemuck::Pod,
        solana_program::{
            account_info::{Account as GetAccount, IntoAccountInfo},
            clock::Epoch,
            entrypoint::MAX_PERMITTED_DATA_INCREASE,
            pubkey::Pubkey,
        },
        spl_pod::{
            bytemuck::pod_bytes_of, optional_keys::OptionalNonZeroPubkey, primitives::PodU64,
        },
        transfer_fee::test::test_transfer_fee_config,
    };

    /// Test fixed-length struct
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
    struct FixedLenMintTest {
        data: [u8; 8],
    }
    impl Extension for FixedLenMintTest {
        const TYPE: ExtensionType = ExtensionType::MintPaddingTest;
    }

    /// Test variable-length struct
    #[derive(Clone, Debug, PartialEq)]
    struct VariableLenMintTest {
        data: Vec<u8>,
    }
    impl Extension for VariableLenMintTest {
        const TYPE: ExtensionType = ExtensionType::VariableLenMintTest;
    }
    impl VariableLenPack for VariableLenMintTest {
        fn pack_into_slice(&self, dst: &mut [u8]) -> Result<(), ProgramError> {
            let data_start = size_of::<u64>();
            let end = data_start + self.data.len();
            if dst.len() < end {
                Err(ProgramError::InvalidAccountData)
            } else {
                dst[..data_start].copy_from_slice(&self.data.len().to_le_bytes());
                dst[data_start..end].copy_from_slice(&self.data);
                Ok(())
            }
        }
        fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
            let data_start = size_of::<u64>();
            let length = u64::from_le_bytes(src[..data_start].try_into().unwrap()) as usize;
            if src[data_start..data_start + length].len() != length {
                return Err(ProgramError::InvalidAccountData);
            }
            let data = Vec::from(&src[data_start..data_start + length]);
            Ok(Self { data })
        }
        fn get_packed_len(&self) -> Result<usize, ProgramError> {
            Ok(size_of::<u64>().saturating_add(self.data.len()))
        }
    }

    const MINT_WITH_EXTENSION: &[u8] = &[
        1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 42, 0, 0, 0, 0, 0, 0, 0, 7, 1, 1, 0, 0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // base mint
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // padding
        1, // account type
        3, 0, // extension type
        32, 0, // length
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, // data
    ];

    #[test]
    fn unpack_opaque_buffer() {
        let state = StateWithExtensions::<Mint>::unpack(MINT_WITH_EXTENSION).unwrap();
        assert_eq!(state.base, TEST_MINT);
        let extension = state.get_extension::<MintCloseAuthority>().unwrap();
        let close_authority =
            OptionalNonZeroPubkey::try_from(Some(Pubkey::new_from_array([1; 32]))).unwrap();
        assert_eq!(extension.close_authority, close_authority);
        assert_eq!(
            state.get_extension::<TransferFeeConfig>(),
            Err(ProgramError::InvalidAccountData)
        );
        assert_eq!(
            StateWithExtensions::<Account>::unpack(MINT_WITH_EXTENSION),
            Err(ProgramError::InvalidAccountData)
        );

        let state = StateWithExtensions::<Mint>::unpack(TEST_MINT_SLICE).unwrap();
        assert_eq!(state.base, TEST_MINT);

        let mut test_mint = TEST_MINT_SLICE.to_vec();
        let state = StateWithExtensionsMut::<Mint>::unpack(&mut test_mint).unwrap();
        assert_eq!(state.base, TEST_MINT);
    }

    #[test]
    fn fail_unpack_opaque_buffer() {
        // input buffer too small
        let mut buffer = vec![0, 3];
        assert_eq!(
            StateWithExtensions::<Mint>::unpack(&buffer),
            Err(ProgramError::InvalidAccountData)
        );
        assert_eq!(
            StateWithExtensionsMut::<Mint>::unpack(&mut buffer),
            Err(ProgramError::InvalidAccountData)
        );
        assert_eq!(
            StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer),
            Err(ProgramError::InvalidAccountData)
        );

        // tweak the account type
        let mut buffer = MINT_WITH_EXTENSION.to_vec();
        buffer[BASE_ACCOUNT_LENGTH] = 3;
        assert_eq!(
            StateWithExtensions::<Mint>::unpack(&buffer),
            Err(ProgramError::InvalidAccountData)
        );

        // clear the mint initialized byte
        let mut buffer = MINT_WITH_EXTENSION.to_vec();
        buffer[45] = 0;
        assert_eq!(
            StateWithExtensions::<Mint>::unpack(&buffer),
            Err(ProgramError::UninitializedAccount)
        );

        // tweak the padding
        let mut buffer = MINT_WITH_EXTENSION.to_vec();
        buffer[Mint::LEN] = 100;
        assert_eq!(
            StateWithExtensions::<Mint>::unpack(&buffer),
            Err(ProgramError::InvalidAccountData)
        );

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
    }

    #[test]
    fn get_extension_types_with_opaque_buffer() {
        // incorrect due to the length
        assert_eq!(
            get_tlv_data_info(&[1, 0, 1, 1]).unwrap_err(),
            ProgramError::InvalidAccountData,
        );
        // incorrect due to the huge enum number
        assert_eq!(
            get_tlv_data_info(&[0, 1, 0, 0]).unwrap_err(),
            ProgramError::InvalidAccountData,
        );
        // correct due to the good enum number and zero length
        assert_eq!(
            get_tlv_data_info(&[1, 0, 0, 0]).unwrap(),
            TlvDataInfo {
                extension_types: vec![ExtensionType::try_from(1).unwrap()],
                used_len: add_type_and_length_to_len(0),
            }
        );
        // correct since it's just uninitialized data at the end
        assert_eq!(
            get_tlv_data_info(&[0, 0]).unwrap(),
            TlvDataInfo {
                extension_types: vec![],
                used_len: 0
            }
        );
    }

    #[test]
    fn mint_with_extension_pack_unpack() {
        let mint_size = ExtensionType::try_calculate_account_len::<Mint>(&[
            ExtensionType::MintCloseAuthority,
            ExtensionType::TransferFeeConfig,
        ])
        .unwrap();
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
        let close_authority =
            OptionalNonZeroPubkey::try_from(Some(Pubkey::new_from_array([1; 32]))).unwrap();
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
            .extend_from_slice(&(pod_get_packed_len::<MintCloseAuthority>() as u16).to_le_bytes());
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
        let unpacked_extension = state.get_extension_mut::<MintCloseAuthority>().unwrap();
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
            .extend_from_slice(&(pod_get_packed_len::<MintCloseAuthority>() as u16).to_le_bytes());
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
            .extend_from_slice(&(pod_get_packed_len::<MintCloseAuthority>() as u16).to_le_bytes());
        expect.extend_from_slice(&[0; 32]); // data
        expect.extend_from_slice(&(ExtensionType::TransferFeeConfig as u16).to_le_bytes());
        expect.extend_from_slice(&(pod_get_packed_len::<TransferFeeConfig>() as u16).to_le_bytes());
        expect.extend_from_slice(pod_bytes_of(&mint_transfer_fee));
        assert_eq!(expect, buffer);

        // fail to init one more extension that does not fit
        let mut state = StateWithExtensionsMut::<Mint>::unpack(&mut buffer).unwrap();
        assert_eq!(
            state.init_extension::<MintPaddingTest>(true),
            Err(ProgramError::InvalidAccountData),
        );
    }

    #[test]
    fn mint_extension_any_order() {
        let mint_size = ExtensionType::try_calculate_account_len::<Mint>(&[
            ExtensionType::MintCloseAuthority,
            ExtensionType::TransferFeeConfig,
        ])
        .unwrap();
        let mut buffer = vec![0; mint_size];

        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        // write extensions
        let close_authority =
            OptionalNonZeroPubkey::try_from(Some(Pubkey::new_from_array([1; 32]))).unwrap();
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

        let close_authority =
            OptionalNonZeroPubkey::try_from(Some(Pubkey::new_from_array([1; 32]))).unwrap();
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
    }

    #[test]
    fn mint_with_multisig_len() {
        let mut buffer = vec![0; Multisig::LEN];
        assert_eq!(
            StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer),
            Err(ProgramError::InvalidAccountData),
        );
        let mint_size =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MintPaddingTest])
                .unwrap();
        assert_eq!(mint_size, Multisig::LEN + size_of::<ExtensionType>());
        let mut buffer = vec![0; mint_size];

        // write base mint
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        state.base = TEST_MINT;
        state.pack_base();
        state.init_account_type().unwrap();

        // write padding
        let extension = state.init_extension::<MintPaddingTest>(true).unwrap();
        extension.padding1 = [1; 128];
        extension.padding2 = [1; 48];
        extension.padding3 = [1; 9];

        assert_eq!(
            &state.get_extension_types().unwrap(),
            &[ExtensionType::MintPaddingTest]
        );

        // check raw buffer
        let mut expect = TEST_MINT_SLICE.to_vec();
        expect.extend_from_slice(&[0; BASE_ACCOUNT_LENGTH - Mint::LEN]); // padding
        expect.push(AccountType::Mint.into());
        expect.extend_from_slice(&(ExtensionType::MintPaddingTest as u16).to_le_bytes());
        expect.extend_from_slice(&(pod_get_packed_len::<MintPaddingTest>() as u16).to_le_bytes());
        expect.extend_from_slice(&vec![1; pod_get_packed_len::<MintPaddingTest>()]);
        expect.extend_from_slice(&(ExtensionType::Uninitialized as u16).to_le_bytes());
        assert_eq!(expect, buffer);
    }

    #[test]
    fn account_with_extension_pack_unpack() {
        let account_size = ExtensionType::try_calculate_account_len::<Account>(&[
            ExtensionType::TransferFeeAmount,
        ])
        .unwrap();
        let mut buffer = vec![0; account_size];

        // fail unpack
        assert_eq!(
            StateWithExtensionsMut::<Account>::unpack(&mut buffer),
            Err(ProgramError::UninitializedAccount),
        );

        let mut state =
            StateWithExtensionsMut::<Account>::unpack_uninitialized(&mut buffer).unwrap();
        // fail init mint extension
        assert_eq!(
            state.init_extension::<TransferFeeConfig>(true),
            Err(ProgramError::InvalidAccountData),
        );
        // success write extension
        let withheld_amount = PodU64::from(u64::MAX);
        let extension = state.init_extension::<TransferFeeAmount>(true).unwrap();
        extension.withheld_amount = withheld_amount;

        assert_eq!(
            &state.get_extension_types().unwrap(),
            &[ExtensionType::TransferFeeAmount]
        );

        // fail unpack again, still no base data
        assert_eq!(
            StateWithExtensionsMut::<Account>::unpack(&mut buffer.clone()),
            Err(ProgramError::UninitializedAccount),
        );

        // write base account
        let mut state =
            StateWithExtensionsMut::<Account>::unpack_uninitialized(&mut buffer).unwrap();
        state.base = TEST_ACCOUNT;
        state.pack_base();
        state.init_account_type().unwrap();
        let base = state.base;

        // check raw buffer
        let mut expect = TEST_ACCOUNT_SLICE.to_vec();
        expect.push(AccountType::Account.into());
        expect.extend_from_slice(&(ExtensionType::TransferFeeAmount as u16).to_le_bytes());
        expect.extend_from_slice(&(pod_get_packed_len::<TransferFeeAmount>() as u16).to_le_bytes());
        expect.extend_from_slice(&u64::from(withheld_amount).to_le_bytes());
        assert_eq!(expect, buffer);

        // check unpacking
        let mut state = StateWithExtensionsMut::<Account>::unpack(&mut buffer).unwrap();
        assert_eq!(state.base, base);
        assert_eq!(
            &state.get_extension_types().unwrap(),
            &[ExtensionType::TransferFeeAmount]
        );

        // update base
        state.base = TEST_ACCOUNT;
        state.base.amount += 100;
        state.pack_base();

        // check unpacking
        let unpacked_extension = state.get_extension_mut::<TransferFeeAmount>().unwrap();
        assert_eq!(*unpacked_extension, TransferFeeAmount { withheld_amount });

        // update extension
        let withheld_amount = PodU64::from(u32::MAX as u64);
        unpacked_extension.withheld_amount = withheld_amount;

        // check updates are propagated
        let base = state.base;
        let state = StateWithExtensions::<Account>::unpack(&buffer).unwrap();
        assert_eq!(state.base, base);
        let unpacked_extension = state.get_extension::<TransferFeeAmount>().unwrap();
        assert_eq!(*unpacked_extension, TransferFeeAmount { withheld_amount });

        // check raw buffer
        let mut expect = vec![0; Account::LEN];
        Account::pack_into_slice(&base, &mut expect);
        expect.push(AccountType::Account.into());
        expect.extend_from_slice(&(ExtensionType::TransferFeeAmount as u16).to_le_bytes());
        expect.extend_from_slice(&(pod_get_packed_len::<TransferFeeAmount>() as u16).to_le_bytes());
        expect.extend_from_slice(&u64::from(withheld_amount).to_le_bytes());
        assert_eq!(expect, buffer);

        // fail unpack as a mint
        assert_eq!(
            StateWithExtensions::<Mint>::unpack(&buffer),
            Err(ProgramError::InvalidAccountData),
        );
    }

    #[test]
    fn account_with_multisig_len() {
        let mut buffer = vec![0; Multisig::LEN];
        assert_eq!(
            StateWithExtensionsMut::<Account>::unpack_uninitialized(&mut buffer),
            Err(ProgramError::InvalidAccountData),
        );
        let account_size = ExtensionType::try_calculate_account_len::<Account>(&[
            ExtensionType::AccountPaddingTest,
        ])
        .unwrap();
        assert_eq!(account_size, Multisig::LEN + size_of::<ExtensionType>());
        let mut buffer = vec![0; account_size];

        // write base account
        let mut state =
            StateWithExtensionsMut::<Account>::unpack_uninitialized(&mut buffer).unwrap();
        state.base = TEST_ACCOUNT;
        state.pack_base();
        state.init_account_type().unwrap();

        // write padding
        let extension = state.init_extension::<AccountPaddingTest>(true).unwrap();
        extension.0.padding1 = [2; 128];
        extension.0.padding2 = [2; 48];
        extension.0.padding3 = [2; 9];

        assert_eq!(
            &state.get_extension_types().unwrap(),
            &[ExtensionType::AccountPaddingTest]
        );

        // check raw buffer
        let mut expect = TEST_ACCOUNT_SLICE.to_vec();
        expect.push(AccountType::Account.into());
        expect.extend_from_slice(&(ExtensionType::AccountPaddingTest as u16).to_le_bytes());
        expect
            .extend_from_slice(&(pod_get_packed_len::<AccountPaddingTest>() as u16).to_le_bytes());
        expect.extend_from_slice(&vec![2; pod_get_packed_len::<AccountPaddingTest>()]);
        expect.extend_from_slice(&(ExtensionType::Uninitialized as u16).to_le_bytes());
        assert_eq!(expect, buffer);
    }

    #[test]
    fn test_set_account_type() {
        // account with buffer big enough for AccountType and Extension
        let mut buffer = TEST_ACCOUNT_SLICE.to_vec();
        let needed_len =
            ExtensionType::try_calculate_account_len::<Account>(&[ExtensionType::ImmutableOwner])
                .unwrap()
                - buffer.len();
        buffer.append(&mut vec![0; needed_len]);
        let err = StateWithExtensionsMut::<Account>::unpack(&mut buffer).unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);
        set_account_type::<Account>(&mut buffer).unwrap();
        // unpack is viable after manual set_account_type
        let mut state = StateWithExtensionsMut::<Account>::unpack(&mut buffer).unwrap();
        assert_eq!(state.base, TEST_ACCOUNT);
        assert_eq!(state.account_type[0], AccountType::Account as u8);
        state.init_extension::<ImmutableOwner>(true).unwrap(); // just confirming initialization works

        // account with buffer big enough for AccountType only
        let mut buffer = TEST_ACCOUNT_SLICE.to_vec();
        buffer.append(&mut vec![0; 2]);
        let err = StateWithExtensionsMut::<Account>::unpack(&mut buffer).unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);
        set_account_type::<Account>(&mut buffer).unwrap();
        // unpack is viable after manual set_account_type
        let state = StateWithExtensionsMut::<Account>::unpack(&mut buffer).unwrap();
        assert_eq!(state.base, TEST_ACCOUNT);
        assert_eq!(state.account_type[0], AccountType::Account as u8);

        // account with AccountType already set => noop
        let mut buffer = TEST_ACCOUNT_SLICE.to_vec();
        buffer.append(&mut vec![2, 0]);
        let _ = StateWithExtensionsMut::<Account>::unpack(&mut buffer).unwrap();
        set_account_type::<Account>(&mut buffer).unwrap();
        let state = StateWithExtensionsMut::<Account>::unpack(&mut buffer).unwrap();
        assert_eq!(state.base, TEST_ACCOUNT);
        assert_eq!(state.account_type[0], AccountType::Account as u8);

        // account with wrong AccountType fails
        let mut buffer = TEST_ACCOUNT_SLICE.to_vec();
        buffer.append(&mut vec![1, 0]);
        let err = StateWithExtensionsMut::<Account>::unpack(&mut buffer).unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);
        let err = set_account_type::<Account>(&mut buffer).unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);

        // mint with buffer big enough for AccountType and Extension
        let mut buffer = TEST_MINT_SLICE.to_vec();
        let needed_len =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MintCloseAuthority])
                .unwrap()
                - buffer.len();
        buffer.append(&mut vec![0; needed_len]);
        let err = StateWithExtensionsMut::<Mint>::unpack(&mut buffer).unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);
        set_account_type::<Mint>(&mut buffer).unwrap();
        // unpack is viable after manual set_account_type
        let mut state = StateWithExtensionsMut::<Mint>::unpack(&mut buffer).unwrap();
        assert_eq!(state.base, TEST_MINT);
        assert_eq!(state.account_type[0], AccountType::Mint as u8);
        state.init_extension::<MintCloseAuthority>(true).unwrap();

        // mint with buffer big enough for AccountType only
        let mut buffer = TEST_MINT_SLICE.to_vec();
        buffer.append(&mut vec![0; Account::LEN - Mint::LEN]);
        buffer.append(&mut vec![0; 2]);
        let err = StateWithExtensionsMut::<Mint>::unpack(&mut buffer).unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);
        set_account_type::<Mint>(&mut buffer).unwrap();
        // unpack is viable after manual set_account_type
        let state = StateWithExtensionsMut::<Mint>::unpack(&mut buffer).unwrap();
        assert_eq!(state.base, TEST_MINT);
        assert_eq!(state.account_type[0], AccountType::Mint as u8);

        // mint with AccountType already set => noop
        let mut buffer = TEST_MINT_SLICE.to_vec();
        buffer.append(&mut vec![0; Account::LEN - Mint::LEN]);
        buffer.append(&mut vec![1, 0]);
        set_account_type::<Mint>(&mut buffer).unwrap();
        let state = StateWithExtensionsMut::<Mint>::unpack(&mut buffer).unwrap();
        assert_eq!(state.base, TEST_MINT);
        assert_eq!(state.account_type[0], AccountType::Mint as u8);

        // mint with wrong AccountType fails
        let mut buffer = TEST_MINT_SLICE.to_vec();
        buffer.append(&mut vec![0; Account::LEN - Mint::LEN]);
        buffer.append(&mut vec![2, 0]);
        let err = StateWithExtensionsMut::<Mint>::unpack(&mut buffer).unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);
        let err = set_account_type::<Mint>(&mut buffer).unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);
    }

    #[test]
    fn test_set_account_type_wrongly() {
        // try to set Account account_type to Mint
        let mut buffer = TEST_ACCOUNT_SLICE.to_vec();
        buffer.append(&mut vec![0; 2]);
        let err = set_account_type::<Mint>(&mut buffer).unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);

        // try to set Mint account_type to Account
        let mut buffer = TEST_MINT_SLICE.to_vec();
        buffer.append(&mut vec![0; Account::LEN - Mint::LEN]);
        buffer.append(&mut vec![0; 2]);
        let err = set_account_type::<Account>(&mut buffer).unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);
    }

    #[test]
    fn test_get_required_init_account_extensions() {
        // Some mint extensions with no required account extensions
        let mint_extensions = vec![
            ExtensionType::MintCloseAuthority,
            ExtensionType::Uninitialized,
        ];
        assert_eq!(
            ExtensionType::get_required_init_account_extensions(&mint_extensions),
            vec![]
        );

        // One mint extension with required account extension, one without
        let mint_extensions = vec![
            ExtensionType::TransferFeeConfig,
            ExtensionType::MintCloseAuthority,
        ];
        assert_eq!(
            ExtensionType::get_required_init_account_extensions(&mint_extensions),
            vec![ExtensionType::TransferFeeAmount]
        );

        // Some mint extensions both with required account extensions
        let mint_extensions = vec![
            ExtensionType::TransferFeeConfig,
            ExtensionType::MintPaddingTest,
        ];
        assert_eq!(
            ExtensionType::get_required_init_account_extensions(&mint_extensions),
            vec![
                ExtensionType::TransferFeeAmount,
                ExtensionType::AccountPaddingTest
            ]
        );

        // Demonstrate that method does not dedupe inputs or outputs
        let mint_extensions = vec![
            ExtensionType::TransferFeeConfig,
            ExtensionType::TransferFeeConfig,
        ];
        assert_eq!(
            ExtensionType::get_required_init_account_extensions(&mint_extensions),
            vec![
                ExtensionType::TransferFeeAmount,
                ExtensionType::TransferFeeAmount
            ]
        );
    }

    #[test]
    fn mint_without_extensions() {
        let space = ExtensionType::try_calculate_account_len::<Mint>(&[]).unwrap();
        let mut buffer = vec![0; space];
        assert_eq!(
            StateWithExtensionsMut::<Account>::unpack_uninitialized(&mut buffer),
            Err(ProgramError::InvalidAccountData),
        );

        // write base account
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        state.base = TEST_MINT;
        state.pack_base();
        state.init_account_type().unwrap();

        // fail init extension
        assert_eq!(
            state.init_extension::<TransferFeeConfig>(true),
            Err(ProgramError::InvalidAccountData),
        );

        assert_eq!(TEST_MINT_SLICE, buffer);
    }

    #[test]
    fn test_init_nonzero_default() {
        let mint_size =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MintPaddingTest])
                .unwrap();
        let mut buffer = vec![0; mint_size];
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        state.base = TEST_MINT;
        state.pack_base();
        state.init_account_type().unwrap();
        let extension = state.init_extension::<MintPaddingTest>(true).unwrap();
        assert_eq!(extension.padding1, [1; 128]);
        assert_eq!(extension.padding2, [2; 48]);
        assert_eq!(extension.padding3, [3; 9]);
    }

    #[test]
    fn test_init_buffer_too_small() {
        let mint_size =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MintCloseAuthority])
                .unwrap();
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

        // OK, there aren't two bytes for the type, but that's fine
        let mut buffer = vec![0; BASE_ACCOUNT_LENGTH + 2];
        let state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        assert_eq!(state.get_extension_types().unwrap(), []);
    }

    #[test]
    fn test_extension_with_no_data() {
        let account_size =
            ExtensionType::try_calculate_account_len::<Account>(&[ExtensionType::ImmutableOwner])
                .unwrap();
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
            get_tlv_data_info(state.tlv_data).unwrap(),
            TlvDataInfo {
                extension_types: vec![ExtensionType::ImmutableOwner],
                used_len: add_type_and_length_to_len(0)
            }
        );
    }

    #[test]
    fn fail_account_len_with_metadata() {
        assert_eq!(
            ExtensionType::try_calculate_account_len::<Mint>(&[
                ExtensionType::MintCloseAuthority,
                ExtensionType::VariableLenMintTest,
                ExtensionType::TransferFeeConfig,
            ])
            .unwrap_err(),
            ProgramError::InvalidArgument
        );
    }

    #[test]
    fn alloc() {
        let variable_len = VariableLenMintTest { data: vec![1] };
        let alloc_size = variable_len.get_packed_len().unwrap();
        let account_size =
            BASE_ACCOUNT_LENGTH + size_of::<AccountType>() + add_type_and_length_to_len(alloc_size);
        let mut buffer = vec![0; account_size];
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        state
            .init_variable_len_extension(&variable_len, false)
            .unwrap();

        // can't double alloc
        assert_eq!(
            state
                .init_variable_len_extension(&variable_len, false)
                .unwrap_err(),
            TokenError::ExtensionAlreadyInitialized.into()
        );

        // unless overwrite is set
        state
            .init_variable_len_extension(&variable_len, true)
            .unwrap();

        // can't change the size during overwrite though
        assert_eq!(
            state
                .init_variable_len_extension(&VariableLenMintTest { data: vec![] }, true)
                .unwrap_err(),
            TokenError::InvalidLengthForAlloc.into()
        );

        // try to write too far, fail earlier
        assert_eq!(
            state
                .init_variable_len_extension(&VariableLenMintTest { data: vec![1, 2] }, true)
                .unwrap_err(),
            ProgramError::InvalidAccountData
        );
    }

    #[test]
    fn realloc() {
        let small_variable_len = VariableLenMintTest {
            data: vec![1, 2, 3],
        };
        let base_variable_len = VariableLenMintTest {
            data: vec![1, 2, 3, 4],
        };
        let big_variable_len = VariableLenMintTest {
            data: vec![1, 2, 3, 4, 5],
        };
        let too_big_variable_len = VariableLenMintTest {
            data: vec![1, 2, 3, 4, 5, 6],
        };
        let account_size =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MetadataPointer])
                .unwrap()
                + add_type_and_length_to_len(big_variable_len.get_packed_len().unwrap());
        let mut buffer = vec![0; account_size];
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();

        // alloc both types
        state
            .init_variable_len_extension(&base_variable_len, false)
            .unwrap();
        let max_pubkey =
            OptionalNonZeroPubkey::try_from(Some(Pubkey::new_from_array([255; 32]))).unwrap();
        let extension = state.init_extension::<MetadataPointer>(false).unwrap();
        extension.authority = max_pubkey;
        extension.metadata_address = max_pubkey;

        // realloc first entry to larger
        state
            .realloc_variable_len_extension(&big_variable_len)
            .unwrap();
        let extension = state
            .get_variable_len_extension::<VariableLenMintTest>()
            .unwrap();
        assert_eq!(extension, big_variable_len);
        let extension = state.get_extension::<MetadataPointer>().unwrap();
        assert_eq!(extension.authority, max_pubkey);
        assert_eq!(extension.metadata_address, max_pubkey);

        // realloc to smaller
        state
            .realloc_variable_len_extension(&small_variable_len)
            .unwrap();
        let extension = state
            .get_variable_len_extension::<VariableLenMintTest>()
            .unwrap();
        assert_eq!(extension, small_variable_len);
        let extension = state.get_extension::<MetadataPointer>().unwrap();
        assert_eq!(extension.authority, max_pubkey);
        assert_eq!(extension.metadata_address, max_pubkey);
        let diff = big_variable_len.get_packed_len().unwrap()
            - small_variable_len.get_packed_len().unwrap();
        assert_eq!(&buffer[account_size - diff..account_size], vec![0; diff]);

        // unpack again since we dropped the last `state`
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        // realloc too much, fails
        assert_eq!(
            state
                .realloc_variable_len_extension(&too_big_variable_len)
                .unwrap_err(),
            ProgramError::InvalidAccountData,
        );
    }

    #[test]
    fn account_len() {
        let small_variable_len = VariableLenMintTest {
            data: vec![20, 30, 40],
        };
        let variable_len = VariableLenMintTest {
            data: vec![20, 30, 40, 50],
        };
        let big_variable_len = VariableLenMintTest {
            data: vec![20, 30, 40, 50, 60],
        };
        let value_len = variable_len.get_packed_len().unwrap();
        let account_size =
            BASE_ACCOUNT_LENGTH + size_of::<AccountType>() + add_type_and_length_to_len(value_len);
        let mut buffer = vec![0; account_size];
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();

        // With a new extension, new length must include padding, 1 byte for
        // account type, 2 bytes for type, 2 for length
        let current_len = state.try_get_account_len().unwrap();
        assert_eq!(current_len, Mint::LEN);
        let new_len = state
            .try_get_new_account_len_for_variable_len_extension::<VariableLenMintTest>(
                &variable_len,
            )
            .unwrap();
        assert_eq!(
            new_len,
            BASE_ACCOUNT_AND_TYPE_LENGTH.saturating_add(add_type_and_length_to_len(value_len))
        );

        state
            .init_variable_len_extension::<VariableLenMintTest>(&variable_len, false)
            .unwrap();
        let current_len = state.try_get_account_len().unwrap();
        assert_eq!(current_len, new_len);

        // Reduce the extension size
        let new_len = state
            .try_get_new_account_len_for_variable_len_extension::<VariableLenMintTest>(
                &small_variable_len,
            )
            .unwrap();
        assert_eq!(current_len.checked_sub(new_len).unwrap(), 1);

        // Increase the extension size
        let new_len = state
            .try_get_new_account_len_for_variable_len_extension::<VariableLenMintTest>(
                &big_variable_len,
            )
            .unwrap();
        assert_eq!(new_len.checked_sub(current_len).unwrap(), 1);

        // Maintain the extension size
        let new_len = state
            .try_get_new_account_len_for_variable_len_extension::<VariableLenMintTest>(
                &variable_len,
            )
            .unwrap();
        assert_eq!(new_len, current_len);
    }

    /// Test helper for mimicking the data layout an on-chain `AccountInfo`,
    /// which permits "reallocs" as the Solana runtime does it
    struct SolanaAccountData {
        data: Vec<u8>,
        lamports: u64,
        owner: Pubkey,
    }
    impl SolanaAccountData {
        /// Create a new fake solana account data. The underlying vector is
        /// overallocated to mimic the runtime
        fn new(account_data: &[u8]) -> Self {
            let mut data = vec![];
            data.extend_from_slice(&(account_data.len() as u64).to_le_bytes());
            data.extend_from_slice(account_data);
            data.extend_from_slice(&[0; MAX_PERMITTED_DATA_INCREASE]);
            Self {
                data,
                lamports: 10,
                owner: Pubkey::new_unique(),
            }
        }

        /// Data lops off the first 8 bytes, since those store the size of the
        /// account for the Solana runtime
        fn data(&self) -> &[u8] {
            let start = size_of::<u64>();
            let len = self.len();
            &self.data[start..start + len]
        }

        /// Gets the runtime length of the account data
        fn len(&self) -> usize {
            self.data
                .get(..size_of::<u64>())
                .and_then(|slice| slice.try_into().ok())
                .map(u64::from_le_bytes)
                .unwrap() as usize
        }
    }
    impl GetAccount for SolanaAccountData {
        fn get(&mut self) -> (&mut u64, &mut [u8], &Pubkey, bool, Epoch) {
            // need to pull out the data here to avoid a double-mutable borrow
            let start = size_of::<u64>();
            let len = self.len();
            (
                &mut self.lamports,
                &mut self.data[start..start + len],
                &self.owner,
                false,
                Epoch::default(),
            )
        }
    }

    #[test]
    fn alloc_new_fixed_len_tlv_in_account_info_from_base_size() {
        let fixed_len = FixedLenMintTest {
            data: [1, 2, 3, 4, 5, 6, 7, 8],
        };
        let value_len = pod_get_packed_len::<FixedLenMintTest>();
        let base_account_size = Mint::LEN;
        let mut buffer = vec![0; base_account_size];
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        state.base = TEST_MINT;
        state.pack_base();

        let mut data = SolanaAccountData::new(&buffer);
        let key = Pubkey::new_unique();
        let account_info = (&key, &mut data).into_account_info();

        alloc_and_serialize::<Mint, _>(&account_info, &fixed_len, false).unwrap();
        let new_account_len = BASE_ACCOUNT_AND_TYPE_LENGTH + add_type_and_length_to_len(value_len);
        assert_eq!(data.len(), new_account_len);
        let state = StateWithExtensions::<Mint>::unpack(data.data()).unwrap();
        assert_eq!(
            state.get_extension::<FixedLenMintTest>().unwrap(),
            &fixed_len,
        );

        // alloc again succeeds with "overwrite"
        let account_info = (&key, &mut data).into_account_info();
        alloc_and_serialize::<Mint, _>(&account_info, &fixed_len, true).unwrap();

        // alloc again fails without "overwrite"
        let account_info = (&key, &mut data).into_account_info();
        assert_eq!(
            alloc_and_serialize::<Mint, _>(&account_info, &fixed_len, false).unwrap_err(),
            TokenError::ExtensionAlreadyInitialized.into()
        );
    }

    #[test]
    fn alloc_new_variable_len_tlv_in_account_info_from_base_size() {
        let variable_len = VariableLenMintTest { data: vec![20, 99] };
        let value_len = variable_len.get_packed_len().unwrap();
        let base_account_size = Mint::LEN;
        let mut buffer = vec![0; base_account_size];
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        state.base = TEST_MINT;
        state.pack_base();

        let mut data = SolanaAccountData::new(&buffer);
        let key = Pubkey::new_unique();
        let account_info = (&key, &mut data).into_account_info();

        alloc_and_serialize_variable_len_extension::<Mint, _>(&account_info, &variable_len, false)
            .unwrap();
        let new_account_len = BASE_ACCOUNT_AND_TYPE_LENGTH + add_type_and_length_to_len(value_len);
        assert_eq!(data.len(), new_account_len);
        let state = StateWithExtensions::<Mint>::unpack(data.data()).unwrap();
        assert_eq!(
            state
                .get_variable_len_extension::<VariableLenMintTest>()
                .unwrap(),
            variable_len
        );

        // alloc again succeeds with "overwrite"
        let account_info = (&key, &mut data).into_account_info();
        alloc_and_serialize_variable_len_extension::<Mint, _>(&account_info, &variable_len, true)
            .unwrap();

        // alloc again fails without "overwrite"
        let account_info = (&key, &mut data).into_account_info();
        assert_eq!(
            alloc_and_serialize_variable_len_extension::<Mint, _>(
                &account_info,
                &variable_len,
                false,
            )
            .unwrap_err(),
            TokenError::ExtensionAlreadyInitialized.into()
        );
    }

    #[test]
    fn alloc_new_fixed_len_tlv_in_account_info_from_extended_size() {
        let fixed_len = FixedLenMintTest {
            data: [1, 2, 3, 4, 5, 6, 7, 8],
        };
        let value_len = pod_get_packed_len::<FixedLenMintTest>();
        let account_size =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::GroupPointer])
                .unwrap()
                + add_type_and_length_to_len(value_len);
        let mut buffer = vec![0; account_size];
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        state.base = TEST_MINT;
        state.pack_base();
        state.init_account_type().unwrap();

        let test_key =
            OptionalNonZeroPubkey::try_from(Some(Pubkey::new_from_array([20; 32]))).unwrap();
        let extension = state.init_extension::<GroupPointer>(false).unwrap();
        extension.authority = test_key;
        extension.group_address = test_key;

        let mut data = SolanaAccountData::new(&buffer);
        let key = Pubkey::new_unique();
        let account_info = (&key, &mut data).into_account_info();

        alloc_and_serialize::<Mint, _>(&account_info, &fixed_len, false).unwrap();
        let new_account_len = BASE_ACCOUNT_AND_TYPE_LENGTH
            + add_type_and_length_to_len(value_len)
            + add_type_and_length_to_len(size_of::<GroupPointer>());
        assert_eq!(data.len(), new_account_len);
        let state = StateWithExtensions::<Mint>::unpack(data.data()).unwrap();
        assert_eq!(
            state.get_extension::<FixedLenMintTest>().unwrap(),
            &fixed_len,
        );
        let extension = state.get_extension::<GroupPointer>().unwrap();
        assert_eq!(extension.authority, test_key);
        assert_eq!(extension.group_address, test_key);

        // alloc again succeeds with "overwrite"
        let account_info = (&key, &mut data).into_account_info();
        alloc_and_serialize::<Mint, _>(&account_info, &fixed_len, true).unwrap();

        // alloc again fails without "overwrite"
        let account_info = (&key, &mut data).into_account_info();
        assert_eq!(
            alloc_and_serialize::<Mint, _>(&account_info, &fixed_len, false).unwrap_err(),
            TokenError::ExtensionAlreadyInitialized.into()
        );
    }

    #[test]
    fn alloc_new_variable_len_tlv_in_account_info_from_extended_size() {
        let variable_len = VariableLenMintTest { data: vec![42, 6] };
        let value_len = variable_len.get_packed_len().unwrap();
        let account_size =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MetadataPointer])
                .unwrap()
                + add_type_and_length_to_len(value_len);
        let mut buffer = vec![0; account_size];
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        state.base = TEST_MINT;
        state.pack_base();
        state.init_account_type().unwrap();

        let test_key =
            OptionalNonZeroPubkey::try_from(Some(Pubkey::new_from_array([20; 32]))).unwrap();
        let extension = state.init_extension::<MetadataPointer>(false).unwrap();
        extension.authority = test_key;
        extension.metadata_address = test_key;

        let mut data = SolanaAccountData::new(&buffer);
        let key = Pubkey::new_unique();
        let account_info = (&key, &mut data).into_account_info();

        alloc_and_serialize_variable_len_extension::<Mint, _>(&account_info, &variable_len, false)
            .unwrap();
        let new_account_len = BASE_ACCOUNT_AND_TYPE_LENGTH
            + add_type_and_length_to_len(value_len)
            + add_type_and_length_to_len(size_of::<MetadataPointer>());
        assert_eq!(data.len(), new_account_len);
        let state = StateWithExtensions::<Mint>::unpack(data.data()).unwrap();
        assert_eq!(
            state
                .get_variable_len_extension::<VariableLenMintTest>()
                .unwrap(),
            variable_len
        );
        let extension = state.get_extension::<MetadataPointer>().unwrap();
        assert_eq!(extension.authority, test_key);
        assert_eq!(extension.metadata_address, test_key);

        // alloc again succeeds with "overwrite"
        let account_info = (&key, &mut data).into_account_info();
        alloc_and_serialize_variable_len_extension::<Mint, _>(&account_info, &variable_len, true)
            .unwrap();

        // alloc again fails without "overwrite"
        let account_info = (&key, &mut data).into_account_info();
        assert_eq!(
            alloc_and_serialize_variable_len_extension::<Mint, _>(
                &account_info,
                &variable_len,
                false,
            )
            .unwrap_err(),
            TokenError::ExtensionAlreadyInitialized.into()
        );
    }

    #[test]
    fn realloc_variable_len_tlv_in_account_info() {
        let variable_len = VariableLenMintTest {
            data: vec![1, 2, 3, 4, 5],
        };
        let alloc_size = variable_len.get_packed_len().unwrap();
        let account_size =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MetadataPointer])
                .unwrap()
                + add_type_and_length_to_len(alloc_size);
        let mut buffer = vec![0; account_size];
        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut buffer).unwrap();
        state.base = TEST_MINT;
        state.pack_base();
        state.init_account_type().unwrap();

        // alloc both types
        state
            .init_variable_len_extension(&variable_len, false)
            .unwrap();
        let max_pubkey =
            OptionalNonZeroPubkey::try_from(Some(Pubkey::new_from_array([255; 32]))).unwrap();
        let extension = state.init_extension::<MetadataPointer>(false).unwrap();
        extension.authority = max_pubkey;
        extension.metadata_address = max_pubkey;

        // reallocate to smaller, make sure existing extension is fine
        let mut data = SolanaAccountData::new(&buffer);
        let key = Pubkey::new_unique();
        let account_info = (&key, &mut data).into_account_info();
        let variable_len = VariableLenMintTest { data: vec![1, 2] };
        alloc_and_serialize_variable_len_extension::<Mint, _>(&account_info, &variable_len, true)
            .unwrap();

        let state = StateWithExtensions::<Mint>::unpack(data.data()).unwrap();
        let extension = state.get_extension::<MetadataPointer>().unwrap();
        assert_eq!(extension.authority, max_pubkey);
        assert_eq!(extension.metadata_address, max_pubkey);
        let extension = state
            .get_variable_len_extension::<VariableLenMintTest>()
            .unwrap();
        assert_eq!(extension, variable_len);
        assert_eq!(data.len(), state.try_get_account_len().unwrap());

        // reallocate to larger
        let account_info = (&key, &mut data).into_account_info();
        let variable_len = VariableLenMintTest {
            data: vec![1, 2, 3, 4, 5, 6, 7],
        };
        alloc_and_serialize_variable_len_extension::<Mint, _>(&account_info, &variable_len, true)
            .unwrap();

        let state = StateWithExtensions::<Mint>::unpack(data.data()).unwrap();
        let extension = state.get_extension::<MetadataPointer>().unwrap();
        assert_eq!(extension.authority, max_pubkey);
        assert_eq!(extension.metadata_address, max_pubkey);
        let extension = state
            .get_variable_len_extension::<VariableLenMintTest>()
            .unwrap();
        assert_eq!(extension, variable_len);
        assert_eq!(data.len(), state.try_get_account_len().unwrap());

        // reallocate to same
        let account_info = (&key, &mut data).into_account_info();
        let variable_len = VariableLenMintTest {
            data: vec![7, 6, 5, 4, 3, 2, 1],
        };
        alloc_and_serialize_variable_len_extension::<Mint, _>(&account_info, &variable_len, true)
            .unwrap();

        let state = StateWithExtensions::<Mint>::unpack(data.data()).unwrap();
        let extension = state.get_extension::<MetadataPointer>().unwrap();
        assert_eq!(extension.authority, max_pubkey);
        assert_eq!(extension.metadata_address, max_pubkey);
        let extension = state
            .get_variable_len_extension::<VariableLenMintTest>()
            .unwrap();
        assert_eq!(extension, variable_len);
        assert_eq!(data.len(), state.try_get_account_len().unwrap());
    }
}
