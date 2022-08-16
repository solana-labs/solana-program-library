use anchor_lang::prelude::*;

#[event]
pub struct NewLeafEvent {
    /// Public key of the merkle roll
    pub id: Pubkey,
    pub leaf: [u8; 32],
}
