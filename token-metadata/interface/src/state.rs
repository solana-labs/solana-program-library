//! Token-metadata interface state types

use {
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    solana_program::{program_error::ProgramError, pubkey::Pubkey},
    spl_discriminator::{Discriminator, SplDiscriminator},
    spl_type_length_value::state::TlvDiscriminator,
    std::convert::TryFrom,
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
impl SplDiscriminator for TokenMetadata {
    /// Please use this discriminator in your program when matching
    const SPL_DISCRIMINATOR: Discriminator =
        Discriminator::new([112, 132, 90, 90, 11, 88, 157, 87]);
}
impl TlvDiscriminator for TokenMetadata {}

#[cfg(test)]
mod tests {
    use {super::*, crate::NAMESPACE, solana_program::hash};

    #[test]
    fn discriminator() {
        let preimage = hash::hashv(&[format!("{NAMESPACE}:token_metadata").as_bytes()]);
        let discriminator =
            Discriminator::try_from(&preimage.as_ref()[..Discriminator::LENGTH]).unwrap();
        assert_eq!(TokenMetadata::TLV_DISCRIMINATOR, discriminator);
    }
}
