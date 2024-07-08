//! Convention for associating token accounts with a user wallet
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use solana_program::pubkey::Pubkey;

/// Derives the associated token account address and bump seed
/// for the given wallet address, token mint and token program id
pub fn get_associated_token_address_and_bump_seed(
    wallet_address: &Pubkey,
    token_mint_address: &Pubkey,
    program_id: &Pubkey,
    token_program_id: &Pubkey,
) -> (Pubkey, u8) {
    get_associated_token_address_and_bump_seed_internal(
        wallet_address,
        token_mint_address,
        program_id,
        token_program_id,
    )
}

const TOKEN_PROGRAM: Pubkey = solana_program::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const ASSOCIATED_TOKEN_PROGRAM: Pubkey = solana_program::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

/// Derives the associated token account address for the given wallet address
/// and token mint
pub fn get_associated_token_address(
    wallet_address: &Pubkey,
    token_mint_address: &Pubkey,
) -> Pubkey {
    get_associated_token_address_with_program_id(
        wallet_address,
        token_mint_address,
        &TOKEN_PROGRAM,
    )
}

/// Derives the associated token account address for the given wallet address,
/// token mint and token program id
pub fn get_associated_token_address_with_program_id(
    wallet_address: &Pubkey,
    token_mint_address: &Pubkey,
    token_program_id: &Pubkey,
) -> Pubkey {
    get_associated_token_address_and_bump_seed(
        wallet_address,
        token_mint_address,
        &ASSOCIATED_TOKEN_PROGRAM,
        token_program_id,
    )
    .0
}

fn get_associated_token_address_and_bump_seed_internal(
    wallet_address: &Pubkey,
    token_mint_address: &Pubkey,
    program_id: &Pubkey,
    token_program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            &wallet_address.to_bytes(),
            &token_program_id.to_bytes(),
            &token_mint_address.to_bytes(),
        ],
        program_id,
    )
}
