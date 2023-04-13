//! State transition types

use {
    crate::{
        pod::{PodAccountMeta, PodSlice, PodSliceMut},
        tlv::{Discriminator, TlvType},
    },
    solana_program::program_error::ProgramError,
};

/// State for all pubkeys required to validate a transfer, accessed through a `PodSlice`
pub struct ExtraAccountMetas;
impl ExtraAccountMetas {
    /// Unpack a buffer with slice data as a pod slice
    pub fn unpack(data: &[u8]) -> Result<PodSlice<'_, PodAccountMeta>, ProgramError> {
        PodSlice::unpack(data)
    }
    /// Initialize pod slice data into the given buffer
    pub fn init(data: &mut [u8]) -> Result<PodSliceMut<'_, PodAccountMeta>, ProgramError> {
        PodSliceMut::unpack(data, /* init */ true)
    }
    /// Get the byte size required to hold `num_items` items
    pub fn byte_size_of(num_items: usize) -> Result<usize, ProgramError> {
        PodSlice::<PodAccountMeta>::byte_size_of(num_items)
    }
}

impl TlvType for ExtraAccountMetas {
    /// First 8 bytes of `hash::hashv(&["permissioned-transfer:validation-pubkeys"])`
    const TYPE: Discriminator = Discriminator::new([250, 175, 124, 64, 235, 120, 63, 195]);
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{DISCRIMINATOR_LENGTH, NAMESPACE},
        solana_program::hash,
    };

    #[test]
    fn discriminator() {
        let preimage = hash::hashv(&[format!("{NAMESPACE}:validation-pubkeys").as_bytes()]);
        let discriminator =
            Discriminator::try_from(&preimage.as_ref()[..DISCRIMINATOR_LENGTH]).unwrap();
        assert_eq!(discriminator, ExtraAccountMetas::TYPE);
    }
}
