//! Instruction creators for the program
use {
    crate::{address::get_associated_token_address_with_program_id, program::id},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        system_program,
    },
};

fn build_associated_token_account_instruction(
    funding_address: &Pubkey,
    wallet_address: &Pubkey,
    token_mint_address: &Pubkey,
    token_program_id: &Pubkey,
    instruction: u8,
) -> Instruction {
    let associated_account_address = get_associated_token_address_with_program_id(
        wallet_address,
        token_mint_address,
        token_program_id,
    );
    // safety check, assert if not a creation instruction, which is only 0 or 1
    assert!(instruction <= 1);
    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funding_address, true),
            AccountMeta::new(associated_account_address, false),
            AccountMeta::new_readonly(*wallet_address, false),
            AccountMeta::new_readonly(*token_mint_address, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(*token_program_id, false),
        ],
        data: vec![instruction],
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
        0, // AssociatedTokenAccountInstruction::Create
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
        1, // AssociatedTokenAccountInstruction::CreateIdempotent
    )
}

/// Creates a `RecoverNested` instruction
pub fn recover_nested(
    wallet_address: &Pubkey,
    owner_token_mint_address: &Pubkey,
    nested_token_mint_address: &Pubkey,
    token_program_id: &Pubkey,
) -> Instruction {
    let owner_associated_account_address = get_associated_token_address_with_program_id(
        wallet_address,
        owner_token_mint_address,
        token_program_id,
    );
    let destination_associated_account_address = get_associated_token_address_with_program_id(
        wallet_address,
        nested_token_mint_address,
        token_program_id,
    );
    let nested_associated_account_address = get_associated_token_address_with_program_id(
        &owner_associated_account_address, // ATA is wrongly used as a wallet_address
        nested_token_mint_address,
        token_program_id,
    );

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(nested_associated_account_address, false),
            AccountMeta::new_readonly(*nested_token_mint_address, false),
            AccountMeta::new(destination_associated_account_address, false),
            AccountMeta::new_readonly(owner_associated_account_address, false),
            AccountMeta::new_readonly(*owner_token_mint_address, false),
            AccountMeta::new(*wallet_address, true),
            AccountMeta::new_readonly(*token_program_id, false),
        ],
        data: vec![2], // AssociatedTokenAccountInstruction::RecoverNested
    }
}
