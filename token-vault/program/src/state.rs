use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::pubkey::Pubkey,
};
/// prefix used for PDAs to avoid certain collision attacks (https://en.wikipedia.org/wiki/Collision_attack#Chosen-prefix_collision_attack)
pub const PREFIX: &str = "vault";

/// Used to tell front end clients that this struct is a ledger struct
pub const VAULT_KEY: u8 = 0;
pub const REGISTRY_KEY: u8 = 1;
pub const EXTERNAL_ACCOUNT_KEY: u8 = 2;

pub const MAX_TOKEN_REGISTRY_SIZE: usize = 1 + 32 + 32 + 32 + 100 + 1;
pub const MAX_VAULT_SIZE: usize = 1 + 32 + 32 + 32 + 1 + 32 + 1 + 32 + 1 + 1;

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum VaultState {
    Inactive,
    Active,
    Combined,
    Deactivated,
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct Vault {
    pub key: u8,
    /// Mint that produces the fractional shares
    pub fraction_mint: Pubkey,
    /// Authority who can make changes to the vault
    pub authority: Pubkey,
    /// treasury where fractional shares are held for redemption by authority
    pub fraction_treasury: Pubkey,
    /// treasury where monies are held for fractional share holders to redeem(burn) shares once buyout is made
    pub redeem_treasury: Pubkey,
    /// Can authority mint more shares from fraction_mint after activation
    pub allow_further_share_creation: bool,
    /// Hashed safety deposit boxes - after each addition of a token, we hash that token key
    /// combined with the current hash on the vault, making a 64 byte array out of [current_hashed, new_box]
    /// and hashing it with sha256 down to a new 32 byte array of u8s and saving it.
    /// We use this to guarantee you withdraw all your tokens later.
    pub hashed_safety_deposit_boxes: [u8; 32],

    /// Must point at an ExternalPriceAccount, which gives permission and price for buyout.
    pub pricing_lookup_address: Pubkey,
    pub token_type_count: u8,
    pub state: VaultState,
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct SafetyDepositBox {
    /// Each token type in a vault has it's own box that contains it's mint and a look-back
    pub key: u8,
    /// Key pointing to the parent vault
    pub vault: Pubkey,
    /// This particular token's mint
    pub token_mint: Pubkey,
    /// Account that stores the tokens under management
    pub safety_deposit_box: Pubkey,
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
    /// Whether or not combination has been allowed for this vault.
    pub allowed_to_combine: bool,
}
