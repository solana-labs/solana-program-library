//! Extensions available to token mints and accounts

use {
    crate::state::{pack_coption_key, unpack_coption_key, Account, Mint, Multisig},
    arrayref::{array_mut_ref, array_ref},
    solana_program::{
        clock::Epoch,
        program_error::ProgramError,
        program_option::COption,
        program_pack::{IsInitialized, Pack, Sealed},
        pubkey::Pubkey,
    },
    std::convert::{TryFrom, TryInto},
};

const TYPE_LEN: usize = 1;
const LENGTH_LEN: usize = 4;

// TODO probably need an immutable version of this for clients
/// Encapsulates base state data (mint or account) with possible extensions
#[derive(Debug, PartialEq)]
pub struct BaseStateWithExtensions<'data, S: BaseState> {
    /// Unpacked base data
    pub base: S,
    /// Raw base data
    pub base_data: &'data mut [u8],
    /// Slice of data containing all TLV data, deserialized on demand
    pub tlv_data: &'data mut [u8],
}
impl<'data, S: BaseState + Pack + IsInitialized> BaseStateWithExtensions<'data, S> {
    /// Unpack the base state portion of the buffer, leaving the extension data as
    /// a serialized slice.
    pub fn unpack(input: &'data mut [u8]) -> Result<Self, ProgramError> {
        let input_len = input.len();
        if input_len == Multisig::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        let (base_data, rest) = input.split_at_mut(S::LEN);
        let base = S::unpack(base_data)?;
        if input_len == S::LEN {
            Ok(Self {
                base,
                base_data,
                tlv_data: rest, // empty slice
            })
        } else {
            let tlv_start_index = Account::LEN.saturating_sub(S::LEN);
            let account_type: AccountType = rest[tlv_start_index].try_into()?;
            if account_type != S::ACCOUNT_TYPE {
                return Err(ProgramError::InvalidAccountData);
            }
            Ok(Self {
                base,
                base_data,
                tlv_data: &mut rest[tlv_start_index..],
            })
        }
    }

    /// Unpack the base state portion of the buffer without checking for initialization,
    /// leaving the extension data as a serialized slice.
    ///
    /// The base state of the struct may be totally unusable.
    pub fn unpack_unchecked(input: &'data mut [u8]) -> Result<Self, ProgramError> {
        let input_len = input.len();
        if input_len == Multisig::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        let (base_data, rest) = input.split_at_mut(S::LEN);
        let base = S::unpack_unchecked(base_data)?;
        let tlv_data = if input_len == S::LEN {
            rest // empty slice
        } else {
            let tlv_start_index = Account::LEN.saturating_sub(S::LEN);
            &mut rest[tlv_start_index..]
        };
        Ok(Self {
            base,
            base_data,
            tlv_data,
        })
    }

    /// Unpack a portion of the TLV data as the desired type
    pub fn unpack_extension<V: Pack + Extension>(&self) -> Result<V, ProgramError> {
        // TODO: this might not be necessary, but may save some footguns?
        if V::ACCOUNT_TYPE != S::ACCOUNT_TYPE {
            return Err(ProgramError::InvalidAccountData);
        }
        let mut start_index = 1; // start one byte in to skip the account type
        while start_index < self.tlv_data.len() {
            let length_start_index = start_index.saturating_add(TYPE_LEN);
            let length_end_index = length_start_index.saturating_add(LENGTH_LEN);

            let extension_type: ExtensionType = self.tlv_data[start_index].try_into()?;
            if extension_type == ExtensionType::Uninitialized {
                start_index = length_start_index;
            } else {
                let length = u32::from_le_bytes(
                    self.tlv_data[length_start_index..length_end_index]
                        .try_into()
                        .unwrap(),
                ) as usize;
                let value_start_index = length_end_index;
                let value_end_index = value_start_index.saturating_add(length);
                if extension_type == V::TYPE {
                    return V::unpack_unchecked(&self.tlv_data[value_start_index..value_end_index]);
                } else {
                    start_index = value_end_index;
                }
            };
        }
        Err(ProgramError::InvalidAccountData)
    }

    /// Packs base state data into the base data portion
    pub fn pack_base(&mut self, new_base: S) {
        self.base = new_base;
        S::pack_into_slice(&self.base, self.base_data);
    }

