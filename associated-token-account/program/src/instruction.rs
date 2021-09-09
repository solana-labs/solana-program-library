//! Program instructions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

use crate::{get_associated_token_address, id};

/// Instructions supported by the AssociatedTokenAccount program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum AssociatedTokenAccountInstruction {
    /// Creates an associated token account for the given wallet address and token mint
    ///
    ///   0. `[writeable,signer]` Funding account (must be a system account)
    ///   1. `[writeable]` Associated token account address to be created
    ///   2. `[]` Wallet address for the new associated token account
    ///   3. `[]` The token mint for the new associated token account
    ///   4. `[]` System program
    ///   5. `[]` SPL Token program
    ///   6. `[]` Rent sysvar
    CreateAssociatedTokenAccount,

    /// Mints tokens to an associated token account
    /// If the account doesn't exist then it'll be created
    MintTo {
        /// Amount to mint
        #[allow(dead_code)]
        amount: u64,
    },
}

/// Creates CreateAssociatedTokenAccount instruction
pub fn create_associated_token_account(
    // Accounts
    funding_address: &Pubkey,
    wallet_address: &Pubkey,
    spl_token_mint_address: &Pubkey,
) -> Instruction {
    let associated_account_address =
        get_associated_token_address(wallet_address, spl_token_mint_address);

    let instruction_data = AssociatedTokenAccountInstruction::CreateAssociatedTokenAccount {};

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funding_address, true),
            AccountMeta::new(associated_account_address, false),
            AccountMeta::new_readonly(*wallet_address, false),
            AccountMeta::new_readonly(*spl_token_mint_address, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: instruction_data.try_to_vec().unwrap(),
    }
}

/// Creates MintTo instruction
pub fn mint_to(
    // Accounts
    mint: &Pubkey,
    mint_authority: &Pubkey,
    wallet: &Pubkey,
    payer: &Pubkey,
    // Args
    amount: u64,
) -> Instruction {
    let associated_account_address = get_associated_token_address(wallet, mint);

    let instruction_data = AssociatedTokenAccountInstruction::MintTo { amount };

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*mint, false),
            AccountMeta::new_readonly(*mint_authority, true),
            AccountMeta::new_readonly(*wallet, false),
            AccountMeta::new(associated_account_address, false),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: instruction_data.try_to_vec().unwrap(),
    }
}
