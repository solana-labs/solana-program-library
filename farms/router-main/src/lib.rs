#![forbid(unsafe_code)]

pub mod add_farm;
pub mod add_fund;
pub mod add_pool;
pub mod add_token;
pub mod add_vault;
mod entrypoint;
pub mod processor;
mod refdb_init;
pub mod refdb_instruction;
pub mod remove_farm;
pub mod remove_fund;
pub mod remove_pool;
pub mod remove_token;
pub mod remove_vault;
pub mod set_admin_signers;
pub mod set_program_admin_signers;
pub mod set_program_single_authority;
pub mod upgrade_program;
