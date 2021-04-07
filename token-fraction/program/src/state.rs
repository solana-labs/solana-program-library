use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::pubkey::Pubkey,
};
/// prefix used for PDAs to avoid certain collision attacks (https://en.wikipedia.org/wiki/Collision_attack#Chosen-prefix_collision_attack)
pub const PREFIX: &str = "fraction";

/// Used to tell front end clients that this struct is a ledger struct
pub const POOL_KEY: u8 = 0;
pub const REGISTRY_KEY: u8 = 1;
pub const EXTERNAL_ACCOUNT_KEY: u8 = 2;

pub const MAX_TOKEN_REGISTRY_SIZE: usize = 1 + 32 + 32 + 32 + 100;
pub const MAX_POOL_SIZE: usize = 1 + 32 + 32 + 32 + 1 + 32 + 1 + 32 + 1;

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub enum PricingLookupType {
    Dex,
    AMM,
    ExternalPriceAccount,
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct FractionalizedTokenPool {
    pub key: u8,
    /// Mint that produces the fractional shares
    pub fraction_mint: Pubkey,
    /// Authority who can make changes to the token ledger
    pub authority: Pubkey,
    /// treasury where monies are held for fractional share holders to redeem(burn) shares once buyout is made
    pub treasury: Pubkey,
    /// Can authority take more shares out of fraction_mint
    pub allow_share_redemption: bool,
    /// Hashed fractionalized token registry lookup - we take each registry key and use it as seed
    /// like [PREFIX, key1, key1, key1] and compare it to this to check to make sure you provide
    /// all keys in each relevant action call. We do this with SHA256.
    pub hashed_fractionalized_token_registry: [u8; 32],

    pub pricing_lookup_type: PricingLookupType,
    pub pricing_lookup_address: Pubkey,
    pub token_type_count: u8,
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct FractionalizedTokenRegistry {
    /// Each token type in a holding account has it's own ledger that contains it's mint and a look-back
    pub key: u8,
    /// Key pointing to the parent pool
    pub fractionalized_token_pool: Pubkey,
    /// This particular token's mint
    pub token_mint: Pubkey,
    /// Vault account that stores the tokens under management
    pub vault: Pubkey,
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct ExternalPriceAccount {
    key: u8,
    price_per_share: u64,
}
