use solana_program::pubkey::Pubkey;

use borsh::{BorshDeserialize, BorshSerialize};

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct PerpetualSwap {
    pub is_long_initialized: bool,
    pub is_short_initialized: bool,
    pub nonce: u8,
    pub token_program_id: Pubkey,
    pub long_margin_pubkey: Pubkey,
    pub long_account_pubkey: Pubkey,
    pub short_margin_pubkey: Pubkey,
    pub short_account_pubkey: Pubkey,
    pub reference_time: u128,
    pub index_price: f64,
    pub mark_price: f64,
    pub minimum_margin: u64,
    pub liquidation_threshold: f64,
    pub funding_rate: f64,
}

impl PerpetualSwap {
    pub const LEN: usize = 218;

    pub fn is_initialized(&self) -> bool {
        self.is_long_initialized && self.is_short_initialized
    }
}