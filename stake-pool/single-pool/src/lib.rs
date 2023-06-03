#![deny(missing_docs)]

//! A program for liquid staking with a single validator

pub mod error;
pub mod instruction;
pub mod processor;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// export current sdk types for downstream users building with a different sdk version
pub use solana_program;
use solana_program::{pubkey::Pubkey, stake};

// XXX TODO FIXME change this
// (XXX ask how do we as a company handle privkeys for our onchain programs?)
solana_program::declare_id!("3cqnsMsT6LE96pxv7GR4di5rLqHDZZbR3FbeSUeRLFqY");

const POOL_STAKE_PREFIX: &[u8] = b"stake";
const POOL_MINT_PREFIX: &[u8] = b"mint";
const POOL_MINT_AUTHORITY_PREFIX: &[u8] = b"mint_authority";
const POOL_STAKE_AUTHORITY_PREFIX: &[u8] = b"stake_authority";
const POOL_MPL_AUTHORITY_PREFIX: &[u8] = b"mpl_authority";

const MINT_DECIMALS: u8 = 9;

const VOTE_STATE_DISCRIMINATOR_END: usize = 4;
const VOTE_STATE_AUTHORIZED_WITHDRAWER_START: usize = 36;
const VOTE_STATE_AUTHORIZED_WITHDRAWER_END: usize = 68;

fn find_address_and_bump(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
    prefix: &[u8],
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[prefix, vote_account_address.as_ref()], program_id)
}

fn find_pool_stake_address_and_bump(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
) -> (Pubkey, u8) {
    find_address_and_bump(program_id, vote_account_address, POOL_STAKE_PREFIX)
}

fn find_pool_mint_address_and_bump(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
) -> (Pubkey, u8) {
    find_address_and_bump(program_id, vote_account_address, POOL_MINT_PREFIX)
}

fn find_pool_stake_authority_address_and_bump(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
) -> (Pubkey, u8) {
    find_address_and_bump(
        program_id,
        vote_account_address,
        POOL_STAKE_AUTHORITY_PREFIX,
    )
}

fn find_pool_mint_authority_address_and_bump(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
) -> (Pubkey, u8) {
    find_address_and_bump(program_id, vote_account_address, POOL_MINT_AUTHORITY_PREFIX)
}

fn find_pool_mpl_authority_address_and_bump(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
) -> (Pubkey, u8) {
    find_address_and_bump(program_id, vote_account_address, POOL_MPL_AUTHORITY_PREFIX)
}

fn find_default_deposit_account_address_and_seed(
    vote_account_address: &Pubkey,
    user_wallet_address: &Pubkey,
) -> (Pubkey, String) {
    let vote_address_str = vote_account_address.to_string();
    let seed = format!("svsp{}", &vote_address_str[0..28]);
    let address =
        Pubkey::create_with_seed(user_wallet_address, &seed, &stake::program::id()).unwrap();

    (address, seed)
}

/// Find the canonical stake account address for a given vote account.
pub fn find_pool_stake_address(program_id: &Pubkey, vote_account_address: &Pubkey) -> Pubkey {
    find_pool_stake_address_and_bump(program_id, vote_account_address).0
}

/// Find the canonical token mint address for a given vote account.
pub fn find_pool_mint_address(program_id: &Pubkey, vote_account_address: &Pubkey) -> Pubkey {
    find_pool_mint_address_and_bump(program_id, vote_account_address).0
}

/// Find the canonical stake authority address for a given vote account.
pub fn find_pool_stake_authority_address(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
) -> Pubkey {
    find_pool_stake_authority_address_and_bump(program_id, vote_account_address).0
}

/// Find the canonical mint authority address for a given vote account.
pub fn find_pool_mint_authority_address(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
) -> Pubkey {
    find_pool_mint_authority_address_and_bump(program_id, vote_account_address).0
}

/// Find the canonical MPL authority address for a given vote account.
pub fn find_pool_mpl_authority_address(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
) -> Pubkey {
    find_pool_mpl_authority_address_and_bump(program_id, vote_account_address).0
}

/// Find the address of the default intermediate account that holds activating user stake before deposit.
pub fn find_default_deposit_account_address(
    vote_account_address: &Pubkey,
    user_wallet_address: &Pubkey,
) -> Pubkey {
    find_default_deposit_account_address_and_seed(vote_account_address, user_wallet_address).0
}
