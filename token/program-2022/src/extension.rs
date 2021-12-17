//! Extensions available to token mints and accounts

use {
    crate::{
        pod::*,
        state::{Account, Mint, Multisig},
    },
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        program_error::ProgramError,
        program_pack::{IsInitialized, Pack, Sealed},
        pubkey::Pubkey,
    },
    std::convert::TryFrom,
};

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

/// Helper struct for returning the indices of the type, length, and value in
/// a TLV entry
struct TlvIndices(usize, usize, usize);
fn get_extension_indices<V: Extension>(
    tlv_data: &[u8],
    init: bool,
) -> Result<TlvIndices, ProgramError> {
    let mut start_index = pod_get_packed_len::<AccountType>(); // start one byte in to skip the account type
    while start_index < tlv_data.len() {
        let type_end_index = start_index.saturating_add(pod_get_packed_len::<ExtensionType>());
        let length_start_index = type_end_index;
        let length_end_index = length_start_index.saturating_add(pod_get_packed_len::<Length>());
        let value_start_index = length_end_index;

        let extension_type =
            pod_from_bytes::<ExtensionType>(&tlv_data[start_index..type_end_index])?;
        // got to an empty spot, can init here, or move forward if not initing
        if *extension_type == ExtensionType::Uninitialized {
            if init {
                return Ok(TlvIndices(
                    start_index,
                    length_start_index,
                    value_start_index,
                ));
            } else {
                start_index = length_start_index;
            }
        } else if *extension_type == V::TYPE {
            // found an instance of the extension that we're initializing, abort!
            if init {
                return Err(ProgramError::InvalidArgument);
            } else {
                return Ok(TlvIndices(
                    start_index,
                    length_start_index,
                    value_start_index,
                ));
            }
        } else {
            let length = pod_from_bytes::<Length>(&tlv_data[length_start_index..length_end_index])?;
            let value_end_index = value_start_index.saturating_add(usize::from(*length));
            start_index = value_end_index;
        }
    }
    Err(ProgramError::InvalidAccountData)
}

fn check_not_multisig(input: &[u8]) -> Result<(), ProgramError> {
    if input.len() == Multisig::LEN {
        Err(ProgramError::InvalidAccountData)
    } else {
        Ok(())
    }
}

fn tlv_index_unchecked<S: BaseState>(rest_input: &[u8]) -> Result<usize, ProgramError> {
    if rest_input.is_empty() {
        Ok(0)
    } else {
        Ok(Account::LEN.saturating_sub(S::LEN))
    }
}

fn tlv_index<S: BaseState>(rest_input: &[u8]) -> Result<usize, ProgramError> {
    if rest_input.is_empty() {
        Ok(0)
    } else {
        let tlv_start_index = Account::LEN.saturating_sub(S::LEN);
        let account_type = AccountType::try_from(rest_input[tlv_start_index])
            .map_err(|_| ProgramError::InvalidAccountData)?;
        if account_type != S::ACCOUNT_TYPE {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(tlv_start_index)
        }
    }
}

/// Encapsulates immutable base state data (mint or account) with possible extensions
#[derive(Debug, PartialEq)]
pub struct StateWithExtensions<'data, S: BaseState> {
    /// Unpacked base data
    pub base: S,
    /// Slice of data containing all TLV data, deserialized on demand
    tlv_data: &'data [u8],
}
impl<'data, S: BaseState> StateWithExtensions<'data, S> {
    /// Unpack base state, leaving the extension data as a slice
    pub fn unpack(input: &'data [u8]) -> Result<Self, ProgramError> {
        check_not_multisig(input)?;
        let (base_data, rest) = input.split_at(S::LEN);
        let base = S::unpack(base_data)?;
        let tlv_start_index = tlv_index::<S>(rest)?;
        Ok(Self {
            base,
            tlv_data: &rest[tlv_start_index..],
        })
    }

    /// Unpack a portion of the TLV data as the desired type
    pub fn get_extension<V: Extension>(&self) -> Result<&V, ProgramError> {
        if V::ACCOUNT_TYPE != S::ACCOUNT_TYPE {
            return Err(ProgramError::InvalidAccountData);
        }
        let TlvIndices(_, length_start_index, value_start_index) =
            get_extension_indices::<V>(self.tlv_data, false)?;
        let length =
            pod_from_bytes::<Length>(&self.tlv_data[length_start_index..value_start_index])?;
        let value_end_index = value_start_index.saturating_add(usize::from(*length));
        pod_from_bytes::<V>(&self.tlv_data[value_start_index..value_end_index])
    }
}

