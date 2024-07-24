use solana_zk_sdk::encryption::pod::grouped_elgamal::{
    PodGroupedElGamalCiphertext2Handles, PodGroupedElGamalCiphertext3Handles,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct PodTransferAmountCiphertext(pub(crate) PodGroupedElGamalCiphertext3Handles);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct PodFeeCiphertext(pub(crate) PodGroupedElGamalCiphertext2Handles);
