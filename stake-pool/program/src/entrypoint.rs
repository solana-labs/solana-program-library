//! Program entrypoint

#![cfg(all(target_os = "solana", not(feature = "no-entrypoint")))]

use {
    crate::{error::StakePoolError, processor::Processor},
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
        error.print::<StakePoolError>();
        Err(error)
    } else {
        Ok(())
    }
}

security_txt! {
    // Required fields
    name: "SPL Stake Pool",
    project_url: "https://spl.solana.com/stake-pool",
    contacts: "link:https://github.com/solana-labs/solana-program-library/security/advisories/new,mailto:security@solana.com,discord:https://solana.com/discord",
    policy: "https://github.com/solana-labs/solana-program-library/blob/master/SECURITY.md",

    // Optional Fields
    preferred_languages: "en",
    source_code: "https://github.com/solana-labs/solana-program-library/tree/master/stake-pool/program",
    source_revision: "b7dd8fee93815b486fce98d3d43d1d0934980226",
    source_release: "stake-pool-v1.0.0",
    auditors: "https://github.com/solana-labs/security-audits#stake-pool"
}
