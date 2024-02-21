//! Token-metadata interface state types

#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
use {
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    solana_program::{
        borsh1::{get_instance_packed_len, try_from_slice_unchecked},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_discriminator::{ArrayDiscriminator, SplDiscriminate},
    spl_pod::optional_keys::OptionalNonZeroPubkey,
    spl_type_length_value::{
        state::{TlvState, TlvStateBorrowed},
        variable_len_pack::VariableLenPack,
    },
};

/// Data struct for all token-metadata, stored in a TLV entry
///
/// The type and length parts must be handled by the TLV library, and not stored
/// as part of this struct.
#[derive(Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct TokenMetadata {
    /// The authority that can sign to update the metadata
    pub update_authority: OptionalNonZeroPubkey,
    /// The associated mint, used to counter spoofing to be sure that metadata
    /// belongs to a particular mint
    pub mint: Pubkey,
    /// The longer name of the token
    pub name: String,
    /// The shortened symbol for the token
    pub symbol: String,
    /// The URI pointing to richer metadata
    pub uri: String,
    /// Any additional metadata about the token as key-value pairs. The program
    /// must avoid storing the same key twice.
    pub additional_metadata: Vec<(String, String)>,
}
impl SplDiscriminate for TokenMetadata {
    /// Please use this discriminator in your program when matching
    const SPL_DISCRIMINATOR: ArrayDiscriminator =
        ArrayDiscriminator::new([112, 132, 90, 90, 11, 88, 157, 87]);
}
impl TokenMetadata {
    /// Gives the total size of this struct as a TLV entry in an account
    pub fn tlv_size_of(&self) -> Result<usize, ProgramError> {
        TlvStateBorrowed::get_base_len()
            .checked_add(get_instance_packed_len(self)?)
            .ok_or(ProgramError::InvalidAccountData)
    }

    /// Updates a field in the metadata struct
    pub fn update(&mut self, field: Field, value: String) {
        match field {
            Field::Name => self.name = value,
            Field::Symbol => self.symbol = value,
            Field::Uri => self.uri = value,
            Field::Key(key) => self.set_key_value(key, value),
        }
    }

    /// Sets a key-value pair in the additional metadata
    ///
    /// If the key is already present, overwrites the existing entry. Otherwise,
    /// adds it to the end.
    pub fn set_key_value(&mut self, new_key: String, new_value: String) {
        for (key, value) in self.additional_metadata.iter_mut() {
            if *key == new_key {
                value.replace_range(.., &new_value);
                return;
            }
        }
        self.additional_metadata.push((new_key, new_value));
    }

    /// Removes the key-value pair given by the provided key. Returns true if
    /// the key was found.
    pub fn remove_key(&mut self, key: &str) -> bool {
        let mut found_key = false;
        self.additional_metadata.retain(|x| {
            let should_retain = x.0 != key;
            if !should_retain {
                found_key = true;
            }
            should_retain
        });
        found_key
    }

    /// Get the slice corresponding to the given start and end range
    pub fn get_slice(data: &[u8], start: Option<u64>, end: Option<u64>) -> Option<&[u8]> {
        let start = start.unwrap_or(0) as usize;
        let end = end.map(|x| x as usize).unwrap_or(data.len());
        data.get(start..end)
    }
}
impl VariableLenPack for TokenMetadata {
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

/// Fields in the metadata account, used for updating
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum Field {
    /// The name field, corresponding to `TokenMetadata.name`
    Name,
    /// The symbol field, corresponding to `TokenMetadata.symbol`
    Symbol,
    /// The uri field, corresponding to `TokenMetadata.uri`
    Uri,
    /// A user field, whose key is given by the associated string
    Key(String),
}

#[cfg(test)]
mod tests {
    use {super::*, crate::NAMESPACE, solana_program::hash};

    #[test]
    fn discriminator() {
        let preimage = hash::hashv(&[format!("{NAMESPACE}:token_metadata").as_bytes()]);
        let discriminator =
            ArrayDiscriminator::try_from(&preimage.as_ref()[..ArrayDiscriminator::LENGTH]).unwrap();
        assert_eq!(TokenMetadata::SPL_DISCRIMINATOR, discriminator);
    }

    #[test]
    fn update() {
        let name = "name".to_string();
        let symbol = "symbol".to_string();
        let uri = "uri".to_string();
        let mut token_metadata = TokenMetadata {
            name,
            symbol,
            uri,
            ..Default::default()
        };

        // updating base fields
        let new_name = "new_name".to_string();
        token_metadata.update(Field::Name, new_name.clone());
        assert_eq!(token_metadata.name, new_name);

        let new_symbol = "new_symbol".to_string();
        token_metadata.update(Field::Symbol, new_symbol.clone());
        assert_eq!(token_metadata.symbol, new_symbol);

        let new_uri = "new_uri".to_string();
        token_metadata.update(Field::Uri, new_uri.clone());
        assert_eq!(token_metadata.uri, new_uri);

        // add new key-value pairs
        let key1 = "key1".to_string();
        let value1 = "value1".to_string();
        token_metadata.update(Field::Key(key1.clone()), value1.clone());
        assert_eq!(token_metadata.additional_metadata.len(), 1);
        assert_eq!(
            token_metadata.additional_metadata[0],
            (key1.clone(), value1.clone())
        );

        let key2 = "key2".to_string();
        let value2 = "value2".to_string();
        token_metadata.update(Field::Key(key2.clone()), value2.clone());
        assert_eq!(token_metadata.additional_metadata.len(), 2);
        assert_eq!(
            token_metadata.additional_metadata[0],
            (key1.clone(), value1)
        );
        assert_eq!(
            token_metadata.additional_metadata[1],
            (key2.clone(), value2.clone())
        );

        // update first key, see that order is preserved
        let new_value1 = "new_value1".to_string();
        token_metadata.update(Field::Key(key1.clone()), new_value1.clone());
        assert_eq!(token_metadata.additional_metadata.len(), 2);
        assert_eq!(token_metadata.additional_metadata[0], (key1, new_value1));
        assert_eq!(token_metadata.additional_metadata[1], (key2, value2));
    }

    #[test]
    fn remove_key() {
        let name = "name".to_string();
        let symbol = "symbol".to_string();
        let uri = "uri".to_string();
        let mut token_metadata = TokenMetadata {
            name,
            symbol,
            uri,
            ..Default::default()
        };

        // add new key-value pair
        let key = "key".to_string();
        let value = "value".to_string();
        token_metadata.update(Field::Key(key.clone()), value.clone());
        assert_eq!(token_metadata.additional_metadata.len(), 1);
        assert_eq!(token_metadata.additional_metadata[0], (key.clone(), value));

        // remove it
        assert!(token_metadata.remove_key(&key));
        assert_eq!(token_metadata.additional_metadata.len(), 0);

        // remove it again, returns false
        assert!(!token_metadata.remove_key(&key));
        assert_eq!(token_metadata.additional_metadata.len(), 0);
    }
}