/// Encapsulates mutable base state data (mint or account) with possible extensions
#[derive(Debug, PartialEq)]
pub struct StateWithExtensionsMut<'data, S: BaseState> {
    /// Unpacked base data
    pub base: S,
    /// Raw base data
    base_data: &'data mut [u8],
    /// Slice of data containing all TLV data, deserialized on demand
    tlv_data: &'data mut [u8],
}
impl<'data, S: BaseState> StateWithExtensionsMut<'data, S> {
    /// Unpack the base state portion of the buffer, leaving the extension data as
    /// a serialized slice.
    pub fn unpack(input: &'data mut [u8]) -> Result<Self, ProgramError> {
        check_not_multisig(input)?;
        let (base_data, rest) = input.split_at_mut(S::LEN);
        let base = S::unpack(base_data)?;
        let tlv_start_index = tlv_index::<S>(rest)?;
        Ok(Self {
            base,
            base_data,
            tlv_data: &mut rest[tlv_start_index..],
        })
    }

    /// Unpack the base state portion of the buffer without checking for initialization,
    /// leaving the extension data as a serialized slice.
    ///
    /// The base state of the struct may be totally unusable.
    pub fn unpack_unchecked(input: &'data mut [u8]) -> Result<Self, ProgramError> {
        check_not_multisig(input)?;
        let (base_data, rest) = input.split_at_mut(S::LEN);
        let base = S::unpack_unchecked(base_data)?;
        let tlv_start_index = tlv_index_unchecked::<S>(rest)?;
        Ok(Self {
            base,
            base_data,
            tlv_data: &mut rest[tlv_start_index..],
        })
    }

    fn get_extension<V: Extension>(&mut self, init: bool) -> Result<&mut V, ProgramError> {
        if V::ACCOUNT_TYPE != S::ACCOUNT_TYPE {
            return Err(ProgramError::InvalidAccountData);
        }
        let TlvIndices(type_start_index, length_start_index, value_start_index) =
            get_extension_indices::<V>(self.tlv_data, init)?;
        if init {
            // write extension type
            let extension_type = pod_from_bytes_mut::<ExtensionType>(
                &mut self.tlv_data[type_start_index..length_start_index],
            )?;
            *extension_type = V::TYPE;
            // write length
            let length_ref = pod_from_bytes_mut::<Length>(
                &mut self.tlv_data[length_start_index..value_start_index],
            )?;
            // maybe this becomes smarter later for dynamically sized extensions
            let length = pod_get_packed_len::<V>();
            *length_ref = Length::try_from(length).unwrap();

            let value_end_index = value_start_index.saturating_add(length);
            pod_from_bytes_mut::<V>(&mut self.tlv_data[value_start_index..value_end_index])
        } else {
            let length =
                pod_from_bytes::<Length>(&self.tlv_data[length_start_index..value_start_index])?;
            let value_end_index = value_start_index.saturating_add(usize::from(*length));
            pod_from_bytes_mut::<V>(&mut self.tlv_data[value_start_index..value_end_index])
        }
    }

    /// Unpack a portion of the TLV data as the desired type
    pub fn get_extension_mut<V: Extension>(&mut self) -> Result<&mut V, ProgramError> {
        self.get_extension(false)
    }

    /// Packs base state data into the base data portion
    pub fn pack_base(&mut self, new_base: S) {
        self.base = new_base;
        S::pack_into_slice(&self.base, self.base_data);
    }

    /// Packs the extension data into an open slot if not already found in the
    /// data buffer, otherwise overwrites itself
    pub fn init_extension<V: Extension>(&mut self) -> Result<&mut V, ProgramError> {
        self.get_extension(true)
    }

    /// Write the account type into the buffer, done during the base
    /// state initialization
    /// Noop if no extensions are present
    pub fn init_account_type(&mut self) {
        // TODO maybe we can do this on `pack_base`, but that means writing this
        // every time there's a change to the base mint / account.
        if !self.tlv_data.is_empty() {
            self.tlv_data[0] = S::ACCOUNT_TYPE as u8;
        }
    }
}

