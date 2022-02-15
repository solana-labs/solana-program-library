//! Program instructions

use {
    crate::{get_associated_token_address_with_program_id, id},
    assert_matches::assert_matches,
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    },
};

/// Instructions supported by the AssociatedTokenAccount program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum AssociatedTokenAccountInstruction {
    /// Creates an associated token account for the given wallet address and token mint
    /// Returns an error if the account exists.
    ///
    ///   0. `[writeable,signer]` Funding account (must be a system account)
    ///   1. `[writeable]` Associated token account address to be created
    ///   2. `[]` Wallet address for the new associated token account
    ///   3. `[]` The token mint for the new associated token account
    ///   4. `[]` System program
    ///   5. `[]` SPL Token program
    Create,
    /// Creates an associated token account for the given wallet address and token mint,
    /// if it doesn't already exist.  Returns an error if the account exists,
    /// but with a different owner.
    ///
    ///   0. `[writeable,signer]` Funding account (must be a system account)
    ///   1. `[writeable]` Associated token account address to be created
    ///   2. `[]` Wallet address for the new associated token account
    ///   3. `[]` The token mint for the new associated token account
    ///   4. `[]` System program
    ///   5. `[]` SPL Token program
    CreateIdempotent,
}

fn build_associated_token_account_instruction(
    funding_address: &Pubkey,
    wallet_address: &Pubkey,
    token_mint_address: &Pubkey,
    token_program_id: &Pubkey,
    instruction: AssociatedTokenAccountInstruction,
) -> Instruction {
    let associated_account_address = get_associated_token_address_with_program_id(
        wallet_address,
        token_mint_address,
        token_program_id,
    );
    // safety check, assert if not a creation instruction
    assert_matches!(
        instruction,
        AssociatedTokenAccountInstruction::Create
            | AssociatedTokenAccountInstruction::CreateIdempotent
    );
    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funding_address, true),
            AccountMeta::new(associated_account_address, false),
            AccountMeta::new_readonly(*wallet_address, false),
            AccountMeta::new_readonly(*token_mint_address, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(*token_program_id, false),
        ],
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates Create instruction
pub fn create_associated_token_account(
    funding_address: &Pubkey,
    wallet_address: &Pubkey,
    token_mint_address: &Pubkey,
    token_program_id: &Pubkey,
) -> Instruction {
    build_associated_token_account_instruction(
        funding_address,
        wallet_address,
        token_mint_address,
        token_program_id,
        AssociatedTokenAccountInstruction::Create,
    )
}

/// Creates CreateIdempotent instruction
pub fn create_associated_token_account_idempotent(
    funding_address: &Pubkey,
    wallet_address: &Pubkey,
    token_mint_address: &Pubkey,
    token_program_id: &Pubkey,
) -> Instruction {
    build_associated_token_account_instruction(
        funding_address,
        wallet_address,
        token_mint_address,
        token_program_id,
        AssociatedTokenAccountInstruction::CreateIdempotent,
    )
}
