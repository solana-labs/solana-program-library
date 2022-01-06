//! Extensions available to token mints and accounts

use {
    crate::{
        extension::{
            mint_close_authority::MintCloseAuthority,
            transfer_fee::{AccountTransferFee, MintTransferFee},
        },
        pod::*,
        state::{Account, Mint, Multisig},
    },
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        program_error::ProgramError,
        program_pack::{IsInitialized, Pack},
    },
    std::{
        convert::{TryFrom, TryInto},
        mem::size_of,
    },
};

mod mint_close_authority;
mod transfer_fee;

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
    let mut start_index = 0;
    while start_index < tlv_data.len() {
        let type_end_index = start_index.saturating_add(size_of::<ExtensionType>());
        let length_start_index = type_end_index;
        let length_end_index = length_start_index.saturating_add(pod_get_packed_len::<Length>());
        let value_start_index = length_end_index;

        let extension_type = ExtensionType::try_from(&tlv_data[start_index..type_end_index])?;
        // got to an empty spot, can init here, or move forward if not initing
        if extension_type == ExtensionType::Uninitialized {
            if init {
                return Ok(TlvIndices(
                    start_index,
                    length_start_index,
                    value_start_index,
                ));
            } else {
                start_index = length_start_index;
            }
        } else if extension_type == V::TYPE {
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
/// Mint: 82 bytes... + 83 bytes of other extension data + [2, 0, 3, 0, 100, ....]
///                                                         ^ data in extension just happens to look like this
///
/// With this approach, we only start writing the TLV data after Account::LEN,
/// which means we always know that the account type is going to be right after
/// that. We do a special case checking for a Multisig length, because those
/// aren't extensible under any circumstances.
const BASE_ACCOUNT_LENGTH: usize = Account::LEN;

fn type_and_tlv_indices_unchecked<S: BaseState>(
    rest_input: &[u8],
) -> Result<(usize, usize), ProgramError> {
    if rest_input.is_empty() {
        Ok((0, 0))
    } else {
        let account_type_index = BASE_ACCOUNT_LENGTH.saturating_sub(S::LEN);
        let tlv_start_index = account_type_index.saturating_add(size_of::<AccountType>());
        Ok((account_type_index, tlv_start_index))
    }
}

fn type_and_tlv_indices<S: BaseState>(rest_input: &[u8]) -> Result<(usize, usize), ProgramError> {
    if rest_input.is_empty() {
        Ok((0, 0))
    } else {
        let type_index = BASE_ACCOUNT_LENGTH.saturating_sub(S::LEN);
        // check padding is all zeroes
        if rest_input[..type_index] != vec![0; type_index] {
            Err(ProgramError::InvalidAccountData)
        } else {
            let tlv_start_index = type_index.saturating_add(size_of::<AccountType>());
            Ok((type_index, tlv_start_index))
        }
    }
}

/// Encapsulates immutable base state data (mint or account) with possible extensions
#[derive(Debug, PartialEq)]
pub struct StateWithExtensions<'data, S: BaseState> {
    /// Unpacked base data
    pub base: S,
    /// Unpacked account type
    pub account_type: AccountType,
    /// Slice of data containing all TLV data, deserialized on demand
    tlv_data: &'data [u8],
}
impl<'data, S: BaseState> StateWithExtensions<'data, S> {
    /// Unpack base state, leaving the extension data as a slice
    pub fn unpack(input: &'data [u8]) -> Result<Self, ProgramError> {
        check_not_multisig(input)?;
        let (base_data, rest) = input.split_at(S::LEN);
        let base = S::unpack(base_data)?;
        let (type_index, tlv_start_index) = type_and_tlv_indices::<S>(rest)?;
        let account_type = AccountType::try_from(rest[type_index])
            .map_err(|_| ProgramError::InvalidAccountData)?;
        check_account_type::<S>(account_type)?;
        Ok(Self {
            base,
            account_type,
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
    /// Writable account type
    account_type: &'data mut [u8],
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
        let (account_type_index, tlv_start_index) = type_and_tlv_indices::<S>(rest)?;
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
    }

    /// Unpack the base state portion of the buffer without checking for initialization,
    /// leaving the extension data as a serialized slice.
    ///
    /// The base state of the struct may be totally unusable.
    pub fn unpack_unchecked(input: &'data mut [u8]) -> Result<Self, ProgramError> {
        check_not_multisig(input)?;
        let (base_data, rest) = input.split_at_mut(S::LEN);
        let base = S::unpack_unchecked(base_data)?;
        let (account_type_index, tlv_start_index) = type_and_tlv_indices_unchecked::<S>(rest)?;
        let (account_type, tlv_data) = rest.split_at_mut(tlv_start_index);
        Ok(Self {
            base,
            base_data,
            account_type: &mut account_type[account_type_index..tlv_start_index],
            tlv_data,
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
            let extension_type_array: [u8; 2] = V::TYPE.into();
            let extension_type_ref = &mut self.tlv_data[type_start_index..length_start_index];
            extension_type_ref.copy_from_slice(&extension_type_array);
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
    /// Noops if there is no room for an extension in the account, needed for
    /// pure base mints / accounts.
    pub fn init_account_type(&mut self) {
        if !self.account_type.is_empty() {
            self.account_type[0] = S::ACCOUNT_TYPE.into();
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
    /// Padding extension used to make an account exactly Multisig::LEN, used for testing
    #[cfg(test)]
    AccountPaddingTest = u16::MAX - 1,
    /// Padding extension used to make a mint exactly Multisig::LEN, used for testing
    #[cfg(test)]
    MintPaddingTest = u16::MAX,
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
    /// Get the data length of the type associated with the enum
    pub fn get_associated_type_len(&self) -> usize {
        match self {
            ExtensionType::Uninitialized => 0,
            ExtensionType::MintTransferFee => pod_get_packed_len::<MintTransferFee>(),
            ExtensionType::AccountTransferFee => pod_get_packed_len::<AccountTransferFee>(),
            ExtensionType::MintCloseAuthority => pod_get_packed_len::<MintCloseAuthority>(),
            #[cfg(test)]
            ExtensionType::AccountPaddingTest => pod_get_packed_len::<AccountPaddingTest>(),
            #[cfg(test)]
            ExtensionType::MintPaddingTest => pod_get_packed_len::<MintPaddingTest>(),
        }
    }
}

/// Get the required account data length for the given ExtensionTypes
pub fn get_account_len(extension_types: &[ExtensionType]) -> usize {
    let extension_size: usize = extension_types
        .iter()
        .map(|e| {
            e.get_associated_type_len()
                .saturating_add(size_of::<ExtensionType>())
                .saturating_add(pod_get_packed_len::<Length>())
        })
        .sum();
    let account_size = extension_size
        .saturating_add(BASE_ACCOUNT_LENGTH)
        .saturating_add(size_of::<AccountType>());
    if account_size == Multisig::LEN {
        account_size.saturating_add(size_of::<ExtensionType>())
    } else {
        account_size
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
pub trait Extension: Pod {
    /// Associated extension type enum, checked at the start of TLV entries
    const TYPE: ExtensionType;
    /// Associated account type enum, checked for compatibility when reading or
    /// writing extensions into the buffer
    const ACCOUNT_TYPE: AccountType;
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
    const ACCOUNT_TYPE: AccountType = AccountType::Mint;
}
/// Account version of the MintPadding
#[cfg(test)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct AccountPaddingTest(MintPaddingTest);
#[cfg(test)]
impl Extension for AccountPaddingTest {
    const TYPE: ExtensionType = ExtensionType::AccountPaddingTest;
    const ACCOUNT_TYPE: AccountType = AccountType::Account;
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::state::test::{TEST_ACCOUNT, TEST_ACCOUNT_SLICE, TEST_MINT, TEST_MINT_SLICE},
        solana_program::pubkey::Pubkey,
        transfer_fee::test::test_mint_transfer_fee,
    };

    const MINT_WITH_EXTENSION: &[u8] = &[
        // base mint
        1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 42, 0, 0, 0, 0, 0, 0, 0, 7, 1, 1, 0, 0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // padding
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // account type
        1, // extension type
        3, 0, // length
        32, 0, // data
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1,
    ];

    #[test]
    fn unpack_opaque_buffer() {
        let state = StateWithExtensions::<Mint>::unpack(MINT_WITH_EXTENSION).unwrap();
        assert_eq!(state.base, TEST_MINT);
        let extension = state.get_extension::<MintCloseAuthority>().unwrap();
        let close_authority = OptionalNonZeroPubkey::try_from(Some(Pubkey::new(&[1; 32]))).unwrap();
        assert_eq!(extension.close_authority, close_authority);
        assert_eq!(
            state.get_extension::<MintTransferFee>(),
            Err(ProgramError::InvalidAccountData)
        );
        assert_eq!(
            StateWithExtensions::<Account>::unpack(MINT_WITH_EXTENSION),
            Err(ProgramError::InvalidAccountData)
        );
    }

    #[test]
    fn fail_unpack_opaque_buffer() {
        // tweak the account type
        let mut buffer = MINT_WITH_EXTENSION.to_vec();
        buffer[BASE_ACCOUNT_LENGTH] = 3;
        assert_eq!(
            StateWithExtensions::<Mint>::unpack(&buffer),
            Err(ProgramError::InvalidAccountData)
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
            state.get_extension::<MintTransferFee>(),
            Err(ProgramError::InvalidAccountData)
        );

        // tweak the length, too big
        let mut buffer = MINT_WITH_EXTENSION.to_vec();
        buffer[BASE_ACCOUNT_LENGTH + 3] = 100;
        let state = StateWithExtensions::<Mint>::unpack(&buffer).unwrap();
        assert_eq!(
            state.get_extension::<MintTransferFee>(),
            Err(ProgramError::InvalidAccountData)
        );

        // tweak the length, too small
        let mut buffer = MINT_WITH_EXTENSION.to_vec();
        buffer[BASE_ACCOUNT_LENGTH + 3] = 10;
        let state = StateWithExtensions::<Mint>::unpack(&buffer).unwrap();
        assert_eq!(
            state.get_extension::<MintTransferFee>(),
            Err(ProgramError::InvalidAccountData)
        );
    }

    #[test]
    fn mint_with_extension_pack_unpack() {
        let mint_size = get_account_len(&[
            ExtensionType::MintCloseAuthority,
            ExtensionType::MintTransferFee,
        ]);
        let mut buffer = vec![0; mint_size];

        // fail unpack
        assert_eq!(
            StateWithExtensionsMut::<Mint>::unpack(&mut buffer),
            Err(ProgramError::UninitializedAccount),
        );

        let mut state = StateWithExtensionsMut::<Mint>::unpack_unchecked(&mut buffer).unwrap();
        // fail init account extension
        assert_eq!(
            state.init_extension::<AccountTransferFee>(),
            Err(ProgramError::InvalidAccountData),
        );

        // success write extension
        let close_authority = OptionalNonZeroPubkey::try_from(Some(Pubkey::new(&[1; 32]))).unwrap();
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
        expect.extend_from_slice(&[0; BASE_ACCOUNT_LENGTH - Mint::LEN]); // padding
        expect.push(AccountType::Mint.into());
        expect.extend_from_slice(&(ExtensionType::MintCloseAuthority as u16).to_le_bytes());
        expect
            .extend_from_slice(&(pod_get_packed_len::<MintCloseAuthority>() as u16).to_le_bytes());
        expect.extend_from_slice(&[1; 32]); // data
        expect.extend_from_slice(&[0; size_of::<ExtensionType>()]);
        expect.extend_from_slice(&[0; size_of::<Length>()]);
        expect.extend_from_slice(&[0; size_of::<MintTransferFee>()]);
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
        let close_authority = OptionalNonZeroPubkey::try_from(None).unwrap();
        unpacked_extension.close_authority = close_authority;

        // check updates are propagated
        let state = StateWithExtensions::<Mint>::unpack(&buffer).unwrap();
        assert_eq!(state.base, new_base);
        let unpacked_extension = state.get_extension::<MintCloseAuthority>().unwrap();
        assert_eq!(*unpacked_extension, MintCloseAuthority { close_authority });

        // check raw buffer
        let mut expect = vec![0; Mint::LEN];
        Mint::pack_into_slice(&new_base, &mut expect);
        expect.extend_from_slice(&[0; BASE_ACCOUNT_LENGTH - Mint::LEN]); // padding
        expect.push(AccountType::Mint.into());
        expect.extend_from_slice(&(ExtensionType::MintCloseAuthority as u16).to_le_bytes());
        expect
            .extend_from_slice(&(pod_get_packed_len::<MintCloseAuthority>() as u16).to_le_bytes());
        expect.extend_from_slice(&[0; 32]);
        expect.extend_from_slice(&[0; size_of::<ExtensionType>()]);
        expect.extend_from_slice(&[0; size_of::<Length>()]);
        expect.extend_from_slice(&[0; size_of::<MintTransferFee>()]);
        assert_eq!(expect, buffer);

        // fail unpack as an account
        assert_eq!(
            StateWithExtensions::<Account>::unpack(&buffer),
            Err(ProgramError::InvalidAccountData),
        );

        let mut state = StateWithExtensionsMut::<Mint>::unpack(&mut buffer).unwrap();
        // init one more extension
        let mint_transfer_fee = test_mint_transfer_fee();
        let new_extension = state.init_extension::<MintTransferFee>().unwrap();
        new_extension.transfer_fee_config_authority =
            mint_transfer_fee.transfer_fee_config_authority;
        new_extension.withheld_withdraw_authority = mint_transfer_fee.withheld_withdraw_authority;
        new_extension.withheld_amount = mint_transfer_fee.withheld_amount;
        new_extension.older_transfer_fee = mint_transfer_fee.older_transfer_fee;
        new_extension.newer_transfer_fee = mint_transfer_fee.newer_transfer_fee;

        // check raw buffer
        let mut expect = vec![0; Mint::LEN];
        Mint::pack_into_slice(&new_base, &mut expect);
        expect.extend_from_slice(&[0; BASE_ACCOUNT_LENGTH - Mint::LEN]); // padding
        expect.push(AccountType::Mint.into());
        expect.extend_from_slice(&(ExtensionType::MintCloseAuthority as u16).to_le_bytes());
        expect
            .extend_from_slice(&(pod_get_packed_len::<MintCloseAuthority>() as u16).to_le_bytes());
        expect.extend_from_slice(&[0; 32]); // data
        expect.extend_from_slice(&(ExtensionType::MintTransferFee as u16).to_le_bytes());
        expect.extend_from_slice(&(pod_get_packed_len::<MintTransferFee>() as u16).to_le_bytes());
        expect.extend_from_slice(pod_bytes_of(&mint_transfer_fee));
        assert_eq!(expect, buffer);

        // fail to init one more extension that does not fit
        let mut state = StateWithExtensionsMut::<Mint>::unpack(&mut buffer).unwrap();
        assert_eq!(
            state.init_extension::<MintPaddingTest>(),
            Err(ProgramError::InvalidAccountData),
        );
    }

    #[test]
    fn mint_extension_any_order() {
        let mint_size = get_account_len(&[
            ExtensionType::MintCloseAuthority,
            ExtensionType::MintTransferFee,
        ]);
        let mut buffer = vec![0; mint_size];

        let mut state = StateWithExtensionsMut::<Mint>::unpack_unchecked(&mut buffer).unwrap();
        // write extensions
        let close_authority = OptionalNonZeroPubkey::try_from(Some(Pubkey::new(&[1; 32]))).unwrap();
        let extension = state.init_extension::<MintCloseAuthority>().unwrap();
        extension.close_authority = close_authority;

        let mint_transfer_fee = test_mint_transfer_fee();
        let extension = state.init_extension::<MintTransferFee>().unwrap();
        extension.transfer_fee_config_authority = mint_transfer_fee.transfer_fee_config_authority;
        extension.withheld_withdraw_authority = mint_transfer_fee.withheld_withdraw_authority;
        extension.withheld_amount = mint_transfer_fee.withheld_amount;
        extension.older_transfer_fee = mint_transfer_fee.older_transfer_fee;
        extension.newer_transfer_fee = mint_transfer_fee.newer_transfer_fee;

        // write base mint
        let mut state = StateWithExtensionsMut::<Mint>::unpack_unchecked(&mut buffer).unwrap();
        let base = TEST_MINT;
        state.pack_base(base);
        assert_eq!(state.base, base);
        state.init_account_type();

        let mut other_buffer = vec![0; mint_size];
        let mut state =
            StateWithExtensionsMut::<Mint>::unpack_unchecked(&mut other_buffer).unwrap();

        // write base mint
        let base = TEST_MINT;
        state.pack_base(base);
        assert_eq!(state.base, base);
        state.init_account_type();

        // write extensions in a different order
        let mint_transfer_fee = test_mint_transfer_fee();
        let extension = state.init_extension::<MintTransferFee>().unwrap();
        extension.transfer_fee_config_authority = mint_transfer_fee.transfer_fee_config_authority;
        extension.withheld_withdraw_authority = mint_transfer_fee.withheld_withdraw_authority;
        extension.withheld_amount = mint_transfer_fee.withheld_amount;
        extension.older_transfer_fee = mint_transfer_fee.older_transfer_fee;
        extension.newer_transfer_fee = mint_transfer_fee.newer_transfer_fee;

        let close_authority = OptionalNonZeroPubkey::try_from(Some(Pubkey::new(&[1; 32]))).unwrap();
        let extension = state.init_extension::<MintCloseAuthority>().unwrap();
        extension.close_authority = close_authority;

        // buffers are NOT the same because written in a different order
        assert_ne!(buffer, other_buffer);
        let state = StateWithExtensions::<Mint>::unpack(&buffer).unwrap();
        let other_state = StateWithExtensions::<Mint>::unpack(&other_buffer).unwrap();

        // BUT mint and extensions are the same
        assert_eq!(
            state.get_extension::<MintTransferFee>().unwrap(),
            other_state.get_extension::<MintTransferFee>().unwrap()
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
            StateWithExtensionsMut::<Mint>::unpack_unchecked(&mut buffer),
            Err(ProgramError::InvalidAccountData),
        );
        let mint_size = get_account_len(&[ExtensionType::MintPaddingTest]);
        assert_eq!(mint_size, Multisig::LEN + size_of::<ExtensionType>());
        let mut buffer = vec![0; mint_size];

        // write base mint
        let mut state = StateWithExtensionsMut::<Mint>::unpack_unchecked(&mut buffer).unwrap();
        let base = TEST_MINT;
        state.pack_base(base);
        assert_eq!(state.base, base);
        state.init_account_type();

        // write padding
        let extension = state.init_extension::<MintPaddingTest>().unwrap();
        extension.padding1 = [1; 128];
        extension.padding2 = [1; 48];
        extension.padding3 = [1; 9];

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
        let account_size = get_account_len(&[ExtensionType::AccountTransferFee]);
        let mut buffer = vec![0; account_size];

        // fail unpack
        assert_eq!(
            StateWithExtensionsMut::<Account>::unpack(&mut buffer),
            Err(ProgramError::UninitializedAccount),
        );

        let mut state = StateWithExtensionsMut::<Account>::unpack_unchecked(&mut buffer).unwrap();
        // fail init mint extension
        assert_eq!(
            state.init_extension::<MintTransferFee>(),
            Err(ProgramError::InvalidAccountData),
        );
        // success write extension
        let withheld_amount = PodU64::from(u64::MAX);
        let extension = state.init_extension::<AccountTransferFee>().unwrap();
        extension.withheld_amount = withheld_amount;

        // fail unpack again, still no base data
        assert_eq!(
            StateWithExtensionsMut::<Account>::unpack(&mut buffer.clone()),
            Err(ProgramError::UninitializedAccount),
        );

        // write base account
        let mut state = StateWithExtensionsMut::<Account>::unpack_unchecked(&mut buffer).unwrap();
        let base = TEST_ACCOUNT;
        state.pack_base(base);
        assert_eq!(state.base, base);
        state.init_account_type();

        // check raw buffer
        let mut expect = TEST_ACCOUNT_SLICE.to_vec();
        expect.push(AccountType::Account.into());
        expect.extend_from_slice(&(ExtensionType::AccountTransferFee as u16).to_le_bytes());
        expect
            .extend_from_slice(&(pod_get_packed_len::<AccountTransferFee>() as u16).to_le_bytes());
        expect.extend_from_slice(&u64::from(withheld_amount).to_le_bytes());
        assert_eq!(expect, buffer);

        // check unpacking
        let mut state = StateWithExtensionsMut::<Account>::unpack(&mut buffer).unwrap();
        assert_eq!(state.base, base);

        // update base
        let mut new_base = TEST_ACCOUNT;
        new_base.amount += 100;
        state.pack_base(new_base);
        assert_eq!(state.base, new_base);

        // check unpacking
        let mut unpacked_extension = state.get_extension_mut::<AccountTransferFee>().unwrap();
        assert_eq!(*unpacked_extension, AccountTransferFee { withheld_amount });

        // update extension
        let withheld_amount = PodU64::from(u32::MAX as u64);
        unpacked_extension.withheld_amount = withheld_amount;

        // check updates are propagated
        let state = StateWithExtensions::<Account>::unpack(&buffer).unwrap();
        assert_eq!(state.base, new_base);
        let unpacked_extension = state.get_extension::<AccountTransferFee>().unwrap();
        assert_eq!(*unpacked_extension, AccountTransferFee { withheld_amount });

        // check raw buffer
        let mut expect = vec![0; Account::LEN];
        Account::pack_into_slice(&new_base, &mut expect);
        expect.push(AccountType::Account.into());
        expect.extend_from_slice(&(ExtensionType::AccountTransferFee as u16).to_le_bytes());
        expect
            .extend_from_slice(&(pod_get_packed_len::<AccountTransferFee>() as u16).to_le_bytes());
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
            StateWithExtensionsMut::<Account>::unpack_unchecked(&mut buffer),
            Err(ProgramError::InvalidAccountData),
        );
        let account_size = get_account_len(&[ExtensionType::AccountPaddingTest]);
        assert_eq!(account_size, Multisig::LEN + size_of::<ExtensionType>());
        let mut buffer = vec![0; account_size];

        // write base account
        let mut state = StateWithExtensionsMut::<Account>::unpack_unchecked(&mut buffer).unwrap();
        let base = TEST_ACCOUNT;
        state.pack_base(base);
        assert_eq!(state.base, base);
        state.init_account_type();

        // write padding
        let extension = state.init_extension::<AccountPaddingTest>().unwrap();
        extension.0.padding1 = [2; 128];
        extension.0.padding2 = [2; 48];
        extension.0.padding3 = [2; 9];

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
}
