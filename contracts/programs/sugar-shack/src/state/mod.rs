use anchor_lang::prelude::*;

#[account]
#[derive(Copy)]
pub struct MarketplaceProperties {
    // Address with admin authority to upgrade properties in this account
    pub authority: Pubkey,
    // The royalty percentage IN BASIS POINTS the marketplace will receive upon purchases through listings
    pub share: u16,
    pub bump: u8,
}

// 8 bytes for discriminator + 32 byte pubkey + 1 share + 1 bump
pub const MARKETPLACE_PROPERTIES_SIZE: usize = 8 + 32 + 2 + 1;
