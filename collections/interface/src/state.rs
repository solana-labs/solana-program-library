//! Token-collection interface state types

use {
    crate::error::TokenCollectionsError,
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

/// Data struct for a `Collection`
#[derive(
    Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema, SplDiscriminate,
)]
#[discriminator_hash_input("spl_token_collections_interface:collection")]
pub struct Collection {
    /// The authority that can sign to update the collection
    pub update_authority: OptionalNonZeroPubkey,
    /// The current number of collection members
    pub size: u64,
    /// The maximum number of collection members
    pub max_size: Option<u64>,
}
impl Collection {
    /// Gives the total size of this struct as a TLV entry in an account
    pub fn tlv_size_of(&self) -> Result<usize, ProgramError> {
        TlvStateBorrowed::get_base_len()
            .checked_add(get_instance_packed_len(self)?)
            .ok_or(ProgramError::InvalidAccountData)
    }

    /// Creates a new `Collection` state
    pub fn new(update_authority: OptionalNonZeroPubkey, max_size: Option<u64>) -> Self {
        Self {
            update_authority,
            size: 0,
            max_size,
        }
    }

    /// Updates the max size for a collection
    pub fn update_max_size(&mut self, max_size: Option<u64>) -> Result<(), ProgramError> {
        // The new max size cannot be less than the current size
        if let Some(new_max_size) = max_size {
            if new_max_size < self.size {
                return Err(TokenCollectionsError::SizeExceedsNewMaxSize.into());
            }
        }
        self.max_size = max_size;
        Ok(())
    }

    /// Updates the size for a collection
    pub fn update_size(&mut self, new_size: u64) -> Result<(), ProgramError> {
        // The new size cannot be greater than the max size
        if let Some(max_size) = self.max_size {
            if new_size > max_size {
                return Err(TokenCollectionsError::SizeExceedsMaxSize.into());
            }
        }
        self.size = new_size;
        Ok(())
    }
}
impl VariableLenPack for Collection {
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

/// Data struct for a `Member` of a `Collection`
#[derive(
    Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema, SplDiscriminate,
)]
#[discriminator_hash_input("spl_token_collections_interface:member")]
pub struct Member {
    /// The pubkey of the `Collection`
    pub collection: Pubkey,
}
impl Member {
    /// Gives the total size of this struct as a TLV entry in an account
    pub fn tlv_size_of(&self) -> Result<usize, ProgramError> {
        TlvStateBorrowed::get_base_len()
            .checked_add(get_instance_packed_len(self)?)
            .ok_or(ProgramError::InvalidAccountData)
    }
}
impl VariableLenPack for Member {
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
        let preimage = hash::hashv(&[format!("{NAMESPACE}:collection").as_bytes()]);
        let discriminator =
            ArrayDiscriminator::try_from(&preimage.as_ref()[..ArrayDiscriminator::LENGTH]).unwrap();
        assert_eq!(Collection::SPL_DISCRIMINATOR, discriminator);

        let preimage = hash::hashv(&[format!("{NAMESPACE}:member").as_bytes()]);
        let discriminator =
            ArrayDiscriminator::try_from(&preimage.as_ref()[..ArrayDiscriminator::LENGTH]).unwrap();
        assert_eq!(Member::SPL_DISCRIMINATOR, discriminator);
    }

    #[test]
    fn update_max_size() {
        // Test with a `Some` max size
        let max_size = Some(10);
        let mut collection = Collection {
            max_size,
            ..Default::default()
        };

        let new_max_size = Some(30);
        collection.update_max_size(new_max_size).unwrap();
        assert_eq!(collection.max_size, new_max_size);

        // Change the current size to 30
        collection.size = 30;

        // Try to set the max size to 20, which is less than the current size
        let new_max_size = Some(20);
        assert_eq!(
            collection.update_max_size(new_max_size),
            Err(ProgramError::from(
                TokenCollectionsError::SizeExceedsNewMaxSize
            ))
        );

        // Test with a `None` max size
        let max_size = None;
        let mut collection = Collection {
            max_size,
            ..Default::default()
        };

        let new_max_size = Some(30);
        collection.update_max_size(new_max_size).unwrap();
        assert_eq!(collection.max_size, new_max_size);
    }

    #[test]
    fn update_current_size() {
        let mut collection = Collection {
            max_size: Some(1),
            ..Default::default()
        };

        collection.update_size(1).unwrap();
        assert_eq!(collection.size, 1);

        // Try to set the current size to 2, which is greater than the max size
        assert_eq!(
            collection.update_size(2),
            Err(ProgramError::from(
                TokenCollectionsError::SizeExceedsMaxSize
            ))
        );

        // Test with a `None` max size
        let mut collection = Collection {
            max_size: None,
            ..Default::default()
        };

        collection.update_size(1).unwrap();
        assert_eq!(collection.size, 1);
    }
}
