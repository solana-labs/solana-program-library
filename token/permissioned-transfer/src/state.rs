//! State transition types

use {
    crate::{
        pod::{PodAccountMeta, PodSliceMut},
        tlv::{Discriminator, Value},
    },
    solana_program::program_error::ProgramError,
};

/// State for all pubkeys required to validate a transfer
pub type ExtraAccountMetas<'a> = PodSliceMut<'a, PodAccountMeta>;

impl<'a> Value for ExtraAccountMetas<'a> {
    /// First 8 bytes of `hash::hashv(&["permissioned-transfer:validation-pubkeys"])`
    const TYPE: Discriminator = Discriminator::new([250, 175, 124, 64, 235, 120, 63, 195]);

    fn try_from_bytes(bytes: &[u8]) -> Result<&Self, ProgramError> {
        Self::unpack(bytes)
    }

    fn try_from_bytes_mut(bytes: &mut [u8]) -> Result<&mut Self, ProgramError> {
        Self::unpack(bytes)
    }

    fn initialize(&mut self) {
        self.initialize()
    }
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
