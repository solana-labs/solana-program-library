//! Types for managing extra account meta keys that may be extracted from some
//! data.
//!
//! This can be either account data from some account in the list of accounts
//! or from the instruction data itself.

#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
use {crate::error::AccountResolutionError, solana_program::program_error::ProgramError};

/// Enum to describe a required key stored in some data.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
pub enum PubkeyData {
    /// Uninitialized configuration byte space.
    Uninitialized,
    /// A pubkey to be resolved from the instruction data.
    ///
    /// Packed as:
    ///     * 1 - Discriminator
    ///     * 1 - Start index of instruction data
    ///
    /// Note: Length is always 32 bytes.
    InstructionData {
        /// The index where the address bytes begin in the instruction data.
        index: u8,
    },
    /// A pubkey to be resolved from the inner data of some account.
    ///
    /// Packed as:
    ///     * 1 - Discriminator
    ///     * 1 - Index of account in accounts list
    ///     * 1 - Start index of account data
    ///
    /// Note: Length is always 32 bytes.
    AccountData {
        /// The index of the account in the entire accounts list.
        account_index: u8,
        /// The index where the address bytes begin in the account data.
        data_index: u8,
    },
}
impl PubkeyData {
    /// Get the size of a pubkey data configuration.
    pub fn tlv_size(&self) -> u8 {
        match self {
            Self::Uninitialized => 0,
            // 1 byte for the discriminator, 1 byte for the index.
            Self::InstructionData { .. } => 1 + 1,
            // 1 byte for the discriminator, 1 byte for the account index,
            // 1 byte for the data index.
            Self::AccountData { .. } => 1 + 1 + 1,
        }
    }

    /// Packs a pubkey data configuration into a slice.
    pub fn pack(&self, dst: &mut [u8]) -> Result<(), ProgramError> {
        // Because no `PubkeyData` variant is larger than 3 bytes, this check
        // is sufficient for the data length.
        if dst.len() != self.tlv_size() as usize {
            return Err(AccountResolutionError::NotEnoughBytesForPubkeyData.into());
        }
        match &self {
            Self::Uninitialized => {
                return Err(AccountResolutionError::InvalidPubkeyDataConfig.into())
            }
            Self::InstructionData { index } => {
                dst[0] = 1;
                dst[1] = *index;
            }
            Self::AccountData {
                account_index,
                data_index,
            } => {
                dst[0] = 2;
                dst[1] = *account_index;
                dst[2] = *data_index;
            }
        }
        Ok(())
    }

    /// Packs a pubkey data configuration into a 32-byte array, filling the
    /// rest with 0s.
    pub fn pack_into_address_config(key_data: &Self) -> Result<[u8; 32], ProgramError> {
        let mut packed = [0u8; 32];
        let tlv_size = key_data.tlv_size() as usize;
        key_data.pack(&mut packed[..tlv_size])?;
        Ok(packed)
    }

    /// Unpacks a pubkey data configuration from a slice.
    pub fn unpack(bytes: &[u8]) -> Result<Self, ProgramError> {
        let (discrim, rest) = bytes
            .split_first()
            .ok_or::<ProgramError>(ProgramError::InvalidAccountData)?;
        match discrim {
            0 => Ok(Self::Uninitialized),
            1 => {
                if rest.is_empty() {
                    return Err(AccountResolutionError::InvalidBytesForPubkeyData.into());
                }
                Ok(Self::InstructionData { index: rest[0] })
            }
            2 => {
                if rest.len() < 2 {
                    return Err(AccountResolutionError::InvalidBytesForPubkeyData.into());
                }
                Ok(Self::AccountData {
                    account_index: rest[0],
                    data_index: rest[1],
                })
            }
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack() {
        // Should fail if the length is too short.
        let key = PubkeyData::InstructionData { index: 0 };
        let mut packed = vec![0u8; key.tlv_size() as usize - 1];
        assert_eq!(
            key.pack(&mut packed).unwrap_err(),
            AccountResolutionError::NotEnoughBytesForPubkeyData.into(),
        );

        // Should fail if the length is too long.
        let key = PubkeyData::InstructionData { index: 0 };
        let mut packed = vec![0u8; key.tlv_size() as usize + 1];
        assert_eq!(
            key.pack(&mut packed).unwrap_err(),
            AccountResolutionError::NotEnoughBytesForPubkeyData.into(),
        );

        // Can't pack a `PubkeyData::Uninitialized`.
        let key = PubkeyData::Uninitialized;
        let mut packed = vec![0u8; key.tlv_size() as usize];
        assert_eq!(
            key.pack(&mut packed).unwrap_err(),
            AccountResolutionError::InvalidPubkeyDataConfig.into(),
        );
    }

    #[test]
    fn test_unpack() {
        // Can unpack zeroes.
        let zeroes = [0u8; 32];
        let key = PubkeyData::unpack(&zeroes).unwrap();
        assert_eq!(key, PubkeyData::Uninitialized);

        // Should fail for empty bytes.
        let bytes = [];
        assert_eq!(
            PubkeyData::unpack(&bytes).unwrap_err(),
            ProgramError::InvalidAccountData
        );
    }

    fn test_pack_unpack_key(key: PubkeyData) {
        let tlv_size = key.tlv_size() as usize;
        let mut packed = vec![0u8; tlv_size];
        key.pack(&mut packed).unwrap();
        let unpacked = PubkeyData::unpack(&packed).unwrap();
        assert_eq!(key, unpacked);
    }

    #[test]
    fn test_pack_unpack() {
        // Instruction data.
        test_pack_unpack_key(PubkeyData::InstructionData { index: 0 });

        // Account data.
        test_pack_unpack_key(PubkeyData::AccountData {
            account_index: 0,
            data_index: 0,
        });
    }
}
