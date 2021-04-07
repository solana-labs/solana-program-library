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

pub const MAX_TOKEN_REGISTRY_SIZE: usize = 1 + 32 + 32 + 32 + 100 + 1;
pub const MAX_POOL_SIZE: usize = 1 + 32 + 32 + 32 + 1 + 32 + 1 + 32 + 1 + 1;

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum PoolState {
    Inactive,
    Active,
    Combined,
    Deactivated,
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct FractionalizedTokenPool {
    pub key: u8,
    /// Mint that produces the fractional shares
    pub fraction_mint: Pubkey,
    /// Authority who can make changes to the token ledger
    pub authority: Pubkey,
    /// treasury where fractional shares are held for redemption by authority
    pub fraction_treasury: Pubkey,
    /// treasury where monies are held for fractional share holders to redeem(burn) shares once buyout is made
    pub redeem_treasury: Pubkey,
    /// Can authority mint more shares from fraction_mint after activation
    pub allow_further_share_creation: bool,
    /// Hashed fractionalized token registry lookup - after each addition of a token, we hash that token key
    /// combined with the current hash on the pool, making a 64 byte array out of [current_hashed, new_registry_key]
    /// and hashing it with sha256 down to a new 32 byte array of u8s and saving it.
    /// We use this to guarantee you withdraw all your tokens later.
    pub hashed_fractionalized_token_registry: [u8; 32],

    /// Must point at an ExternalPriceAccount, which gives permission and price for buyout.
    pub pricing_lookup_address: Pubkey,
    pub token_type_count: u8,
    pub state: PoolState,
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
    /// the order in the array of registries
    pub order: u8,
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct ExternalPriceAccount {
    pub key: u8,
    pub price_per_share: u64,
    /// Mint of the currency we are pricing the shares against, should be same as redeem_treasury.
    /// Most likely will be USDC mint most of the time.
    pub price_mint: Pubkey,
    /// Whether or not combination has been allowed for this pool.
    pub allowed_to_combine: bool,
}
