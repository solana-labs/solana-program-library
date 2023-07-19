//! Token-edition interface state types

use {
    crate::error::TokenEditionsError,
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    solana_program::{
        borsh::{get_instance_packed_len, try_from_slice_unchecked},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_discriminator::SplDiscriminate,
    spl_type_length_value::{
        state::{TlvState, TlvStateBorrowed},
        variable_len_pack::VariableLenPack,
    },
};

/// A Pubkey that encodes `None` as all `0`, meant to be usable as a Pod type,
/// similar to all NonZero* number types from the bytemuck library.
#[derive(Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
#[repr(transparent)]
pub struct OptionalNonZeroPubkey(Pubkey);
impl TryFrom<Option<Pubkey>> for OptionalNonZeroPubkey {
    type Error = ProgramError;
    fn try_from(p: Option<Pubkey>) -> Result<Self, Self::Error> {
        match p {
            None => Ok(Self(Pubkey::default())),
            Some(pubkey) => {
                if pubkey == Pubkey::default() {
                    Err(ProgramError::InvalidArgument)
                } else {
                    Ok(Self(pubkey))
                }
            }
        }
    }
}
impl From<OptionalNonZeroPubkey> for Option<Pubkey> {
    fn from(p: OptionalNonZeroPubkey) -> Self {
        if p.0 == Pubkey::default() {
            None
        } else {
            Some(p.0)
        }
    }
}

/// Get the slice corresponding to the given start and end range
pub fn get_emit_slice(data: &[u8], start: Option<u64>, end: Option<u64>) -> Option<&[u8]> {
    let start = start.unwrap_or(0) as usize;
    let end = end.map(|x| x as usize).unwrap_or(data.len());
    data.get(start..end)
}

/// Data struct for an `Original` print
#[derive(
    Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema, SplDiscriminate,
)]
#[discriminator_hash_input("spl_token_editions_interface:print")]
pub struct Original {
    /// The authority that can sign to update the original print
    pub update_authority: OptionalNonZeroPubkey,
    /// The current supply of copies of this print
    pub supply: u64,
    /// The maximum supply of copies of this print
    pub max_supply: Option<u64>,
}
impl Original {
    /// Gives the total size of this struct as a TLV entry in an account
    pub fn tlv_size_of(&self) -> Result<usize, ProgramError> {
        TlvStateBorrowed::get_base_len()
            .checked_add(get_instance_packed_len(self)?)
            .ok_or(ProgramError::InvalidAccountData)
    }

    /// Creates a new `Original` print state
    pub fn new(update_authority: OptionalNonZeroPubkey, max_supply: Option<u64>) -> Self {
        Self {
            update_authority,
            supply: 0,
            max_supply,
        }
    }

    /// Updates the max supply for an original print
    pub fn update_max_supply(&mut self, max_supply: Option<u64>) -> Result<(), ProgramError> {
        // The new max supply cannot be less than the current supply
        if let Some(new_max_supply) = max_supply {
            if new_max_supply < self.supply {
                return Err(TokenEditionsError::SupplyExceedsNewMaxSupply.into());
            }
        } else if self.supply > 0 {
            return Err(TokenEditionsError::SupplyExceedsNewMaxSupply.into());
        }
        self.max_supply = max_supply;
        Ok(())
    }

    /// Updates the supply for an original print
    pub fn update_supply(&mut self, new_supply: u64) -> Result<(), ProgramError> {
        // The new supply cannot be greater than the max supply
        if let Some(max_supply) = self.max_supply {
            if new_supply > max_supply {
                return Err(TokenEditionsError::SupplyExceedsMaxSupply.into());
            }
        } else if new_supply > 0 {
            return Err(TokenEditionsError::SupplyExceedsMaxSupply.into());
        }
        self.supply = new_supply;
        Ok(())
    }
}
impl VariableLenPack for Original {
    fn pack_into_slice(&self, dst: &mut [u8]) -> Result<(), ProgramError> {
        borsh::to_writer(&mut dst[..], self).map_err(Into::into)
    }
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        try_from_slice_unchecked(src).map_err(Into::into)
    }
    fn get_packed_len(&self) -> Result<usize, ProgramError> {
        get_instance_packed_len(self).map_err(Into::into)
    }
}

/// Data struct for a `Reprint` of an `Original` print
#[derive(
    Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema, SplDiscriminate,
)]
#[discriminator_hash_input("spl_token_editions_interface:print")]
pub struct Reprint {
    /// The pubkey of the `Original` print
    pub original: Pubkey,
    /// The copy number of this `Reprint`
    pub copy: u64,
}
impl Reprint {
    /// Gives the total size of this struct as a TLV entry in an account
    pub fn tlv_size_of(&self) -> Result<usize, ProgramError> {
        TlvStateBorrowed::get_base_len()
            .checked_add(get_instance_packed_len(self)?)
            .ok_or(ProgramError::InvalidAccountData)
    }
}
impl VariableLenPack for Reprint {
    fn pack_into_slice(&self, dst: &mut [u8]) -> Result<(), ProgramError> {
        borsh::to_writer(&mut dst[..], self).map_err(Into::into)
    }
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        try_from_slice_unchecked(src).map_err(Into::into)
    }
    fn get_packed_len(&self) -> Result<usize, ProgramError> {
        get_instance_packed_len(self).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::NAMESPACE, solana_program::hash, spl_discriminator::ArrayDiscriminator};

    #[test]
    fn discriminators() {
        let preimage = hash::hashv(&[format!("{NAMESPACE}:print").as_bytes()]);
        let discriminator =
            ArrayDiscriminator::try_from(&preimage.as_ref()[..ArrayDiscriminator::LENGTH]).unwrap();
        assert_eq!(Original::SPL_DISCRIMINATOR, discriminator);
        assert_eq!(Reprint::SPL_DISCRIMINATOR, discriminator);
    }

    #[test]
    fn update_max_supply() {
        // Test with a `Some` max supply
        let max_supply = Some(10);
        let mut original_print = Original {
            max_supply,
            ..Default::default()
        };

        let new_max_supply = Some(30);
        original_print.update_max_supply(new_max_supply).unwrap();
        assert_eq!(original_print.max_supply, new_max_supply);

        // Change the current supply to 30
        original_print.supply = 30;

        // Try to set the max supply to 20, which is less than the current supply
        let new_max_supply = Some(20);
        assert_eq!(
            original_print.update_max_supply(new_max_supply),
            Err(ProgramError::from(
                TokenEditionsError::SupplyExceedsNewMaxSupply
            ))
        );

        // Test with a `None` max supply
        let max_supply = None;
        let mut original_print = Original {
            max_supply,
            ..Default::default()
        };

        let new_max_supply = Some(30);
        original_print.update_max_supply(new_max_supply).unwrap();
        assert_eq!(original_print.max_supply, new_max_supply);
    }
}
