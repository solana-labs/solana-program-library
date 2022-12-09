//! A program demonstrating how to register a token manager program
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod entrypoint;
pub mod processor;

pub use solana_program;
use solana_program::{
    declare_id,
    instruction::{AccountMeta, Instruction},
    pubkey::{Pubkey, PUBKEY_BYTES},
    system_program,
};

/// Generates the registration address for a mint
///
/// The registration address defines the program id to be used for the transfer
/// resolution
pub fn find_manager_registration_address(program_id: &Pubkey, mint_address: &Pubkey) -> Pubkey {
    find_manager_registration_address_internal(program_id, mint_address).0
}

pub(crate) fn find_manager_registration_address_internal(
    program_id: &Pubkey,
    mint_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[mint_address.as_ref()], program_id)
}

/// Create instruction to register the mint with a manager program id
pub fn create_register_instruction(
    program_id: &Pubkey,
    payer: &Pubkey,
    mint: &Pubkey,
    mint_authority: &Pubkey,
    manager_registration: &Pubkey,
    manager_program: &Pubkey,
) -> Instruction {
    Instruction::new_with_bytes(
        *program_id,
        &[],
        vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*mint_authority, true),
            AccountMeta::new(*manager_registration, false),
            AccountMeta::new_readonly(*manager_program, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    )
}

/// Create instruction to transfer tokens using the given management program
pub fn create_get_transfer_accounts_instruction(
    program_id: &Pubkey,
    mint: &Pubkey,
    unified_transfer: &Pubkey,
) -> Instruction {
    let instruction_bytes = vec![8u8]; // for the example, this is hard-coded from the managed token program
    Instruction::new_with_bytes(
        *program_id,
        &instruction_bytes,
        vec![
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*unified_transfer, false),
        ],
    )
}

/// Create instruction to transfer tokens using the given management program
pub fn create_unified_transfer_instruction(
    program_id: &Pubkey,
    source: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    owner: &Pubkey,
    amount: u64,
    additional_metas: &[AccountMeta],
) -> Instruction {
    let mut instruction_bytes = vec![9u8]; // for the example, this is hard-coded from
                                           // the managed token program
    instruction_bytes.extend_from_slice(&amount.to_le_bytes());
    let mut accounts = vec![
        AccountMeta::new(*source, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*destination, false),
        AccountMeta::new_readonly(*owner, true),
    ];
    accounts.extend_from_slice(additional_metas);
    Instruction::new_with_bytes(*program_id, &instruction_bytes, accounts)
}

/// Size of an encoded account meta in the return data
pub const ACCOUNT_META_BYTES: usize = 34;
const IS_SIGNER_BYTE: usize = 33;

/// Convert from the return data format to an account meta to be used in an instruction.
///
/// The format goes:
/// * 32 bytes for the pubkey
/// * 1 byte for is_writable, 0 == readonly
/// * 1 byte for is_signer, 0 == not signer
pub fn bytes_to_account_meta(bytes: &[u8]) -> AccountMeta {
    // return data is capped at 0, so fill it out
    let mut filled_bytes = vec![0u8; ACCOUNT_META_BYTES];
    filled_bytes[0..bytes.len()].copy_from_slice(bytes);
    let is_signer = filled_bytes[IS_SIGNER_BYTE] != 0;
    let pubkey = Pubkey::new(&filled_bytes[0..PUBKEY_BYTES]);
    if filled_bytes[PUBKEY_BYTES] == 0 {
        AccountMeta::new_readonly(pubkey, is_signer)
    } else {
        AccountMeta::new(pubkey, is_signer)
    }
}

declare_id!("TMreguGXkTM37TkytTJ4mQMgEBaYSBajFsuFFHL25DJ");
