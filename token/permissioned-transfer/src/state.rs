//! State transition types

use {
    crate::tlv::{Discriminator, Value},
    bytemuck::{Pod, Zeroable},
    solana_program::pubkey::Pubkey,
};

const NUM_KEYS: usize = 3;
/// State for all pubkeys required to validate a transfer
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ValidationPubkeys {
    /// Number of `Pubkey` instances in the slice
    pub length: u16,
    /// Slice of required pubkeys to validate a transfer, along with the normal
    /// checked-transfer accounts and this account.
    pub pubkeys: [Pubkey; NUM_KEYS],
}

/// First 8 bytes of `hash::hashv(&["permissioned-transfer:validation-pubkeys"])`
impl Value for ValidationPubkeys {
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
        assert_eq!(discriminator, ValidationPubkeys::TYPE);
    }
}