    /// Packs the extension data into an open slot if not already found in the
    /// data buffer, otherwise overwrites itself
    pub fn pack_extension<V: Pack + Extension>(
        &mut self,
        extension: V,
    ) -> Result<(), ProgramError> {
        // TODO: this might not be necessary, but may save some footguns?
        if V::ACCOUNT_TYPE != S::ACCOUNT_TYPE {
            return Err(ProgramError::InvalidAccountData);
        }
        let mut start_index = 1; // start one byte in for the account type
        while start_index < self.tlv_data.len() {
            let length_start_index = start_index.saturating_add(TYPE_LEN);
            let length_end_index = length_start_index.saturating_add(LENGTH_LEN);

            let value_start_index = length_end_index;
            let extension_type: ExtensionType = self.tlv_data[start_index].try_into()?;
            if extension_type == ExtensionType::Uninitialized {
                // Got to an uninitialized spot, write here
                self.tlv_data[start_index] = V::TYPE as u8;
                let length = &mut self.tlv_data[length_start_index..length_end_index];

                // maybe this becomes smarter later for dynamically sized extensions
                length.copy_from_slice(&(V::LEN as u32).to_le_bytes());

                let value_end_index = value_start_index.saturating_add(V::LEN);
                return Pack::pack(
                    extension,
                    &mut self.tlv_data[value_start_index..value_end_index],
                );
            } else {
                let length = u32::from_le_bytes(
                    self.tlv_data[length_start_index..length_end_index]
                        .try_into()
                        .unwrap(),
                ) as usize;
                let value_end_index = value_start_index.saturating_add(length);
                if extension_type == V::TYPE {
                    return Pack::pack(
                        extension,
                        &mut self.tlv_data[value_start_index..value_end_index],
                    );
                } else {
                    start_index = value_end_index;
                }
            }
        }
        Err(ProgramError::AccountDataTooSmall)
    }

    /// Write the account type into the buffer, done during the base
    /// state initialization
    /// Noop if no extensions are present
    pub fn pack_account_type(&mut self) {
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
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
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
impl TryFrom<u8> for AccountType {
    type Error = ProgramError;

    fn try_from(account_type: u8) -> Result<Self, Self::Error> {
        match account_type {
            0 => Ok(AccountType::Uninitialized),
            1 => Ok(AccountType::Mint),
            2 => Ok(AccountType::Account),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}

/// Extensions that can be applied to mints or accounts.  Mint extensions must only be
/// applied to mint accounts, and account extensions must only be applied to token holding
/// accounts.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
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
impl TryFrom<u8> for ExtensionType {
    type Error = ProgramError;

    fn try_from(extension_type: u8) -> Result<Self, Self::Error> {
        match extension_type {
            0 => Ok(ExtensionType::Uninitialized),
            1 => Ok(ExtensionType::MintTransferFee),
            2 => Ok(ExtensionType::AccountTransferFee),
            3 => Ok(ExtensionType::MintCloseAuthority),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}
impl ExtensionType {
    /// Get the data length of the type associated with the enum
    pub fn get_associated_type_len(&self) -> usize {
        match self {
            ExtensionType::Uninitialized => 0,
            ExtensionType::MintTransferFee => MintTransferFee::LEN,
            ExtensionType::AccountTransferFee => AccountTransferFee::LEN,
            ExtensionType::MintCloseAuthority => MintCloseAuthority::LEN,
        }
    }
}

/// Get the required account data length for the given ExtensionTypes
pub fn get_account_len(extension_types: &[ExtensionType]) -> usize {
    let extension_size: usize = extension_types
        .iter()
        .map(|e| {
            e.get_associated_type_len()
                .saturating_add(TYPE_LEN)
                .saturating_add(LENGTH_LEN)
        })
        .sum();
    let total_extension_size = if extension_size == Multisig::LEN {
        extension_size + 1
    } else {
        extension_size
    };
    total_extension_size
        .saturating_add(Account::LEN)
        .saturating_add(1) // for account type
}

/// Trait for base states, specifying the associated enum
pub trait BaseState {
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
    /// Associated account type enum, checked for compatibility when reading or
    /// writing extensions into the buffer
    const ACCOUNT_TYPE: AccountType;
}

/// Close authority extension data for mints.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MintCloseAuthority {
    /// Optional authority to close the mint
    pub close_authority: COption<Pubkey>,
}
impl Sealed for MintCloseAuthority {}
impl Pack for MintCloseAuthority {
    const LEN: usize = 36;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, 36];
        let close_authority = unpack_coption_key(src)?;
        Ok(MintCloseAuthority { close_authority })
    }
    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, 36];
        let &MintCloseAuthority {
            ref close_authority,
        } = self;
        pack_coption_key(close_authority, dst);
    }
}
impl Extension for MintCloseAuthority {
    const TYPE: ExtensionType = ExtensionType::MintCloseAuthority;
    const ACCOUNT_TYPE: AccountType = AccountType::Mint;
}

/// Transfer fee information
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransferFee {
    /// First epoch where the transfer fee takes effect
    pub epoch: Epoch,
    /// Amount of transfer collected as fees, expressed as basis points of the
    /// transfer amount, ie. increments of 0.01%
    pub transfer_fee_basis_points: u16,
    /// Maximum fee assessed on transfers, expressed as an amount of tokens
    pub maximum_fee: u64,
}

/// Transfer fee extension data for mints.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MintTransferFee {
    /// Optional authority to set the fee
    pub transfer_fee_config_authority: COption<Pubkey>,
    /// Withdraw from mint instructions must be signed by this key
    pub withheld_withdraw_authority: COption<Pubkey>,
    /// Withheld transfer fee tokens that have been moved to the mint for withdrawal
    pub withheld_amount: u64,
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
#[derive(Clone, Debug, Default, PartialEq)]
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
    fn mint_close_authority_pack_unpack() {
        let check = MintCloseAuthority {
            close_authority: COption::Some(Pubkey::new(&[1; 32])),
        };
        let mut packed = vec![0; MintCloseAuthority::get_packed_len()];
        Pack::pack(check.clone(), &mut packed).unwrap();
        let mut expect = vec![1, 0, 0, 0];
        expect.extend_from_slice(&[1; 32]);
        assert_eq!(packed, expect);
        let unpacked = MintCloseAuthority::unpack_from_slice(&packed).unwrap();
        assert_eq!(unpacked, check);
    }

