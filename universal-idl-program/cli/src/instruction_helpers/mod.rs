use anyhow::Result;
use solana_program::pubkey::Pubkey;

pub mod common;
pub use common::*;

pub mod fetch;
pub use fetch::*;

pub mod set_idl_from_buffer;
pub use set_idl_from_buffer::*;

pub mod create_idl;
pub use create_idl::*;

pub mod write_buffer;
pub use write_buffer::*;

pub mod declare_frozen_authority;
pub use declare_frozen_authority::*;

pub mod buffer;
pub use buffer::*;

pub mod set_authority;
pub use set_authority::*;

pub mod close_account;
pub use close_account::*;

pub fn upgrade(
    overrides: CliOverrides,
    program_id: Pubkey,
    payer_filepath: &str,
    program_authority_filepath: &str,
    idl_filepath: &str,
) -> Result<()> {
    write_buffer(
        overrides.clone(),
        program_id,
        payer_filepath,
        program_authority_filepath,
        &idl_filepath,
    )?;
    set_buffer(
        overrides.clone(),
        program_id,
        payer_filepath,
        program_authority_filepath,
    )?;
    Ok(())
}
