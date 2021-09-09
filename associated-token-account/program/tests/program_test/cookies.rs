use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;

#[derive(Debug)]
pub struct MintCookie {
    pub address: Pubkey,
    pub mint_authority: Keypair,
}

#[derive(Debug)]
pub struct TokenAccountCookie {
    pub address: Pubkey,
}

#[derive(Debug)]
pub struct WalletCookie {
    pub address: Pubkey,
}