    #[test]
    fn mint_with_extensions_pack_unpack() {
        let mint_size = get_account_len(&[ExtensionType::MintCloseAuthority]);
        let mut buffer = vec![0; mint_size];

        // fail unpack
        assert_eq!(
            BaseStateWithExtensions::<Mint>::unpack(&mut buffer),
            Err(ProgramError::UninitializedAccount),
        );

        let mut state = BaseStateWithExtensions::<Mint>::unpack_unchecked(&mut buffer).unwrap();
        // fail pack account extension, since this is a mint!
        assert_eq!(
            state.pack_extension(AccountTransferFee { withheld_amount: 0 }),
            Err(ProgramError::InvalidAccountData),
        );

        // success write extension
        let extension = MintCloseAuthority {
            close_authority: COption::Some(Pubkey::new(&[1; 32])),
        };
        state.pack_extension(extension.clone()).unwrap();

        // fail unpack again, still no base data
        assert_eq!(
            BaseStateWithExtensions::<Mint>::unpack(&mut buffer.clone()),
            Err(ProgramError::UninitializedAccount),
        );

        // write base mint
        let mut state = BaseStateWithExtensions::<Mint>::unpack_unchecked(&mut buffer).unwrap();
        let base = TEST_MINT;
        state.pack_base(base);
        assert_eq!(state.base, base);
        state.pack_account_type();

        // check raw buffer
        let mut expect = TEST_MINT_SLICE.to_vec();
        expect.extend_from_slice(&[0; Account::LEN - Mint::LEN]); // padding
        expect.push(AccountType::Mint as u8);
        expect.push(ExtensionType::MintCloseAuthority as u8);
        expect.extend_from_slice(&(MintCloseAuthority::LEN as u32).to_le_bytes());
        expect.extend_from_slice(&[1, 0, 0, 0]);
        expect.extend_from_slice(&[1; 32]);
        assert_eq!(expect, buffer);

        // check unpacking
        let mut state = BaseStateWithExtensions::<Mint>::unpack(&mut buffer).unwrap();
        assert_eq!(state.base, base);
        let unpacked_extension = state.unpack_extension::<MintCloseAuthority>().unwrap();
        assert_eq!(unpacked_extension, extension);

        // update base
        let mut new_base = TEST_MINT;
        new_base.supply += 100;
        state.pack_base(new_base);
        assert_eq!(state.base, new_base);

        // update extension
        let new_extension = MintCloseAuthority {
            close_authority: COption::Some(Pubkey::new(&[2; 32])),
        };
        state.pack_extension(new_extension.clone()).unwrap();

        // fail writing an additional extension
        assert_eq!(
            state.pack_extension(MintTransferFee::default()),
            Err(ProgramError::AccountDataTooSmall)
        );

        // check updates are propagated
        let state = BaseStateWithExtensions::<Mint>::unpack(&mut buffer).unwrap();
        assert_eq!(state.base, new_base);
        let unpacked_extension = state.unpack_extension::<MintCloseAuthority>().unwrap();
        assert_eq!(unpacked_extension, new_extension);

        // fail unpack as an account
        assert_eq!(
            BaseStateWithExtensions::<Account>::unpack(&mut buffer),
            Err(ProgramError::InvalidAccountData),
        );
    }
}
