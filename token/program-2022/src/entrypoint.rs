//! Program entrypoint

use {
    crate::{error::TokenError, processor::Processor},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, program_error::PrintProgramError,
        pubkey::Pubkey,
    },
    solana_security_txt::security_txt,
};

solana_program::entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if let Err(error) = Processor::process(program_id, accounts, instruction_data) {
        // catch the error so we can print it
        error.print::<TokenError>();
        return Err(error);
    }
    Ok(())
}

security_txt! {
    // Required fields
    name: "SPL Token-2022",
    project_url: "https://spl.solana.com/token-2022",
    contacts: "link:https://github.com/solana-labs/solana-program-library/security/advisories/new,mailto:security@solana.com,discord:https://solana.com/discord",
    policy: "https://github.com/solana-labs/solana-program-library/blob/master/SECURITY.md",

    // Optional Fields
    preferred_languages: "en",
    source_code: "https://github.com/solana-labs/solana-program-library/tree/master/token/program-2022",
    source_revision: "15ebdb6440a4585a908ee3d91429561d64afebf6",
    source_release: "token-2022-v1.0.0",
    auditors: "https://github.com/solana-labs/security-audits#token-2022"
}
