use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::pubkey::Pubkey,
};
/// prefix used for PDAs to avoid certain collision attacks (https://en.wikipedia.org/wiki/Collision_attack#Chosen-prefix_collision_attack)
pub const PREFIX: &str = "vault";

/// Used to tell front end clients that this struct is a ledger struct
pub const VAULT_KEY: u8 = 0;
pub const SAFETY_DEPOSIT_KEY: u8 = 1;
pub const EXTERNAL_ACCOUNT_KEY: u8 = 2;

pub const MAX_TOKEN_REGISTRY_SIZE: usize = 1 + 32 + 32 + 32 + 100 + 1;
pub const MAX_VAULT_SIZE: usize = 1 + 32 + 32 + 32 + 32 + 1 + 32 + 1 + 32 + 1 + 1 + 8;
pub const MAX_EXTERNAL_ACCOUNT_SIZE: usize = 1 + 8 + 32 + 1;
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
    /// Store token program used
    pub token_program: Pubkey,
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

    /// Must point at an ExternalPriceAccount, which gives permission and price for buyout.
    pub pricing_lookup_address: Pubkey,
    /// In inactive state, we use this to set the order key on Safety Deposit Boxes being added so
    /// the users know how to reconstruct the hashed_safety_deposit_boxes sha256 in the right order, and
    /// then we increment it and save so the next safety deposit box gets the next number.
    /// In the Combined state during token redemption by authority, we use it as a decrementing counter each time
    /// The authority of the vault withdrawals a Safety Deposit contents to count down how many
    /// are left to be opened and closed down. Once this hits zero, and the fraction mint has zero shares,
    /// then we can deactivate the vault.
    pub token_type_count: u8,
    pub state: VaultState,

    /// Once combination happens, we copy price per share to vault so that if something nefarious happens
    /// to external price account, like price change, we still have the math 'saved' for use in our calcs
    pub locked_price_per_share: u64,
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
    pub store: Pubkey,
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
