//! Convention for associating token accounts with a primary account (such as a user wallet)

use crate::{instruction::*, *};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::rent::Rent,
};
use speedy::Writable;

/// Derives the associated SPL token address the `primary_account_address` and SPL token mint
///
/// This address can then be passed to `create_associated_token_account` or otherwise used as a
/// normal SPL token account address.
pub fn get_associated_token_address(
    primary_account_address: &Pubkey,
    spl_token_mint_address: &Pubkey,
) -> Pubkey {
    get_associated_address_and_bump_seed(
        primary_account_address,
        &spl_token::id(),
        &[&spl_token_mint_address],
    )
    .0
}

/// Create an associated token account
///
/// Important: `TokenInstruction::InitializeAccount` MUST be included as the next instruction in
/// the transaction that creates the associated token account using the returned instruction.
/// Otherwise another party can acquire ownership of the uninitialized associated account.
///
/// Accounts expected by this instruction:
///
///   0. `[writeable]` Associated address
///   1. `[]` Primary address of the associated account (typically a system account)
///   2. `[]` Address of program that will own the associated account
///   3. `[writeable,signer]` Funding account (must be a system account)
///   4. `[]` System program
///   5. `[]` The SPL token mint for the associated account
///
pub fn create_associated_token_account(
    primary_account_address: &Pubkey,
    spl_token_mint_address: &Pubkey,
    funding_address: &Pubkey,
    rent: &Rent,
) -> Instruction {
    let associated_account_address =
        get_associated_token_address(primary_account_address, spl_token_mint_address);
    let space = spl_token::state::Account::LEN;
    let lamports = rent.minimum_balance(space).max(1);
    let space = space as u64;

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(associated_account_address, false),
            AccountMeta::new_readonly(*primary_account_address, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(*funding_address, true),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(*spl_token_mint_address, false),
        ],
        data: InstructionData { lamports, space }.write_to_vec().unwrap(),
    }
}
