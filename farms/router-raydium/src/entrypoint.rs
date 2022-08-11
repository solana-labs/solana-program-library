//! Program entrypoint

#![cfg(not(feature = "no-entrypoint"))]

solana_security_txt::security_txt! {
    name: "Solana Farms",
    project_url: "https://github.com/solana-labs/solana-program-library/tree/master/farms",
    contacts: "email:solana.farms@protonmail.com",
    policy: "",
    preferred_languages: "en",
    auditors: "Halborn"
}

use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};

entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    crate::processor::process_instruction(program_id, accounts, instruction_data)
}