/// Different kinds of accounts. Note that `Mint`, `Account`, and `Multisig` types
/// are determined exclusively by the size of the account, and are not included in
/// the account data. `AccountType` is only included if extensions have been
/// initialized.
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
// TODO This kind of stinks, but there's no way to automatically implement Pod
// on an enum, which almost defeats the purpose of in-place serde!
// Happy to try something else here though.
#[allow(unsafe_code)]
unsafe impl Zeroable for AccountType {}
#[allow(unsafe_code)]
unsafe impl Pod for AccountType {}

/// Extensions that can be applied to mints or accounts.  Mint extensions must only be
/// applied to mint accounts, and account extensions must only be applied to token holding
/// accounts.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, TryFromPrimitive, IntoPrimitive)]
pub enum ExtensionType {
    /// Used as padding if the account size would otherwise be 355, same as a multisig
    Uninitialized,
    /// Includes a transfer fee and accompanying authorities to withdraw and set the fee
    MintTransferFee,
    /// Includes withheld transfer fees
    AccountTransferFee,
    /// Includes an optional mint close authority
    MintCloseAuthority,
}
impl ExtensionType {
    /// Get the data length of the type associated with the enum
    pub fn get_associated_type_len(&self) -> usize {
        match self {
            ExtensionType::Uninitialized => 0,
            ExtensionType::MintTransferFee => pod_get_packed_len::<MintTransferFee>(),
            ExtensionType::AccountTransferFee => pod_get_packed_len::<AccountTransferFee>(),
            ExtensionType::MintCloseAuthority => pod_get_packed_len::<MintCloseAuthority>(),
        }
    }
}
#[allow(unsafe_code)]
unsafe impl Zeroable for ExtensionType {}
#[allow(unsafe_code)]
unsafe impl Pod for ExtensionType {}

/// Get the required account data length for the given ExtensionTypes
pub fn get_account_len(extension_types: &[ExtensionType]) -> usize {
    let extension_size: usize = extension_types
        .iter()
        .map(|e| {
            e.get_associated_type_len()
                .saturating_add(pod_get_packed_len::<ExtensionType>())
                .saturating_add(pod_get_packed_len::<Length>())
        })
        .sum();
    let total_extension_size = if extension_size == Multisig::LEN {
        extension_size + 1
    } else {
        extension_size
    };
    total_extension_size
        .saturating_add(Account::LEN)
        .saturating_add(pod_get_packed_len::<AccountType>())
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
pub trait Extension: Pod {
    /// Associated extension type enum, checked at the start of TLV entries
    const TYPE: ExtensionType;
    /// Associated account type enum, checked for compatibility when reading or
    /// writing extensions into the buffer
    const ACCOUNT_TYPE: AccountType;
}

/// Close authority extension data for mints.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct MintCloseAuthority {
    /// Optional authority to close the mint
    pub close_authority: PodOption<Pubkey>,
}
impl Extension for MintCloseAuthority {
    const TYPE: ExtensionType = ExtensionType::MintCloseAuthority;
    const ACCOUNT_TYPE: AccountType = AccountType::Mint;
}

/// Transfer fee information
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct TransferFee {
    /// First epoch where the transfer fee takes effect
    pub epoch: PodU64, // Epoch,
    /// Maximum fee assessed on transfers, expressed as an amount of tokens
    pub maximum_fee: PodU64,
    /// Amount of transfer collected as fees, expressed as basis points of the
    /// transfer amount, ie. increments of 0.01%
    pub transfer_fee_basis_points: PodU16,
}

/// Transfer fee extension data for mints.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct MintTransferFee {
    /// Optional authority to set the fee
    pub transfer_fee_config_authority: PodOption<Pubkey>,
    /// Withdraw from mint instructions must be signed by this key
    pub withheld_withdraw_authority: PodOption<Pubkey>,
    /// Withheld transfer fee tokens that have been moved to the mint for withdrawal
    pub withheld_amount: PodU64,
    /// Older transfer fee, used if the current epoch < new_transfer_fee.epoch
    pub older_transfer_fee: TransferFee,
    /// Newer transfer fee, used if the current epoch >= new_transfer_fee.epoch
    pub newer_transfer_fee: TransferFee,
}
impl Sealed for MintTransferFee {}
impl Pack for MintTransferFee {
    const LEN: usize = 36 + 36 + 8 + 18 + 18;
    fn unpack_from_slice(_src: &[u8]) -> Result<Self, ProgramError> {
        unimplemented!();
    }
    fn pack_into_slice(&self, _dst: &mut [u8]) {
        unimplemented!();
    }
}
impl Extension for MintTransferFee {
    const TYPE: ExtensionType = ExtensionType::MintTransferFee;
    const ACCOUNT_TYPE: AccountType = AccountType::Mint;
}

