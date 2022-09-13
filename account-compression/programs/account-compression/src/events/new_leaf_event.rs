use anchor_lang::prelude::*;

#[event]
pub struct NewLeafEvent {
    /// Public key of the concurrent merkle tree account
    pub id: Pubkey,
    /// Needed by off-chain indexers to track new data
    pub leaf: [u8; 32],
}