/// Transfer fee extension data for accounts.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct AccountTransferFee {
    /// Amount withheld during transfers, to be harvested to the mint
    pub withheld_amount: u64,
}
impl Sealed for AccountTransferFee {}
impl Pack for AccountTransferFee {
    const LEN: usize = 8;
    fn unpack_from_slice(_src: &[u8]) -> Result<Self, ProgramError> {
        unimplemented!();
    }
    fn pack_into_slice(&self, _dst: &mut [u8]) {
        unimplemented!();
    }
}
impl Extension for AccountTransferFee {
    const TYPE: ExtensionType = ExtensionType::AccountTransferFee;
    const ACCOUNT_TYPE: AccountType = AccountType::Account;
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::state::test::{TEST_MINT, TEST_MINT_SLICE};

    #[test]
    fn mint_with_extensions_pack_unpack() {
        let mint_size = get_account_len(&[ExtensionType::MintCloseAuthority]);
        let mut buffer = vec![0; mint_size];

        // fail unpack
        assert_eq!(
            StateWithExtensionsMut::<Mint>::unpack(&mut buffer),
            Err(ProgramError::UninitializedAccount),
        );

        let mut state = StateWithExtensionsMut::<Mint>::unpack_unchecked(&mut buffer).unwrap();
        // success write extension
        let close_authority = PodOption::Some(Pubkey::new(&[1; 32]));
        let extension = state.init_extension::<MintCloseAuthority>().unwrap();
        extension.close_authority = close_authority;

        // fail unpack again, still no base data
        assert_eq!(
            StateWithExtensionsMut::<Mint>::unpack(&mut buffer.clone()),
            Err(ProgramError::UninitializedAccount),
        );

        // write base mint
        let mut state = StateWithExtensionsMut::<Mint>::unpack_unchecked(&mut buffer).unwrap();
        let base = TEST_MINT;
        state.pack_base(base);
        assert_eq!(state.base, base);
        state.init_account_type();

        // check raw buffer
        let mut expect = TEST_MINT_SLICE.to_vec();
        expect.extend_from_slice(&[0; Account::LEN - Mint::LEN]); // padding
        expect.push(AccountType::Mint as u8);
        expect.extend_from_slice(&(ExtensionType::MintCloseAuthority as u16).to_le_bytes());
        expect
            .extend_from_slice(&(pod_get_packed_len::<MintCloseAuthority>() as u16).to_le_bytes());
        expect.extend_from_slice(&[1]);
        expect.extend_from_slice(&[1; 32]);
        assert_eq!(expect, buffer);

        // check unpacking
        let mut state = StateWithExtensionsMut::<Mint>::unpack(&mut buffer).unwrap();
        assert_eq!(state.base, base);

        // update base
        let mut new_base = TEST_MINT;
        new_base.supply += 100;
        state.pack_base(new_base);
        assert_eq!(state.base, new_base);

        // check unpacking
        let mut unpacked_extension = state.get_extension_mut::<MintCloseAuthority>().unwrap();
        assert_eq!(*unpacked_extension, MintCloseAuthority { close_authority });

        // update extension
        let close_authority = PodOption::None;
        unpacked_extension.close_authority = close_authority;

        // check updates are propagated
        let state = StateWithExtensions::<Mint>::unpack(&buffer).unwrap();
        assert_eq!(state.base, new_base);
        let unpacked_extension = state.get_extension::<MintCloseAuthority>().unwrap();
        assert_eq!(*unpacked_extension, MintCloseAuthority { close_authority });

        // check raw buffer
        let mut expect = vec![0; Mint::LEN];
        Mint::pack_into_slice(&new_base, &mut expect);
        expect.extend_from_slice(&[0; Account::LEN - Mint::LEN]); // padding
        expect.push(AccountType::Mint as u8);
        expect.extend_from_slice(&(ExtensionType::MintCloseAuthority as u16).to_le_bytes());
        expect
            .extend_from_slice(&(pod_get_packed_len::<MintCloseAuthority>() as u16).to_le_bytes());
        expect.extend_from_slice(&[0]);
        expect.extend_from_slice(&[0; 32]);
        assert_eq!(expect, buffer);

        // fail unpack as an account
        assert_eq!(
            StateWithExtensions::<Account>::unpack(&buffer),
            Err(ProgramError::InvalidAccountData),
        );
    }
}
